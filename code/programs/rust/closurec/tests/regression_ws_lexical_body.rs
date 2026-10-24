//! Regression test for #18124: WHITESPACE_ONLY must not flatten a block that
//! holds a lone `let`/`const` into a statement body slot.
//!
//! `if(a){let x=1;}` printed as `if(a)let x=1;`, and `while(o){let x=1;}` as
//! `while(o)let x=1;`. A body slot takes a Statement, and a lexical
//! declaration is not one, so neither output parses. Upstream keeps the
//! braces (`isBlockDeclOrDo`). The gap-032, gap-074/076/079, gap-080 and do-body
//! flatten passes now refuse a body that starts with a lexical declaration
//! (`starts_lexical_declaration`).
//!
//! Every `expected` string was captured from the pinned
//! `closure-compiler-v20260915` jar under Java 21.0.10 with
//! `--compilation_level WHITESPACE_ONLY`. When `node` is on `PATH`, each output
//! must also parse.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

/// `(source, upstream WHITESPACE_ONLY output)`.
const CASES: &[(&str, &str)] = &[
    ("if(a){let x=1;}", "if(a){let x=1};"),
    ("if(a){let x=1}", "if(a){let x=1};"),
    ("while(o){let x=1;}", "while(o){let x=1};"),
    ("with(o){const x=1;}", "with(o){const x=1};"),
    ("if(a){}else{let x=1;}", "if(a);else{let x=1};"),
    ("do{let x=1;}while(a)", "do{let x=1}while(a);"),
    ("for(;;){const x=1;}", "for(;;){const x=1};"),
    ("if(a){let [x]=b;}", "if(a){let [x]=b};"),
    ("if(a){let {x}=b;}", "if(a){let {x}=b};"),
    ("if(a){using(b);}", "if(a)using(b);"),
    ("if(a){const x=1}else{b()}", "if(a){const x=1}else b();"),
    ("if(a){b()}", "if(a)b();"),
    ("while(o){x++}", "while(o)x++;"),
    ("l:{let x=1}", "l:{let x=1};"),
    ("if(a)b();else{let c=1;}", "if(a)b();else{let c=1};"),
];

fn compile_ws(source: &str) -> String {
    // Tests run in parallel in one process: a counter keeps scratch dirs apart.
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-ws-lexical-{}-{n}-{}",
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

#[test]
fn whitespace_only_lexical_bodies_match_upstream() {
    for (source, expected) in CASES {
        assert_eq!(&compile_ws(source), expected, "source: {source:?}");
    }
}

#[test]
fn whitespace_only_lexical_body_output_parses() {
    for (source, _) in CASES {
        let out = compile_ws(source);
        // `new Function` parses without running; skipped without node.
        let Ok(run) = Command::new("node")
            .args(["-e", "new Function(process.argv[1])", &out])
            .output()
        else {
            eprintln!("node not on PATH; skipping the parse check");
            return;
        };
        assert!(
            run.status.success(),
            "{source:?} -> {out:?} does not parse: {}",
            String::from_utf8_lossy(&run.stderr),
        );
    }
}
