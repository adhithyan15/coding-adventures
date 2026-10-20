//! Regression test for #16009: a long expression chain must compile or fail
//! with a diagnostic, never abort on a stack overflow (CLOC42).
//!
//! ## The crash
//!
//! ```js
//! var x = 1;
//! console.log(x + x + x + /* … 10,000 terms … */ + x);
//! ```
//!
//! The grammar parses that chain as one flat CST node, so the parser's
//! rule-depth guard never sees it. The bridge folds it into a typed AST 10,000
//! levels deep, and the recursive passes overflowed the stack:
//! `thread 'main' has overflowed its stack`, SIGABRT, exit 134, no output.
//!
//! ## The fix this pins
//!
//! 1. closurec measures chain depth before building the typed AST
//!    (`check_chain_spine` with `MAX_CHAIN_SPINE` = 16,384). Past the bound it
//!    fails at the parse stage with a located diagnostic and no output.
//! 2. The compile runs on a thread with a 1 GiB stack, and constant-fold's
//!    worker got the same, so every chain *within* the bound compiles on every
//!    platform.
//!
//! The second test is the one that guards the future. It compiles, in a debug
//! build, the deepest chain of every shape that the bound admits. If a later
//! change grows a pass's stack frames enough to make the bound unsafe, this
//! test aborts in CI before a user's build does.

use coding_adventures_javascript_parser::{check_chain_spine, parse_javascript_typed};
use coding_adventures_javascript_tokens::EsVersion;
use std::path::PathBuf;
use std::process::{Command, Output};

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

/// Mirrors `closurec::run::MAX_CHAIN_SPINE` (a binary crate's items can't be
/// imported here). The diagnostic assertion below ties the two together: if
/// they drift apart, the "spine limit 16384" text stops matching.
const MAX_CHAIN_SPINE: usize = 16_384;

/// One chain shape: `prefix` + `link` × n + `suffix`.
struct Shape {
    name: &'static str,
    prefix: &'static str,
    link: &'static str,
    suffix: &'static str,
}

const SHAPES: &[Shape] = &[
    Shape { name: "additive", prefix: "var x=1;console.log(x", link: "+x", suffix: ");" },
    Shape { name: "logical", prefix: "var x=1;console.log(x", link: "&&x", suffix: ");" },
    Shape { name: "member", prefix: "var x={};console.log(x", link: ".a", suffix: ");" },
    Shape { name: "optional", prefix: "var x={};console.log(x", link: "?.a", suffix: ");" },
    Shape { name: "index", prefix: "var x=[];console.log(x", link: "[0]", suffix: ");" },
    Shape {
        name: "call",
        prefix: "var f=function(){return f};f",
        link: "()",
        suffix: ";",
    },
];

impl Shape {
    fn source(&self, links: usize) -> String {
        format!("{}{}{}", self.prefix, self.link.repeat(links), self.suffix)
    }

    fn within_bound(&self, links: usize) -> bool {
        let cst = parse_javascript_typed(&self.source(links), EsVersion::Es2025)
            .expect("chain shapes parse");
        check_chain_spine(&cst, MAX_CHAIN_SPINE).is_ok()
    }

    /// The largest link count the bound admits. Height grows linearly in the
    /// link count, so two samples locate it; the exact edge is then
    /// confirmed against the real measure.
    fn deepest_admitted(&self) -> usize {
        let height = |links: usize| {
            let cst = parse_javascript_typed(&self.source(links), EsVersion::Es2025)
                .expect("chain shapes parse");
            (1..MAX_CHAIN_SPINE * 4)
                .find(|&limit| check_chain_spine(&cst, limit).is_ok())
                .expect("small chains have a small height")
        };
        // Sample past the prelude (`var f=function(){…}` is itself ~50 units
        // deep), where the chain is the tallest thing in the program.
        let (h100, h200) = (height(100), height(200));
        let per_link = (h200 - h100) / 100;
        let mut links = 100 + (MAX_CHAIN_SPINE - h100) / per_link;
        while !self.within_bound(links) {
            links -= 1;
        }
        while self.within_bound(links + 1) {
            links += 1;
        }
        links
    }
}

/// Write `source` to a fresh file and compile it.
fn compile(tag: &str, source: &str, level: &str) -> Output {
    let dir = std::env::temp_dir().join(format!("closurec-chain-depth-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path: PathBuf = dir.join(format!("{tag}-{level}.js"));
    std::fs::write(&path, source).expect("write input");
    let out = Command::new(BINARY)
        .args(["--compilation_level", level, "--js"])
        .arg(&path)
        .output()
        .expect("run closurec");
    let _ = std::fs::remove_file(&path);
    out
}

/// The process ended with an exit code — not killed by a signal, which is how
/// a stack overflow shows up (SIGABRT).
fn exited_normally(out: &Output) -> bool {
    out.status.code().is_some()
}

#[test]
fn the_issue_repro_fails_cleanly_instead_of_aborting() {
    // 10,000 `+` operands cost about 20,000 spine units, past the bound.
    let source = format!("var x = 1;\nconsole.log({});\n", vec!["x"; 10_000].join(" + "));
    for level in ["SIMPLE", "ADVANCED"] {
        let out = compile("issue-16009", &source, level);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(exited_normally(&out), "{level}: killed by a signal: {stderr}");
        assert!(!out.status.success(), "{level}: a 10,000-term chain is past the bound");
        assert!(out.stdout.is_empty(), "{level}: no output on failure");
        assert!(
            stderr.contains(&format!(
                "{level} compilation failed at parse stage: JavaScript expression chain at 2:13 \
                 is nested too deeply to compile (spine limit {MAX_CHAIN_SPINE})"
            )),
            "{level}: unexpected diagnostic: {stderr}"
        );
    }
}

#[test]
fn every_chain_shape_compiles_up_to_the_bound_and_fails_cleanly_past_it() {
    // Sequential on purpose: each deep compile can touch a few hundred MiB of
    // stack in a debug build, so running them in parallel would only add
    // memory pressure.
    for shape in SHAPES {
        let deepest = shape.deepest_admitted();
        for level in ["SIMPLE", "ADVANCED"] {
            let at_bound = compile(shape.name, &shape.source(deepest), level);
            assert!(
                at_bound.status.success(),
                "{} × {deepest} at {level} must compile: {:?} {}",
                shape.name,
                at_bound.status,
                String::from_utf8_lossy(&at_bound.stderr),
            );

            let past = compile(shape.name, &shape.source(deepest + 1), level);
            let stderr = String::from_utf8_lossy(&past.stderr);
            assert!(exited_normally(&past), "{} past the bound: killed: {stderr}", shape.name);
            assert!(!past.status.success() && past.stdout.is_empty());
            assert!(stderr.contains("is nested too deeply to compile"), "{stderr}");
        }
    }
}
