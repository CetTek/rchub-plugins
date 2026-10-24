//! MIT. Harmless std-only ABI2 example, not a validator/sandbox or production plugin.
//! Only canonical host IPC for the accompanying bounded toggle descriptor.
//! The small field reader is NOT a general JSON parser. Use a strict parser and
//! the SDK contract for a real application accepting more varied payloads.
use std::io::{self, Read, Write};
fn field(raw: &str, key: &str) -> String {
    let marker = format!("\"{key}\":\"");
    let start = raw.find(&marker).unwrap() + marker.len();
    raw[start..].split('"').next().unwrap().into()
}
fn send(i: &str, c: &str, body: &str) {
    let raw = format!("{{\"protocol\":\"rchub-app-v2-experimental\",\"instanceId\":\"{i}\",\"contextId\":\"{c}\",\"sender\":\"plugin\",\"body\":{body}}}");
    let mut out = io::stdout().lock();
    out.write_all(&(raw.len() as u32).to_be_bytes()).unwrap();
    out.write_all(raw.as_bytes()).unwrap(); out.flush().unwrap();
}
fn result(i: &str,c: &str,id: &str,op: &str,name: &str,status: &str,payload: &str) {
    send(i,c,&format!("{{\"kind\":\"result\",\"requestId\":\"{id}\",\"operation\":\"{op}\",\"name\":\"{name}\",\"status\":\"{status}\",\"payload\":{payload}}}"));
}
fn main() {
    let mut input=io::stdin().lock();
    let (mut enabled,mut revision,mut surface)=(false,0,false);
    let mut subscription: Option<(String,String)>=None;
    loop {
        let mut header=[0;4]; if input.read_exact(&mut header).is_err(){break}
        let n=u32::from_be_bytes(header) as usize; if n==0 || n>65536 {break}
        let mut bytes=vec![0;n]; input.read_exact(&mut bytes).unwrap();
        let raw=String::from_utf8(bytes).unwrap();
        let i=field(&raw,"instanceId"); let c=field(&raw,"contextId");
        match field(&raw,"kind").as_str() {
            "hello"=> {
                if !raw.contains("\"version\":2") { break }
                send(&i,&c,"{\"kind\":\"welcome\",\"version\":2,\"features\":[\"native-process-v1\",\"events-v1\",\"surfaces-v1\"]}");
                send(&i,&c,"{\"kind\":\"lifecycle\",\"state\":\"started\"}"); continue
            },
            "shutdown"=>{send(&i,&c,"{\"kind\":\"lifecycle\",\"state\":\"stopped\"}");break},
            "request"=>(),
            _=>continue,
        }
        let rid=field(&raw,"requestId"); let op=field(&raw,"operation"); let name=field(&raw,"name");
        let mut status="ok"; let mut payload="{}";
        match op.as_str() {
            "surface.open"=>{surface=true;revision=0},
            "surface.close"=>surface=false,
            "subscribe"=>subscription=Some((rid.clone(),name.clone())),
            "unsubscribe"=>subscription=None,
            "invoke" if name.ends_with(".apply")=>{enabled=raw.contains("\"enabled\":true");payload="{\"accepted\":true}"},
            _=>status="rejected",
        }
        result(&i,&c,&rid,&op,&name,status,payload);
        if op=="invoke" && name.ends_with(".apply") {
            if let Some((sid,event))=&subscription {
                send(&i,&c,&format!("{{\"kind\":\"event\",\"subscriptionId\":\"{sid}\",\"name\":\"{event}\",\"payload\":{{\"revision\":1}}}}"));
            }
        }
        if surface && matches!(op.as_str(),"surface.open"|"invoke") {
            revision+=1;
            send(&i,&c,&format!("{{\"kind\":\"state\",\"surfaceId\":\"controls\",\"revision\":{revision},\"values\":{{\"enabled\":{enabled},\"revision\":{},\"items\":[{{\"label\":\"Generic item\"}}]}}}}",if enabled{1}else{0}));
        }
    }
}
