//! Shared source, separately compiled into each official package's executable.
//! No host modules, shell-selected commands, or host-native capability imports.
mod audio;
mod providers;

use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::io::{self, Read, Write};
use std::sync::{atomic::{AtomicBool, Ordering}, mpsc, Arc, Mutex};

const PROTOCOL: &str = "rchub-app-v2-experimental";
const FRAME_LIMIT: usize = 1048576;
const CONFIG: &str = include_str!(env!("RCHUB_OFFICIAL_CONFIG"));

#[derive(Deserialize)]
struct Config {
    id: String,
    actions: Vec<Action>,
    state: BTreeMap<String, Value>,
    #[serde(rename = "requiredFeatures")]
    required_features: Vec<String>,
}
#[derive(Deserialize)]
struct Action {
    id: String,
    capability: String,
    input: Value,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Envelope {
    protocol: String,
    instance_id: String,
    context_id: String,
    sender: String,
    body: Value,
}

type Pending = Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>;

fn read_frame(input: &mut impl Read) -> Result<Option<Envelope>, ()> {
    let mut header = [0; 4];
    match input.read(&mut header[..1]).map_err(|_| ())? {
        0 => return Ok(None),
        1 => (),
        _ => return Err(()),
    }
    input.read_exact(&mut header[1..]).map_err(|_| ())?;
    let size = u32::from_be_bytes(header) as usize;
    if size == 0 || size > FRAME_LIMIT { return Err(()); }
    let mut raw = vec![0; size];
    input.read_exact(&mut raw).map_err(|_| ())?;
    let envelope: Envelope = serde_json::from_slice(&raw).map_err(|_| ())?;
    if envelope.protocol != PROTOCOL || envelope.sender != "host" {
        return Err(());
    }
    Ok(Some(envelope))
}

fn send(context: &(String, String), body: Value) -> Result<(), ()> {
    let raw = serde_json::to_vec(&json!({"protocol": PROTOCOL,
        "instanceId": context.0, "contextId": context.1,
        "sender": "plugin", "body": body})).map_err(|_| ())?;
    if raw.len() > FRAME_LIMIT { return Err(()); }
    let mut output = io::stdout().lock();
    output.write_all(&(raw.len() as u32).to_be_bytes()).map_err(|_| ())?;
    output.write_all(&raw).map_err(|_| ())?;
    output.flush().map_err(|_| ())
}

fn revoke(pending: &Pending) {
    if let Ok(requests) = pending.lock() {
        for active in requests.values() { active.store(false, Ordering::Release); }
    }
}

fn payload_valid(schema: &Value, value: &Value) -> bool {
    match schema["type"].as_str() {
        Some("object") => value.as_object().is_some_and(|object| {
            let Some(fields) = schema["properties"].as_object() else { return false; };
            let Some(required) = schema["required"].as_array() else { return false; };
            required.iter().all(|key| key.as_str().is_some_and(|key| object.contains_key(key)))
                && object.iter().all(|(key, value)| fields.get(key).is_some_and(|field| payload_valid(field, value)))
        }),
        Some("string") => value.as_str().is_some_and(|s| {
            !s.contains('\0') && s.chars().count() <= schema["maxLength"].as_u64().unwrap_or(0) as usize
                && schema.get("enum").is_none_or(|allowed| allowed.as_array().is_some_and(|values| values.contains(value)))
        }),
        Some("integer") => value.as_i64().is_some_and(|n| {
            n >= schema["minimum"].as_i64().unwrap_or(i64::MAX)
                && n <= schema["maximum"].as_i64().unwrap_or(i64::MIN)
        }),
        Some("boolean") => value.is_boolean(),
        _ => false,
    }
}

fn invoke(action: &Action, payload: Value, active: Arc<AtomicBool>, context: Arc<AtomicBool>) -> Result<Value, ()> {
    if !payload_valid(&action.input, &payload) { return Err(()); }
    let current = || active.load(Ordering::Acquire) && context.load(Ordering::Acquire);
    match action.capability.as_str() {
        #[cfg(windows)]
        "system.read" if action.id == "battery" => providers::windows::battery(&payload, &current),
        #[cfg(target_os = "linux")]
        "system.read" if action.id == "battery" => providers::linux_battery(&payload, &current),
        "audio.read" | "audio.write" => {
            let args = payload.as_object().ok_or(())?;
            let (op, value) = match action.id.as_str() {
                "read" | "audio_read" if args.is_empty() => (0, 0),
                "set_volume" | "audio_set_volume" => (1, args.get("volume").and_then(Value::as_i64).ok_or(())? as i32),
                "set_mute" | "audio_set_mute" => (2, i32::from(args.get("muted").and_then(Value::as_bool).ok_or(())?)),
                _ => return Err(()),
            };
            let state = audio::State::decode(audio::invoke(op, value, &current)?)?;
            serde_json::to_value(state).map_err(|_| ())
        }
        #[cfg(windows)]
        "speech.write" => {
            // Host owns grant/authentication decisions. Both signals remain
            // observable by the helper supervisor after acknowledgement.
            let authority = Arc::new(providers::speech::Authority::new(
                Arc::clone(&active), Arc::clone(&context),
            ));
            providers::speech::invoke("speech.write", payload, &current, &authority)
        }
        cap => providers::invoke(cap, payload, &current),
    }
}

fn initial_state(config: &Config, context: &Arc<AtomicBool>, active: &Arc<AtomicBool>) -> Value {
    // Read actual device state on opening a surface. Operation-status fields
    // stay absent until an operation has really produced them.
    let mut values = serde_json::Map::new();
    for action in &config.actions {
        if matches!(action.capability.as_str(), "system.read" | "audio.read" | "display.read")
            && action.input["required"] == json!([]) {
            if let Ok(observed) = invoke(action, json!({}), Arc::clone(active), Arc::clone(context)) {
                if let Some(fields) = observed.as_object() { values.extend(fields.clone()); }
            }
        }
    }
    values.retain(|key, _| config.state.contains_key(key));
    Value::Object(values)
}
// Action results are returned to their requester, but surface state is shared.
// Clipboard contents must never be published there, even by a misconfigured manifest.
fn merge_action_state(config: &Config, action: &Action, state: &mut Value, result: &Value) -> Result<bool, ()> {
    if matches!(action.capability.as_str(), "clipboard.read" | "clipboard.write") {
        return Ok(false);
    }
    let Some(fields) = result.as_object() else { return Ok(false); };
    let state = state.as_object_mut().ok_or(())?;
    let mut published = false;
    for (key, value) in fields {
        if config.state.contains_key(key) {
            state.insert(key.clone(), value.clone());
            published = true;
        }
    }
    Ok(published)
}


fn run() -> Result<(), ()> {
    #[cfg(windows)]
    if std::env::args().nth(1).as_deref() == Some("--plugin-foreground-helper") {
        return if providers::windows::foreground_helper() { Ok(()) } else { Err(()) };
    }
    let config: Config = serde_json::from_str(CONFIG).map_err(|_| ())?;
    let context_active = Arc::new(AtomicBool::new(true));
    let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
    let reader_pending = Arc::clone(&pending);
    let reader_active = Arc::clone(&context_active);
    let (tx, rx) = mpsc::sync_channel(16);
    std::thread::Builder::new().name("native-input".into()).spawn(move || {
        let mut input = io::stdin().lock();
        let mut context = None;
        while let Ok(Some(envelope)) = read_frame(&mut input) {
            let identity = (envelope.instance_id.clone(), envelope.context_id.clone());
            if context.as_ref().is_some_and(|expected| expected != &identity) { break; }
            context.get_or_insert(identity);
            match envelope.body["kind"].as_str() {
                Some("cancel") => {
                    if let Some(id) = envelope.body["requestId"].as_str() {
                        if let Ok(requests) = reader_pending.lock() {
                            if let Some(active) = requests.get(id) { active.store(false, Ordering::Release); }
                        }
                    }
                }
                Some("shutdown") => {
                    reader_active.store(false, Ordering::Release);
                    revoke(&reader_pending);
                }
                Some("request") => {
                    let Some(id) = envelope.body["requestId"].as_str() else { break; };
                    let Ok(mut requests) = reader_pending.lock() else { break; };
                    if requests.len() >= 256 || requests.contains_key(id) { break; }
                    requests.insert(id.to_owned(), Arc::new(AtomicBool::new(true)));
                }
                _ => (),
            }
            if tx.send(envelope).is_err() { break; }
        }
        reader_active.store(false, Ordering::Release);
        revoke(&reader_pending);
    }).map_err(|_| ())?;
    let hello = rx.recv().map_err(|_| ())?;
    let context = (hello.instance_id, hello.context_id);
    let features = &config.required_features;
    let host_features = hello.body["features"].as_array().ok_or(())?;
    let version = hello.body["version"].as_u64().filter(|n| matches!(n, 1 | 2)).ok_or(())?;
    if hello.body["kind"] != "hello" || !features.iter().all(|f| host_features.contains(&json!(f))) { return Err(()); }
    send(&context, json!({"kind":"welcome","version":version,"features":features}))?;
    send(&context, json!({"kind":"lifecycle","state":"started"}))?;
    let mut state = json!({});
    let mut surface_revision: Option<u64> = None;
    let mut highwater = 0u64;
    loop {
        let envelope = match rx.recv_timeout(std::time::Duration::from_secs(1)) {
            Ok(envelope) => envelope,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if surface_revision.is_some() && context_active.load(Ordering::Acquire) {
                    let observed = initial_state(&config, &context_active, &Arc::new(AtomicBool::new(true)));
                    if let Some(fields) = observed.as_object().filter(|fields| !fields.is_empty()) {
                        let previous = state.clone();
                        state.as_object_mut().ok_or(())?.extend(fields.clone());
                        if state != previous {
                            let revision = surface_revision.as_mut().ok_or(())?;
                            *revision += 1;
                            send(&context, json!({"kind":"state","surfaceId":"controls","revision":revision,"values":state}))?;
                        }
                    }
                }
                continue;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return Err(()),
        };
        let body = envelope.body;
        match body["kind"].as_str() {
            Some("shutdown") => {
                send(&context, json!({"kind":"lifecycle","state":"stopped"}))?;
                return Ok(());
            }
            Some("cancel") => continue,
            Some("request") => (),
            _ => return Err(()),
        }
        let id = body["requestId"].as_str().ok_or(())?;
        let number = id.parse::<u64>().map_err(|_| ())?;
        if number <= highwater || number > 9007199254740991 || number.to_string() != id { return Err(()); }
        highwater = number;
        let op = body["operation"].as_str().ok_or(())?;
        let name = body["name"].as_str().ok_or(())?;
        let payload = body["payload"].clone();
        let active = pending.lock().map_err(|_| ())?.get(id).cloned().ok_or(())?;
        let mut status = "rejected";
        let mut output = json!({});
        let mut changed = false;
        if active.load(Ordering::Acquire) && context_active.load(Ordering::Acquire) {
            if op == "invoke" {
                if let Some(action) = config.actions.iter().find(|action| format!("{}.{}", config.id, action.id) == name) {
                    if !payload_valid(&action.input, &payload) {
                        send(&context, json!({"kind":"result","requestId":id,"operation":op,"name":name,"status":"rejected","payload":{}}))?;
                        pending.lock().map_err(|_| ())?.remove(id);
                        continue;
                    }
                    match invoke(action, payload, Arc::clone(&active), Arc::clone(&context_active)) {
                        Ok(result) => {
                            status = "ok";
                            changed = merge_action_state(&config, action, &mut state, &result)?;
                            output = result;
                        }
                        // An OS call may have committed before failure/cancellation.
                        // Never describe an uncertain effect as safely rejected.
                        Err(()) => status = "unknown",
                    }
                }
            } else if name == "rchub.surface.lifecycle" && payload == json!({"surfaceId":"controls"}) {
                match op {
                    "surface.open" if surface_revision.is_none() => {
                        state = initial_state(&config, &context_active, &active);
                        for action in &config.actions {
                            if action.capability == "apps.read" {
                                if let Ok(observed) = invoke(action, json!({}), Arc::clone(&active), Arc::clone(&context_active)) {
                                    if let Some(fields) = observed.as_object() {
                                        state.as_object_mut().ok_or(())?.extend(fields.clone());
                                    }
                                }
                            }
                        }
                        surface_revision = Some(0);
                        changed = true;
                        status = "ok";
                    }
                    "surface.close" if surface_revision.is_some() => {
                        surface_revision = None;
                        status = "ok";
                    }
                    _ => (),
                }
            }
        } else {
            status = "cancelled";
        }
        send(&context, json!({"kind":"result","requestId":id,"operation":op,"name":name,"status":status,"payload":output}))?;
        pending.lock().map_err(|_| ())?.remove(id);
        if changed {
            if let Some(revision) = &mut surface_revision {
                *revision += 1;
                send(&context, json!({"kind":"state","surfaceId":"controls","revision":revision,"values":state}))?;
            }
        }
    }
}

fn main() {
    if run().is_err() { std::process::exit(2); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_parity_actions_have_bounded_inputs() {
        let apps: Config = serde_json::from_str(include_str!("../../apps/native-source.json")).unwrap();
        let launch = apps.actions.iter().find(|a| a.id == "launch").unwrap();
        assert_eq!(launch.capability, "apps.write");
        assert!(payload_valid(&launch.input, &json!({"name":"Calculator"})));
        assert!(!payload_valid(&launch.input, &json!({"path":"C:\\app.exe"})));
        assert!(!payload_valid(&launch.input, &json!({"name":"x".repeat(121)})));
        let system: Config = serde_json::from_str(include_str!("../../system/native-source.json")).unwrap();
        let battery = system.actions.iter().find(|a| a.id == "battery").unwrap();
        assert_eq!(battery.capability, "system.read");
        assert!(payload_valid(&battery.input, &json!({})));
        assert!(!payload_valid(&battery.input, &json!({"extra":true})));
    }

    fn framed(body: Value) -> Vec<u8> {
        let value = json!({"protocol":PROTOCOL,"instanceId":"instance-1",
            "contextId":"context-1","sender":"host","body":body});
        let raw = serde_json::to_vec(&value).unwrap();
        let mut output = (raw.len() as u32).to_be_bytes().to_vec();
        output.extend(raw);
        output
    }

    #[test]
    fn frame_reader_rejects_truncation_and_oversized_input() {
        assert!(read_frame(&mut &b""[..]).unwrap().is_none());
        assert!(read_frame(&mut &b"\0\0"[..]).is_err());
        assert!(read_frame(&mut &(1048577u32.to_be_bytes())[..]).is_err());
        assert!(read_frame(&mut &b"\0\0\0\x08{}"[..]).is_err());
        assert!(read_frame(&mut &framed(json!({"kind":"shutdown","reason":"host-exit"}))[..]).unwrap().is_some());
    }

    #[test]
    fn action_validation_keeps_enums_types_bounds_and_required_fields() {
        let schema = json!({"type":"object","properties":{
            "action":{"type":"string","maxLength":8,"enum":["lock","sleep"]},
            "force":{"type":"boolean"},
            "delay":{"type":"integer","minimum":0,"maximum":86400}},"required":["action"]});
        assert!(payload_valid(&schema, &json!({"action":"lock"})));
        assert!(payload_valid(&schema, &json!({"action":"sleep","force":false,"delay":0})));
        for invalid in [json!({}),json!({"action":"shell"}),json!({"action":"lock","force":"true"}),
            json!({"action":"lock","delay":86401}),json!({"action":"lock","extra":true})] {
            assert!(!payload_valid(&schema, &invalid));
        }
    }

    #[test]
    fn clipboard_text_is_not_reduced_to_4096() {
        let schema = json!({"type":"string","maxLength":8192});
        assert!(payload_valid(&schema, &json!("x".repeat(8192))));
        assert!(!payload_valid(&schema, &json!("x".repeat(8193))));
    }

    #[test]
    fn clipboard_results_remain_request_private() {
        let config: Config = serde_json::from_str(include_str!("../../clipboard/native-source.json")).unwrap();
        assert!(!config.state.contains_key("text"));
        let secret = json!({"text":"private clipboard contents"});
        let mut state = json!({});
        for capability in ["clipboard.read", "clipboard.write"] {
            let action = config.actions.iter().find(|action| action.capability == capability).unwrap();
            let result = secret.clone();
            assert!(!merge_action_state(&config, action, &mut state, &result).unwrap());
            assert_eq!(state, json!({}));
            assert_eq!(result, secret); // The requesting client still receives the result.
        }
        // A future manifest mistake must not make clipboard contents public.
        let mut misconfigured = config;
        misconfigured.state.insert("text".into(), json!({"type":"string"}));
        let action = misconfigured.actions.iter().find(|action| action.capability == "clipboard.read").unwrap();
        assert!(!merge_action_state(&misconfigured, action, &mut state, &secret).unwrap());
        assert_eq!(state, json!({}));
    }

    #[test]
    fn action_state_publishes_only_declared_fields() {
        let config = Config {
            id: "example".into(), actions: vec![], required_features: vec![],
            state: BTreeMap::from([("volume".into(), json!({"type":"integer"}))]),
        };
        let action = Action { id: "read".into(), capability: "audio.read".into(), input: json!({}) };
        let mut state = json!({});
        assert!(merge_action_state(&config, &action, &mut state, &json!({"volume":42,"private":"secret"})).unwrap());
        assert_eq!(state, json!({"volume":42}));
    }

    #[test]
    fn context_revocation_cancels_every_pending_request() {
        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
        let active = Arc::new(AtomicBool::new(true));
        pending.lock().unwrap().insert("1".into(), Arc::clone(&active));
        revoke(&pending);
        assert!(!active.load(Ordering::Acquire));
    }
}
