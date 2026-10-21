//! Regression test for #17886, #17888 and part of #17887: the ECMAScript
//! token patterns agree with JavaScript on where templates, comments and
//! regexes end.
//!
//! ## The bugs
//!
//! Each pattern let a token run past the point where JavaScript ends it. Text
//! the engine treats as a comment or string was then lexed as code, or code
//! was swallowed into a token:
//!
//! - **Templates.** `\$[^{]` matched `$` followed by a backtick, so
//!   `` `$` `` did not end. The template ran on to the next backtick, which
//!   could sit inside a comment, and the comment's text became code (#17886).
//! - **Block comments.** `([^*]|\*[^/])*` ate the `*` that starts `*/`, so
//!   `/***/` and `/* a **/` were not comments (#17888).
//! - **Line comments and hashbangs.** `[^\n]*` didn't stop at `\r`, which also
//!   ends a line, so the code after it was swallowed (#17887).
//! - **Regexes.** A regex literal can never hold a line terminator, but `\r`
//!   was allowed. U+2028 and U+2029 stay with the separate refusal check
//!   (#17875), because not every language's regex engine accepts those
//!   escapes in a shared `.tokens` file.
//!
//! ## What this test checks
//!
//! Each case compiles at SIMPLE and ADVANCED. When `node` is on `PATH`, the
//! output must print exactly what the source prints.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

const CASES: &[&str] = &[
    // #17886: the comment's text must not run.
    "var x = `$` // `; console.log('PWN')\nconsole.log(x)",
    "var x = `a$$`; console.log(x)",
    // Substitution forms (`$${y}`, `${y}$`) are covered by javascript-lexer's
    // unit tests: SIMPLE can't parse any `${…}` until #17854.
    "console.log(`$a`, `$`)",
    // #17888: comments that end in `**/`.
    "var x = 4;\n/***/ console.log(x)",
    "/* note **/ console.log('ok')",
    "/**\n * doc\n **/\nfunction f(){ return 3 }\nconsole.log(f())",
    // #17887: `\r` ends a line comment.
    "var a = 1; // c\rconsole.log('RUN', a)",
];

fn compile(source: &str, level: &str) -> String {
    // Tests run in parallel in one process: a counter keeps scratch dirs apart.
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-token-patterns-{}-{n}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir(&dir).expect("fresh scratch dir");
    let path = dir.join("input.js");
    std::fs::write(&path, format!("{source}\n")).expect("write input");
    let out = Command::new(BINARY)
        .args(["--compilation_level", level, "--js"])
        .arg(&path)
        .output()
        .expect("run closurec");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "{level} {source:?}: exit {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8(out.stdout).expect("UTF-8 output")
}

/// Run `program` under node: `(succeeded, stdout, stderr)`, or `None`
/// without node.
fn run_node(program: &str) -> Option<(bool, String, String)> {
    let run = Command::new("node").args(["-e", program]).output().ok()?;
    Some((
        run.status.success(),
        String::from_utf8_lossy(&run.stdout).into_owned(),
        String::from_utf8_lossy(&run.stderr).into_owned(),
    ))
}

#[test]
fn token_boundaries_match_javascript() {
    // Every case compiles at both levels, with or without node; only the
    // runtime comparison needs node (security review: an early `return`
    // used to skip compiling the remaining cases).
    let have_node = run_node("0").is_some();
    if !have_node {
        eprintln!("node not on PATH; compiling only, no runtime check");
    }
    for source in CASES {
        let outputs: Vec<(&str, String)> =
            ["SIMPLE", "ADVANCED"].iter().map(|l| (*l, compile(source, l))).collect();
        if !have_node {
            continue;
        }
        let (ok, expected, err) = run_node(source).expect("node is on PATH");
        assert!(ok, "source {source:?} must run: {err}");
        for (level, out) in outputs {
            let (ok, got, err) = run_node(&out).expect("node is on PATH");
            assert!(ok, "{level} {source:?} -> {out:?} throws: {err}");
            assert_eq!(got, expected, "{level} {source:?} -> {out:?}");
        }
    }
}
