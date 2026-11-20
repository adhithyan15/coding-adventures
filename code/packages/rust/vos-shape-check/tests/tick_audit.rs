//! # The tick audit (VOS-039d, closing VOS-D089)
//!
//! The linearity tests of this crate do not read a clock; they read a
//! deterministic *step counter* (`src/steps.rs`).  That counter only sees the
//! loops that call `steps::tick()`.  A scanner loop that forgot to tick is
//! invisible to it: if such a loop turned quadratic, every linearity test would
//! stay green.  This file closes that hole with a rule and a self-check.
//!
//! ## The rule
//!
//! Every `while`, `loop` and `for` loop in `src/**/*.rs` (code that is compiled
//! outside `#[cfg(test)]`) must do exactly one of:
//!
//! * call `tick()` in its own body (any path: `crate::steps::tick()`,
//!   `steps::tick()`), where "its own body" means not inside a *nested* loop's
//!   body: an inner loop's tick counts the inner loop's iterations, not the
//!   outer loop's; or
//! * carry, on the line immediately above the loop keyword, a comment
//!   `// bounded: <reason>` with a non-empty reason, saying why the trip count
//!   is bounded by a constant or by one step of an already-ticked outer loop.
//!
//! ```text
//!   // bounded: fixed pair of editions
//!   for edition in [Edition::E2021, Edition::E2024] { ... }      <- accepted
//!
//!   for t in toks { tick(); ... }                                 <- accepted
//!
//!   while j < n { j += 1; }                                       <- REJECTED
//! ```
//!
//! ## How loops are found (no second lexer)
//!
//! The audit does not scan Rust text.  It lexes each file with the crate's own
//! RS01 grammar-driven lexer and indexes it with [`FileCtx`] (bracket pairs and
//! the `#[cfg(test)]` mask), then works on tokens:
//!
//! * `while` and `loop` are always loops;
//! * `for` is a loop only if an `in` token follows before the body's `{`
//!   (at bracket depth zero).  `impl X for Y { .. }` has no `in`; the
//!   higher-ranked bound `for<'a>` is skipped by its `<`;
//! * the body is the first `{` after the keyword that is not inside a
//!   bracket pair (a closure in the head sits in parentheses);
//! * a tick is the three tokens `tick ( )` inside the body.
//!
//! The only text the audit reads directly is the single line above the loop
//! keyword, for the `// bounded:` marker: the lexer discards comments, so the
//! marker cannot be found among tokens.
//!
//! ## What this does not cover
//!
//! Iterator chains (`.iter().map(..).collect()`) hide loops without a loop
//! keyword.  The audited crate uses them for small bounded things only; a new
//! chain over input-sized data in a scanner is a review matter, not a test.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use vos_shape_check::lexer::{lex_documents, Edition, Kind, Token};
use vos_shape_check::syntax::FileCtx;

/// One loop found in a file: token indices of its keyword and body braces.
#[derive(Clone, Copy, Debug)]
struct Loop {
    kw: usize,
    open: usize,
    close: usize,
}

/// How an audited loop satisfied the rule (or did not).
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Verdict {
    Ticked,
    Bounded,
    Unaccounted,
}

/// The body-opening `{` of a loop whose keyword is at `kw`, if it is a loop.
///
/// Walk forward from the keyword, hopping over every bracket pair, to the
/// first `{`.  For `for`, an `in` token must have been seen on the way
/// (otherwise this is `impl X for Y` or similar) and a `<` right after the
/// keyword means the higher-ranked bound `for<'a>`.
fn body_open(f: &FileCtx, kw: usize) -> Option<usize> {
    let t = &f.toks;
    let is_for = t[kw].text == "for";
    if is_for
        && t.get(kw + 1)
            .is_some_and(|x| x.kind == Kind::Punct && x.text == "<")
    {
        return None;
    }
    let mut seen_in = false;
    let mut j = kw + 1;
    while j < t.len() {
        let x = &t[j];
        if x.kind == Kind::Punct {
            match x.text.as_str() {
                "{" => return (!is_for || seen_in).then_some(j),
                "(" | "[" => {
                    j = f.pair[j] + 1;
                    continue;
                }
                ";" | "}" | ")" | "]" => return None,
                _ => {}
            }
        } else if x.kind == Kind::Ident && x.text == "in" {
            seen_in = true;
        }
        j += 1;
    }
    None
}

/// Every loop of a file, in token order.
fn loops_of(f: &FileCtx) -> Vec<Loop> {
    let mut out = Vec::new();
    for (i, x) in f.toks.iter().enumerate() {
        let kw = x.kind == Kind::Ident
            && matches!(x.text.as_str(), "while" | "loop" | "for")
            && !f.in_attr[i];
        if !kw {
            continue;
        }
        if let Some(open) = body_open(f, i) {
            out.push(Loop {
                kw: i,
                open,
                close: f.pair[open],
            });
        }
    }
    out
}

/// Is `tick ( )` among the loop's own tokens (nested loops' bodies skipped)?
fn ticks_itself(f: &FileCtx, l: &Loop, all: &[Loop]) -> bool {
    let t: &[Token] = &f.toks;
    let mut k = l.open + 1;
    while k < l.close {
        if let Some(inner) = all.iter().find(|m| m.kw == k) {
            k = inner.close + 1;
            continue;
        }
        if t[k].kind == Kind::Ident
            && t[k].text == "tick"
            && t.get(k + 1)
                .is_some_and(|x| x.kind == Kind::Punct && x.text == "(")
            && t.get(k + 2)
                .is_some_and(|x| x.kind == Kind::Punct && x.text == ")")
        {
            return true;
        }
        k += 1;
    }
    false
}

/// Does the line above `line` (1-based) read `// bounded: <reason>`?
fn marked_bounded(src: &str, line: u32) -> bool {
    let Some(prev) = (line as usize).checked_sub(2) else {
        return false;
    };
    src.lines()
        .nth(prev)
        .and_then(|l| l.trim_start().strip_prefix("// bounded:"))
        .is_some_and(|reason| !reason.trim().is_empty())
}

/// Judge every loop of one lexed file: `(line, verdict)`.
fn judge(path: &str, src: &str, toks: Vec<Token>) -> Vec<(u32, Verdict)> {
    let f = FileCtx::from_tokens(path, toks).unwrap_or_else(|e| panic!("{path}: {}", e.message));
    let all = loops_of(&f);
    all.iter()
        .map(|l| {
            let line = f.toks[l.kw].line;
            let v = if ticks_itself(&f, l, &all) {
                Verdict::Ticked
            } else if marked_bounded(src, line) {
                Verdict::Bounded
            } else {
                Verdict::Unaccounted
            };
            (line, v)
        })
        .collect()
}

/// Judge many sources with ONE grammar lexer (building it costs about a second).
fn judge_all(sources: &[(String, String)]) -> Vec<(String, Vec<(u32, Verdict)>)> {
    let docs: Vec<&str> = sources.iter().map(|(_, s)| s.as_str()).collect();
    lex_documents(&docs, Edition::E2021)
        .into_iter()
        .zip(sources)
        .map(|(r, (path, src))| {
            let toks = r.unwrap_or_else(|e| panic!("{path}: cannot lex: {}", e.message));
            (path.clone(), judge(path, src, toks))
        })
        .collect()
}

/// Every `.rs` file under `dir`, sorted, as (crate-relative path, text).
fn rust_files(root: &Path, dir: &Path, out: &mut Vec<(String, String)>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.expect("directory entry").path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            rust_files(root, &p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            let rel = p
                .strip_prefix(root)
                .expect("under root")
                .to_string_lossy()
                .replace('\\', "/");
            let text = fs::read_to_string(&p).unwrap_or_else(|e| panic!("{rel}: {e}"));
            out.push((rel, text));
        }
    }
}

/// The crate's own source obeys the rule.
#[test]
fn every_loop_in_the_crate_ticks_or_is_marked_bounded() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut sources = Vec::new();
    rust_files(root, &root.join("src"), &mut sources);
    assert!(
        sources.len() >= 15,
        "found only {} source files",
        sources.len()
    );

    let (mut ticked, mut bounded) = (0usize, 0usize);
    let mut bad = Vec::new();
    for (path, verdicts) in judge_all(&sources) {
        for (line, v) in verdicts {
            match v {
                Verdict::Ticked => ticked += 1,
                Verdict::Bounded => bounded += 1,
                Verdict::Unaccounted => bad.push(format!("{path}:{line}")),
            }
        }
    }
    eprintln!("tick audit: {ticked} ticked loops, {bounded} marked `// bounded:`");
    assert!(
        bad.is_empty(),
        "loops with neither `tick()` in their body nor a `// bounded: <reason>` line above \
         (a quadratic regression in them would be invisible to the step counter; see \
         tests/tick_audit.rs):\n  {}",
        bad.join("\n  ")
    );
    // a scan that found nothing would pass vacuously
    assert!(
        ticked + bounded >= 100,
        "audit saw only {} loops",
        ticked + bounded
    );
}

/// Negative controls: the checker accepts and rejects what it says it does.
#[test]
fn the_audit_itself_is_checked() {
    let cases: [(&str, &str, &[Verdict]); 12] = [
        (
            "unticked while",
            "fn f() { let mut i = 0; while i < 9 { i += 1; } }\n",
            &[Verdict::Unaccounted],
        ),
        (
            "unticked loop",
            "fn f() { loop { break; } }\n",
            &[Verdict::Unaccounted],
        ),
        (
            "unticked for",
            "fn f(v: &[u8]) { for x in v { let _ = x; } }\n",
            &[Verdict::Unaccounted],
        ),
        (
            "ticked while",
            "fn f() { let mut i = 0; while i < 9 { crate::steps::tick(); i += 1; } }\n",
            &[Verdict::Ticked],
        ),
        (
            "bare and path-qualified tick",
            "fn f(v: &[u8]) {\n for x in v { tick(); }\n for y in v { steps::tick(); }\n}\n",
            &[Verdict::Ticked, Verdict::Ticked],
        ),
        (
            "bounded marker with a reason",
            "fn f() {\n    // bounded: iterates a 3-element table\n    for x in [1, 2, 3] { let _ = x; }\n}\n",
            &[Verdict::Bounded],
        ),
        (
            "bounded marker without a reason",
            "fn f() {\n    // bounded:\n    for x in [1, 2, 3] { let _ = x; }\n}\n",
            &[Verdict::Unaccounted],
        ),
        (
            "marker two lines above does not count",
            "fn f() {\n    // bounded: a reason\n\n    for x in [1, 2, 3] { let _ = x; }\n}\n",
            &[Verdict::Unaccounted],
        ),
        (
            "inner tick does not excuse the outer loop",
            "fn f(v: &[u8]) {\n for x in v {\n  for y in v { tick(); }\n }\n}\n",
            &[Verdict::Unaccounted, Verdict::Ticked],
        ),
        (
            "impl-for and for<'a> are not loops",
            "struct S;\nimpl Clone for S { fn clone(&self) -> S { S } }\nfn g<F>(f: F) where F: for<'a> Fn(&'a u8) {}\n",
            &[],
        ),
        (
            "a for in a string or a cfg(test) item is not a loop",
            "const S: &str = \"while true { }\";\n#[cfg(test)]\nmod tests { fn t() { loop {} } }\n",
            &[],
        ),
        (
            "labelled loop with a closure in its head",
            "fn f(v: &[u8]) { 'a: while v.iter().any(|x| { *x > 0 }) { tick(); break 'a; } }\n",
            &[Verdict::Ticked],
        ),
    ];
    let sources: Vec<(String, String)> = cases
        .iter()
        .map(|(name, src, _)| ((*name).to_string(), (*src).to_string()))
        .collect();
    for ((name, got), (_, _, want)) in judge_all(&sources).iter().zip(cases.iter()) {
        let got: Vec<Verdict> = got.iter().map(|(_, v)| *v).collect();
        assert_eq!(&got, want, "{name}");
    }
}

// ---------------------------------------------------------------------------
// Linearity of the paths that used to be invisible to the counter
// ---------------------------------------------------------------------------
//
// Before the audit, the lexer's adapter loops, the shebang scanner and the
// outer loop of `mask_cfg_test` did not tick, so no linearity test could see
// them go quadratic.  Each case below grows ONLY that path (n, 4n, 16n units)
// and asserts the count grows linearly; the counts are meaningful because
// those loops now tick.  (A block comment, however long, is one token of the
// RS01 grammar lexer and costs this crate's adapter one step, so no case grows
// the length of a single comment.)

use common::*;

#[test]
fn lexing_is_linear_in_tokens_comments_and_shebang_scans() {
    // plain tokens: the adapter's main loop
    scales_linearly("lexing: plain tokens", 1_500, 4, |n| {
        let src = format!("{HEADER}{}", "fn a() {} ".repeat(n));
        run_counted(&lib(&src)).1
    });
    // many block comments: the adapter drops each in its own inner scan
    scales_linearly("lexing: many block comments", 1_500, 4, |n| {
        let src = format!("{HEADER}{}fn a() {{}}\n", "/* c */ ".repeat(n));
        run_counted(&lib(&src)).1
    });
    // `#!` followed by comments: the shebang test skips them one by one
    scales_linearly("lexing: comments after `#!`", 1_500, 4, |n| {
        let src = format!(
            "#!{}[no_std]\n#![forbid(unsafe_code)]\n",
            "/* c */ ".repeat(n)
        );
        run_counted(&lib(&src)).1
    });
}

#[test]
fn the_cfg_test_mask_is_linear_in_attributes_kept_and_dropped() {
    // attributes that are NOT `cfg(test)`: the mask's outer loop visits each
    scales_linearly("mask_cfg_test: kept attributes", 1_500, 4, |n| {
        let src = format!("{HEADER}{}", "#[inline] fn a() {} ".repeat(n));
        run_counted(&lib(&src)).1
    });
    // `cfg(test)` items interleaved with kept ones: each is cut out once
    scales_linearly("mask_cfg_test: alternating", 600, 4, |n| {
        let src = format!(
            "{HEADER}{}",
            "#[cfg(test)] fn t() {} #[inline] fn a() {} ".repeat(n)
        );
        run_counted(&lib(&src)).1
    });
}
