//! Regression test for #18146 (Part 3): Lexical declarations in single-statement contexts.
//!
//! ECMA-262 §14.1.1 defines Statement to exclude LexicalDeclaration (`let` and `const`).
//! A LexicalDeclaration is only permitted in StatementListItem contexts (blocks, case/default
//! clauses, module/script top-level, and class static blocks).
//!
//! Subordinate statements in `if`, `while`, `do-while`, `for`, `with`, and `label:` take a
//! single Statement. Inputs like `if (1) let\nx = 1` or `if (1) const x = 1;` must be refused
//! fail-closed and never compiled to a different program (e.g. `let x=1;`).

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

/// Programs that must be refused at parse stage, never compiled to a different program.
const REFUSED: &[&str] = &[
    "if (1) let\nx = 1;",
    "if (1) const x = 1;",
    "if (x) let y = 2;",
    "if (x) const y = 2;",
    "while (1) let x = 1;",
    "while (1) const x = 1;",
    "for (;;) const x = 1;",
    "for (;;) let x = 1;",
    "with (o) let x = 1;",
    "lbl: let x = 1;",
    "lbl: const x = 1;",
];

/// Valid programs with lexical declarations in statement-list contexts.
const VALID: &[&str] = &[
    "if (1) { let x = 1; console.log(x); }",
    "if (1) { const x = 2; console.log(x); }",
    "while (false) { const x = 1; } console.log('ok');",
    "let x = 10; const y = 20; console.log(x + y);",
    "switch (1) { case 1: let a = 30; console.log(a); break; default: const b = 40; console.log(b); }",
    "class A { static { let s = 99; console.log(s); } }",
];

fn run_compiler(source: &str, level: &str) -> std::process::Output {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-if-let-18146-{}-{n}-{}",
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
        .args(["--compilation_level", level, "--strict_mode_input=false", "--js"])
        .arg(&path)
        .output()
        .expect("run closurec");
    let _ = std::fs::remove_dir_all(&dir);
    out
}

#[test]
fn single_statement_lexical_declarations_are_refused() {
    for src in REFUSED {
        for level in ["SIMPLE", "ADVANCED"] {
            let out = run_compiler(src, level);
            assert!(
                !out.status.success(),
                "Program must be refused at {level}, but succeeded: {src:?}\nOutput: {}",
                String::from_utf8_lossy(&out.stdout)
            );
        }
    }
}

#[test]
fn valid_lexical_declarations_in_blocks_and_lists_compile() {
    for src in VALID {
        for level in ["SIMPLE", "ADVANCED", "WHITESPACE_ONLY"] {
            let out = run_compiler(src, level);
            assert!(
                out.status.success(),
                "Valid program failed at {level}: {src:?}\nStderr: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            let compiled = String::from_utf8(out.stdout).expect("UTF-8 stdout");

            // If node is available, verify runtime semantics match source.
            if let Ok(src_node) = Command::new("node").args(["-e", src]).output() {
                if src_node.status.success() {
                    let compiled_node = Command::new("node").args(["-e", &compiled]).output().expect("run compiled in node");
                    assert_eq!(
                        src_node.stdout, compiled_node.stdout,
                        "Mismatch at {level} for {src:?}"
                    );
                }
            }
        }
    }
}
