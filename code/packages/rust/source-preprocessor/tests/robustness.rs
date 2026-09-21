//! Randomised robustness sweep — the no-panic / no-hang oracle.
//!
//! PREP01 §6 ends with a promise: *no input, malformed or otherwise, may cause
//! a panic, an abort, an out-of-bounds index, a non-terminating loop, or a
//! silent truncation of the token stream.* Every other test in this crate
//! checks a case somebody thought of. This one exists for the cases nobody did
//! — which, historically, is how preprocessor bugs actually arrive.
//!
//! ## What this is, and what it is not
//!
//! This is a **deterministic randomised sweep**, not coverage-guided fuzzing.
//! It generates pseudo-random directive-dense programs from a fixed seed and
//! asserts the engine always terminates with `Ok` or `Err` and never panics.
//!
//! It is *not* libFuzzer. A real `cargo-fuzz` target needs a nightly toolchain,
//! which this repo's CI does not use, so it cannot be a merge gate here. The
//! honest summary is that this sweep covers shallow malformed input very well
//! and deep structured input poorly; a coverage-guided target is recorded as
//! follow-up work rather than quietly claimed. Seeds are fixed so a failure is
//! reproducible rather than a once-seen mystery.
//!
//! ## Why the alphabet is directive-dense
//!
//! Uniformly random tokens would almost never produce a well-formed `@if`, so
//! the interesting state machine — nesting, branch selection, unterminated
//! groups — would go untested. The alphabet below is deliberately weighted
//! toward directives and their fragments, so the generator spends its time in
//! the code paths that have edges.

use coding_adventures_source_preprocessor::{
    bounds::Bounds,
    diag::PpError,
    dialect::{Dialect, Directive},
    fs::{IncludeRequest, MemoryFs},
    preprocess,
    source_map::FileId,
};
use lexer::token::{Token, TokenType};

/// xorshift64*. Deterministic, tiny, and good enough to shake out shapes —
/// this is a robustness sweep, not a statistics exercise.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

struct FuzzDialect;

impl Dialect for FuzzDialect {
    fn classify(&self, line: &[Token]) -> Option<Result<Directive, PpError>> {
        match line.first()?.value.as_str() {
            "@if" => Some(Ok(Directive::If(line[1..].to_vec()))),
            "@else" => Some(Ok(Directive::Else)),
            "@end" => Some(Ok(Directive::EndIf)),
            "@include" => Some(Ok(Directive::Include(IncludeRequest {
                spelling: line.get(1).map(|t| t.value.clone()).unwrap_or_default(),
                from: None,
                system: false,
            }))),
            "@define" => Some(Ok(Directive::Define {
                name: line.get(1).map(|t| t.value.clone()).unwrap_or_default(),
                body: line.get(2..).unwrap_or(&[]).to_vec(),
            })),
            _ => None,
        }
    }

    fn eval_condition(&self, tokens: &[Token]) -> Result<bool, PpError> {
        // Total on every input, including empty and nonsense. A dialect that
        // panicked here would mask engine bugs behind dialect bugs.
        Ok(tokens.first().map(|t| t.value != "0").unwrap_or(false))
    }

    fn lex(&self, text: &str, _file: FileId) -> Result<Vec<Token>, PpError> {
        Ok(text
            .lines()
            .enumerate()
            .flat_map(|(i, l)| {
                l.split_whitespace()
                    .map(move |w| Token {
                        type_: TokenType::Name,
                        value: w.to_string(),
                        line: i + 1,
                        column: 1,
                        type_name: None,
                        flags: None,
                        cv: None,
                    })
                    .collect::<Vec<_>>()
            })
            .collect())
    }
}

const ALPHABET: &[&str] = &[
    "@if", "@else", "@end", "@include", "@define", // directives
    "0", "1", "(", ")", "==", "&&", // condition fragments
    "a.oct", "b.oct", "missing.oct", // include targets, some absent
    "fn", "main", "{", "}", ";", "x", // ordinary source
];

fn generate(rng: &mut Rng, max_lines: usize) -> Vec<Token> {
    let lines = 1 + rng.below(max_lines);
    let mut out = Vec::new();
    for line in 1..=lines {
        let words = rng.below(5);
        for _ in 0..words {
            out.push(Token {
                type_: TokenType::Name,
                value: ALPHABET[rng.below(ALPHABET.len())].to_string(),
                line,
                column: 1,
                type_name: None,
                flags: None,
                cv: None,
            });
        }
    }
    out
}

/// The oracle: for any input, the engine returns — and if it returns `Ok`, the
/// source map still describes the stream beside it.
fn check(tokens: Vec<Token>, bounds: Bounds) {
    let mut fs = MemoryFs::new();
    fs.insert("a.oct", "@include b.oct\nalpha");
    fs.insert("b.oct", "beta @if 1 gamma @end");
    let main = fs.insert("<main>", "");

    match preprocess(tokens, main, &FuzzDialect, &mut fs, bounds) {
        Ok(out) => {
            // A silently truncated stream would show up here as a map that no
            // longer matches -- which is the failure this check exists for.
            assert!(
                out.map.check_len(out.tokens.len()).is_ok(),
                "map and stream disagree after a successful run"
            );
        }
        Err(_) => {
            // A located refusal is a correct outcome for malformed input.
        }
    }
}

#[test]
fn random_directive_dense_programs_never_panic() {
    // Tight bounds on purpose: they make the bound-exhaustion paths reachable
    // within short programs, so the sweep actually exercises them instead of
    // always finishing well under budget.
    let bounds = Bounds {
        fuel: 5_000,
        tokens_produced: 2_000,
        total_inclusions: 50,
        include_depth: 10,
        conditional_depth: 12,
        ..Bounds::default()
    };

    for seed in 1..=2_000u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        check(generate(&mut rng, 24), bounds);
    }
}

#[test]
fn deeply_nested_conditionals_terminate_without_blowing_the_stack() {
    // The explicit-stack requirement, checked directly. A recursive engine
    // would abort here rather than produce a diagnostic, and an abort is not
    // catchable -- so "it returned at all" IS the assertion.
    let mut tokens = Vec::new();
    for line in 1..=5_000 {
        tokens.push(Token {
            type_: TokenType::Name,
            value: "@if".into(),
            line,
            column: 1,
            type_name: None,
            flags: None,
            cv: None,
        });
        tokens.push(Token {
            type_: TokenType::Name,
            value: "1".into(),
            line,
            column: 2,
            type_name: None,
            flags: None,
            cv: None,
        });
    }
    check(tokens, Bounds::default());
}

#[test]
fn a_long_include_chain_terminates_without_blowing_the_stack() {
    // Same property for the other recursive-looking traversal. The chain is
    // longer than the depth bound, so the bound is what stops it -- as a
    // diagnostic, not a crash.
    let mut fs = MemoryFs::new();
    for i in 0..500 {
        fs.insert(format!("c{i}.oct"), format!("@include c{}.oct", i + 1));
    }
    fs.insert("c500.oct", "bottom");
    let main = fs.insert("<main>", "");

    let tokens = vec![
        Token { type_: TokenType::Name, value: "@include".into(), line: 1, column: 1, type_name: None, flags: None, cv: None },
        Token { type_: TokenType::Name, value: "c0.oct".into(), line: 1, column: 2, type_name: None, flags: None, cv: None },
    ];

    let r = preprocess(tokens, main, &FuzzDialect, &mut fs, Bounds::default());
    let e = r.err().expect("a 500-deep chain must hit the depth bound");
    assert!(e.to_string().contains("nested deeper"), "{e}");
}

#[test]
fn pathological_grouping_in_a_condition_terminates() {
    let mut tokens = vec![Token {
        type_: TokenType::Name,
        value: "@if".into(),
        line: 1,
        column: 1,
        type_name: None,
        flags: None,
        cv: None,
    }];
    for i in 0..10_000 {
        tokens.push(Token {
            type_: TokenType::Name,
            value: "(".into(),
            line: 1,
            column: i + 2,
            type_name: None,
            flags: None,
            cv: None,
        });
    }
    check(tokens, Bounds::default());
}

#[test]
fn an_empty_program_is_fine() {
    check(Vec::new(), Bounds::default());
}
