use std::io::{self, BufRead, Read, Write};

use serde_json::{json, Value};
use sysinfo::System;

const MAX_LINE: usize = 64 * 1024;

fn system_info() -> Value {
    let mut system = System::new();
    system.refresh_cpu();
    system.refresh_memory();
    let brand = system.cpus().first().map(|cpu| cpu.brand()).filter(|s| !s.is_empty());
    json!({
        "os": {
            "name": System::name(),
            "version": System::os_version(),
            "architecture": std::env::consts::ARCH,
        },
        "cpu": {
            "brand": brand,
            "logical_cores": system.cpus().len(),
        },
        "memory": {
            "total_bytes": system.total_memory(),
            "available_bytes": system.available_memory(),
        },
    })
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
                let id = message.get("id").and_then(Value::as_str).filter(|id| !id.is_empty()).ok_or("request requires a nonempty string id")?;
                let response = match message.get("method").and_then(Value::as_str) {
                    Some("system.info") if message.get("params").and_then(Value::as_object).is_some_and(|params| params.is_empty()) => {
                        json!({"v":2,"type":"response","id":id,"ok":true,"result":system_info()})
                    }
                    Some("system.info") => json!({"v":2,"type":"response","id":id,"ok":false,"error":{"code":"invalid_params","message":"system.info requires an empty params object"}}),
                    _ => json!({"v":2,"type":"response","id":id,"ok":false,"error":{"code":"method_not_found","message":"Unknown method"}}),
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
        eprintln!("system info plugin: {error}");
        std::process::exit(1);
    }
}
