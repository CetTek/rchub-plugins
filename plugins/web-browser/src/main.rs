use std::io::{self, BufRead, Read, Write};

use serde_json::{json, Map, Value};

const MAX_LINE: usize = 64 * 1024;
const MAX_URL_BYTES: usize = 2048;

#[derive(Debug)]
struct RequestError {
    code: &'static str,
    message: String,
}

impl RequestError {
    fn invalid_params(message: impl Into<String>) -> Self {
        Self { code: "invalid_params", message: message.into() }
    }

    fn invalid_url(message: impl Into<String>) -> Self {
        Self { code: "invalid_url", message: message.into() }
    }

    fn into_json(self) -> Value {
        json!({ "code": self.code, "message": self.message })
    }
}

/// Accepts ASCII only, so length in bytes and characters is identical and the
/// value can be handed to the operating system without an encoding guess.
/// A bare host or host:port is upgraded to HTTPS; anything else that looks
/// like a scheme must be http or https.
fn normalize_url(raw: &str) -> Result<String, RequestError> {
    if raw.is_empty() {
        return Err(RequestError::invalid_url("URL must not be empty"));
    }
    if raw.len() > MAX_URL_BYTES {
        return Err(RequestError::invalid_url(format!(
            "URL must be at most {MAX_URL_BYTES} bytes"
        )));
    }
    if !raw.is_ascii() {
        return Err(RequestError::invalid_url(
            "URL must use ASCII characters; percent-encode other characters",
        ));
    }
    if raw.bytes().any(|byte| byte < 0x21 || byte > 0x7e) {
        return Err(RequestError::invalid_url(
            "URL must not contain whitespace or control characters",
        ));
    }
    if raw.contains('\\') {
        return Err(RequestError::invalid_url(
            "URL must not contain backslashes",
        ));
    }

    let lower = raw.to_ascii_lowercase();
    let candidate = if lower.starts_with("http://") || lower.starts_with("https://") {
        raw.to_owned()
    } else {
        // A colon before the first path/query/fragment separator can mean a
        // scheme, a host:port pair, or an IPv6 literal. Reject the genuinely
        // ambiguous single-label form instead of guessing.
        let boundary = raw
            .find(|character| matches!(character, '/' | '?' | '#'))
            .unwrap_or(raw.len());
        let head = &raw[..boundary];
        if let Some(colon) = head.find(':') {
            let prefix = &head[..colon];
            let after = &raw[colon + 1..];
            if after.starts_with("//") {
                return Err(RequestError::invalid_url(
                    "Only http and https URLs are supported",
                ));
            }
            let host_shaped = prefix.contains('.')
                || prefix.eq_ignore_ascii_case("localhost")
                || raw.starts_with('[');
            if !host_shaped {
                return Err(RequestError::invalid_url(
                    "Only http and https URLs are supported; add https:// to a host name",
                ));
            }
        }
        format!("https://{raw}")
    };

    if candidate.len() > MAX_URL_BYTES {
        return Err(RequestError::invalid_url(format!(
            "URL must be at most {MAX_URL_BYTES} bytes"
        )));
    }

    let scheme_end = candidate
        .find("://")
        .ok_or_else(|| RequestError::invalid_url("Only http and https URLs are supported"))?;
    let scheme = &candidate[..scheme_end];
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return Err(RequestError::invalid_url(
            "Only http and https URLs are supported",
        ));
    }

    let rest = &candidate[scheme_end + 3..];
    let authority_end = rest
        .find(|character| matches!(character, '/' | '?' | '#'))
        .unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    let tail = &rest[authority_end..];

    if authority.is_empty() {
        return Err(RequestError::invalid_url(
            "URL must include a host, such as google.com",
        ));
    }
    validate_authority(authority)?;
    validate_tail(tail)?;

    Ok(format!(
        "{}://{}{}",
        scheme.to_ascii_lowercase(),
        authority,
        tail
    ))
}

fn validate_authority(authority: &str) -> Result<(), RequestError> {
    if authority.contains('@') {
        return Err(RequestError::invalid_url(
            "URLs with a username or password are not supported",
        ));
    }

    if authority.starts_with('[') {
        let close = authority
            .find(']')
            .ok_or_else(|| RequestError::invalid_url("The host name is not valid"))?;
        let host = &authority[1..close];
        if host.is_empty()
            || !host.contains(':')
            || !host
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() || byte == b':' || byte == b'.')
        {
            return Err(RequestError::invalid_url("The host name is not valid"));
        }
        let remainder = &authority[close + 1..];
        if remainder.is_empty() {
            return Ok(());
        }
        let port = remainder
            .strip_prefix(':')
            .ok_or_else(|| RequestError::invalid_url("The host name is not valid"))?;
        return validate_port(port);
    }

    if authority.matches(':').count() > 1 {
        return Err(RequestError::invalid_url(
            "IPv6 addresses must be enclosed in brackets",
        ));
    }
    let (host, port) = match authority.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    };
    if let Some(port) = port {
        validate_port(port)?;
    }
    validate_host(host)
}

fn validate_port(port: &str) -> Result<(), RequestError> {
    if port.is_empty()
        || port.len() > 5
        || !port.bytes().all(|byte| byte.is_ascii_digit())
        || port.parse::<u32>().map(|value| !(1..=65535).contains(&value)).unwrap_or(true)
    {
        return Err(RequestError::invalid_url(
            "The port must be a number from 1 to 65535",
        ));
    }
    Ok(())
}

fn validate_host(host: &str) -> Result<(), RequestError> {
    if host.is_empty() {
        return Err(RequestError::invalid_url(
            "URL must include a host, such as google.com",
        ));
    }
    if host.len() > 253 || host.starts_with('.') || host.ends_with('.') {
        return Err(RequestError::invalid_url("The host name is not valid"));
    }
    if !host.contains('.') && host.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(RequestError::invalid_url(
            "IP addresses must use dotted-decimal notation",
        ));
    }
    for label in host.split('.') {
        if label.is_empty()
            || label.len() > 63
            || label.starts_with('-')
            || label.ends_with('-')
            || !label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(RequestError::invalid_url("The host name is not valid"));
        }
    }
    Ok(())
}

fn validate_tail(tail: &str) -> Result<(), RequestError> {
    let bytes = tail.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if matches!(byte, b'"' | b'<' | b'>' | b'`' | b'{' | b'}' | b'|' | b'^') {
            return Err(RequestError::invalid_url(
                "The URL contains characters that are not supported",
            ));
        }
        if byte == b'%' {
            if index + 2 >= bytes.len()
                || !bytes[index + 1].is_ascii_hexdigit()
                || !bytes[index + 2].is_ascii_hexdigit()
            {
                return Err(RequestError::invalid_url(
                    "The URL contains an invalid percent-encoded sequence",
                ));
            }
            let decoded = u8::from_str_radix(&tail[index + 1..index + 3], 16)
                .expect("two hex digits were checked");
            if decoded < 0x20 || decoded == 0x7f {
                return Err(RequestError::invalid_url(
                    "Percent-encoded control characters are not supported",
                ));
            }
            index += 3;
            continue;
        }
        index += 1;
    }
    Ok(())
}

fn parse_open(params: Option<&Map<String, Value>>) -> Result<String, RequestError> {
    let params = params
        .ok_or_else(|| RequestError::invalid_params("browser.open requires params"))?;
    if params.len() != 1 {
        return Err(RequestError::invalid_params(
            "browser.open requires exactly one field: url",
        ));
    }
    let raw = params
        .get("url")
        .and_then(Value::as_str)
        .ok_or_else(|| RequestError::invalid_params("url must be a string"))?;
    normalize_url(raw)
}

#[cfg(target_os = "windows")]
#[link(name = "shell32")]
extern "system" {
    fn ShellExecuteW(
        hwnd: *mut std::ffi::c_void,
        operation: *const u16,
        file: *const u16,
        parameters: *const u16,
        directory: *const u16,
        show_command: i32,
    ) -> isize;
}

/// Uses the operating system's default URL handler. The URL was validated as
/// an absolute HTTP(S) URL before reaching this point, and no shell is
/// involved, so no user-chosen program or command string is executed.
#[cfg(target_os = "windows")]
fn open_url(url: &str) -> Result<(), String> {
    const SW_SHOWNORMAL: i32 = 1;
    let operation: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();
    let file: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    if result <= 32 {
        Err(format!(
            "Windows could not open the default browser (ShellExecute code {result})"
        ))
    } else {
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn open_url(url: &str) -> Result<(), String> {
    use std::process::{Command, Stdio};

    let mut child = Command::new("xdg-open")
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                "The default browser opener (xdg-open) is not available on this system"
                    .to_owned()
            } else {
                format!("Could not start the default browser opener: {error}")
            }
        })?;
    // xdg-open normally exits after handing the URL to the desktop. Reap it
    // without blocking the protocol loop.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn open_url(_url: &str) -> Result<(), String> {
    Err("Opening a web browser is supported only on Windows and Linux".to_owned())
}

fn dispatch_with<F>(
    method: &str,
    params: Option<&Map<String, Value>>,
    open: F,
) -> Result<Value, RequestError>
where
    F: FnOnce(&str) -> Result<(), String>,
{
    match method {
        "browser.open" => {
            let url = parse_open(params)?;
            open(&url).map_err(|message| RequestError {
                code: "launch_failed",
                message,
            })?;
            Ok(json!({ "opened": true, "url": url }))
        }
        _ => Err(RequestError {
            code: "method_not_found",
            message: "Unknown method".into(),
        }),
    }
}

fn dispatch(method: &str, params: Option<&Map<String, Value>>) -> Result<Value, RequestError> {
    dispatch_with(method, params, open_url)
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
        let bytes = input
            .by_ref()
            .take((MAX_LINE + 1) as u64)
            .read_until(b'\n', &mut line)?;
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
                    .filter(|id| !id.is_empty() && id.len() <= 128)
                    .ok_or("request requires a nonempty string id of at most 128 bytes")?;
                let response = match message.get("method").and_then(Value::as_str) {
                    Some(method) => {
                        match dispatch(method, message.get("params").and_then(Value::as_object)) {
                            Ok(result) => {
                                json!({"v":2,"type":"response","id":id,"ok":true,"result":result})
                            }
                            Err(error) => {
                                json!({"v":2,"type":"response","id":id,"ok":false,"error":error.into_json()})
                            }
                        }
                    }
                    None => json!({
                        "v":2,
                        "type":"response",
                        "id":id,
                        "ok":false,
                        "error":{"code":"method_not_found","message":"Unknown method"}
                    }),
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
        eprintln!("web-browser plugin: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(value: Value) -> Map<String, Value> {
        serde_json::from_value(value).expect("test params must be an object")
    }

    fn url_error(raw: &str) -> String {
        normalize_url(raw).expect_err("URL should be rejected").message
    }

    #[test]
    fn accepts_absolute_http_and_https() {
        assert_eq!(
            normalize_url("https://example.com/path?q=1#top").unwrap(),
            "https://example.com/path?q=1#top"
        );
        assert_eq!(
            normalize_url("HTTP://Example.COM").unwrap(),
            "http://Example.COM"
        );
    }

    #[test]
    fn upgrades_bare_hosts_and_ports_to_https() {
        assert_eq!(normalize_url("google.com").unwrap(), "https://google.com");
        assert_eq!(
            normalize_url("example.com:8443/path").unwrap(),
            "https://example.com:8443/path"
        );
        assert_eq!(
            normalize_url("localhost:8080").unwrap(),
            "https://localhost:8080"
        );
        assert_eq!(
            normalize_url("[::1]:8080").unwrap(),
            "https://[::1]:8080"
        );
    }

    #[test]
    fn rejects_credentials_and_non_http_schemes() {
        assert!(url_error("https://user:pass@example.com").contains("username"));
        assert!(url_error("file:///etc/passwd").contains("http and https"));
        assert!(url_error("javascript:alert(1)").contains("http and https"));
        assert!(url_error("ftp://example.com").contains("http and https"));
        assert!(url_error("data:text/html,hello").contains("http and https"));
    }

    #[test]
    fn rejects_ambiguous_or_malformed_hosts() {
        assert!(url_error("https://").contains("host"));
        assert!(url_error("https:///path").contains("host"));
        assert!(url_error("https://exa_mple.com").contains("host"));
        assert!(url_error("https://-example.com").contains("host"));
        assert!(url_error("https://example.com:0").contains("port"));
        assert!(url_error("https://example.com:70000").contains("port"));
        assert!(url_error("https://example.com:abc").contains("port"));
        assert!(url_error("https://127.0.0.1:99999").contains("port"));
        assert!(url_error("https://::1").contains("brackets"));
        assert!(url_error("https://12345").contains("dotted-decimal"));
        assert!(url_error("intranet:8080").contains("add https://"));
    }

    #[test]
    fn rejects_whitespace_controls_and_ascii_tricks() {
        assert!(url_error("https://example.com/a b").contains("whitespace"));
        assert!(url_error("https://example.com/\n").contains("whitespace"));
        assert!(url_error("https://example.com/\u{00e9}").contains("ASCII"));
        assert!(url_error("https://example.com\\path").contains("backslash"));
        assert!(url_error("https://example.com/<script>").contains("not supported"));
    }

    #[test]
    fn validates_percent_encoding_without_decoding_the_url() {
        assert_eq!(
            normalize_url("https://example.com/a%20b").unwrap(),
            "https://example.com/a%20b"
        );
        assert!(url_error("https://example.com/%zz").contains("percent"));
        assert!(url_error("https://example.com/%0a").contains("control"));
        assert!(url_error("https://example.com/%7F").contains("control"));
    }

    #[test]
    fn enforces_the_length_bound_on_input_and_normalized_url() {
        assert!(url_error(&"a".repeat(MAX_URL_BYTES + 1)).contains("at most"));
        let over_limit = format!("https://{}/", "a".repeat(MAX_URL_BYTES));
        assert!(url_error(&over_limit).contains("at most"));
    }

    #[test]
    fn requires_exactly_one_string_url_field() {
        let extra = params(json!({"url": "https://example.com", "target": "other"}));
        assert_eq!(
            parse_open(Some(&extra)).unwrap_err().code,
            "invalid_params"
        );
        let numeric = params(json!({"url": 7}));
        assert_eq!(
            parse_open(Some(&numeric)).unwrap_err().code,
            "invalid_params"
        );
        assert_eq!(parse_open(None).unwrap_err().code, "invalid_params");
    }

    #[test]
    fn valid_dispatch_normalizes_before_calling_the_opener() {
        let request = params(json!({"url": "example.com/docs"}));
        let mut opened = None;
        let result = dispatch_with("browser.open", Some(&request), |url| {
            opened = Some(url.to_owned());
            Ok(())
        })
        .unwrap();
        assert_eq!(opened.as_deref(), Some("https://example.com/docs"));
        assert_eq!(result["opened"], json!(true));
        assert_eq!(result["url"], json!("https://example.com/docs"));
    }

    #[test]
    fn invalid_dispatch_never_calls_the_opener() {
        let request = params(json!({"url": "file:///etc/passwd"}));
        let error = dispatch_with("browser.open", Some(&request), |_| {
            panic!("opener must not run for an invalid URL")
        })
        .unwrap_err();
        assert_eq!(error.code, "invalid_url");
    }

    #[test]
    fn launch_errors_are_reported_as_launch_failures() {
        let request = params(json!({"url": "https://example.com"}));
        let error = dispatch_with("browser.open", Some(&request), |_| {
            Err("no default browser".to_owned())
        })
        .unwrap_err();
        assert_eq!(error.code, "launch_failed");
        assert_eq!(error.message, "no default browser");
    }

    #[test]
    fn unknown_methods_are_rejected_without_opening() {
        let error =
            dispatch_with("browser.openUrl", Some(&Map::new()), |_| {
                panic!("opener must not run for an unknown method")
            })
            .unwrap_err();
        assert_eq!(error.code, "method_not_found");
    }
}
