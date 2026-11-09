//! The independent observer: classify an `strace -f -v` trace (VER08b
//! section 6). Pure text processing, so it is unit-tested on recorded
//! fixtures on every platform.
//!
//! strace lines look like this (with `-f -o file`, a pid prefix is added):
//!
//! ```text
//!   4242  execve("/x/test", ["/x/test", "--rcg-child"], []) = 0
//!   4242  write(2, "RCG-ARMED 9f3c\n", 15) = 15       <- the boundary marker
//!   4242  brk(NULL)                         = 0x55d0
//!   4243  futex(0x7f.., FUTEX_WAIT, ... <unfinished ...>
//!   4243  <... futex resumed>)              = 0
//!   4242  openat(AT_FDCWD, "/etc/passwd", O_RDONLY <unfinished ...>
//!   4242  --- SIGSYS {si_signo=SIGSYS, ...} ---
//!   4242  +++ killed by SIGSYS (core dumped) +++
//!   4242  +++ exited with 0 +++
//! ```
//!
//! Only lines *after* the marker are evidence about the library. strace prints
//! the call before the kernel acts on it (ptrace stops precede seccomp), so a
//! call that seccomp kills still appears in the trace; that is what lets the
//! observer and the enforcer be compared.

use crate::policy::{family_of, RuntimePolicy};
use std::collections::BTreeMap;

/// One recognised trace line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A syscall entry (complete or `<unfinished ...>`).
    Syscall {
        /// Process/thread id.
        pid: u32,
        /// Syscall name.
        name: String,
        /// Raw argument text between the parentheses (may be truncated).
        args: String,
    },
    /// A `<... name resumed>` continuation (not counted again).
    Resumed,
    /// `--- SIGxxx ... ---`
    Signal {
        /// Signal name, e.g. `SIGSYS`.
        name: String,
    },
    /// `+++ exited with N +++`
    Exited {
        /// Process id.
        pid: u32,
        /// Exit status.
        code: i32,
    },
    /// `+++ killed by SIGxxx +++`
    Killed {
        /// Process id.
        pid: u32,
        /// Signal name.
        signal: String,
    },
    /// Anything else (blank, strace diagnostics).
    Other,
}

/// Parse one trace line.
pub fn parse_line(line: &str) -> Event {
    let line = line.trim_end();
    if line.is_empty() {
        return Event::Other;
    }
    // With -f the line starts with the pid; "[pid N]" form also exists when
    // output goes to a terminal.
    let (pid, rest) = split_pid(line);
    let rest = rest.trim_start();
    if let Some(body) = rest.strip_prefix("+++ ") {
        if let Some(code) = body.strip_prefix("exited with ") {
            let digits: String = code.chars().take_while(char::is_ascii_digit).collect();
            return Event::Exited {
                pid,
                code: digits.parse().unwrap_or(-1),
            };
        }
        if let Some(sig) = body.strip_prefix("killed by ") {
            let name: String = sig
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            return Event::Killed { pid, signal: name };
        }
        return Event::Other;
    }
    if let Some(body) = rest.strip_prefix("--- ") {
        let name: String = body
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        return Event::Signal { name };
    }
    if rest.starts_with("<...") {
        return Event::Resumed;
    }
    // name(args...
    let Some(open) = rest.find('(') else {
        return Event::Other;
    };
    let name = &rest[..open];
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Event::Other;
    }
    Event::Syscall {
        pid,
        name: name.to_string(),
        args: rest[open + 1..].to_string(),
    }
}

fn split_pid(line: &str) -> (u32, &str) {
    if let Some(rest) = line.strip_prefix("[pid") {
        let rest = rest.trim_start();
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if let Some(after) = rest[digits.len()..].strip_prefix(']') {
            return (digits.parse().unwrap_or(0), after);
        }
    }
    let digits: String = line.chars().take_while(char::is_ascii_digit).collect();
    if !digits.is_empty() {
        let rest = &line[digits.len()..];
        if rest.starts_with(char::is_whitespace) {
            return (digits.parse().unwrap_or(0), rest);
        }
    }
    (0, line)
}

/// How the traced main process ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ending {
    /// `+++ exited with N +++`
    Exited(i32),
    /// `+++ killed by SIG... +++`
    Killed(String),
    /// No epilogue line was found.
    Unknown,
}

/// One post-arm call outside the allowed set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// Syscall name.
    pub name: String,
    /// Coarse family (`fs`, `net`, ... or `unclassified`).
    pub family: &'static str,
    /// The argument text (truncated to 80 chars) for the report.
    pub args: String,
}

/// Everything the observer concluded from one trace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Analysis {
    /// True if the ARMED marker line was found.
    pub armed: bool,
    /// Landlock/seccomp status text from the marker line (`landlock=1 ...`).
    pub marker_detail: String,
    /// Environment strings from the first `execve` (None if none seen).
    pub exec_env: Option<Vec<String>>,
    /// Post-arm syscall name -> count.
    pub post_arm: BTreeMap<String, u64>,
    /// Count of syscalls before the marker (setup, not evidence).
    pub pre_arm_calls: u64,
    /// Post-arm calls outside baseline + declared.
    pub violations: Vec<Violation>,
    /// How the main process ended.
    pub ending: Ending,
    /// SIGSYS was delivered to the tracee.
    pub saw_sigsys: bool,
}

/// Classify a whole trace against `policy`. `nonce` identifies the marker.
pub fn analyze(trace: &str, nonce: &str, policy: &RuntimePolicy) -> Analysis {
    let marker = format!("RCG-ARMED {nonce}");
    let mut analysis = Analysis {
        armed: false,
        marker_detail: String::new(),
        exec_env: None,
        post_arm: BTreeMap::new(),
        pre_arm_calls: 0,
        violations: Vec::new(),
        ending: Ending::Unknown,
        saw_sigsys: false,
    };
    let mut main_pid: Option<u32> = None;
    for line in trace.lines() {
        match parse_line(line) {
            Event::Syscall { pid, name, args } => {
                if main_pid.is_none() {
                    main_pid = Some(pid);
                }
                if name == "execve" && analysis.exec_env.is_none() {
                    analysis.exec_env = Some(parse_execve_env(&args));
                }
                if !analysis.armed {
                    analysis.pre_arm_calls += 1;
                    if name == "write" && args.contains(&marker) {
                        analysis.armed = true;
                        analysis.marker_detail = marker_detail(&args, &marker);
                    } else {
                        continue;
                    }
                }
                *analysis.post_arm.entry(name.clone()).or_insert(0) += 1;
                if !policy.allows(&name, &args) {
                    analysis.violations.push(Violation {
                        family: family_of(&name),
                        args: args.chars().take(80).collect(),
                        name,
                    });
                }
            }
            Event::Signal { name } => {
                if name == "SIGSYS" {
                    analysis.saw_sigsys = true;
                }
            }
            Event::Exited { pid, code } => {
                if Some(pid) == main_pid {
                    analysis.ending = Ending::Exited(code);
                }
            }
            Event::Killed { pid, signal } => {
                if Some(pid) == main_pid {
                    analysis.ending = Ending::Killed(signal);
                }
            }
            Event::Resumed | Event::Other => {}
        }
    }
    analysis
}

/// Extract the text after the nonce inside the marker's quoted string:
/// `"RCG-ARMED 9f3c landlock=1 abi=7 seccomp=1\n"` -> `landlock=1 abi=7 seccomp=1`.
fn marker_detail(args: &str, marker: &str) -> String {
    let after = args.split(marker).nth(1).unwrap_or("");
    let end = after.find("\\n").unwrap_or(after.len());
    after[..end].trim().to_string()
}

/// Pull the environment array out of `execve("path", [argv], [envp]) = 0`.
///
/// With `strace -v` the third argument is printed in full as `["A=B", ...]`.
/// The parse is deliberately small: find the last top-level `[` ... `]`
/// group and split its double-quoted strings.
pub fn parse_execve_env(args: &str) -> Vec<String> {
    let mut groups: Vec<(usize, usize)> = Vec::new();
    let (mut depth, mut start, mut in_string, mut escaped) = (0usize, 0usize, false, false);
    for (index, ch) in args.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '[' => {
                if depth == 0 {
                    start = index;
                }
                depth += 1;
            }
            ']' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    groups.push((start, index));
                }
            }
            _ => {}
        }
    }
    let Some(&(from, to)) = groups.get(1) else {
        return Vec::new();
    };
    let body = &args[from + 1..to];
    let mut vars = Vec::new();
    let (mut current, mut in_string, mut escaped) = (String::new(), false, false);
    for ch in body.chars() {
        if in_string {
            if escaped {
                current.push(ch);
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
                vars.push(std::mem::take(&mut current));
            } else {
                current.push(ch);
            }
        } else if ch == '"' {
            in_string = true;
        }
    }
    vars
}

/// Environment variable *names* in `exec_env` that the policy does not allow.
pub fn env_violations(exec_env: &[String], policy: &RuntimePolicy) -> Vec<String> {
    exec_env
        .iter()
        .map(|entry| entry.split('=').next().unwrap_or(entry).to_string())
        .filter(|name| !policy.env_name_allowed(name))
        .collect()
}
