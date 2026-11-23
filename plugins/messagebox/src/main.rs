use serde_json::{json, Value};
use std::collections::VecDeque;
use std::io::{self, BufRead, Read, Write};
use std::process::{Command, Stdio};

const MAX_LINE: usize = 64 * 1024;
const MAX_TEXT: usize = 2000;
const MAX_TITLE: usize = 100;
const HISTORY: usize = 10;

// The script is constant. User-controlled JSON is passed only through stdin.
const DISPLAY: &str = r#"[Console]::InputEncoding = [System.Text.Encoding]::UTF8
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices; public static class RchubDwm { [DllImport("dwmapi.dll")] public static extern int DwmSetWindowAttribute(IntPtr hwnd, int attr, ref int value, int size); [DllImport("user32.dll")] public static extern bool SetProcessDPIAware(); }'
[void][RchubDwm]::SetProcessDPIAware()
[System.Windows.Forms.Application]::EnableVisualStyles()
$d = [Console]::In.ReadToEnd() | ConvertFrom-Json
$k = [System.Drawing.Graphics]::FromHwnd([IntPtr]::Zero).DpiX / 96
function S($n) { [int][Math]::Round($n * $k) }
$bg = [System.Drawing.ColorTranslator]::FromHtml('#1f1f23')
$ink = [System.Drawing.ColorTranslator]::FromHtml('#f2f2f5')
$muted = [System.Drawing.ColorTranslator]::FromHtml('#a0a0aa')
$accent = [System.Drawing.ColorTranslator]::FromHtml('#1683f8')
$form = New-Object System.Windows.Forms.Form
$form.Text = $d.title
$form.TopMost = $true
$form.ShowInTaskbar = $true
$form.StartPosition = 'CenterScreen'
$form.BackColor = $bg
$form.ForeColor = $ink
$form.MinimizeBox = $false
$form.MaximizeBox = $false
$form.FormBorderStyle = 'FixedDialog'
$form.ClientSize = New-Object System.Drawing.Size((S 520), (S 240))
$form.Padding = New-Object System.Windows.Forms.Padding((S 28), (S 24), (S 28), (S 20))
$form.KeyPreview = $true
$form.Add_KeyDown({ if ($_.KeyCode -eq 'Escape' -or $_.KeyCode -eq 'Enter') { $form.Close() } })
$form.Add_HandleCreated({ $v = 1; [void][RchubDwm]::DwmSetWindowAttribute($form.Handle, 20, [ref]$v, 4); [void][RchubDwm]::DwmSetWindowAttribute($form.Handle, 19, [ref]$v, 4) })
$button = New-Object System.Windows.Forms.Button
$button.Text = if ($d.language -eq 'tr') { 'Tamam' } else { 'OK' }
$button.Dock = 'Bottom'
$button.Height = (S 44)
$button.FlatStyle = 'Flat'
$button.FlatAppearance.BorderSize = 0
$button.BackColor = $accent
$button.ForeColor = [System.Drawing.Color]::White
$button.Font = New-Object System.Drawing.Font('Segoe UI Semibold', 11)
$button.Cursor = [System.Windows.Forms.Cursors]::Hand
$button.Add_Click({ $form.Close() })
$gap = New-Object System.Windows.Forms.Panel
$gap.Dock = 'Bottom'
$gap.Height = (S 16)
$gap.BackColor = $bg
$panel = New-Object System.Windows.Forms.Panel
$panel.Dock = 'Fill'
$panel.AutoScroll = $true
$panel.BackColor = $bg
$label = New-Object System.Windows.Forms.Label
$label.AutoSize = $true
$label.MaximumSize = New-Object System.Drawing.Size((S 440), 0)
$label.Font = New-Object System.Drawing.Font('Segoe UI', 15)
$label.ForeColor = $ink
$label.BackColor = $bg
$label.Text = [string]$d.text
$panel.Controls.Add($label)
$form.Controls.Add($panel)
$need = $label.PreferredHeight + (S 24) + (S 16) + (S 44) + (S 20) + (S 24)
if ($d.seconds -gt 0) {
  $count = New-Object System.Windows.Forms.Label
  $count.Dock = 'Bottom'
  $count.Height = (S 26)
  $count.TextAlign = 'MiddleRight'
  $count.Font = New-Object System.Drawing.Font('Segoe UI', 9)
  $count.ForeColor = $muted
  $count.BackColor = $bg
  $unit = if ($d.language -eq 'tr') { ' sn' } else { ' s' }
  $count.Text = [string]$d.seconds + $unit
  $track = New-Object System.Windows.Forms.Panel
  $track.Dock = 'Bottom'
  $track.Height = (S 3)
  $track.BackColor = [System.Drawing.ColorTranslator]::FromHtml('#33333b')
  $bar = New-Object System.Windows.Forms.Panel
  $bar.Dock = 'Left'
  $bar.BackColor = $accent
  $track.Controls.Add($bar)
  $form.Controls.Add($count)
  $form.Controls.Add($track)
  $need += (S 29)
}
$form.Controls.Add($gap)
$form.Controls.Add($button)
$h = [Math]::Max((S 200), [Math]::Min((S 560), $need))
$form.ClientSize = New-Object System.Drawing.Size((S 520), $h)
if ($d.seconds -gt 0) {
  $total = [double]$d.seconds
  $end = [DateTime]::UtcNow.AddSeconds($total)
  $shown = [int]$d.seconds
  $bar.Width = $track.ClientSize.Width
  $timer = New-Object System.Windows.Forms.Timer
  $timer.Interval = 50
  $timer.Add_Tick({
    $remain = ($end - [DateTime]::UtcNow).TotalSeconds
    if ($remain -le 0) { $timer.Stop(); $form.Close(); return }
    $bar.Width = [int]($track.ClientSize.Width * $remain / $total)
    $n = [int][Math]::Ceiling($remain)
    if ($n -ne $shown) { $shown = $n; $count.Text = [string]$n + $unit }
  })
  $timer.Start()
}
$form.Add_Shown({ $form.Activate(); $button.Focus() })
[void]$form.ShowDialog()
"#;
const SPEAK: &str = r#"[Console]::InputEncoding = [System.Text.Encoding]::UTF8
$d = [Console]::In.ReadToEnd() | ConvertFrom-Json
Add-Type -AssemblyName System.Speech
$s = New-Object System.Speech.Synthesis.SpeechSynthesizer
$voice = $s.GetInstalledVoices() | Where-Object { $_.Enabled -and $_.VoiceInfo.Culture.TwoLetterISOLanguageName -eq $d.language } | Select-Object -First 1
if ($voice) { $s.SelectVoice($voice.VoiceInfo.Name) }
$s.Rate = $d.rate
$s.Volume = $d.volume
$s.Speak([string]$d.text)
$s.Dispose()
"#;
const SPEAK_GOOGLE: &str = r#"[Console]::InputEncoding = [System.Text.Encoding]::UTF8
$ErrorActionPreference = 'Stop'
$d = [Console]::In.ReadToEnd() | ConvertFrom-Json
Add-Type -AssemblyName PresentationCore
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$text = ([string]$d.text -replace '\s+', ' ').Trim()
$chunks = New-Object System.Collections.Generic.List[string]
while ($text.Length -gt 0) {
  if ($text.Length -le 180) { $chunks.Add($text); break }
  $cut = $text.LastIndexOfAny([char[]]'.!?;,: ', 179)
  if ($cut -lt 40) { $cut = 179 }
  $chunks.Add($text.Substring(0, $cut + 1).Trim())
  $text = $text.Substring($cut + 1).Trim()
}
$speed = [Math]::Pow(1.07, [double]$d.rate)
try {
foreach ($c in $chunks) {
  if (-not $c) { continue }
  $file = Join-Path ([IO.Path]::GetTempPath()) ('rchub-msg-' + [Guid]::NewGuid().ToString('N') + '.mp3')
  try {
    $url = 'https://translate.google.com/translate_tts?ie=UTF-8&client=tw-ob&tl=' + $d.language + '&q=' + [Uri]::EscapeDataString($c)
    Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $file -UserAgent 'Mozilla/5.0' -TimeoutSec 20
    $p = New-Object System.Windows.Media.MediaPlayer
    $p.Open([Uri]$file)
    $p.Volume = [double]$d.volume / 100
    $p.SpeedRatio = $speed
    $waited = 0
    while (-not $p.NaturalDuration.HasTimeSpan -and $waited -lt 5000) { Start-Sleep -Milliseconds 50; $waited += 50 }
    $p.Play()
    $length = if ($p.NaturalDuration.HasTimeSpan) { $p.NaturalDuration.TimeSpan.TotalSeconds / $speed + 0.4 } else { 6 }
    Start-Sleep -Milliseconds ([int]($length * 1000))
    $p.Close()
  } finally { Remove-Item -LiteralPath $file -ErrorAction SilentlyContinue }
}
} catch {
  Add-Type -AssemblyName System.Speech
  $s = New-Object System.Speech.Synthesis.SpeechSynthesizer
  $s.Volume = $d.volume
  $s.Speak([string]$d.text)
}
"#;

fn clean(value: &str, max: usize) -> Option<String> {
    if value.chars().count() > max { return None; }
    Some(value.chars().filter(|c| !c.is_control() || *c == '\n' || *c == '\t').collect())
}

fn launch(script: &str, data: &Value) -> io::Result<()> {
    use std::os::windows::process::CommandExt;
    let mut child = Command::new("powershell.exe")
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-STA", "-Command", script])
        .creation_flags(0x08000000)
        .stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn()?;
    let mut stdin = child.stdin.take().ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "stdin unavailable"))?;
    serde_json::to_writer(&mut stdin, data)?;
    drop(stdin);
    std::thread::spawn(move || { let _ = child.wait(); });
    Ok(())
}

fn response(id: &str, method: &str, params: &Value, history: &mut VecDeque<Value>) -> Value {
    let error = |code: &str, message: &str| json!({"v":2,"type":"response","id":id,"ok":false,"error":{"code":code,"message":message}});
    if !params.is_object() { return error("invalid_params", "params must be an object"); }
    let result = match method {
        "history" => json!({"messages": history}),
        "send" => {
            let Some(text) = params.get("text").and_then(Value::as_str).and_then(|s| clean(s, MAX_TEXT)).filter(|s| !s.trim().is_empty()) else { return error("invalid_params", "text must contain 1–2000 characters"); };
            let Some(title) = params.get("title").and_then(Value::as_str).and_then(|s| clean(s, MAX_TITLE)) else { return error("invalid_params", "title must be at most 100 characters"); };
            let Some(mode @ ("show" | "speak" | "both")) = params.get("mode").and_then(Value::as_str) else { return error("invalid_params", "invalid mode"); };
            let Some(seconds) = params.get("seconds").and_then(Value::as_u64).filter(|n| *n <= 300) else { return error("invalid_params", "seconds must be between 0 and 300"); };
            let Some(rate) = params.get("rate").and_then(Value::as_i64).filter(|n| (-10..=10).contains(n)) else { return error("invalid_params", "rate must be between -10 and 10"); };
            let Some(volume) = params.get("volume").and_then(Value::as_u64).filter(|n| *n <= 100) else { return error("invalid_params", "volume must be between 0 and 100"); };
            let Some(language @ ("tr" | "en")) = params.get("language").and_then(Value::as_str) else { return error("invalid_params", "invalid language"); };
            let engine = match params.get("engine").and_then(Value::as_str) { None | Some("system") => "system", Some("google") => "google", _ => return error("invalid_params", "invalid engine") };
            let data = json!({"text":text,"engine":engine,"title":if title.trim().is_empty() { "Message" } else { &title },"seconds":seconds,"rate":rate,"volume":volume,"language":language});
            if mode != "speak" && launch(DISPLAY, &data).is_err() { return error("launch_failed", "Could not start the message window"); }
            if mode != "show" && launch(if engine == "google" { SPEAK_GOOGLE } else { SPEAK }, &data).is_err() { return error("launch_failed", "Could not start speech; the window may already have opened"); }
            history.push_front(json!({"text":text,"title":title,"mode":mode}));
            history.truncate(HISTORY);
            json!({"accepted":true})
        },
        _ => return error("method_not_found", "Unknown method"),
    };
    json!({"v":2,"type":"response","id":id,"ok":true,"result":result})
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin(); let mut input = stdin.lock();
    let stdout = io::stdout(); let mut output = stdout.lock();
    let mut line = Vec::new(); let mut ready = false; let mut history = VecDeque::new();
    loop {
        line.clear();
        if input.by_ref().take((MAX_LINE + 1) as u64).read_until(b'\n', &mut line)? == 0 { return Err("host closed stdin".into()); }
        if line.len() > MAX_LINE || !line.ends_with(b"\n") { return Err("invalid protocol line".into()); }
        let message: Value = serde_json::from_slice(&line)?;
        if message.get("v").and_then(Value::as_u64) != Some(2) { return Err("unsupported protocol version".into()); }
        match (ready, message.get("type").and_then(Value::as_str)) {
            (false, Some("hello")) => { output.write_all(b"{\"v\":2,\"type\":\"ready\"}\n")?; output.flush()?; ready = true; },
            (true, Some("stop")) => return Ok(()),
            (true, Some("request")) => {
                let id = message.get("id").and_then(Value::as_str).filter(|s| !s.is_empty() && s.len() <= 128).ok_or("invalid id")?;
                let reply = response(id, message.get("method").and_then(Value::as_str).unwrap_or(""), message.get("params").unwrap_or(&Value::Null), &mut history);
                serde_json::to_writer(&mut output, &reply)?; output.write_all(b"\n")?; output.flush()?;
            },
            _ => return Err("unexpected protocol message".into()),
        }
    }
}
fn main() { if let Err(e) = run() { eprintln!("messagebox plugin: {e}"); std::process::exit(1); } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn cleaning_and_limits() {
        assert_eq!(clean("a\0b\nc\t", 20).unwrap(), "ab\nc\t");
        assert!(clean(&"ü".repeat(2001), 2000).is_none());
    }
    #[test] fn invalid_send_does_not_launch() {
        let mut h = VecDeque::new();
        let r = response("1", "send", &json!({"text":"\u{0000}","title":"","mode":"both","seconds":0,"rate":0,"volume":100,"language":"tr"}), &mut h);
        assert_eq!(r["ok"], false); assert!(h.is_empty());
    }
}
