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
            "@define" => {
                // Function-like when the token after the name is `(`, object-
                // like otherwise.
                //
                // This used to hardcode `params: None`, and that hole was not
                // cosmetic: the sweep could never generate a function-like
                // macro, so the entire argument-pre-expansion path -- the one
                // place the expander recurses natively, and the one a security
                // review used to abort the process from a 21 KB file -- was
                // outside the randomised oracle. A fuzz alphabet that cannot
                // reach a code path is not fuzzing it.
                let name = line.get(1).map(|t| t.value.clone()).unwrap_or_default();
                let params = if line.get(2).is_some_and(|t| t.value == "(") {
                    let mut ps = Vec::new();
                    let mut i = 3;
                    while let Some(t) = line.get(i) {
                        if t.value == ")" {
                            break;
                        }
                        if t.value != "," {
                            ps.push(t.value.clone());
                        }
                        i += 1;
                    }
                    Some(ps)
                } else {
                    None
                };
                let body_at = match &params {
                    Some(ps) => 4 + ps.len() * 2,
                    None => 2,
                };
                Some(Ok(Directive::Define {
                    name,
                    params,
                    body: line.get(body_at..).unwrap_or(&[]).to_vec(),
                }))
            }
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

/// Build a token stream from one source line per `&str`, as `engine.rs`'s
/// helper does. Local to this file because the two suites are independent.
fn program(lines: &[&str]) -> Vec<Token> {
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        for word in line.split_whitespace() {
            out.push(Token {
                type_: TokenType::Name,
                value: word.to_string(),
                line: i + 1,
                column: 1,
                type_name: None,
                flags: None,
                cv: None,
            });
        }
    }
    out
}

const ALPHABET: &[&str] = &[
    "@if", "@else", "@end", "@include", "@define", // directives
    "0", "1", "(", ")", ",", "==", "&&", // condition and argument fragments
    "a.oct", "b.oct", "missing.oct", // include targets, some absent
    "fn", "main", "{", "}", ";", "x", // ordinary source
    // Macro names, deliberately few and deliberately overlapping with the
    // names the sweep also DEFINES, so the generator keeps producing
    // self-referential and mutually-recursive definitions by accident. Those
    // are the shapes that make a naive expander loop forever, and a wide
    // alphabet would almost never hit them.
    "M", "N", "M", "N",
];

/// Emit one whole logical line from `words`.
fn emit_line(out: &mut Vec<Token>, line: usize, words: &[&str]) {
    for (i, w) in words.iter().enumerate() {
        out.push(Token {
            type_: TokenType::Name,
            value: (*w).to_string(),
            line,
            column: i + 1,
            type_name: None,
            flags: None,
            cv: None,
        });
    }
}

fn generate(rng: &mut Rng, max_lines: usize) -> Vec<Token> {
    let lines = 1 + rng.below(max_lines);
    let mut out = Vec::new();
    for line in 1..=lines {
        // One line in six is a PAIRED function-like define or its invocation.
        //
        // Without this the sweep could not reach function-like macros at all,
        // and the alphabet change that was supposed to fix that was cosmetic:
        // the shortest usable `@define M ( x ) body` is six tokens, while the
        // random arm emits at most four per line. A census over these very
        // seeds found 1001 `@define` lines, 22 function-like, and ZERO with a
        // parameter and a non-empty body -- `pre_expand_args` was entered zero
        // times across every sweep.
        //
        // That matters because argument pre-expansion is the only place the
        // expander recurses, and it is where the stack-overflow finding and
        // the quadratic-substitution finding both lived. A fuzz alphabet that
        // cannot reach a code path is not fuzzing it.
        match rng.below(6) {
            0 => emit_line(&mut out, line, &["@define", "M", "(", "x", ")", "x", "N"]),
            1 => emit_line(&mut out, line, &["@define", "N", "(", "y", ")", "M", "(", "y", ")"]),
            2 => emit_line(&mut out, line, &["M", "(", "1", ")"]),
            3 => emit_line(&mut out, line, &["N", "(", "M", "(", "1", ")", ")"]),
            _ => {
                let words = rng.below(5);
                for i in 0..words {
                    out.push(Token {
                        type_: TokenType::Name,
                        value: ALPHABET[rng.below(ALPHABET.len())].to_string(),
                        line,
                        column: i + 1,
                        type_name: None,
                        flags: None,
                        cv: None,
                    });
                }
            }
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

// ===========================================================================
// Slice 2: the same oracle, extended over macro definition and expansion
// ===========================================================================

/// Randomised macro-dense programs terminate and never panic.
///
/// Required by PREP01 §7 rather than optional. Slice 1's sweep exercised an
/// engine that had no macros at all, while the bounds most likely to be
/// attacked — tokens produced, fuel, macro depth, hide-set cost — all guard
/// subsystems that only come into existence in this slice.
///
/// The alphabet reuses two macro names (`M`, `N`) as both definition targets
/// and body content, so the generator produces self-referential and mutually
/// recursive definitions constantly. That is the point: those are exactly the
/// shapes a naive expander loops on, and a realistic alphabet would almost
/// never generate them.
#[test]
fn random_macro_dense_programs_terminate_and_never_panic() {
    let bounds = Bounds {
        fuel: 20_000,
        tokens_produced: 5_000,
        macro_depth: 16,
        total_inclusions: 20,
        include_depth: 8,
        conditional_depth: 12,
        arg_group_depth: 16,
        ..Bounds::default()
    };

    for seed in 1..=3_000u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        check(generate(&mut rng, 20), bounds);
    }
}

/// A doubling chain is refused rather than exhausting memory.
///
/// Forty levels of `@define An A(n-1) A(n-1)` is 2^40 tokens from a few lines
/// of source. This is the attack the "count tokens PRODUCED, not emitted"
/// rule exists for, driven through the whole engine rather than the expander
/// alone.
#[test]
fn a_doubling_macro_chain_is_bounded_end_to_end() {
    let mut lines: Vec<String> = vec!["@define A0 x".to_string()];
    for i in 1..40 {
        lines.push(format!("@define A{i} A{} A{}", i - 1, i - 1));
    }
    lines.push("A39".to_string());
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();

    let mut fs = MemoryFs::new();
    let main = fs.insert("<main>", "");
    let bounds = Bounds { tokens_produced: 50_000, ..Bounds::default() };

    let err = match preprocess(program(&refs), main, &FuzzDialect, &mut fs, bounds) {
        Ok(_) => panic!("a 2^40 doubling chain must hit a bound, not run to completion"),
        Err(e) => e,
    };
    let msg = err.to_string();
    assert!(
        msg.contains("tokens") || msg.contains("budget"),
        "expected a resource diagnostic, got: {msg}"
    );
}

/// A bomb hidden inside a skipped group is never built.
///
/// The group is skipped, so the definitions are never installed and the
/// invocation never expands. If this ever starts failing on a bound, the
/// skipped-group guard has regressed and every expansion attack is reachable
/// from inside `@if 0`.
#[test]
fn a_doubling_chain_inside_a_skipped_group_costs_nothing() {
    let mut lines: Vec<String> = vec!["@if 0".to_string(), "@define A0 x".to_string()];
    for i in 1..40 {
        lines.push(format!("@define A{i} A{} A{}", i - 1, i - 1));
    }
    lines.push("A39".to_string());
    lines.push("@end".to_string());
    lines.push("survivor".to_string());
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();

    let mut fs = MemoryFs::new();
    let main = fs.insert("<main>", "");
    // A budget far too small to build the bomb: it must never be touched.
    let bounds = Bounds { tokens_produced: 200, ..Bounds::default() };

    let out = match preprocess(program(&refs), main, &FuzzDialect, &mut fs, bounds) {
        Ok(o) => o,
        Err(e) => panic!("a skipped group must not expand anything, but: {e}"),
    };
    let values: Vec<&str> = out.tokens.iter().map(|t| t.value.as_str()).collect();
    assert_eq!(values, ["survivor"]);
}

/// The sweep must actually REACH function-like macro expansion.
///
/// This test exists because the previous attempt to fix this blind spot looked
/// right and reached nothing. `FuzzDialect` was taught to parse parameter
/// lists, which appeared to open the path -- but the generator emitted at most
/// four tokens per line while the shortest usable `@define M ( x ) body` is
/// six, so a census over these very seeds found 1001 `@define` lines, 22
/// function-like, and **zero** with a parameter and a non-empty body.
/// `pre_expand_args` was entered zero times.
///
/// A coverage claim that is not measured is a coverage claim that is wrong.
/// So the sweep now asserts its own reach rather than asserting it in a
/// comment.
#[test]
fn the_sweep_actually_generates_function_like_macro_invocations() {
    let mut defines_with_params = 0usize;
    let mut invocations = 0usize;

    for seed in 1..=3_000u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let toks = generate(&mut rng, 20);

        // Group by line, the unit the dialect classifies.
        let mut by_line: std::collections::BTreeMap<usize, Vec<&str>> = Default::default();
        for t in &toks {
            by_line.entry(t.line).or_default().push(t.value.as_str());
        }
        for words in by_line.values() {
            if words.first() == Some(&"@define")
                && words.get(2) == Some(&"(")
                && words.len() > 5
            {
                defines_with_params += 1;
            }
            if matches!(words.first(), Some(&"M") | Some(&"N")) && words.get(1) == Some(&"(") {
                invocations += 1;
            }
        }
    }

    assert!(
        defines_with_params > 100,
        "only {defines_with_params} function-like defines with a body in 3000 seeds — \
         the generator cannot reach argument pre-expansion, which is where the \
         stack-overflow and quadratic-substitution findings both lived"
    );
    assert!(
        invocations > 100,
        "only {invocations} function-like invocations in 3000 seeds — a define \
         nothing calls exercises no expansion"
    );
}
