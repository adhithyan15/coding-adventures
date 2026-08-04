use std::io::{self, BufRead, Write};

const PROTOCOL: &str = "chief-agent-stdio-v1";

fn main() {
    let mode = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "success".to_string());
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line.expect("read test host input");
        if mode == "eof" {
            return;
        }
        if mode == "exit" {
            std::process::exit(42);
        }
        if mode == "malformed" {
            writeln!(stdout, "not-json").expect("write malformed response");
            stdout.flush().expect("flush malformed response");
            continue;
        }
        let message_id = extract_field(&line, "message_id").expect("message_id field");
        let response_id = if mode == "mismatch" {
            "01980000-0000-7000-8000-000000000099"
        } else {
            message_id
        };
        writeln!(
            stdout,
            "{{\"protocol\":\"{PROTOCOL}\",\"kind\":\"response\",\"input_message_id\":\"{response_id}\",\"content_type\":\"text/plain; charset=utf-8\",\"payload_b64\":\"d29ybGQ=\"}}"
        )
        .expect("write response");
        stdout.flush().expect("flush response");
    }
}

fn extract_field<'a>(line: &'a str, field: &str) -> Option<&'a str> {
    let marker = format!("\"{field}\":\"");
    let value = line.split_once(&marker)?.1;
    value.split_once('"').map(|(value, _)| value)
}
