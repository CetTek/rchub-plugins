use std::io::{self, BufRead, Read, Write};

use base64::{engine::general_purpose::STANDARD, Engine};
use image::{codecs::jpeg::JpegEncoder, imageops::FilterType, DynamicImage};
use screenshots::Screen;
use serde_json::{json, Value};

const MAX_LINE: usize = 64 * 1024;
// Leaves room for JSON envelopes and escaping in the remote relay as well as IPC.
const MAX_JPEG: usize = 30 * 1024;

fn capture() -> Result<Value, String> {
    let screens = Screen::all().map_err(|e| format!("Unable to enumerate displays: {e}"))?;
    let screen = screens.iter().find(|s| s.display_info.is_primary)
        .ok_or("No primary display is available")?;
    let raw = screen.capture().map_err(|e| format!("Unable to capture display: {e}"))?;
    let original_width = raw.width();
    let original_height = raw.height();
    let mut image = DynamicImage::ImageRgba8(raw).thumbnail(800, 800).to_rgb8();
    loop {
        for quality in [70, 50, 30] {
            let mut jpeg = Vec::new();
            JpegEncoder::new_with_quality(&mut jpeg, quality)
                .encode_image(&image).map_err(|e| format!("Unable to encode screenshot: {e}"))?;
            if jpeg.len() <= MAX_JPEG {
                return Ok(json!({"mime":"image/jpeg", "base64":STANDARD.encode(jpeg),
                    "width":image.width(), "height":image.height(),
                    "original_width":original_width, "original_height":original_height}));
            }
        }
        if image.width() <= 100 && image.height() <= 100 {
            return Err("Screenshot could not be encoded within the size limit".into());
        }
        let width = (image.width() * 3 / 4).max(1);
        let height = (image.height() * 3 / 4).max(1);
        image = image::imageops::resize(&image, width, height, FilterType::Triangle);
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
        if bytes == 0 { return Err("host closed stdin".into()); }
        if line.len() > MAX_LINE || !line.ends_with(b"\n") { return Err("invalid or oversized protocol line".into()); }
        let message: Value = serde_json::from_slice(&line)?;
        if message.get("v").and_then(Value::as_u64) != Some(2) { return Err("unsupported protocol version".into()); }
        match (ready, message.get("type").and_then(Value::as_str)) {
            (false, Some("hello")) => {
                output.write_all(b"{\"v\":2,\"type\":\"ready\"}\n")?;
                output.flush()?;
                ready = true;
            }
            (true, Some("stop")) => return Ok(()),
            (true, Some("request")) => {
                let id = message.get("id").and_then(Value::as_str).filter(|id| !id.is_empty() && id.len() <= 128).ok_or("invalid request id")?;
                let response = match message.get("method").and_then(Value::as_str) {
                    Some("screenshot.capture") if message.get("params").and_then(Value::as_object).is_some_and(|p| p.is_empty()) => match capture() {
                        Ok(result) => json!({"v":2,"type":"response","id":id,"ok":true,"result":result}),
                        Err(message) => json!({"v":2,"type":"response","id":id,"ok":false,"error":{"code":"capture_failed","message":message}}),
                    },
                    Some("screenshot.capture") => json!({"v":2,"type":"response","id":id,"ok":false,"error":{"code":"invalid_params","message":"screenshot.capture requires an empty params object"}}),
                    _ => json!({"v":2,"type":"response","id":id,"ok":false,"error":{"code":"method_not_found","message":"Unknown method"}}),
                };
                let encoded = serde_json::to_vec(&response)?;
                if encoded.len() > MAX_LINE { return Err("response exceeds protocol limit".into()); }
                output.write_all(&encoded)?;
                output.write_all(b"\n")?;
                output.flush()?;
            }
            _ => return Err("unexpected protocol message".into()),
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("screenshot plugin: {error}");
        std::process::exit(1);
    }
}
