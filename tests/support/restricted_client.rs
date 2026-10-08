// A local process fixture, not a model client. It never connects outside loopback.
use std::{io::{Read, Write}, net::TcpStream, time::Duration};

fn request(base: &str, method: &str, path: &str, body: &str) -> bool {
    let (address, prefix) = base.strip_prefix("http://").unwrap().split_once('/').unwrap();
    assert!(address.starts_with("127.0.0.1:"));
    let mut stream = TcpStream::connect(address).unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    write!(stream, "{method} /{prefix}{path} HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer fixture-subscription\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response.starts_with("HTTP/1.1 200")
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--version") { println!("codex-cli fixture"); return; }
    if args.iter().any(|arg| arg == "--help") { println!("--ignore-user-config --ephemeral --strict-config"); return; }
    let model = &args[args.iter().position(|arg| arg == "-m").unwrap() + 1];
    if model == "fail" { std::process::exit(3); }
    if model == "hang" { std::thread::sleep(Duration::from_secs(5)); return; }
    let provider = args.iter().find(|arg| arg.starts_with("model_providers.bqb_gate=")).unwrap();
    let base = provider.split("base_url=\"").nth(1).unwrap().split('"').next().unwrap();
    assert!(std::env::var_os("OPENAI_API_KEY").is_none());
    assert!(std::env::var_os("ANTHROPIC_API_KEY").is_none());
    if model == "catalogue" { assert!(request(base, "GET", "/models?client_version=fixture", "")); }
    let body = format!(r#"{{"model":"{model}","stream":true,"reasoning":{{"effort":"medium"}},"instructions":"INJECTED INSTRUCTIONS","input":[{{"role":"user","content":"COPIED ANSWER CANARY"}}],"tools":[{{"type":"web_search"}}],"tool_choice":"required","previous_response_id":"history"}}"#);
    let path = if model == "route" { "/arbitrary-site" } else { "/responses" };
    if !request(base, "POST", path, &body) { std::process::exit(4); }
    if model == "repeat" && !request(base, "POST", path, &body) { std::process::exit(5); }
}
