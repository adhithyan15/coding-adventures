//! Regression test for #18137: WHITESPACE_ONLY dropped the line break after a
//! `#!` hashbang line, so `#!/usr/bin/env node⏎console.log(1)` printed as
//! `#!/usr/bin/env nodeconsole.log(1);`. The program became part of the
//! comment, and nothing ran.
//!
//! The hashbang is now dropped like any comment. Every `expected` string was
//! captured from the pinned `closure-compiler-v20260915` jar under Java
//! 21.0.10 with `--compilation_level WHITESPACE_ONLY`; upstream drops the
//! hashbang at every level. When `node` is on `PATH`, each output must also
//! print what its source prints.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

/// `(source, upstream WHITESPACE_ONLY output)`.
const CASES: &[(&str, &str)] = &[
    ("#!/usr/bin/env node\nconsole.log(1)\n", "console.log(1);"),
    (
        "#!/usr/bin/env node\n\n// c\nvar x = 1;\nconsole.log(x)\n",
        "var x=1;console.log(x);",
    ),
];

fn compile_ws(source: &str) -> String {
    // Tests run in parallel in one process: a counter keeps scratch dirs apart.
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-ws-hashbang-{}-{n}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir(&dir).expect("fresh scratch dir");
    let path = dir.join("input.js");
    std::fs::write(&path, source).expect("write input");
    let out = Command::new(BINARY)
        .args(["--compilation_level", "WHITESPACE_ONLY", "--js"])
        .arg(&path)
        .output()
        .expect("run closurec");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "{source:?}: exit {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8(out.stdout)
        .expect("UTF-8 output")
        .trim_end_matches('\n')
        .to_string()
}

/// Run `node -e code` and return what it printed, or `None` when node isn't
/// installed. A hashbang is valid at the very start of `node -e` code too.
fn node_stdout(code: &str) -> Option<String> {
    let out = Command::new("node").args(["-e", code]).output().ok()?;
    assert!(
        out.status.success(),
        "node -e {code:?} failed: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[test]
fn whitespace_only_hashbang_matches_upstream() {
    for (source, expected) in CASES {
        assert_eq!(&compile_ws(source), expected, "source: {source:?}");
    }
}

#[test]
fn whitespace_only_hashbang_output_prints_what_the_source_prints() {
    for (source, _) in CASES {
        let Some(source_says) = node_stdout(source) else {
            eprintln!("node not on PATH; skipping the runtime comparison");
            return;
        };
        let compiled = compile_ws(source);
        let compiled_says = node_stdout(&compiled).expect("node ran a moment ago");
        assert_eq!(compiled_says, source_says, "{source:?} -> {compiled:?}");
    }
}
