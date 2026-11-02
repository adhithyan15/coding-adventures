//! #18814 — differential tests: the linear fusion passes against the
//! quadratic ones they replaced.
//!
//! The production passes in `lib.rs` were rewritten to compact the token
//! vector in one pass instead of calling `Vec::remove` once per fusion.
//! The rewrite must not change a single token, so every input here is lexed
//! twice, up to the end of the state machine, and then run through:
//!
//! - the production pipeline, [`RubyLexer::run_era_token_fusions`], and
//! - the reference pipeline, [`RubyLexer::ref_apply_era_token_fusions`],
//!   whose rewritten steps are the pre-#18814 bodies kept verbatim in
//!   `reference_passes.rs`.
//!
//! After **every** step the two lexers must hold the same tokens (type,
//! value, line, column, flags — the whole `Token`) and the same
//! `whitespace_before_token` flags, so a divergence names the pass that
//! caused it rather than surfacing three passes later.
//!
//! The inputs are:
//!
//! 1. every string literal in this crate's `lib.rs` — which is every
//!    snippet the crate's unit tests lex, plus some harmless extras such as
//!    error messages;
//! 2. every `.rb` file in the Ruby corpora this repo already keeps
//!    (`code/grammars/ruby/corpus/` and `ruby-parser`'s fixtures);
//! 3. a thousand seeded random "fragment soups" built from the shapes
//!    the passes fuse (`0` `x1F`, `1` `.` `5`, `e` `+` `3`, `>` `>=`, `&`
//!    `.`, `-` `>`, `2` `r`, …) and their near misses (a space or a line
//!    break in between), which reach orderings no hand-written test does.
//!
//! Each input runs under every era in [`ERA_VERSIONS`], because the era
//! decides which passes run at all.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use lexer::token::Token;

use super::{RubyLexer, ERA_VERSIONS};

/// One step's view of a lexer: the step name, the tokens and the
/// whitespace flags right after it ran.
type Snapshot = (String, Vec<Token>, Vec<bool>);

/// Lex `source` up to the end of the state machine, before any fusion pass,
/// and return the tokens and whitespace flags at that point.  `None` when
/// the engine itself refuses the input (no pipeline runs on it in
/// production either).
///
/// The era only gates the fusion passes (`run_era_token_fusions` is the one
/// reader of `self.era`), so this stream is the same for every era and is
/// lexed once per input, with the 1.8 baseline.
pub(crate) fn prefusion(source: &str) -> Option<(Vec<Token>, Vec<bool>)> {
    let mut lexer = RubyLexer::new("1.8").ok()?;
    lexer.push(source).ok()?;
    lexer.drain_machine().ok()?;
    lexer.remove_retired_heredoc_tags();
    Some((lexer.tokens, lexer.whitespace_before_token))
}

/// Two scratch lexers per era — one for each pipeline — built once and
/// reloaded with each input's pre-fusion stream, because building a lexer
/// (its state machine) costs far more than the passes themselves.
struct Pipelines {
    by_era: Vec<(&'static str, RubyLexer, RubyLexer)>,
}

impl Pipelines {
    fn new() -> Self {
        let by_era = ERA_VERSIONS
            .iter()
            .map(|&era| {
                (
                    era,
                    RubyLexer::new(era).unwrap(),
                    RubyLexer::new(era).unwrap(),
                )
            })
            .collect();
        Self { by_era }
    }

    /// Run both pipelines on `pre` under `era`, returning their per-step
    /// snapshots (production first).
    fn run(&mut self, pre: &(Vec<Token>, Vec<bool>), era: &str) -> (Vec<Snapshot>, Vec<Snapshot>) {
        let (_, new, old) = self.by_era.iter_mut().find(|(e, _, _)| *e == era).unwrap();
        new.tokens = pre.0.clone();
        new.whitespace_before_token = pre.1.clone();
        old.tokens = pre.0.clone();
        old.whitespace_before_token = pre.1.clone();
        let mut new_steps = Vec::new();
        new.run_era_token_fusions(&mut |name, lx| {
            new_steps.push((
                name.to_string(),
                lx.tokens.clone(),
                lx.whitespace_before_token.clone(),
            ));
        });
        let mut old_steps = Vec::new();
        old.ref_apply_era_token_fusions(&mut |name, lx| {
            old_steps.push((
                name.to_string(),
                lx.tokens.clone(),
                lx.whitespace_before_token.clone(),
            ));
        });
        (new_steps, old_steps)
    }
}

/// Tallies across a whole differential run, so a test can prove it was not
/// vacuous: every rewritten pass must actually have fused something.
#[derive(Default)]
struct Tally {
    inputs: usize,
    refused: usize,
    /// Step name → number of runs on which that step removed tokens.
    fused: BTreeMap<String, usize>,
}

/// Compare the two pipelines on one input under each of `eras`, and panic
/// with the first diverging step.
fn assert_same(source: &str, eras: &[&str], pipes: &mut Pipelines, tally: &mut Tally) {
    tally.inputs += 1;
    let Some(pre) = prefusion(source) else {
        tally.refused += 1;
        return;
    };
    for &era in eras {
        let (new_steps, old_steps) = pipes.run(&pre, era);
        let names = |s: &[Snapshot]| s.iter().map(|(n, _, _)| n.clone()).collect::<Vec<_>>();
        assert_eq!(
            names(&new_steps),
            names(&old_steps),
            "step order differs (era {era}, source {source:?})"
        );
        let mut before = pre.0.len();
        for ((name, new_toks, new_ws), (_, old_toks, old_ws)) in new_steps.iter().zip(&old_steps) {
            assert_eq!(
                new_toks, old_toks,
                "#18814: `{name}` changed the token stream (era {era}, source {source:?})"
            );
            assert_eq!(
                new_ws, old_ws,
                "#18814: `{name}` changed the whitespace flags (era {era}, source {source:?})"
            );
            if new_toks.len() < before {
                *tally.fused.entry(name.clone()).or_default() += 1;
            }
            before = new_toks.len();
        }
    }
}

// ---------------------------------------------------------------------------
// Inputs 1: the string literals of lib.rs.
// ---------------------------------------------------------------------------

/// Every string literal in a Rust source file, unescaped.
///
/// Small and deliberately forgiving — it only has to find the snippets the
/// unit tests lex, not validate Rust.  It knows `//` comments (skipped, so a
/// quote in prose does not open a string), char literals and lifetimes
/// (`'"'` is not a string, `'a` is not a char), raw strings (`r"…"`,
/// `r#"…"#`) and the escapes Rust allows in a normal string (`\n`, `\t`,
/// `\r`, `\0`, `\\`, `\"`, `\'`, `\x41`, `\u{1F600}`, and a `\` at the end
/// of a line, which swallows the line break and the next line's indent).
fn rust_string_literals(src: &str) -> Vec<String> {
    let c: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let ident = |ch: char| ch.is_alphanumeric() || ch == '_';
    while i < c.len() {
        match c[i] {
            '/' if c.get(i + 1) == Some(&'/') => {
                while i < c.len() && c[i] != '\n' {
                    i += 1;
                }
            }
            '\'' => {
                // A char literal is `'x'` or `'\…'`; anything else is a
                // lifetime or a label, whose `'` we just step over.
                if c.get(i + 1) == Some(&'\\') {
                    i += 2;
                    while i < c.len() && c[i] != '\'' {
                        i += 1;
                    }
                    i += 1;
                } else if c.get(i + 2) == Some(&'\'') {
                    i += 3;
                } else {
                    i += 1;
                }
            }
            'r' if (i == 0 || !ident(c[i - 1]))
                && matches!(c.get(i + 1), Some('"') | Some('#')) =>
            {
                let mut j = i + 1;
                let mut hashes = 0;
                while c.get(j) == Some(&'#') {
                    hashes += 1;
                    j += 1;
                }
                if c.get(j) != Some(&'"') {
                    i += 1;
                    continue;
                }
                j += 1;
                let start = j;
                loop {
                    if j >= c.len() {
                        return out;
                    }
                    if c[j] == '"' && (1..=hashes).all(|h| c.get(j + h) == Some(&'#')) {
                        break;
                    }
                    j += 1;
                }
                out.push(c[start..j].iter().collect());
                i = j + 1 + hashes;
            }
            '"' => {
                let mut s = String::new();
                let mut j = i + 1;
                while j < c.len() && c[j] != '"' {
                    if c[j] != '\\' {
                        s.push(c[j]);
                        j += 1;
                        continue;
                    }
                    j += 1;
                    match c.get(j) {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('r') => s.push('\r'),
                        Some('0') => s.push('\0'),
                        Some('x') => {
                            let hex: String = c[j + 1..(j + 3).min(c.len())].iter().collect();
                            if let Ok(v) = u8::from_str_radix(&hex, 16) {
                                s.push(v as char);
                            }
                            j += 2;
                        }
                        Some('u') => {
                            let close = (j..c.len()).find(|&k| c[k] == '}').unwrap_or(j);
                            let hex: String = c[(j + 2).min(close)..close].iter().collect();
                            if let Some(ch) =
                                u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32)
                            {
                                s.push(ch);
                            }
                            j = close;
                        }
                        Some('\n') => {
                            while c.get(j + 1).is_some_and(|ch| ch.is_whitespace()) {
                                j += 1;
                            }
                        }
                        Some(&other) => s.push(other),
                        None => {}
                    }
                    j += 1;
                }
                out.push(s);
                i = j + 1;
            }
            _ => i += 1,
        }
    }
    out
}

#[test]
fn rust_string_literal_harvester_reads_escapes_and_raw_strings() {
    let src = r##"let a = "x\n\t\"q\"\\"; // "not this"
        let b = r#"raw "quoted""#; let c = '"'; let d: &'static str = "\u{41}\x42";
        let e = "joined \
                 line";"##;
    assert_eq!(
        rust_string_literals(src),
        vec![
            "x\n\t\"q\"\\".to_string(),
            "raw \"quoted\"".to_string(),
            "AB".to_string(),
            "joined line".to_string(),
        ]
    );
}

fn lib_rs_literals() -> Vec<String> {
    let mut v = rust_string_literals(include_str!("lib.rs"));
    v.sort();
    v.dedup();
    v
}

// ---------------------------------------------------------------------------
// Inputs 2: the repo's Ruby corpora.
// ---------------------------------------------------------------------------

fn rb_files_under(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|e| {
        panic!(
            "#18814 differential corpus {} is missing: {e}",
            dir.display()
        )
    });
    for entry in entries {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rb_files_under(&path, out);
        } else if path.extension().is_some_and(|e| e == "rb") {
            out.push(path);
        }
    }
}

fn corpus_files() -> Vec<(String, String)> {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rb_files_under(&crate_dir.join("../../../grammars/ruby/corpus"), &mut files);
    rb_files_under(&crate_dir.join("../ruby-parser/tests/fixtures"), &mut files);
    files.sort();
    files
        .into_iter()
        .map(|p| {
            let src = std::fs::read_to_string(&p).unwrap();
            (p.display().to_string(), src)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Inputs 3: seeded fragment soups.
// ---------------------------------------------------------------------------

/// The pieces the soups are built from: the halves of every fused pair,
/// the whole fused spellings, heredoc openers and terminators, and the
/// separators that must block a fusion (space, tab, line break, `;`).
#[rustfmt::skip]
const FRAGMENTS: &[&str] = &[
    // radix integers
    "0", "x1F", "b101", "o17", "d10", "xZZ", "0x1F",
    // floats and exponents
    "1", "5", "1_000", ".", "e", "E", "e10", "E5", "1.5", "1e+3", "2.5e-3",
    // ranges
    "..", "...",
    // shifts and comparisons
    ">", ">=", ">>", ">>=", "<", "<<", "<=",
    // compound assignment halves
    "=", "+", "-", "*", "/", "%", "**", "&", "|", "^", "||", "&&",
    // lambda, safe navigation, numeric suffixes
    "->", "&.", "r", "i", "2r", "3i",
    // brackets and punctuation
    "(", ")", "[", "]", "{", "}", ",", ";", ":", "::",
    // names, keywords, numbered params
    "a", "b", "foo", "puts", "x", "_1", "_2", "if", "unless", "while", "until",
    "end", "do", "|x|", "then", "else", "begin", "rescue", "super", "yield",
    "return", "nil", "true", "def",
    // literals
    "\"s\"", "'t'", ":sym", "%w[a b]",
    // heredocs: openers (after `x = ` / `(` so they are openers, not `<<`)
    // and terminator lines (#18814 review)
    "x = <<A", "(<<A", ",<<~B", "x = <<-A", "\nA\n", "\n  B\n", "\nbody\n",
    // separators: the near misses
    " ", " ", " ", "\t", "\n", "\n", "\n\n",
];

/// A tiny deterministic generator (64-bit LCG, Knuth's MMIX constants), so
/// a failing soup reproduces from its index alone.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn soup(seed: u64) -> String {
    let mut rng = Lcg(seed);
    let len = 1 + rng.below(40);
    (0..len)
        .map(|_| FRAGMENTS[rng.below(FRAGMENTS.len())])
        .collect()
}

// ---------------------------------------------------------------------------
// The tests.
// ---------------------------------------------------------------------------

/// The crate's own test snippets, under every era.
#[test]
fn linear_passes_match_reference_on_lib_rs_test_inputs() {
    let inputs = lib_rs_literals();
    assert!(
        inputs.len() > 500,
        "harvested only {} literals",
        inputs.len()
    );
    let mut pipes = Pipelines::new();
    let mut tally = Tally::default();
    for src in &inputs {
        assert_same(src, ERA_VERSIONS, &mut pipes, &mut tally);
    }
    eprintln!(
        "#18814 differential (lib.rs literals): {} inputs x every era, {} refused by the engine, fusions per step: {:?}",
        tally.inputs, tally.refused, tally.fused
    );
    for pass in [
        "fuse_range_ops",
        "fuse_radix_integers",
        "fuse_float_literals",
        "fuse_right_shifts",
        "fuse_compound_assigns",
        "fuse_lambda_arrow",
        "fuse_numeric_suffixes",
        "fuse_safe_nav",
    ] {
        assert!(
            tally.fused.get(pass).copied().unwrap_or(0) > 0,
            "#18814 differential never exercised a fusion in `{pass}`"
        );
    }
}

/// The repo's Ruby corpora, under every era.
#[test]
fn linear_passes_match_reference_on_ruby_corpora() {
    let files = corpus_files();
    assert!(files.len() >= 40, "found only {} corpus files", files.len());
    let mut pipes = Pipelines::new();
    let mut tally = Tally::default();
    for (_name, src) in &files {
        assert_same(src, ERA_VERSIONS, &mut pipes, &mut tally);
    }
    eprintln!(
        "#18814 differential (corpora, {} files): {} inputs x every era, {} refused, fusions per step: {:?}",
        files.len(),
        tally.inputs,
        tally.refused,
        tally.fused
    );
}

/// Seeded fragment soups, under every era.
#[test]
fn linear_passes_match_reference_on_fragment_soups() {
    const SOUPS: u64 = 1000;
    let mut pipes = Pipelines::new();
    let mut tally = Tally::default();
    for seed in 0..SOUPS {
        let src = soup(seed);
        assert_same(&src, ERA_VERSIONS, &mut pipes, &mut tally);
    }
    eprintln!(
        "#18814 differential (soups): {} inputs x every era, {} refused, fusions per step: {:?}",
        tally.inputs, tally.refused, tally.fused
    );
    for pass in [
        "fuse_range_ops",
        "fuse_radix_integers",
        "fuse_float_literals",
        "fuse_right_shifts",
        "fuse_compound_assigns",
        "fuse_lambda_arrow",
        "fuse_numeric_suffixes",
        "fuse_safe_nav",
    ] {
        assert!(
            tally.fused.get(pass).copied().unwrap_or(0) > 10,
            "#18814 soups exercised `{pass}` only {:?} times",
            tally.fused.get(pass)
        );
    }
}

// ---------------------------------------------------------------------------
// Dropping the engine's step trace (`forget_engine_trace`).
// ---------------------------------------------------------------------------

/// Lex `source` to completion with the engine trace dropped after every
/// `limit` steps; return the tokens and diagnostics.
fn lex_with_trace_limit(
    source: &str,
    limit: usize,
) -> (Result<Vec<Token>, String>, Vec<super::Diagnostic>) {
    let mut lexer = RubyLexer::new("3.3").unwrap();
    lexer.engine_trace_limit = limit;
    let result = lexer
        .push(source)
        .map_err(|e| e.to_string())
        .and_then(|()| lexer.finish())
        .map(|()| lexer.drain_tokens());
    (result, lexer.diagnostics.clone())
}

/// Dropping the trace after EVERY step — so in every engine state, inside
/// strings, interpolations, heredoc openers, regexes and comments — lexes
/// exactly what never dropping it does.
#[test]
fn dropping_the_engine_trace_never_changes_the_stream() {
    let mut inputs: Vec<String> = lib_rs_literals();
    inputs.extend(corpus_files().into_iter().map(|(_, src)| src));
    inputs.extend((0..100).map(soup));
    for src in &inputs {
        assert_eq!(
            lex_with_trace_limit(src, 1),
            lex_with_trace_limit(src, usize::MAX),
            "dropping the engine trace changed the lexing of {src:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Heredoc tags: removed in one sweep (#18814 review).
// ---------------------------------------------------------------------------

/// What a whole lex leaves behind: the tokens, the whitespace flags (the
/// fusion passes read them, so they are part of the result), and the
/// diagnostics, or the error.
type WholeLex = Result<(Vec<Token>, Vec<bool>, Vec<super::Diagnostic>), String>;

/// Lex `source` under `era` from start to finish, finalizing heredocs with
/// the pre-#18814 body (`reference`) or the production one.
fn whole_lex(source: &str, era: &str, reference: bool) -> WholeLex {
    let mut lexer = RubyLexer::new(era).unwrap();
    lexer.reference_heredocs = reference;
    lexer.push(source).map_err(|e| e.to_string())?;
    lexer.finish()?;
    let tokens = lexer.drain_tokens();
    Ok((tokens, lexer.whitespace_before_token.clone(), lexer.diagnostics.clone()))
}

/// Inputs that put many heredocs, of every variant, in many positions.
fn heredoc_inputs() -> Vec<String> {
    let mut v: Vec<String> = [
        "x = <<A\nbody\nA\nputs x\n",
        "foo(<<A, <<B)\na\nA\nb\nB\nx&.y\n",
        "foo(<<A,<<A,<<A)\nA\nA\nA\n1..2\n",
        "a = <<~X\n    one\n  two\n  X\nb = <<-Y\n  y\n  Y\nc = 1.5e-3\n",
        "x = <<A\nunterminated\n",
        "x = <<A; y = <<B\nA\nB\nf = ->(z) { z }\n",
        "x = <<END\nEND\na >>= 2\n",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    // k openers on one line, then k terminators: the shape that was
    // quadratic.
    for k in [1, 2, 3, 7, 50] {
        v.push(format!("foo({}1)\n{}x&.y\n", "<<A,".repeat(k), "A\n".repeat(k)));
    }
    v
}

/// Removing the heredoc tags in one sweep at the end lexes exactly what
/// removing each one at once (the pre-#18814 code) did: same tokens, same
/// whitespace flags, same diagnostics.
///
/// Only inputs that contain `<<` can open a heredoc, so the others are
/// skipped (both paths are the same code for them).  One era is enough:
/// heredocs are lexed before any era-gated pass runs, and 3.3 runs all of
/// those passes over the result.
#[test]
fn batched_heredoc_tag_removal_matches_reference() {
    let mut inputs = heredoc_inputs();
    inputs.extend(lib_rs_literals());
    inputs.extend(corpus_files().into_iter().map(|(_, src)| src));
    inputs.extend((0..1000).map(soup));
    let mut with_heredocs = 0;
    for src in inputs.iter().filter(|s| s.contains("<<")) {
        let new = whole_lex(src, "3.3", false);
        let old = whole_lex(src, "3.3", true);
        assert_eq!(new, old, "#18814: heredoc tag removal changed the lexing of {src:?}");
        if new.as_ref().is_ok_and(|(t, _, _)| t.iter().any(|t| t.value.starts_with("<<") && t.value.contains('\n'))) {
            with_heredocs += 1;
        }
    }
    assert!(with_heredocs > 50, "only {with_heredocs} inputs had a heredoc");
}
