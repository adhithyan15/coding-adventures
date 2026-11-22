//! Hostile-input security regression and fuzz tests for `actionscript-compiler` (SWF03).
//!
//! Covers:
//! - CWE-674: Deep `{}` / `()` / `[]` / `!` / object-literal nesting on a 1 MiB stack
//! - CWE-1321: Prototype pollution (`__proto__`, `prototype`, `constructor`) across AS1/AS2/AS3
//! - Infinite-loop instruction step limit enforcement
//! - Unterminated literals, malformed tokens, and invalid UTF-8-derived sources
//! - Seeded 1,000-case token-soup fuzz campaign asserting zero panics

#![forbid(unsafe_code)]

use actionscript_compiler::{
    compile_actionscript, run_actionscript, AsCompilerError, AsDialect, MAX_AST_DEPTH,
};

struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_usize(&mut self, bound: usize) -> usize {
        (self.next_u64() as usize) % bound
    }
}

fn run_on_1mib_stack<F: FnOnce() + Send + 'static>(f: F) {
    let handle = std::thread::Builder::new()
        .stack_size(1024 * 1024)
        .spawn(f)
        .expect("spawn 1 MiB test thread");
    handle.join().expect("1 MiB stack test thread panicked");
}

#[test]
fn hostile_deep_brace_nesting_returns_recursion_depth_exceeded() {
    run_on_1mib_stack(|| {
        for depth in [MAX_AST_DEPTH + 1, 200, 500] {
            let src = format!("{}{}", "{".repeat(depth), "}".repeat(depth));
            for dialect in [AsDialect::As1, AsDialect::As2, AsDialect::As3] {
                let err = compile_actionscript(&src, dialect, "deep_braces")
                    .expect_err("deep brace nesting must fail with RecursionDepthExceeded");
                assert_eq!(err, AsCompilerError::RecursionDepthExceeded);
            }
        }
    });
}

#[test]
fn hostile_deep_paren_bracket_and_bang_nesting_bounded() {
    run_on_1mib_stack(|| {
        let parens = format!("{}1{}", "(".repeat(200), ")".repeat(200));
        let err = compile_actionscript(&parens, AsDialect::As3, "deep_parens")
            .expect_err("deep paren nesting must fail");
        assert_eq!(err, AsCompilerError::RecursionDepthExceeded);

        let brackets = format!("{}1{}", "[".repeat(200), "]".repeat(200));
        let err = compile_actionscript(&brackets, AsDialect::As3, "deep_brackets")
            .expect_err("deep bracket nesting must fail");
        assert_eq!(err, AsCompilerError::RecursionDepthExceeded);

        let bangs = format!("{}true;", "!".repeat(200));
        let err = compile_actionscript(&bangs, AsDialect::As3, "deep_bangs")
            .expect_err("deep unary ! nesting must fail");
        assert_eq!(err, AsCompilerError::RecursionDepthExceeded);

        let obj_nest = format!("var x = {}1{};", "{a:".repeat(200), "}".repeat(200));
        let err = compile_actionscript(&obj_nest, AsDialect::As2, "deep_obj")
            .expect_err("deep object literal nesting must fail");
        assert_eq!(err, AsCompilerError::RecursionDepthExceeded);
    });
}

#[test]
fn hostile_cwe1321_prototype_pollution_blocked_all_dialects() {
    let payloads = [
        "var o = {}; o.__proto__ = 1; trace(o.__proto__);",
        "var o = {}; o.prototype = 1; trace(o.prototype);",
        "var o = {}; o.constructor = 1; trace(o.constructor);",
        "var o = {}; trace(o.__proto__);",
        "var o = {}; trace(o.prototype);",
        "var o = {}; trace(o.constructor);",
        "var o = {}; o.__proto__();",
        "var o = {}; o.constructor();",
        "var o = { __proto__: 42 };",
        "var o = { PROTOTYPE: 42 };",
        "var o = {}; o.CONSTRUCTOR = 1;",
    ];

    for dialect in [AsDialect::As1, AsDialect::As2, AsDialect::As3] {
        for payload in payloads {
            let res = run_actionscript(payload, dialect);
            assert!(
                res.is_err(),
                "dialect {:?} must reject CWE-1321 payload: {}",
                dialect,
                payload
            );
        }
    }
}

#[test]
fn hostile_infinite_loop_hits_vm_step_limit() {
    let src = "var i = 0; while (true) { i = i + 1; }";
    for dialect in [AsDialect::As1, AsDialect::As2, AsDialect::As3] {
        let res = run_actionscript(src, dialect);
        assert!(
            matches!(res, Err(AsCompilerError::ExecutionError(_))),
            "infinite loop must terminate via ExecutionError step limit"
        );
    }
}

#[test]
fn hostile_unterminated_literals_and_lossy_utf8() {
    let malformed_cases = [
        "var s = \"unterminated string;",
        "var s = 'unterminated single quote;",
        "/* unterminated block comment",
        "var x = 1e9999999999999999999999999999;",
        "var x = 0x;",
        "\0\x01\x02\x7f\u{FFFD}var a = ;",
        "class Foo extends { function bar( {}",
        "package com.example { public dynamic final class {",
        "try { throw 1; } catch () {} finally {",
    ];

    for case in malformed_cases {
        for dialect in [AsDialect::As1, AsDialect::As2, AsDialect::As3] {
            let _ = compile_actionscript(case, dialect, "malformed");
        }
    }

    // Invalid raw byte sequences decoded lossily must not panic in lexer/parser/lowering.
    let mut rng = SplitMix64(0xCAFE_BABE_1234_5678);
    for _ in 0..100 {
        let len = rng.next_usize(128);
        let raw: Vec<u8> = (0..len).map(|_| rng.next_u64() as u8).collect();
        let lossy = String::from_utf8_lossy(&raw);
        let _ = compile_actionscript(&lossy, AsDialect::As3, "lossy_utf8");
    }
}

#[test]
fn hostile_token_soup_fuzz_1000_cases_no_panics() {
    const TOKENS: &[&str] = &[
        "{",
        "}",
        "(",
        ")",
        "[",
        "]",
        ";",
        ":",
        ",",
        ".",
        "=",
        "==",
        "===",
        "!",
        "+",
        "-",
        "*",
        "/",
        "%",
        "?",
        "&&",
        "||",
        "var",
        "const",
        "function",
        "class",
        "interface",
        "package",
        "import",
        "namespace",
        "public",
        "private",
        "static",
        "internal",
        "final",
        "dynamic",
        "override",
        "if",
        "else",
        "while",
        "do",
        "for",
        "in",
        "return",
        "break",
        "continue",
        "try",
        "catch",
        "finally",
        "throw",
        "new",
        "Vector",
        ".<",
        ">",
        "is",
        "as",
        "stop",
        "play",
        "gotoAndPlay",
        "gotoAndStop",
        "tellTarget",
        "call",
        "onClipEvent",
        "trace",
        "eval",
        "this",
        "_root",
        "_parent",
        "_global",
        "__proto__",
        "prototype",
        "constructor",
        "x",
        "0",
        "\"str\"",
        "true",
        "null",
        "undefined",
    ];

    let mut rng = SplitMix64(0xA5C0_5EED_0000_1000);
    let dialects = [AsDialect::As1, AsDialect::As2, AsDialect::As3];

    for case_idx in 0..1_000 {
        let dialect = dialects[case_idx % dialects.len()];
        let count = 1 + rng.next_usize(64);
        let mut src = String::with_capacity(count * 6);
        for _ in 0..count {
            src.push_str(TOKENS[rng.next_usize(TOKENS.len())]);
            src.push(' ');
        }
        let _ = compile_actionscript(&src, dialect, "fuzz_case");
    }
}
