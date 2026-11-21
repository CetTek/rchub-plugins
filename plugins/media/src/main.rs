use std::io::{self, BufRead, Read, Write};

use serde_json::{json, Value};

const MAX_LINE: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    PlayPause,
    Next,
    Previous,
    VolumeGet,
    VolumeSet(u8),
    ToggleMute,
    Status,
    Seek(i64),
}

fn parse_action(method: &str, params: &Value) -> Result<Action, (&'static str, String)> {
    let object = params
        .as_object()
        .ok_or(("invalid_params", "params must be an object".to_string()))?;
    match method {
        "media.play_pause" if object.is_empty() => Ok(Action::PlayPause),
        "media.next" if object.is_empty() => Ok(Action::Next),
        "media.previous" if object.is_empty() => Ok(Action::Previous),
        "media.status" if object.is_empty() => Ok(Action::Status),
        "volume.get" if object.is_empty() => Ok(Action::VolumeGet),
        "volume.toggle_mute" if object.is_empty() => Ok(Action::ToggleMute),
        "volume.set" if object.len() == 1 => match object.get("level").and_then(Value::as_u64) {
            Some(level @ 0..=100) => Ok(Action::VolumeSet(level as u8)),
            _ => Err((
                "invalid_params",
                "level must be an integer from 0 to 100".to_string(),
            )),
        },
        "media.seek" if object.len() == 1 => match object.get("positionMs").and_then(Value::as_i64)
        {
            Some(position) if position >= 0 => Ok(Action::Seek(position)),
            _ => Err((
                "invalid_params",
                "positionMs must be a non-negative integer".to_string(),
            )),
        },
        "media.play_pause"
        | "media.next"
        | "media.previous"
        | "media.status"
        | "volume.get"
        | "volume.toggle_mute"
        | "volume.set"
        | "media.seek" => Err(("invalid_params", "Invalid parameters".to_string())),
        _ => Err(("method_not_found", "Unknown method".to_string())),
    }
}

fn unavailable_status() -> Value {
    json!({
        "available": false,
        "title": "",
        "artist": "",
        "album": "",
        "playing": false,
        "positionMs": 0,
        "durationMs": 0,
        "canSeek": false,
    })
}

fn execute(method: &str, params: &Value) -> Result<Value, (&'static str, String)> {
    let action = parse_action(method, params)?;
    platform::run(action)
}

const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encodes bytes as standard padded base64. Kept local so the plugin has no extra dependency.
fn base64_encode(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0] as u32;
        let second = chunk.get(1).copied().unwrap_or(0) as u32;
        let third = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (first << 16) | (second << 8) | third;
        encoded.push(BASE64_ALPHABET[(triple >> 18) as usize & 0x3f] as char);
        encoded.push(BASE64_ALPHABET[(triple >> 12) as usize & 0x3f] as char);
        if chunk.len() > 1 {
            encoded.push(BASE64_ALPHABET[(triple >> 6) as usize & 0x3f] as char);
        } else {
            encoded.push('=');
        }
        if chunk.len() > 2 {
            encoded.push(BASE64_ALPHABET[triple as usize & 0x3f] as char);
        } else {
            encoded.push('=');
        }
    }
    encoded
}

#[cfg(windows)]
mod platform {
    use std::sync::{Mutex, Once, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde_json::{json, Value};
    use windows::core::{Error as WindowsError, IUnknown, GUID};
    use windows::Media::Control::{
        GlobalSystemMediaTransportControlsSession,
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionMediaProperties,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    };
    use windows::Storage::Streams::DataReader;
    use windows::Win32::Foundation::BOOL;
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{
        eMultimedia, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        keybd_event, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE,
        VK_MEDIA_PREV_TRACK,
    };

    use super::{base64_encode, Action};

    type Failure = (&'static str, String);

    /// Largest thumbnail we are willing to inline. Base64 adds ~33%, keeping the response well
    /// below the 64 KiB relay envelope.
    const ARTWORK_MAX_BYTES: u64 = 32 * 1024;
    const UNIX_TO_FILETIME_100NS: i64 = 116_444_736_000_000_000;
    const TICKS_PER_MS: i64 = 10_000;

    fn windows_error(error: WindowsError) -> Failure {
        ("platform_error", error.to_string())
    }

    fn initialize() {
        static INITIALIZE: Once = Once::new();
        INITIALIZE.call_once(|| unsafe {
            // WinRT activation routes through COM. A multithreaded apartment lets the blocking
            // IAsyncOperation::get() calls below complete without a message pump.
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        });
    }

    fn session_manager() -> Result<GlobalSystemMediaTransportControlsSessionManager, Failure> {
        static MANAGER: OnceLock<GlobalSystemMediaTransportControlsSessionManager> = OnceLock::new();
        if let Some(manager) = MANAGER.get() {
            return Ok(manager.clone());
        }
        initialize();
        let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
            .and_then(|operation| operation.get())
            .map_err(windows_error)?;
        Ok(MANAGER.get_or_init(|| manager).clone())
    }

    fn current_session() -> Result<Option<GlobalSystemMediaTransportControlsSession>, Failure> {
        let manager = session_manager()?;
        // An empty session list surfaces as a null object, which windows-rs reports as an error.
        Ok(manager.GetCurrentSession().ok())
    }

    fn filetime_now() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| (elapsed.as_nanos() as i64) / 100 + UNIX_TO_FILETIME_100NS)
            .unwrap_or(0)
    }

    fn artwork_data_url(
        properties: &GlobalSystemMediaTransportControlsSessionMediaProperties,
    ) -> Option<String> {
        let reference = properties.Thumbnail().ok()?;
        let stream = reference.OpenReadAsync().ok()?.get().ok()?;
        let size = stream.Size().ok()?;
        if size == 0 || size > ARTWORK_MAX_BYTES {
            return None;
        }
        let content_type = stream.ContentType().map(|value| value.to_string()).ok()?;
        let content_type = if content_type.starts_with("image/") && content_type.len() <= 64 {
            content_type
        } else {
            "image/png".to_string()
        };
        let input = stream.GetInputStreamAt(0).ok()?;
        let reader = DataReader::CreateDataReader(&input).ok()?;
        let loaded = reader.LoadAsync(size as u32).ok()?.get().ok()?;
        if loaded == 0 {
            return None;
        }
        let mut bytes = vec![0u8; loaded as usize];
        reader.ReadBytes(&mut bytes).ok()?;
        Some(format!("data:{content_type};base64,{}", base64_encode(&bytes)))
    }

    struct CachedArtwork {
        key: String,
        data_url: Option<String>,
    }

    /// The UI polls about once per second, so avoid re-reading the same thumbnail on every poll.
    static ARTWORK_CACHE: Mutex<Option<CachedArtwork>> = Mutex::new(None);

    fn artwork_for(
        key: String,
        properties: &GlobalSystemMediaTransportControlsSessionMediaProperties,
    ) -> Option<String> {
        if let Ok(cache) = ARTWORK_CACHE.lock() {
            if let Some(cached) = cache.as_ref() {
                if cached.key == key {
                    return cached.data_url.clone();
                }
            }
        }
        let data_url = artwork_data_url(properties);
        if let Ok(mut cache) = ARTWORK_CACHE.lock() {
            *cache = Some(CachedArtwork {
                key,
                data_url: data_url.clone(),
            });
        }
        data_url
    }

    fn status() -> Result<Value, Failure> {
        let Some(session) = current_session()? else {
            return Ok(super::unavailable_status());
        };
        let properties = session
            .TryGetMediaPropertiesAsync()
            .and_then(|operation| operation.get())
            .map_err(windows_error)?;
        let playback = session.GetPlaybackInfo().map_err(windows_error)?;
        let playing = matches!(
            playback.PlaybackStatus(),
            Ok(status) if status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing
        );
        let position_control = playback
            .Controls()
            .ok()
            .and_then(|controls| controls.IsPlaybackPositionEnabled().ok())
            .unwrap_or(false);

        let (position_ms, duration_ms, can_seek) = match session.GetTimelineProperties() {
            Ok(timeline) => {
                let start = timeline.StartTime().map(|value| value.Duration).unwrap_or(0);
                let end = timeline.EndTime().map(|value| value.Duration).unwrap_or(0);
                let mut position = timeline.Position().map(|value| value.Duration).unwrap_or(0);
                if playing {
                    if let Ok(updated) = timeline.LastUpdatedTime() {
                        let now = filetime_now();
                        if updated.UniversalTime > 0 && now > updated.UniversalTime {
                            position = position.saturating_add(now - updated.UniversalTime);
                        }
                    }
                }
                let duration = end.saturating_sub(start);
                let position = if duration > 0 {
                    position.saturating_sub(start).clamp(0, duration)
                } else {
                    position.saturating_sub(start).max(0)
                };
                (
                    position / TICKS_PER_MS,
                    duration / TICKS_PER_MS,
                    position_control && duration > 0,
                )
            }
            Err(_) => (0, 0, false),
        };

        let title = properties
            .Title()
            .map(|value| value.to_string())
            .unwrap_or_default();
        let album = properties
            .AlbumTitle()
            .map(|value| value.to_string())
            .unwrap_or_default();
        let artist = properties
            .Artist()
            .ok()
            .map(|value| value.to_string())
            .filter(|value| !value.is_empty())
            .or_else(|| {
                properties
                    .AlbumArtist()
                    .ok()
                    .map(|value| value.to_string())
                    .filter(|value| !value.is_empty())
            })
            .unwrap_or_default();

        let mut result = json!({
            "available": true,
            "title": title,
            "artist": artist,
            "album": album,
            "playing": playing,
            "positionMs": position_ms,
            "durationMs": duration_ms,
            "canSeek": can_seek,
        });
        if let Ok(source) = session.SourceAppUserModelId() {
            let source = source.to_string();
            if !source.is_empty() {
                result["sourceApp"] = Value::String(source);
            }
        }
        if let Some(artwork) = artwork_for(
            format!("{title}\u{1f}{artist}\u{1f}{album}\u{1f}{duration_ms}"),
            &properties,
        ) {
            result["artworkDataUrl"] = Value::String(artwork);
        }
        Ok(result)
    }

    fn endpoint_volume() -> Result<IAudioEndpointVolume, Failure> {
        initialize();
        unsafe {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None::<&IUnknown>, CLSCTX_ALL)
                    .map_err(windows_error)?;
            let device = enumerator
                .GetDefaultAudioEndpoint(eRender, eMultimedia)
                .map_err(windows_error)?;
            device
                .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
                .map_err(windows_error)
        }
    }

    fn volume_response(endpoint: &IAudioEndpointVolume) -> Result<Value, Failure> {
        unsafe {
            let level = endpoint
                .GetMasterVolumeLevelScalar()
                .map_err(windows_error)?;
            let muted: BOOL = endpoint.GetMute().map_err(windows_error)?;
            let percent = ((level * 100.0).round() as i64).clamp(0, 100) as u32;
            Ok(json!({ "level": percent, "muted": muted.0 != 0 }))
        }
    }

    fn set_volume(level: u8) -> Result<Value, Failure> {
        let endpoint = endpoint_volume()?;
        unsafe {
            endpoint
                .SetMasterVolumeLevelScalar(level as f32 / 100.0, &GUID::zeroed())
                .map_err(windows_error)?;
        }
        volume_response(&endpoint)
    }

    fn toggle_mute() -> Result<Value, Failure> {
        let endpoint = endpoint_volume()?;
        unsafe {
            let muted: BOOL = endpoint.GetMute().map_err(windows_error)?;
            endpoint
                .SetMute(BOOL(if muted.0 != 0 { 0 } else { 1 }), &GUID::zeroed())
                .map_err(windows_error)?;
        }
        volume_response(&endpoint)
    }

    fn send_media_key(key: u16) {
        unsafe {
            keybd_event(key as u8, 0, KEYBD_EVENT_FLAGS(0), 0);
            keybd_event(key as u8, 0, KEYEVENTF_KEYUP, 0);
        }
    }

    fn toggle_play_pause() -> Result<Value, Failure> {
        if let Some(session) = current_session()? {
            if let Ok(true) = session
                .TryTogglePlayPauseAsync()
                .and_then(|operation| operation.get())
            {
                return Ok(json!({ "sent": true }));
            }
        }
        send_media_key(VK_MEDIA_PLAY_PAUSE.0);
        Ok(json!({ "sent": true }))
    }

    fn skip_next() -> Result<Value, Failure> {
        if let Some(session) = current_session()? {
            if let Ok(true) = session
                .TrySkipNextAsync()
                .and_then(|operation| operation.get())
            {
                return Ok(json!({ "sent": true }));
            }
        }
        send_media_key(VK_MEDIA_NEXT_TRACK.0);
        Ok(json!({ "sent": true }))
    }

    fn skip_previous() -> Result<Value, Failure> {
        if let Some(session) = current_session()? {
            if let Ok(true) = session
                .TrySkipPreviousAsync()
                .and_then(|operation| operation.get())
            {
                return Ok(json!({ "sent": true }));
            }
        }
        send_media_key(VK_MEDIA_PREV_TRACK.0);
        Ok(json!({ "sent": true }))
    }

    fn seek(position_ms: i64) -> Result<Value, Failure> {
        let Some(session) = current_session()? else {
            return Err(("platform_error", "No active media session".to_string()));
        };
        let playback = session.GetPlaybackInfo().map_err(windows_error)?;
        let can_seek = playback
            .Controls()
            .ok()
            .and_then(|controls| controls.IsPlaybackPositionEnabled().ok())
            .unwrap_or(false);
        if !can_seek {
            return Err((
                "unsupported",
                "The current media session does not support seeking".to_string(),
            ));
        }
        let requested = position_ms.checked_mul(TICKS_PER_MS).ok_or((
            "invalid_params",
            "positionMs is too large".to_string(),
        ))?;
        // SMTC positions are relative to the session timeline origin, which is normally zero.
        let origin = session
            .GetTimelineProperties()
            .ok()
            .and_then(|timeline| timeline.StartTime().ok())
            .map(|value| value.Duration)
            .unwrap_or(0);
        let requested = origin.saturating_add(requested);
        let accepted = session
            .TryChangePlaybackPositionAsync(requested)
            .and_then(|operation| operation.get())
            .map_err(windows_error)?;
        if !accepted {
            return Err((
                "unsupported",
                "The current media session rejected the seek".to_string(),
            ));
        }
        Ok(json!({ "positionMs": position_ms }))
    }

    pub fn run(action: Action) -> Result<Value, Failure> {
        match action {
            Action::PlayPause => toggle_play_pause(),
            Action::Next => skip_next(),
            Action::Previous => skip_previous(),
            Action::VolumeGet => {
                let endpoint = endpoint_volume()?;
                volume_response(&endpoint)
            }
            Action::VolumeSet(level) => set_volume(level),
            Action::ToggleMute => toggle_mute(),
            Action::Status => status(),
            Action::Seek(position_ms) => seek(position_ms),
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use serde_json::Value;

    use super::Action;

    pub fn run(_action: Action) -> Result<Value, (&'static str, String)> {
        Err((
            "platform_error",
            "Media control is only implemented for Windows".to_string(),
        ))
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    let mut line = Vec::new();
    let mut ready = false;
    loop {
        line.clear();
        let bytes = input.by_ref().take((MAX_LINE + 1) as u64).read_until(b'\n', &mut line)?;
        if bytes == 0 {
            return Err("host closed stdin".into());
        }
        if line.len() > MAX_LINE || !line.ends_with(b"\n") {
            return Err("invalid or oversized protocol line".into());
        }
        let message: Value = serde_json::from_slice(&line)?;
        if message.get("v").and_then(Value::as_u64) != Some(2) {
            return Err("unsupported protocol version".into());
        }
        match (ready, message.get("type").and_then(Value::as_str)) {
            (false, Some("hello")) => {
                output.write_all(b"{\"v\":2,\"type\":\"ready\"}\n")?;
                output.flush()?;
                ready = true;
            }
            (true, Some("stop")) => return Ok(()),
            (true, Some("request")) => {
                let id = message
                    .get("id")
                    .and_then(Value::as_str)
                    .filter(|id| !id.is_empty())
                    .ok_or("request requires a nonempty string id")?;
                let response = match execute(
                    message.get("method").and_then(Value::as_str).unwrap_or(""),
                    message.get("params").unwrap_or(&Value::Null),
                ) {
                    Ok(result) => json!({"v":2,"type":"response","id":id,"ok":true,"result":result}),
                    Err((code, message)) => {
                        json!({"v":2,"type":"response","id":id,"ok":false,"error":{"code":code,"message":message}})
                    }
                };
                serde_json::to_writer(&mut output, &response)?;
                output.write_all(b"\n")?;
                output.flush()?;
            }
            _ => return Err("unexpected protocol message".into()),
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("media plugin: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_parameterless_methods() {
        for (method, expected) in [
            ("media.play_pause", Action::PlayPause),
            ("media.next", Action::Next),
            ("media.previous", Action::Previous),
            ("media.status", Action::Status),
            ("volume.get", Action::VolumeGet),
            ("volume.toggle_mute", Action::ToggleMute),
        ] {
            assert_eq!(parse_action(method, &json!({})), Ok(expected), "{method}");
        }
    }

    #[test]
    fn rejects_extra_parameters() {
        assert_eq!(
            parse_action("media.status", &json!({"unexpected": true})),
            Err(("invalid_params", "Invalid parameters".to_string()))
        );
        assert_eq!(
            parse_action("media.next", &Value::Null),
            Err(("invalid_params", "params must be an object".to_string()))
        );
    }

    #[test]
    fn parses_volume_and_seek_arguments() {
        assert_eq!(
            parse_action("volume.set", &json!({"level": 0})),
            Ok(Action::VolumeSet(0))
        );
        assert_eq!(
            parse_action("volume.set", &json!({"level": 100})),
            Ok(Action::VolumeSet(100))
        );
        assert_eq!(
            parse_action("volume.set", &json!({"level": 101})),
            Err((
                "invalid_params",
                "level must be an integer from 0 to 100".to_string()
            ))
        );
        assert_eq!(
            parse_action("media.seek", &json!({"positionMs": 12_345})),
            Ok(Action::Seek(12_345))
        );
        assert_eq!(
            parse_action("media.seek", &json!({"positionMs": -1})),
            Err((
                "invalid_params",
                "positionMs must be a non-negative integer".to_string()
            ))
        );
        assert_eq!(
            parse_action("media.seek", &json!({"positionMs": 1.5})),
            Err((
                "invalid_params",
                "positionMs must be a non-negative integer".to_string()
            ))
        );
    }

    #[test]
    fn unknown_method_is_reported() {
        assert_eq!(
            parse_action("media.stop", &json!({})),
            Err(("method_not_found", "Unknown method".to_string()))
        );
    }

    #[test]
    fn unavailable_status_has_the_documented_shape() {
        assert_eq!(
            unavailable_status(),
            json!({
                "available": false,
                "title": "",
                "artist": "",
                "album": "",
                "playing": false,
                "positionMs": 0,
                "durationMs": 0,
                "canSeek": false,
            })
        );
    }

    #[test]
    fn base64_matches_reference_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64_encode(&[0xff, 0xfe, 0xfd]), "//79");
    }
}
