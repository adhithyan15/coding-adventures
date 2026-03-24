use cli_builder::{load_spec_from_file, Parser};
use cli_builder::types::ParserOutput;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if text.len() <= width {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return vec!["".to_string()];
    }

    let mut current_line = String::new();
    for word in words {
        if current_line.len() + word.len() + 1 <= width {
            if current_line.is_empty() {
                current_line = word.to_string();
            } else {
                current_line.push(' ');
                current_line.push_str(word);
            }
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    lines
}

fn format_bubble(lines: &[String], is_think: bool) -> String {
    if lines.is_empty() {
        return String::new();
    }

    let max_len = lines.iter().map(|l| l.len()).max().unwrap_or(0);
    let border_top = format!(" {}", "_".repeat(max_len + 2));
    let border_bottom = format!(" {}", "-".repeat(max_len + 2));

    let mut result = vec![border_top];

    if lines.len() == 1 {
        let (start, end) = if is_think { ("(", ")") } else { ("<", ">") };
        result.push(format!("{} {:<width$} {}", start, lines[0], end, width = max_len));
    } else {
        for (i, line) in lines.iter().enumerate() {
            let (start, end) = if is_think {
                ("(", ")")
            } else if i == 0 {
                ("/", "\\")
            } else if i == lines.len() - 1 {
                ("\\", "/")
            } else {
                ("|", "|")
            };
            result.push(format!("{} {:<width$} {}", start, line, end, width = max_len));
        }
    }

    result.push(border_bottom);
    result.join("\n")
}

fn load_cow(cow_name: &str, root: &Path) -> String {
    let mut cow_path = root.join(format!("code/specs/cows/{}.cow", cow_name));
    if !cow_path.exists() {
        cow_path = root.join("code/specs/cows/default.cow");
    }

    let content = fs::read_to_string(cow_path).unwrap_or_else(|_| "Error loading cow".to_string());

    let re = Regex::new(r"(?s)<<EOC;\n(.*?)EOC").unwrap();
    if let Some(caps) = re.captures(&content) {
        if let Some(m) = caps.get(1) {
            return m.as_str().to_string();
        }
    }
    content
}

fn find_root() -> PathBuf {
    let mut curr = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    for _ in 0..10 {
        if curr.join("code/specs/cowsay.json").exists() {
            return curr;
        }
        if let Some(parent) = curr.parent() {
            curr = parent.to_path_buf();
        } else {
            break;
        }
    }
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn main() {
    let root = find_root();
    let spec_path = root.join("code/specs/cowsay.json");
    let spec = load_spec_from_file(spec_path.to_str().unwrap()).expect("Failed to load spec");

    let parser = Parser::new(spec);
    let args: Vec<String> = env::args().collect();

    match parser.parse(&args) {
        Ok(ParserOutput::Parse(result)) => handle_parse_result(result, &root),
        Ok(ParserOutput::Help(h)) => {
            print!("{}", h.text);
        }
        Ok(ParserOutput::Version(v)) => {
            println!("{}", v.version);
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}

fn handle_parse_result(result: cli_builder::types::ParseResult, root: &Path) {
    let flags = result.flags;
    let args = result.arguments;

    // Handle message
    let mut message = String::new();
    if let Some(Value::Array(parts)) = args.get("message") {
        if parts.is_empty() {
            // Read from stdin if it's not a TTY
            if !atty::is(atty::Stream::Stdin) {
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer).ok();
                message = buffer.trim().to_string();
            } else {
                return;
            }
        } else {
            message = parts
                .iter()
                .map(|v| v.as_str().unwrap_or("").to_string())
                .collect::<Vec<_>>()
                .join(" ");
        }
    }

    if message.is_empty() {
        return;
    }

    // Handle modes
    let mut eyes = flags
        .get("eyes")
        .and_then(|v| v.as_str())
        .unwrap_or("oo")
        .to_string();
    let mut tongue = flags
        .get("tongue")
        .and_then(|v| v.as_str())
        .unwrap_or("  ")
        .to_string();

    if flags.get("borg").and_then(|v| v.as_bool()).unwrap_or(false) {
        eyes = "==".to_string();
    }
    if flags.get("dead").and_then(|v| v.as_bool()).unwrap_or(false) {
        eyes = "XX".to_string();
        tongue = "U ".to_string();
    }
    if flags.get("greedy").and_then(|v| v.as_bool()).unwrap_or(false) {
        eyes = "$$".to_string();
    }
    if flags.get("paranoid").and_then(|v| v.as_bool()).unwrap_or(false) {
        eyes = "@@".to_string();
    }
    if flags.get("stoned").and_then(|v| v.as_bool()).unwrap_or(false) {
        eyes = "xx".to_string();
        tongue = "U ".to_string();
    }
    if flags.get("tired").and_then(|v| v.as_bool()).unwrap_or(false) {
        eyes = "--".to_string();
    }
    if flags.get("wired").and_then(|v| v.as_bool()).unwrap_or(false) {
        eyes = "OO".to_string();
    }
    if flags.get("youthful").and_then(|v| v.as_bool()).unwrap_or(false) {
        eyes = "..".to_string();
    }

    // Force 2 chars
    eyes = format!("{:<2}", eyes).chars().take(2).collect();
    tongue = format!("{:<2}", tongue).chars().take(2).collect();

    // Handle wrapping
    let mut lines = Vec::new();
    let nowrap = flags.get("nowrap").and_then(|v| v.as_bool()).unwrap_or(false);
    if nowrap {
        lines = message.split('\n').map(|s| s.to_string()).collect();
    } else {
        let width = flags
            .get("width")
            .and_then(|v| v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)))
            .unwrap_or(40) as usize;

        for line in message.split('\n') {
            if line.is_empty() {
                lines.push("".to_string());
            } else {
                lines.extend(wrap_text(line, width));
            }
        }
    }

    // Handle speech vs thought
    let mut is_think = flags.get("think").and_then(|v| v.as_bool()).unwrap_or(false);
    if let Some(exe) = env::args().next() {
        if exe.ends_with("cowthink") {
            is_think = true;
        }
    }
    let thoughts = if is_think { "o" } else { "\\" };

    // Generate bubble
    let bubble = format_bubble(&lines, is_think);

    // Load and render cow
    let cowfile = flags
        .get("cowfile")
        .and_then(|v| v.as_str())
        .unwrap_or("default");
    let mut cow_template = load_cow(cowfile, root);

    // Replace placeholders
    cow_template = cow_template.replace("$eyes", &eyes);
    cow_template = cow_template.replace("$tongue", &tongue);
    cow_template = cow_template.replace("$thoughts", thoughts);

    // Final unescape
    let cow = cow_template.replace("\\\\", "\\");

    println!("{}", bubble);
    println!("{}", cow);
}

