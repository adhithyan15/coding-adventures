//! #18820 — after a heredoc, adjacency-gated fusions see the right
//! whitespace.
//!
//! `finalize_heredoc` removed a heredoc's tag token but not its entry in
//! the parallel whitespace flags, so every later token read the flag of the
//! token before it.  `x &.y` after a heredoc lexed `&` `.` (the `.` took the
//! space before `&`), `1 ..2` lexed `.` `.`, and `x = 1.5` lexed `1` `.` `5`.
//!
//! Each case lexes one line after a heredoc and requires the spellings Ruby
//! 3.3's `Ripper.lex` gives for that line (recorded below from Ruby 3.3.6,
//! and re-checked live when a `ruby` executable is on the `PATH`) — the
//! same spellings the line gets with no heredoc before it.

use std::io::Write;
use std::process::{Command, Stdio};

use coding_adventures_ruby_lexer::{source_spelling, tokenize_ruby_for_version};
use lexer::token::TokenType;

/// (line after the heredoc, Ripper's spellings of it).
const CASES: &[(&str, &[&str])] = &[
    ("x &.y", &["x", "&.", "y"]),
    ("x & .y", &["x", "&", ".", "y"]),
    ("1 ..2", &["1", "..", "2"]),
    ("r = 1...2", &["r", "=", "1", "...", "2"]),
    ("f = ->(x) { x }", &["f", "=", "->", "(", "x", ")", "{", "x", "}"]),
    ("x = 1.5", &["x", "=", "1.5"]),
    ("y = 0x1F", &["y", "=", "0x1F"]),
    ("z = 2r", &["z", "=", "2r"]),
    ("w = 3i", &["w", "=", "3i"]),
    ("c = 1.5e-3", &["c", "=", "1.5e-3"]),
    ("d = 1e10", &["d", "=", "1e10"]),
    ("a >>= 1", &["a", ">>=", "1"]),
    ("b += 1", &["b", "+=", "1"]),
    ("q ||= 1", &["q", "||=", "1"]),
    ("v = a >> b", &["v", "=", "a", ">>", "b"]),
];

/// Heredoc shapes to put before each line: one heredoc, two on one line,
/// a squiggly one, and two heredoc lines in a row.
const PREFIXES: &[&str] = &[
    "foo(<<A)\nA\n",
    "foo(<<A, <<B)\nbody\nA\nB\n",
    "s = <<~T\n  text\n  T\n",
    "a = <<A\nA\nb = <<B\nB\n",
];

/// Our spellings of the tokens on the last line of `source`: everything
/// after the last `Newline` before it, minus synthetic tokens and `Eof`.
fn last_line_spellings(source: &str) -> Vec<String> {
    let tokens = tokenize_ruby_for_version(source, "3.3").unwrap();
    let start = tokens
        .iter()
        .rposition(|t| t.type_ == TokenType::Newline)
        .map_or(0, |i| i + 1);
    tokens[start..]
        .iter()
        .filter(|t| t.type_ != TokenType::Eof)
        .filter_map(source_spelling)
        .map(str::to_string)
        .collect()
}

#[test]
fn fusions_after_a_heredoc_match_ruby() {
    for &(line, ruby) in CASES {
        assert_eq!(last_line_spellings(line), ruby, "{line:?} with no heredoc");
        for prefix in PREFIXES {
            let source = format!("{prefix}{line}");
            assert_eq!(last_line_spellings(&source), ruby, "#18820: {source:?}");
        }
    }
}

/// The recorded spellings are what the installed Ruby says, when there is
/// one (CI images without Ruby still run the test above).
#[test]
fn recorded_spellings_match_live_ripper() {
    let script = "puts Ripper.lex(STDIN.read).reject { |(_, ev, _, _)| \
                  %i[on_sp on_nl on_ignored_nl].include?(ev) }.map { |t| t[2] }";
    for &(line, recorded) in CASES {
        let child = Command::new("ruby")
            .args(["-rripper", "-e", script])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        let Ok(mut child) = child else {
            eprintln!("#18820: no `ruby` on PATH; checked against the recorded spellings only");
            return;
        };
        child.stdin.take().unwrap().write_all(line.as_bytes()).unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(out.status.success(), "ruby failed on {line:?}");
        let live: Vec<String> = String::from_utf8(out.stdout).unwrap().lines().map(str::to_string).collect();
        assert_eq!(live, recorded, "Ripper disagrees with the recorded spellings of {line:?}");
    }
}
