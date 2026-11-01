//! Bounded, package-owned Speech message window.
//!
//! Acknowledgement means one thing only: the owned helper process created and
//! showed its message window. It is not audio completion. The helper keeps
//! playing for at most eight seconds and the window stays until the user closes
//! it with OK or the caption close button.
//!
//! Security shape:
//! - The helper is a fixed PowerShell 5 source block, streamed on stdin after one
//!   quoted, base64-encoded JSON data line. Guest text is decoded as JSON and never
//!   parsed as PowerShell; base64 cannot terminate the quoted literal.
//! - Capacity is reserved atomically before a helper is spawned, so concurrent
//!   requests can never exceed [`MAX_WINDOWS`] live windows.
//! - The child is assigned to a host-owned job object before it receives the
//!   payload, so a failed assignment is a hard error and never a detached,
//!   unowned process. Every ownership transfer is guarded: any failure after the
//!   window is visible kills and reaps the helper instead of leaking it.
//! - Visibility is confirmed against the exact owned child PID and a real,
//!   visible, non-empty top-level window. Nothing else counts as success.
//! - Host authentication and installed-grant checks precede every request.
//!   After acknowledgement, playback watches the protocol context lifetime.
//!   Shutdown, EOF and cancellation revoke it; helper jobs die with this process.
//!   This native executable is not a sandbox or a replacement for host checks.
//! - Revocation is a handshake, not a best-effort post: the helper must confirm
//!   on stdout that playback stopped. Without that confirmation the owned child
//!   is terminated as security cleanup. Normal playback completion never closes
//!   the window.
//! - Leaving the PC application terminates every helper through
//!   `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, including a crash.
//! - TopMost is an ordinary window style. It does not dismiss a UAC secure
//!   desktop, capture Alt+F4, hide Task Manager or make the window unclosable.

use serde_json::{json, Value};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// At most four live message windows, so helpers and their threads stay bounded.
const MAX_WINDOWS: usize = 4;
/// Window visibility must be proven within the existing ten second operation bound.
const VISIBLE_DEADLINE: Duration = Duration::from_secs(10);
const POLL: Duration = Duration::from_millis(25);
/// App-scoped private message; the helper stops playback, keeps the window, acks.
const STOP_MESSAGE: u32 = 0x8000 + 0x51;
const STOP_ACK: &str = "rchub:stopped";
const DONE_ACK: &str = "rchub:done";
const START_ACK: &str = "rchub:started";
const STOP_ACK_TIMEOUT: Duration = Duration::from_millis(500);
/// The helper proves it consumed the stream, so the tail write cannot block for long.
const WRITE_DEADLINE: Duration = Duration::from_secs(1);
const SPEECH_PERMISSION: &str = "speech.write";

/// Process-context lifetime supplied by the native protocol owner.
/// Authentication/grant decisions remain in the host before a request is sent;
/// this flag never impersonates a user, persisted grant, or host capability.
pub(crate) struct Authority {
    request: Arc<std::sync::atomic::AtomicBool>,
    context: Arc<std::sync::atomic::AtomicBool>,
}

impl Authority {
    pub(crate) fn new(
        request: Arc<std::sync::atomic::AtomicBool>,
        context: Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        Self { request, context }
    }
    fn authorized(&self) -> bool {
        self.request.load(Ordering::Acquire) && self.context.load(Ordering::Acquire)
    }
}

/// A panicking probe is a revoked probe; it must never unwind the supervisor.
fn ongoing(authority: &Authority) -> bool {
    catch_unwind(AssertUnwindSafe(|| authority.authorized())).unwrap_or(false)
}

/// Fixed helper source, streamed on stdin. Remote text arrives as a separate
/// base64 JSON line and is never executable code.
const SPEECH_HELPER: &str = r#"
$ErrorActionPreference = 'Stop'
try {
    Add-Type -AssemblyName System.Speech, System.Windows.Forms, System.Drawing
    Add-Type -ReferencedAssemblies System.Speech,System.Windows.Forms,System.Drawing,System.Core -TypeDefinition @'
using System;
using System.Drawing;
using System.Linq;
using System.Speech.Synthesis;
using System.Windows.Forms;

public static class RcHubSpeech {
    const int StopMessage = 0x8051;
    const int PlaybackLimitMs = 8000;

    // Legacy Google voice for short explicit-language messages; strictly bounded.
    // Any failure returns null and the caller falls back to local voices.
    public static byte[] Download(string text, string language) {
        if (text.Length > 200 || (language != "tr" && language != "en")) return null;
        try {
            var request = (System.Net.HttpWebRequest)System.Net.WebRequest.Create("https://translate.google.com/translate_tts?ie=UTF-8&client=tw-ob&tl=" + language + "&q=" + Uri.EscapeDataString(text));
            request.Timeout = 700;
            request.ReadWriteTimeout = 200;
            request.AllowAutoRedirect = false;
            request.UserAgent = "Mozilla/5.0";
            var elapsed = System.Diagnostics.Stopwatch.StartNew();
            using (var response = (System.Net.HttpWebResponse)request.GetResponse()) {
                if (response.StatusCode != System.Net.HttpStatusCode.OK || response.ContentLength > 262144) return null;
                using (var stream = response.GetResponseStream())
                using (var bytes = new System.IO.MemoryStream()) {
                    var buffer = new byte[4096];
                    int count;
                    while ((count = stream.Read(buffer, 0, buffer.Length)) > 0) {
                        if (elapsed.ElapsedMilliseconds > 1200 || bytes.Length + count > 262144) return null;
                        bytes.Write(buffer, 0, count);
                    }
                    return bytes.Length == 0 ? null : bytes.ToArray();
                }
            }
        } catch { return null; }
    }

    static readonly Color Surface = Color.FromArgb(0x0E, 0x12, 0x16);
    static readonly Color Raised = Color.FromArgb(0x14, 0x1A, 0x20);
    static readonly Color Edge = Color.FromArgb(0x1E, 0x25, 0x2C);
    static readonly Color Body = Color.FromArgb(0xF4, 0xF6, 0xF8);
    static readonly Color Muted = Color.FromArgb(0x9A, 0xA4, 0xB0);
    static readonly Color Accent = Color.FromArgb(0x3D, 0xDC, 0x84);
    static readonly Color AccentDeep = Color.FromArgb(0x1F, 0xA5, 0x5F);
    static readonly Color OnAccent = Color.FromArgb(0x07, 0x08, 0x0A);

    [System.Runtime.InteropServices.DllImport("user32.dll")]
    static extern bool SetProcessDpiAwarenessContext(IntPtr value);

    [System.Runtime.InteropServices.DllImport("dwmapi.dll")]
    static extern int DwmSetWindowAttribute(IntPtr window, int attribute, ref int value, int size);

    sealed class ActionButton : Button {
        public Color Fill = Color.Gray;
        public Color Hover = Color.Gray;
        public Color Ink = Color.Black;
        public int Ring = 2;
        bool hover;

        public ActionButton() {
            SetStyle(ControlStyles.UserPaint | ControlStyles.AllPaintingInWmPaint | ControlStyles.OptimizedDoubleBuffer | ControlStyles.ResizeRedraw, true);
            FlatStyle = FlatStyle.Flat;
            FlatAppearance.BorderSize = 0;
            Cursor = Cursors.Hand;
            TabStop = true;
        }

        protected override void OnMouseEnter(EventArgs e) {
            hover = true;
            Invalidate();
            base.OnMouseEnter(e);
        }

        protected override void OnMouseLeave(EventArgs e) {
            hover = false;
            Invalidate();
            base.OnMouseLeave(e);
        }

        protected override void OnGotFocus(EventArgs e) {
            Invalidate();
            base.OnGotFocus(e);
        }

        protected override void OnLostFocus(EventArgs e) {
            Invalidate();
            base.OnLostFocus(e);
        }

        protected override void OnPaint(PaintEventArgs e) {
            var graphics = e.Graphics;
            graphics.Clear(Parent == null ? SystemColors.Control : Parent.BackColor);
            using (var brush = new SolidBrush(hover ? Hover : Fill)) {
                graphics.FillRectangle(brush, ClientRectangle);
            }
            TextRenderer.DrawText(graphics, Text, Font, ClientRectangle, Ink,
                TextFormatFlags.HorizontalCenter | TextFormatFlags.VerticalCenter | TextFormatFlags.EndEllipsis);
            if (Focused) {
                using (var brush = new SolidBrush(Ink)) {
                    int width = ClientSize.Width;
                    int height = ClientSize.Height;
                    graphics.FillRectangle(brush, 0, 0, width, Ring);
                    graphics.FillRectangle(brush, 0, height - Ring, width, Ring);
                    graphics.FillRectangle(brush, 0, Ring, Ring, height - 2 * Ring);
                    graphics.FillRectangle(brush, width - Ring, Ring, Ring, height - 2 * Ring);
                }
            }
        }
    }

    sealed class MessageForm : Form {
        readonly string text;
        readonly string language;
        System.IO.Stream audio;
        readonly bool muted;
        SpeechSynthesizer voice;
        readonly Timer timer = new Timer();
        readonly Label status = new Label();
        readonly ActionButton ok = new ActionButton();
        readonly System.Diagnostics.Stopwatch elapsed = new System.Diagnostics.Stopwatch();
        System.Media.SoundPlayer player;
        bool speaking;
        bool stopped;
        bool stopRequested;
        bool highContrast;
        long audioDuration;
        float scale = 1f;

        public MessageForm(string text, string language, System.IO.Stream audio, bool muted) {
            this.text = text;
            this.language = language ?? "";
            this.audio = audio;
            this.muted = muted;
            highContrast = SystemInformation.HighContrast;
            using (var graphics = CreateGraphics()) scale = graphics.DpiX / 96f;
            AutoScaleMode = AutoScaleMode.Dpi;
            StartPosition = FormStartPosition.CenterScreen;
            TopMost = true;
            ShowInTaskbar = true;
            FormBorderStyle = FormBorderStyle.Sizable;
            MaximizeBox = false;
            MinimizeBox = false;
            ShowIcon = false;
            Text = language == "tr" ? "RCHub \u00B7 Mesaj" : "RCHub \u00B7 Message";
            AccessibleRole = AccessibleRole.Dialog;
            AccessibleName = Text;
            BackColor = highContrast ? SystemColors.Window : Surface;
            DoubleBuffered = true;
            Build();
        }

        int S(int pixels) { return (int)Math.Round(pixels * scale); }

        string Message(string tr, string en) { return language == "tr" ? tr : en; }

        void Build() {
            var root = new TableLayoutPanel();
            root.Dock = DockStyle.Fill;
            root.ColumnCount = 1;
            root.RowCount = 3;
            root.Padding = new Padding(S(20), S(16), S(20), S(18));
            root.BackColor = highContrast ? SystemColors.Window : Surface;
            root.RowStyles.Add(new RowStyle(SizeType.AutoSize));
            root.RowStyles.Add(new RowStyle(SizeType.Percent, 100f));
            root.RowStyles.Add(new RowStyle(SizeType.AutoSize));

            var eyebrow = new Label();
            eyebrow.AutoSize = true;
            eyebrow.Text = Message("RCHUB \u00B7 MESAJ", "RCHUB \u00B7 MESSAGE");
            eyebrow.Font = new Font("Segoe UI", 9.5f, FontStyle.Bold);
            eyebrow.ForeColor = highContrast ? SystemColors.WindowText : Accent;
            eyebrow.Margin = new Padding(0, 0, 0, S(10));
            root.Controls.Add(eyebrow, 0, 0);

            var frame = new Panel();
            frame.Dock = DockStyle.Fill;
            frame.Padding = new Padding(1);
            frame.BackColor = highContrast ? SystemColors.ControlDark : Edge;

            var inset = new Panel();
            inset.Dock = DockStyle.Fill;
            inset.Padding = new Padding(S(12));
            inset.BackColor = highContrast ? SystemColors.Window : Raised;

            var body = new RichTextBox();
            body.Dock = DockStyle.Fill;
            body.Text = text;
            body.ReadOnly = true;
            body.BorderStyle = BorderStyle.None;
            body.BackColor = highContrast ? SystemColors.Window : Raised;
            body.ForeColor = highContrast ? SystemColors.WindowText : Body;
            body.Font = new Font("Segoe UI", 12f, FontStyle.Regular);
            body.ScrollBars = RichTextBoxScrollBars.Vertical;
            body.WordWrap = true;
            body.DetectUrls = false;
            body.ShortcutsEnabled = true;
            body.TabStop = true;
            body.AccessibleName = Message("\u0130leti", "Message");
            inset.Controls.Add(body);
            frame.Controls.Add(inset);
            root.Controls.Add(frame, 0, 1);

            var footer = new TableLayoutPanel();
            footer.Dock = DockStyle.Fill;
            footer.ColumnCount = 2;
            footer.RowCount = 1;
            footer.Margin = new Padding(0, S(14), 0, 0);
            footer.BackColor = highContrast ? SystemColors.Window : Surface;
            footer.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100f));
            footer.ColumnStyles.Add(new ColumnStyle(SizeType.AutoSize));

            status.AutoSize = true;
            status.Anchor = AnchorStyles.Left;
            status.Text = Message("Haz\u0131r", "Ready");
            status.Font = new Font("Segoe UI", 9.5f, FontStyle.Regular);
            status.ForeColor = highContrast ? SystemColors.WindowText : Muted;
            status.Margin = new Padding(0);
            footer.Controls.Add(status, 0, 0);

            ok.Text = Message("Tamam", "OK");
            ok.AutoSize = false;
            ok.Size = new Size(S(104), S(38));
            ok.Anchor = AnchorStyles.Right;
            ok.Fill = highContrast ? SystemColors.Highlight : Accent;
            ok.Hover = highContrast ? SystemColors.Highlight : AccentDeep;
            ok.Ink = highContrast ? SystemColors.HighlightText : OnAccent;
            ok.Ring = S(2);
            ok.Font = new Font("Segoe UI", 10f, FontStyle.Bold);
            ok.AccessibleName = ok.Text;
            ok.Margin = new Padding(0);
            ok.Click += delegate { Close(); };
            footer.Controls.Add(ok, 1, 0);
            root.Controls.Add(footer, 0, 2);

            Controls.Add(root);
            ClientSize = new Size(S(560), S(360));
            MinimumSize = new Size(S(420), S(260));
            AcceptButton = ok;

            timer.Interval = 100;
            timer.Tick += delegate { Tick(); };
            Shown += delegate { ok.Focus(); SetStatus(Message("Ses haz\u0131rlan\u0131yor...", "Preparing audio...")); };
            FormClosed += delegate { Teardown(); };
        }

        protected override void OnHandleCreated(EventArgs e) {
            base.OnHandleCreated(e);
            if (highContrast) return;
            int enabled = 1;
            try {
                if (DwmSetWindowAttribute(Handle, 20, ref enabled, 4) != 0) {
                    enabled = 1;
                    DwmSetWindowAttribute(Handle, 19, ref enabled, 4);
                }
            } catch {
            }
        }

        protected override void WndProc(ref Message message) {
            if (message.Msg == StopMessage) {
                bool wasSpeaking = speaking;
                stopRequested = true;
                StopPlayback();
                if (wasSpeaking) SetStatus(Message("Ses durduruldu", "Playback stopped"));
                return;
            }
            base.WndProc(ref message);
        }

        void Post(Action action) {
            if (!IsHandleCreated || IsDisposed) return;
            try {
                BeginInvoke(action);
            } catch (ObjectDisposedException) {
            } catch (InvalidOperationException) {
            }
        }

        void SetStatus(string value) {
            if (IsDisposed) return;
            status.Text = value;
        }

        void Start() {
            if (stopped || IsDisposed) return;
            if (muted) {
                SetStatus(Message("Sessiz test", "Muted test"));
                return;
            }
            try {
            if (audio == null) {
                voice = new SpeechSynthesizer();
                var installed = language.Length == 0
                    ? null
                    : voice.GetInstalledVoices().FirstOrDefault(v =>
                        v.Enabled && v.VoiceInfo.Culture.TwoLetterISOLanguageName == language);
                if (language.Length != 0) {
                    if (installed == null) {
                        StopPlayback();
                        SetStatus(Message("Ses kullan\u0131lamad\u0131", "Audio unavailable"));
                        return;
                    }
                    voice.SelectVoice(installed.VoiceInfo.Name);
                }
            }
            speaking = true;
            Console.Out.WriteLine("rchub:started");
            Console.Out.Flush();
            // The eight second cap is anchored to playback start, not to form creation.
            elapsed.Start();
            timer.Start();
            SetStatus(Message("Seslendiriliyor...", "Speaking..."));
            if (audio == null) {
                voice.SpeakCompleted += delegate(object sender, SpeakCompletedEventArgs e) {
                    Post(delegate { Finish(e.Error != null); });
                };
                voice.SpeakAsync(text);
            } else {
                player = new System.Media.SoundPlayer(audio);
                player.Load();
                // SoundPlayer.Play returns on this UI thread: a queued worker
                // cannot begin playback after a stop/close acknowledgement.
                audioDuration = WaveDuration(audio);
                if (audioDuration <= 0) throw new InvalidOperationException("Invalid WAV");
                player.Play();
            }
            } catch { StopPlayback(); SetStatus(Message("Ses kullan\u0131lamad\u0131", "Audio unavailable")); }
        }

        static long WaveDuration(System.IO.Stream stream) {
            long position = stream.Position;
            try {
                stream.Position = 0;
                var reader = new System.IO.BinaryReader(stream);
                if (new string(reader.ReadChars(4)) != "RIFF") return 0;
                reader.ReadUInt32();
                if (new string(reader.ReadChars(4)) != "WAVE") return 0;
                uint rate = 0;
                while (stream.Position + 8 <= stream.Length) {
                    string id = new string(reader.ReadChars(4));
                    uint size = reader.ReadUInt32();
                    long next = stream.Position + size + (size % 2);
                    if (next > stream.Length) return 0;
                    if (id == "fmt " && size >= 16) {
                        reader.ReadUInt16(); reader.ReadUInt16(); reader.ReadUInt32();
                        rate = reader.ReadUInt32();
                    }
                    if (id == "data" && rate > 0) return Math.Max(1, (long)size * 1000 / rate);
                    stream.Position = next;
                }
                return 0;
            } finally { stream.Position = position; }
        }

        public void Deliver(System.IO.Stream prepared) {
            Post(delegate { if (!stopped) { audio = prepared; Start(); } });
        }

        void Finish(bool failed) {
            if (!speaking || stopped) return;
            speaking = false;
            stopped = true;
            timer.Stop();
            elapsed.Stop();
            SetStatus(failed
                ? Message("Ses kullan\u0131lamad\u0131", "Audio unavailable")
                : Message("Seslendirme bitti", "Playback finished"));
            Done();
        }

        void Done() {
            try { Console.Out.WriteLine("rchub:done"); Console.Out.Flush(); } catch { }
        }

        void Tick() {
            if (stopped) { ConfirmStopped(); return; }
            if (!speaking) return;
            if (player != null && elapsed.ElapsedMilliseconds >= audioDuration && audioDuration < PlaybackLimitMs) {
                player.Stop(); Finish(false); return;
            }
            if (elapsed.ElapsedMilliseconds < PlaybackLimitMs) return;
            StopPlayback();
            SetStatus(Message("Ses s\xFCresi s\u0131n\u0131r\u0131na ula\u015f\u0131ld\u0131", "Playback time limit reached"));
        }

        void StopPlayback() {
            speaking = false;
            stopped = true;
            timer.Stop();
            elapsed.Stop();
            try {
                if (voice != null) voice.SpeakAsyncCancelAll();
            } catch {
            }
            if (player != null) {
                try {
                    player.Stop();
                } catch {
                }
            }
            ConfirmStopped();
        }

        void ConfirmStopped() {
            // SAPI cancellation is asynchronous. An ack is evidence of Ready,
            // not merely that cancellation was requested. Host kills on timeout.
            if (voice != null && voice.State != SynthesizerState.Ready) {
                timer.Start(); return;
            }
            timer.Stop();
            if (stopRequested) {
                try { Console.Out.WriteLine("rchub:stopped"); Console.Out.Flush(); } catch { }
                stopRequested = false;
            }
            Done();
        }

        void Teardown() {
            timer.Stop();
            elapsed.Stop();
            StopPlayback();
            if (voice != null) voice.Dispose();
            if (player != null) player.Dispose();
            timer.Dispose();
        }
    }

    static MessageForm currentForm;
    static System.Threading.Thread ui;
    public static void Open(string text, string language, bool muted) {
        var ready = new System.Threading.ManualResetEventSlim(false);
        Exception failure = null;
        ui = new System.Threading.Thread(delegate() {
        try {
            SetProcessDpiAwarenessContext(new IntPtr(-4));
        } catch {
        }
        try {
            Application.EnableVisualStyles();
            using (var form = new MessageForm(text, language, null, muted)) {
                currentForm = form;
                form.Shown += delegate { ready.Set(); };
                Application.Run(form);
            }
        } catch (Exception error) { failure = error; }
        finally { ready.Set(); }
        });
        ui.SetApartmentState(System.Threading.ApartmentState.STA);
        ui.IsBackground = true;
        ui.Start();
        if (!ready.Wait(5000) || failure != null) throw new InvalidOperationException("Window unavailable", failure);
    }

    public static int Run(string text, string language, System.IO.Stream audio, bool muted) {
        currentForm.Deliver(audio);
        ui.Join();
        return 0;
    }
}
'@
    $audio = $null
    $synth = $null
    $speechStream = $null
    $googleInput = $null
    $googleOutput = $null
    $googleMemory = $null
    $muted = [bool]$payload.muted
    # Show before any network/voice preparation; native visibility is the ack.
    [RcHubSpeech]::Open([string]$payload.text, [string]$payload.language, $muted)
    try {
        # Explicitly selected short messages use the legacy Google voice first.
        # Legacy text-only requests stay local and never send their text online.
        if (!$muted -and $payload.language -and ([string]$payload.text).Length -le 200) {
            $mp3 = [RcHubSpeech]::Download([string]$payload.text, [string]$payload.language)
            if ($mp3) {
                try {
                    Add-Type -AssemblyName System.Runtime.WindowsRuntime
                    $null = [Windows.Media.Transcoding.MediaTranscoder,Windows.Media.Transcoding,ContentType=WindowsRuntime]
                    $null = [Windows.Media.MediaProperties.MediaEncodingProfile,Windows.Media.MediaProperties,ContentType=WindowsRuntime]
                    $null = [Windows.Storage.Streams.InMemoryRandomAccessStream,Windows.Storage.Streams,ContentType=WindowsRuntime]
                    $googleMemory = New-Object System.IO.MemoryStream(,$mp3)
                    $googleInput = [System.IO.WindowsRuntimeStreamExtensions]::AsRandomAccessStream($googleMemory)
                    $googleOutput = New-Object Windows.Storage.Streams.InMemoryRandomAccessStream
                    $transcoder = New-Object Windows.Media.Transcoding.MediaTranscoder
                    $profile = [Windows.Media.MediaProperties.MediaEncodingProfile]::CreateWav([Windows.Media.MediaProperties.AudioEncodingQuality]::Medium)
                    $operation = $transcoder.PrepareStreamTranscodeAsync($googleInput,$googleOutput,$profile)
                    $method = [System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.IsGenericMethod -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' } | Select-Object -First 1
                    $task = $method.MakeGenericMethod([Windows.Media.Transcoding.PrepareTranscodeResult]).Invoke($null,@($operation))
                    if (!$task.Wait(300)) { $operation.Cancel(); throw 'decode timeout' }
                    $prepared = $task.Result
                    if (!$prepared.CanTranscode) { throw 'unsupported audio' }
                    $operation = $prepared.TranscodeAsync()
                    $method = [System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.IsGenericMethod -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncActionWithProgress`1' } | Select-Object -First 1
                    $task = $method.MakeGenericMethod([double]).Invoke($null,@($operation))
                    if (!$task.Wait(300)) { $operation.Cancel(); throw 'decode timeout' }
                    if ($googleOutput.Size -gt 4194304) { throw 'audio too large' }
                    $googleOutput.Seek(0)
                    $audio = [System.IO.WindowsRuntimeStreamExtensions]::AsStreamForRead($googleOutput)
                } catch { $audio = $null }
            }
        }
        if (!$muted -and !$audio -and $payload.language) {
            # OneCore includes installed Windows language voices absent from legacy SAPI.
            # This only reads installed voices and synthesizes locally; no registry writes.
            try {
                Add-Type -AssemblyName System.Runtime.WindowsRuntime
                $null = [Windows.Media.SpeechSynthesis.SpeechSynthesizer,Windows.Media.SpeechSynthesis,ContentType=WindowsRuntime]
                $selected = [Windows.Media.SpeechSynthesis.SpeechSynthesizer]::AllVoices | Where-Object { $_.Language.Split('-')[0] -eq $payload.language } | Select-Object -First 1
                if ($selected) {
                    $synth = New-Object Windows.Media.SpeechSynthesis.SpeechSynthesizer
                    $synth.Voice = $selected
                    $operation = $synth.SynthesizeTextToStreamAsync([string]$payload.text)
                    $method = [System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.IsGenericMethod -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' } | Select-Object -First 1
                    $task = $method.MakeGenericMethod([Windows.Media.SpeechSynthesis.SpeechSynthesisStream]).Invoke($null,@($operation))
                    if (!$task.Wait(2000)) { $operation.Cancel(); throw 'synthesis timeout' }
                    $speechStream = $task.Result
                    $audio = [System.IO.WindowsRuntimeStreamExtensions]::AsStreamForRead($speechStream)
                }
            } catch { $audio = $null }
        }
        $result = [RcHubSpeech]::Run([string]$payload.text, [string]$payload.language, $audio, $muted)
    } finally {
        if ($audio) { $audio.Dispose() }
        if ($speechStream) { $speechStream.Dispose() }
        if ($synth) { $synth.Dispose() }
        if ($googleInput) { $googleInput.Dispose() }
        if ($googleOutput) { $googleOutput.Dispose() }
        if ($googleMemory) { $googleMemory.Dispose() }
    }
    exit $result
} catch { exit 1 }
"#;

/// One quoted base64 JSON data line followed by the fixed helper source.
///
/// Guest text only ever appears inside the base64 payload, which cannot contain a
/// quote, a newline or PowerShell syntax.
fn helper_stream(text: &str, language: Option<&str>, muted: bool) -> String {
    use base64::Engine;
    let payload = json!({"text": text, "language": language, "muted": muted}).to_string();
    debug_assert!(!payload.contains('\n') && !payload.contains('\r'));
    format!(
        "$__rchub = [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String('{}'))\n$payload = $__rchub | ConvertFrom-Json\n{}\n",
        base64::engine::general_purpose::STANDARD.encode(payload.as_bytes()),
        SPEECH_HELPER
    )
}

/// Validates the typed capability input and shows one message window.
pub(crate) fn invoke(
    cap: &str,
    input: Value,
    current: &dyn Fn() -> bool,
    authority: &Arc<Authority>,
) -> Result<Value, ()> {
    if cap != SPEECH_PERMISSION {
        return Err(());
    }
    let o = input.as_object().ok_or(())?;
    if !(1..=2).contains(&o.len()) || !o.keys().all(|k| matches!(k.as_str(), "text" | "language")) {
        return Err(());
    }
    let text = o
        .get("text")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty() && s.encode_utf16().count() <= 2000 && !s.contains('\0'))
        .ok_or(())?;
    let language = match o.get("language") {
        None => None,
        Some(Value::String(s)) if matches!(s.as_str(), "tr" | "en") => Some(s.as_str()),
        _ => return Err(()),
    };
    speak(text, language, current, authority, false)?;
    Ok(json!({"accepted":true}))
}

/// Crate-internal entry used by live validation harnesses. `muted` exists only for
/// disposable, muted runs: it shows the window with no voice, network or playback.
#[doc(hidden)]
pub(crate) fn speak(
    text: &str,
    language: Option<&str>,
    current: &dyn Fn() -> bool,
    authority: &Arc<Authority>,
    muted: bool,
) -> Result<(), ()> {
    speak_stream(helper_stream(text, language, muted), current, authority)
}

/// Owns a spawned helper until the supervisor takes over. Dropping it kills and
/// reaps the process, so no failure path can leak a visible window or audio.
struct OwnedChild {
    child: Option<std::process::Child>,
}

impl OwnedChild {
    fn new(child: std::process::Child) -> Self {
        Self { child: Some(child) }
    }

    fn pid(&self) -> u32 {
        self.child
            .as_ref()
            .map(std::process::Child::id)
            .unwrap_or(0)
    }

    fn inner(&mut self) -> Result<&mut std::process::Child, ()> {
        self.child.as_mut().ok_or(())
    }

    fn take_stdout(&mut self) -> Option<std::process::ChildStdout> {
        self.child.as_mut().and_then(|child| child.stdout.take())
    }
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Supervisor bookkeeping: live sessions plus capacity reserved by in-flight
/// spawns, so the limit is enforced before any concurrent spawn.
struct Supervisor {
    sessions: Vec<(u32, isize)>,
    reserved: usize,
}

static SUPERVISOR: OnceLock<Mutex<Supervisor>> = OnceLock::new();

fn supervisor() -> &'static Mutex<Supervisor> {
    SUPERVISOR.get_or_init(|| {
        Mutex::new(Supervisor {
            sessions: Vec::new(),
            reserved: 0,
        })
    })
}

/// Capacity reservation. `None` means the bounded window limit is already reached.
struct Slot {
    pid: Option<u32>,
}

impl Slot {
    fn reserve() -> Option<Self> {
        let mut state = supervisor().lock().ok()?;
        if state.sessions.len() + state.reserved >= MAX_WINDOWS {
            return None;
        }
        state.reserved += 1;
        Some(Self { pid: None })
    }

    /// Move the reservation into a live session. Fails if bookkeeping is poisoned.
    fn commit(&mut self, pid: u32, window: isize) -> bool {
        match supervisor().lock() {
            Ok(mut state) => {
                state.reserved = state.reserved.saturating_sub(1);
                state.sessions.push((pid, window));
                self.pid = Some(pid);
                true
            }
            Err(_) => false,
        }
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        let Ok(mut state) = supervisor().lock() else {
            return;
        };
        match self.pid {
            Some(pid) => {
                if let Some(index) = state.sessions.iter().position(|entry| entry.0 == pid) {
                    state.sessions.remove(index);
                }
            }
            None => state.reserved = state.reserved.saturating_sub(1),
        }
    }
}

/**
 * Live writer threads never exceed the window bound: one per in-flight helper,
 * released when the stream is written or the helper is killed.
 */
static WRITERS: AtomicUsize = AtomicUsize::new(0);

struct WriterSlot;

impl WriterSlot {
    fn acquire() -> Option<Self> {
        let current = WRITERS.fetch_add(1, Ordering::SeqCst);
        if current >= MAX_WINDOWS {
            WRITERS.fetch_sub(1, Ordering::SeqCst);
            return None;
        }
        Some(Self)
    }
}

impl Drop for WriterSlot {
    fn drop(&mut self) {
        WRITERS.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Helper stop acknowledgement, read from the owned child's stdout pipe.
struct AckReader {
    signal: Receiver<HelperEvent>,
}

#[derive(PartialEq, Eq)]
enum HelperEvent {
    Stopped,
    Done,
    Started,
}

impl AckReader {
    fn spawn(mut stdout: std::process::ChildStdout) -> Self {
        let (sender, signal) = mpsc::channel();
        std::thread::spawn(move || {
            use std::io::Read;
            let mut buffer = [0u8; 256];
            let mut parser = StopAckParser::default();
            let mut done = 0usize;
            let mut started = 0usize;
            loop {
                match stdout.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => {
                        if parser.feed(&buffer[..read]) {
                            let _ = sender.send(HelperEvent::Stopped);
                        }
                        for &byte in &buffer[..read] {
                            if byte == START_ACK.as_bytes()[started] {
                                started += 1;
                                if started == START_ACK.len() {
                                    let _ = sender.send(HelperEvent::Started);
                                    started = 0;
                                }
                            } else {
                                started = usize::from(byte == START_ACK.as_bytes()[0]);
                            }
                            if byte == DONE_ACK.as_bytes()[done] {
                                done += 1;
                                if done == DONE_ACK.len() {
                                    let _ = sender.send(HelperEvent::Done);
                                    done = 0;
                                }
                            } else {
                                done = usize::from(byte == DONE_ACK.as_bytes()[0]);
                            }
                        }
                    }
                }
            }
        });
        Self { signal }
    }

    /// Bounded wait; a late ack is buffered by the channel and still observed.
    fn wait(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            match self
                .signal
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            {
                Ok(HelperEvent::Stopped) => return true,
                Ok(HelperEvent::Done | HelperEvent::Started) => (),
                Err(_) => return false,
            }
        }
    }
}

/// Fixed-size streaming token state; pipe reads may split anywhere in the ack.
#[derive(Default)]
struct StopAckParser {
    matched: usize,
}
impl StopAckParser {
    fn feed(&mut self, bytes: &[u8]) -> bool {
        for &byte in bytes {
            if byte == STOP_ACK.as_bytes()[self.matched] {
                self.matched += 1;
                if self.matched == STOP_ACK.len() {
                    self.matched = 0;
                    return true;
                }
            } else {
                self.matched = usize::from(byte == STOP_ACK.as_bytes()[0]);
            }
        }
        false
    }
}

fn window_belongs_to(pid: u32, window: isize) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowThreadProcessId, IsWindowVisible};
    if pid == 0 || window == 0 {
        return false;
    }
    let mut owner = 0u32;
    unsafe { GetWindowThreadProcessId(window as _, &mut owner) };
    owner == pid && unsafe { IsWindowVisible(window as _) } != 0
}

/// The supervisor closure. Factored so a spawn failure (or the closure being
/// dropped before it runs) provably kills the child and releases the slot.
fn supervisor_job(
    owned: OwnedChild,
    pid: u32,
    window: isize,
    authority: Arc<Authority>,
    slot: Slot,
) -> impl FnOnce() + Send + 'static {
    move || supervise(owned, pid, window, authority, slot)
}

fn launch_supervisor(
    owned: OwnedChild,
    pid: u32,
    window: isize,
    authority: Arc<Authority>,
    slot: Slot,
) -> Result<(), ()> {
    let job = supervisor_job(owned, pid, window, authority, slot);
    std::thread::Builder::new()
        .name("rchub-speech-window".into())
        .spawn(job)
        .map(|_| ())
        .map_err(|_| ())
}

/// Ends an owned helper whose window is gone: an already exited child is simply
/// reaped, a still-running one is terminated so no audio survives the window.
fn close_owned(owned: &mut OwnedChild) {
    if let Ok(child) = owned.inner() {
        if matches!(child.try_wait(), Ok(None)) {
            let _ = child.kill();
        }
    }
}

fn supervise(
    mut owned: OwnedChild,
    pid: u32,
    window: isize,
    authority: Arc<Authority>,
    slot: Slot,
) {
    use windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW;
    let ack = owned.take_stdout().map(AckReader::spawn);
    let preparation = Instant::now();
    let mut playback = None;
    let mut decided = false;
    // Preparation may finish long after visibility. Keep the authority probe live
    // until stop is confirmed or the owned window/process exits, never a guessed
    // ten-second interval which could miss later playback.
    while !decided {
        let exited = match owned.inner().map(std::process::Child::try_wait) {
            Ok(Ok(Some(_))) | Ok(Err(_)) | Err(_) => true,
            Ok(Ok(None)) => false,
        };
        if exited {
            break;
        }
        if !window_belongs_to(pid, window) {
            // No visible owned window means nothing the user can close; never
            // leave audio running without a window. Same security-cleanup rule.
            close_owned(&mut owned);
            break;
        }
        let mut done = false;
        if let Some(reader) = &ack {
            for event in reader.signal.try_iter() {
                match event {
                    HelperEvent::Done => done = true,
                    HelperEvent::Started if playback.is_none() => playback = Some(Instant::now()),
                    _ => (),
                }
            }
        }
        if done {
            // The fixed helper made stop terminal before emitting done. It can
            // never start again, even if preparation completed after revocation.
            break;
        }
        // UI timer is the primary eight-second cap. This independent host
        // watchdog also stops/kills a stalled UI, including delayed preparation.
        let expired = playback
            .is_some_and(|start: Instant| start.elapsed() >= Duration::from_secs(8))
            || preparation.elapsed() >= Duration::from_secs(30);
        if !expired && ongoing(&authority) {
            std::thread::sleep(Duration::from_millis(100));
            continue;
        }
        // Revocation: stop playback and require the helper's own confirmation.
        // HWND reuse is rejected by re-checking the exact child PID first.
        if !window_belongs_to(pid, window) {
            close_owned(&mut owned);
            break;
        }
        let posted = unsafe { PostMessageW(window as _, STOP_MESSAGE, 0, 0) };
        let acknowledged =
            posted != 0 && ack.as_ref().is_some_and(|ack| ack.wait(STOP_ACK_TIMEOUT));
        if !acknowledged {
            // Security cleanup exception: no proven stop means the owned child dies.
            close_owned(&mut owned);
            break;
        }
        // Playback is proven stopped; the readable text window stays.
        decided = true;
    }
    // No further authorization work: block on the helper's own lifetime.
    let _ = owned.inner().map(std::process::Child::wait);
    drop(slot);
}

fn speak_stream(
    stream: String,
    current: &dyn Fn() -> bool,
    authority: &Arc<Authority>,
) -> Result<(), ()> {
    use std::{
        io::Write,
        os::windows::{io::AsRawHandle, process::CommandExt},
        process::{Command, Stdio},
    };
    use windows_sys::Win32::{
        Foundation::HANDLE,
        System::{JobObjects::AssignProcessToJobObject, Threading::CREATE_NO_WINDOW},
    };

    if !current() {
        return Err(());
    }
    // Reserve capacity before spawning so concurrent requests cannot overshoot.
    let mut slot = Slot::reserve().ok_or(())?;
    let job = job()?;
    if !current() {
        return Err(());
    }
    let system = std::env::var_os("SystemRoot").ok_or(())?;
    let executable =
        std::path::PathBuf::from(system).join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let spawned = Command::new(executable)
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-STA",
            "-Command",
            "-",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| ())?;
    let mut owned = OwnedChild::new(spawned);
    let pid = owned.pid();
    if !current() {
        return Err(());
    }
    // The helper blocks on stdin until the payload arrives, so ownership is proven
    // before it can create a window or touch audio. A failed assignment is fatal.
    {
        let child = owned.inner()?;
        if unsafe { AssignProcessToJobObject(job as HANDLE, child.as_raw_handle() as HANDLE) } == 0
        {
            return Err(());
        }
    }
    if !current() {
        return Err(());
    }
    let stdin = owned.inner()?.stdin.take().ok_or(())?;
    let writer_slot = WriterSlot::acquire().ok_or(())?;
    let mut writer = Some(
        std::thread::Builder::new()
            .name("rchub-speech-write".into())
            .spawn(move || {
                let _slot = writer_slot;
                let mut stdin = stdin;
                stdin.write_all(stream.as_bytes())
            })
            .map_err(|_| ())?,
    );

    let deadline = Instant::now() + VISIBLE_DEADLINE;
    let window = loop {
        if !current() || Instant::now() >= deadline {
            return Err(());
        }
        // A failed or broken pipe is a failed start, not a long wait.
        if writer
            .as_ref()
            .is_some_and(std::thread::JoinHandle::is_finished)
        {
            let written = writer.take().expect("writer handle").join();
            if !matches!(written, Ok(Ok(()))) {
                return Err(());
            }
        }
        match owned.inner()?.try_wait() {
            Ok(Some(_)) | Err(_) => return Err(()),
            Ok(None) => {}
        }
        if let Some(window) = find_visible_window(pid) {
            break window;
        }
        std::thread::sleep(POLL);
    };
    // The window proves the fixed script ran, so the writer can only be draining
    // its tail. Bound that drain and treat any write failure as a failed start.
    if let Some(handle) = writer.take() {
        let write_deadline = Instant::now() + WRITE_DEADLINE;
        let written = loop {
            if handle.is_finished() {
                break handle.join();
            }
            if Instant::now() >= write_deadline {
                return Err(());
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        if !matches!(written, Ok(Ok(()))) {
            return Err(());
        }
    }
    if !current() {
        return Err(());
    }
    if !slot.commit(pid, window) {
        return Err(());
    }
    launch_supervisor(owned, pid, window, Arc::clone(authority), slot)
}

/// Host-owned job object, created once. Closing the app (including a crash) closes
/// the handle and terminates every helper that was assigned to it.
fn job() -> Result<usize, ()> {
    use windows_sys::Win32::System::JobObjects::{
        CreateJobObjectW, JobObjectExtendedLimitInformation, SetInformationJobObject,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    static JOB: OnceLock<Option<usize>> = OnceLock::new();
    JOB.get_or_init(|| {
        let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if handle.is_null() {
            return None;
        }
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let applied = unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const core::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if applied == 0 {
            unsafe { windows_sys::Win32::Foundation::CloseHandle(handle) };
            None
        } else {
            Some(handle as usize)
        }
    })
    .as_ref()
    .copied()
    .ok_or(())
}

/// Exact owned-child visibility. A window of another process never satisfies it.
fn find_visible_window(pid: u32) -> Option<isize> {
    use windows_sys::Win32::{
        Foundation::{HWND, RECT},
        UI::WindowsAndMessaging::*,
    };
    if pid == 0 {
        return None;
    }
    unsafe extern "system" fn collect(window: HWND, data: isize) -> i32 {
        let target = &mut *(data as *mut (u32, Option<isize>));
        let mut owner = 0u32;
        unsafe { GetWindowThreadProcessId(window, &mut owner) };
        if owner != target.0 || unsafe { IsWindowVisible(window) } == 0 {
            return 1;
        }
        let mut title = [0u16; 128];
        if unsafe { GetWindowTextW(window, title.as_mut_ptr(), title.len() as i32) } <= 0 {
            return 1;
        }
        let mut rect: RECT = unsafe { std::mem::zeroed() };
        if unsafe { GetWindowRect(window, &mut rect) } == 0
            || rect.right <= rect.left
            || rect.bottom <= rect.top
        {
            return 1;
        }
        target.1 = Some(window as isize);
        0
    }
    let mut found = (pid, None);
    unsafe { EnumWindows(Some(collect), &mut found as *mut _ as isize) };
    found.1
}
