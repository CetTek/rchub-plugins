use std::io::{self, BufRead, Read, Write};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde_json::{json, Value};

const MAX_LINE: usize = 64 * 1024;
const MAX_TEXT: usize = 256 * 1024;
const MAX_CHUNK: usize = 4096;
struct Transfer { text: String, expected: usize, offset: usize, write: bool, touched: Instant }
type Transfers = HashMap<String, Transfer>;

#[link(name = "bcrypt")]
extern "system" { fn BCryptGenRandom(algorithm: *mut std::ffi::c_void, buffer: *mut u8, len: u32, flags: u32) -> i32; }
fn token() -> Option<String> {
    let mut bytes = [0u8; 16];
    if unsafe { BCryptGenRandom(std::ptr::null_mut(), bytes.as_mut_ptr(), 16, 2) } < 0 { return None; }
    Some(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

#[cfg(not(windows))]
compile_error!("This clipboard example currently supports Windows only.");

fn response(id: &str, method: Option<&str>, params: Option<&Value>, transfers: &mut Transfers) -> Value {
    let error = |code: &str, message: &str| json!({"v":2,"type":"response","id":id,"ok":false,"error":{"code":code,"message":message}});
    let Some(params) = params.and_then(Value::as_object) else {
        return error("invalid_params", "params must be an object");
    };
    transfers.retain(|_, t| t.touched.elapsed() < Duration::from_secs(60));
    let result = match method {
        Some("clipboard.read.begin") | Some("clipboard.write.begin") => {
            let write = method == Some("clipboard.write.begin");
            if transfers.len() >= 4 { return error("transfer_limit", "Too many active transfers"); }
            let (text, expected) = if write {
                let Some(length) = params.get("length").and_then(Value::as_u64).filter(|_| params.len() == 1).and_then(|n| usize::try_from(n).ok()).filter(|n| *n <= MAX_TEXT) else { return error("invalid_params", "Invalid transfer length"); };
                (String::with_capacity(length), length)
            } else {
                if !params.is_empty() { return error("invalid_params", "Invalid parameters"); }
                let text = match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.get_text()) {
                    Ok(text) => text, Err(_) => return error("clipboard_unavailable", "Unable to read text from the clipboard"),
                };
                if text.len() > MAX_TEXT { return error("too_large", "Clipboard text exceeds 262144 UTF-8 bytes"); }
                if text.contains('\0') { return error("invalid_params", "Text cannot contain NUL"); }
                let len = text.len(); (text, len)
            };
            let Some(token) = token().filter(|token| !transfers.contains_key(token)) else { return error("transfer_unavailable", "Unable to create transfer"); };
            transfers.insert(token.clone(), Transfer { text, expected, offset: 0, write, touched: Instant::now() });
            json!({"transfer":token, "length":expected})
        }
        Some("clipboard.read.chunk") | Some("clipboard.write.chunk") | Some("clipboard.write.commit") | Some("clipboard.cancel") => {
            let Some(token) = params.get("transfer").and_then(Value::as_str) else { return error("invalid_params", "Invalid transfer"); };
            let Some(transfer) = transfers.get_mut(token) else { return error("invalid_params", "Unknown transfer"); };
            transfer.touched = Instant::now();
            match method.unwrap() {
                "clipboard.cancel" if params.len() == 1 => { transfers.remove(token); json!({}) }
                "clipboard.read.chunk" if !transfer.write && params.len() == 2 && params.get("offset").and_then(Value::as_u64) == Some(transfer.offset as u64) => {
                    let mut end = (transfer.offset + MAX_CHUNK).min(transfer.text.len());
                    while !transfer.text.is_char_boundary(end) { end -= 1; }
                    let part = &transfer.text[transfer.offset..end];
                    transfer.offset = end;
                    json!({"text":part, "done":end == transfer.expected})
                }
                "clipboard.write.chunk" if transfer.write && params.len() == 3 && params.get("offset").and_then(Value::as_u64) == Some(transfer.offset as u64) => {
                    let Some(part) = params.get("text").and_then(Value::as_str).filter(|s| !s.is_empty() && s.len() <= MAX_CHUNK && !s.contains('\0')) else { transfers.remove(token); return error("invalid_params", "Invalid chunk"); };
                    if transfer.offset + part.len() > transfer.expected { transfers.remove(token); return error("invalid_params", "Transfer exceeds declared length"); }
                    transfer.text.push_str(part); transfer.offset += part.len(); json!({})
                }
                "clipboard.write.commit" if transfer.write && params.len() == 1 && transfer.offset == transfer.expected => {
                    let transfer = transfers.remove(token).unwrap();
                    match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(transfer.text)) {
                        Ok(()) => json!({}), Err(_) => return error("clipboard_unavailable", "Unable to write text to the clipboard"),
                    }
                }
                _ => { transfers.remove(token); return error("invalid_params", "Invalid transfer sequence"); }
            }
        }
        _ => return error("method_not_found", "Unknown method"),
    };
    json!({"v":2,"type":"response","id":id,"ok":true,"result":result})
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    let mut line = Vec::new();
    let mut ready = false;
    let mut transfers = Transfers::new();
    loop {
        line.clear();
        let bytes = input.by_ref().take((MAX_LINE + 1) as u64).read_until(b'\n', &mut line)?;
        if bytes == 0 { return Err("host closed stdin".into()); }
        if line.len() > MAX_LINE || !line.ends_with(b"\n") { return Err("invalid or oversized protocol line".into()); }
        let message: Value = serde_json::from_slice(&line)?;
        if message.get("v").and_then(Value::as_u64) != Some(2) { return Err("unsupported protocol version".into()); }
        match (ready, message.get("type").and_then(Value::as_str)) {
            (false, Some("hello")) => { output.write_all(b"{\"v\":2,\"type\":\"ready\"}\n")?; output.flush()?; ready = true; }
            (true, Some("stop")) => return Ok(()),
            (true, Some("request")) => {
                let id = message.get("id").and_then(Value::as_str).filter(|id| !id.is_empty() && id.len() <= 128).ok_or("invalid request id")?;
                let reply = response(id, message.get("method").and_then(Value::as_str), message.get("params"), &mut transfers);
                serde_json::to_writer(&mut output, &reply)?;
                output.write_all(b"\n")?;
                output.flush()?;
            }
            _ => return Err("unexpected protocol message".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn escaped_snapshot_chunks_remain_bounded_and_ordered() {
        let mut state = Transfers::new();
        let text = "\u{1}".repeat(MAX_TEXT);
        state.insert("secret".into(), Transfer { expected: text.len(), text, offset: 0, write: false, touched: Instant::now() });
        let mut offset = 0;
        while offset < MAX_TEXT {
            let reply = request(&mut state, "clipboard.read.chunk", json!({"transfer":"secret","offset":offset}));
            assert!(serde_json::to_vec(&reply).unwrap().len() < 32768);
            let part = reply["result"]["text"].as_str().unwrap();
            assert_eq!(part.len(), MAX_CHUNK);
            offset += part.len();
            assert_eq!(reply["result"]["done"], offset == MAX_TEXT);
        }
        assert_eq!(request(&mut state, "clipboard.cancel", json!({"transfer":"secret"}))["ok"], true);
    }
    fn request(state: &mut Transfers, method: &str, params: Value) -> Value { response("test", Some(method), Some(&params), state) }
    #[test]
    fn bounded_ordered_unicode_writes() {
        let mut state = Transfers::new();
        let begin = request(&mut state, "clipboard.write.begin", json!({"length":262144}));
        let token = begin["result"]["transfer"].as_str().unwrap().to_owned();
        assert_eq!(request(&mut state, "clipboard.write.chunk", json!({"transfer":token,"offset":0,"text":"😀"}))["ok"], true);
        assert_eq!(request(&mut state, "clipboard.write.chunk", json!({"transfer":token,"offset":0,"text":"x"}))["ok"], false);
        assert!(state.is_empty());
        assert_eq!(request(&mut state, "clipboard.write.begin", json!({"length":262145}))["ok"], false);
    }
    #[test]
    fn incomplete_nul_and_expired_transfers_are_discarded() {
        let mut state = Transfers::new();
        let begin = request(&mut state, "clipboard.write.begin", json!({"length":4}));
        let token = begin["result"]["transfer"].as_str().unwrap().to_owned();
        assert_eq!(request(&mut state, "clipboard.write.commit", json!({"transfer":token}))["ok"], false);
        let begin = request(&mut state, "clipboard.write.begin", json!({"length":4}));
        let token = begin["result"]["transfer"].as_str().unwrap().to_owned();
        assert_eq!(request(&mut state, "clipboard.write.chunk", json!({"transfer":token,"offset":0,"text":"\0"}))["ok"], false);
        let begin = request(&mut state, "clipboard.write.begin", json!({"length":0}));
        let token = begin["result"]["transfer"].as_str().unwrap().to_owned();
        state.get_mut(&token).unwrap().touched = Instant::now() - Duration::from_secs(61);
        assert_eq!(request(&mut state, "clipboard.cancel", json!({"transfer":token}))["ok"], false);
        assert!(state.is_empty());
    }
}

fn main() {
    if let Err(error) = run() { eprintln!("clipboard plugin: {error}"); std::process::exit(1); }
}
