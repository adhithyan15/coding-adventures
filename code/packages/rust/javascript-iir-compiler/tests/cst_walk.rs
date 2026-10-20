//! The compiler walks the parse tree (LANG84 decision 6).
//!
//! Since 0.3.0 the only input of the IIR compiler is the generic
//! `GrammarASTNode` CST that `GrammarParser` builds from `es2020.grammar`.
//! This file pins that down four ways:
//!
//! 1. **Source checks** — `src/lib.rs` never names the typed AST
//!    (`javascript_ast`), the CST → typed-AST bridge, or the parser entry
//!    points that run it, and `Cargo.toml` keeps `javascript-ast` out of the
//!    compiler's dependencies. No later edit can quietly put the typed lift
//!    back on the IIR path.
//! 2. **The public CST entry point** — `compile_ast` takes a tree from either
//!    public CST parse, agrees with `compile_source`, and rejects what the
//!    old typed-AST stage rejected with the same text and precedence.
//! 3. **An oracle check** — the CST number decoder matches the typed AST's
//!    `NumericLiteral` values (the typed AST is used here as a test oracle
//!    only, which is why `javascript-ast` is a dev-dependency).
//! 4. **Hand-built trees** — malformed shapes are errors, never panics, and a
//!    tower of wrapper nodes far deeper than any parse cannot exhaust the
//!    stack.

use coding_adventures_javascript_ast::{
    statement::TaggedStatement, Expression, ProgramItem, Statement,
};
use coding_adventures_javascript_parser::{
    parse_javascript_cst_division_only, parse_javascript_program, parse_javascript_typed,
};
use coding_adventures_javascript_tokens::EsVersion;
use javascript_iir_compiler::{compile_ast, compile_source, run_source};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

// ============================================================
// 1. Source checks: no typed lift on the IIR path
// ============================================================

/// The compiler's own source, as compiled into this test binary.
const LIB_RS: &str = include_str!("../src/lib.rs");
const CARGO_TOML: &str = include_str!("../Cargo.toml");

/// The *code* lines of `src/lib.rs`: everything outside `//` comments (doc
/// comments included), with string literals blanked. Comments and error
/// texts may talk about the old pipeline; only code matters here.
fn code_lines() -> Vec<String> {
    LIB_RS
        .lines()
        .map(|line| {
            let mut code = String::new();
            let mut in_string = false;
            let mut chars = line.chars().peekable();
            while let Some(c) = chars.next() {
                match c {
                    '\\' if in_string => {
                        chars.next();
                    }
                    '"' => {
                        in_string = !in_string;
                        code.push('"');
                    }
                    '/' if !in_string && chars.peek() == Some(&'/') => break,
                    _ if in_string => {}
                    _ => code.push(c),
                }
            }
            code
        })
        .filter(|line| !line.trim().is_empty())
        .collect()
}

#[test]
fn compiler_source_never_names_the_typed_ast_or_the_bridge() {
    // Built with `concat!` so this test's own source does not contain the
    // forbidden words as plain literals.
    let forbidden = [
        concat!("javascript", "_ast"),
        concat!("javascript", "-ast"),
        concat!("bridge", "::"),
        concat!("grammar_to", "_program"),
        concat!("parse_javascript", "_program"),
        concat!("Program", "Item"),
    ];
    let lines = code_lines();
    assert!(lines.len() > 500, "expected to scan the whole compiler");
    for line in &lines {
        for word in forbidden {
            assert!(
                !line.contains(word),
                "src/lib.rs code mentions `{word}` — the IIR compiler must walk the \
                 GrammarASTNode, not the typed AST:\n{line}"
            );
        }
    }
}

#[test]
fn compiler_source_imports_only_cst_entry_points_from_the_parser_crate() {
    // Every item the compiler names from the parser crate must be on this
    // list: the two public CST parses. The typed entries are absent.
    let allowed = [
        "parse_javascript_cst_division_only",
        "parse_javascript_typed",
    ];
    let prefix = "coding_adventures_javascript_parser::";
    let mut seen = 0;
    for line in code_lines() {
        let mut rest = line.as_str();
        while let Some(i) = rest.find(prefix) {
            rest = &rest[i + prefix.len()..];
            let names: Vec<&str> = if let Some(group) = rest.strip_prefix('{') {
                let end = group.find('}').expect("closing brace on the same line");
                group[..end]
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .collect()
            } else {
                let end = rest
                    .find(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .unwrap_or(rest.len());
                vec![&rest[..end]]
            };
            for name in names {
                seen += 1;
                assert!(
                    allowed.contains(&name),
                    "src/lib.rs uses `{prefix}{name}`; only {allowed:?} are allowed on the \
                     IIR path (LANG84: the compiler's only input is the parse tree)"
                );
            }
        }
    }
    assert!(seen >= 1, "expected the compiler to import its CST parse");
}

#[test]
fn the_typed_ast_crate_is_a_dev_dependency_only() {
    // The `[dependencies]` table ends where the next table starts.
    let dependencies = CARGO_TOML
        .split("[dependencies]")
        .nth(1)
        .expect("a [dependencies] table")
        .split("\n[")
        .next()
        .unwrap();
    assert!(dependencies.contains("coding-adventures-javascript-parser"));
    assert!(dependencies.contains("\nparser = "));
    assert!(
        !dependencies.contains(concat!("javascript", "-ast")),
        "javascript-ast must not be a normal dependency of the IIR compiler"
    );
}

// ============================================================
// 2. compile_ast — the CST entry point
// ============================================================

fn division_only(source: &str) -> GrammarASTNode {
    parse_javascript_cst_division_only(source, EsVersion::Es2020).unwrap()
}

fn regex_aware(source: &str) -> GrammarASTNode {
    parse_javascript_typed(source, EsVersion::Es2020).unwrap()
}

#[test]
fn compile_ast_agrees_with_compile_source_on_both_cst_parses() {
    for source in [
        "console.log(1 + 2, 3 / 2, -(4 * 2));",
        "let x = 1 / 2; const y = x * 4; console.log(x, y, -y / x);",
        "let x = 3; const y = x * 4; console.log(x, y, -y - x);",
        "console.log(a); let a = 2;",
        "console.log(8 - 2 + 1, 8 / 2 / 2, 2 * 3 - 4 / 2);",
        "",
    ] {
        let expected = format!("{:?}", compile_source(source, "m").unwrap());
        let module = compile_ast(&division_only(source), "m").unwrap();
        assert_eq!(format!("{module:?}"), expected, "division-only: {source:?}");
        // The regex-aware parse reads `/ … /` as a regex literal, so only
        // slash-free sources mean the same thing under both goals.
        if !source.contains('/') {
            let module = compile_ast(&regex_aware(source), "m").unwrap();
            assert_eq!(format!("{module:?}"), expected, "regex-aware: {source:?}");
        }
    }
}

#[test]
fn compile_ast_rejects_a_non_program_root_in_the_old_wording() {
    let cst = division_only("console.log(1);");
    let ASTNodeOrToken::Node(element) = &cst.children[0] else {
        panic!("expected a source_element node");
    };
    assert_eq!(
        compile_ast(element, "m").unwrap_err(),
        "InternalError in bridge for rule 'source_element': expected root rule 'program', \
         got 'source_element'"
    );
}

#[test]
fn literals_the_old_stage_could_not_decode_are_still_rejected_verbatim() {
    // The CST keeps literals as text, so this tree builds fine; the
    // validation pass must reject it exactly as the typed stage did.
    for (source, literal) in [
        ("console.log(0x10000000000000000);", "0x10000000000000000"),
        (
            "let a = 1 + 0b10000000000000000000000000000000000000000000000000000000000000000;",
            "0b10000000000000000000000000000000000000000000000000000000000000000",
        ),
    ] {
        let error = compile_source(source, "m").unwrap_err();
        assert_eq!(
            error,
            format!(
                "InternalError in bridge for rule 'primary_expression': failed to parse \
                 numeric literal '{literal}'"
            )
        );
    }
    // The largest u64 still decodes (to the nearest f64, 2^64).
    assert_eq!(
        run_source("console.log(0xffffffffffffffff / 0x10000000000000000);")
            .unwrap_err()
            .message,
        "InternalError in bridge for rule 'primary_expression': failed to parse numeric \
         literal '0x10000000000000000'"
    );
    assert_eq!(
        run_source("console.log(0xffffffffffffffff / 0x8000000000000000);").unwrap(),
        "2\n"
    );
}

#[test]
fn destructuring_is_rejected_with_its_location() {
    assert_eq!(
        compile_source("console.log(1);\nlet [a] = [1];", "m").unwrap_err(),
        "UnsupportedSyntax: grammar rule 'binding_pattern' at 2:5 is not yet covered by \
         Phase 1 of the typed AST"
    );
    assert!(compile_source("var {a} = b;", "m")
        .unwrap_err()
        .contains("'binding_pattern' at 1:5"));
}

#[test]
fn whole_program_validation_still_beats_early_errors_and_lowering() {
    // The typed stage converted the whole script before the compiler's
    // duplicate-name pass ran, so its errors win from anywhere in the file.
    for source in [
        "let a = 1; let a = 2; console.log(0x10000000000000000);",
        "console.log(z); var x = 1; console.log(0x10000000000000000);",
        "if (1) 1; console.log(0x10000000000000000);",
    ] {
        let error = compile_source(source, "m").unwrap_err();
        assert!(
            error.contains("failed to parse numeric literal"),
            "{source}: {error}"
        );
    }
    let error = compile_source("let a = 1; let a = 2; let [b] = c;", "m").unwrap_err();
    assert!(error.contains("binding_pattern"), "{error}");
}

// ============================================================
// 3. Oracle: number decoding matches the typed AST
// ============================================================

#[test]
fn number_decoding_matches_the_typed_ast_oracle() {
    let literals = [
        "0",
        "7",
        "42",
        "1.5",
        ".5",
        "5.",
        "5.e1",
        "1e3",
        "1E-3",
        "2.5e+2",
        "0x1F",
        "0X1f",
        "0xffffffffffffffff",
        "0b101",
        "0B0",
        "0o17",
        "0O777",
        "017",
        "010",
        "08",
        "019",
        "09.5",
        "0.000001",
        "100000000000000000000",
        "1e400",
        "0e0",
        "00",
    ];
    for literal in literals {
        let source = format!("{literal};");
        let program = parse_javascript_program(&source, EsVersion::Es2020).unwrap();
        let ProgramItem::Statement(Statement::Tagged(TaggedStatement::ExpressionStatement(
            statement,
        ))) = &program.body[0]
        else {
            panic!("expected an expression statement for {literal}");
        };
        let Expression::NumericLiteral(number) = &statement.expression else {
            panic!("expected a numeric literal for {literal}");
        };
        // Compile `const` the CST way and read the operand back.
        let module = compile_source(&source, "m").unwrap();
        let constant = &module.functions[0].instructions[0];
        assert_eq!(constant.op, "const", "{literal}");
        let interpreter_ir::Operand::Float(value) = constant.srcs[0] else {
            panic!("expected a float operand for {literal}");
        };
        assert_eq!(
            value.to_bits(),
            number.value.to_bits(),
            "{literal}: CST {value} vs typed {}",
            number.value
        );
    }
}

// ============================================================
// 4. Hand-built trees and CST edge cases
// ============================================================

/// The first node of rule `rule`, depth first.
fn find_mut<'t>(node: &'t mut GrammarASTNode, rule: &str) -> &'t mut GrammarASTNode {
    fn walk<'t>(node: &'t mut GrammarASTNode, rule: &str) -> Option<&'t mut GrammarASTNode> {
        if node.rule_name == rule {
            return Some(node);
        }
        node.children.iter_mut().find_map(|child| match child {
            ASTNodeOrToken::Node(inner) => walk(inner, rule),
            ASTNodeOrToken::Token(_) => None,
        })
    }
    walk(node, rule).unwrap_or_else(|| panic!("no `{rule}` node"))
}

/// A source, the rule of the node to damage, and the damage to do.
type Forgery = (&'static str, &'static str, fn(&mut GrammarASTNode));

#[test]
fn malformed_hand_built_trees_are_errors_not_panics() {
    let edits: [Forgery; 10] = [
        // A binary node whose operator was removed: [A, B].
        ("1 + 2;", "additive_expression", |n| {
            n.children.remove(1);
        }),
        // A binary node with a foreign operator token: `1 * 2` read as additive.
        ("1 * 2;", "multiplicative_expression", |n| {
            n.rule_name = "additive_expression".into();
        }),
        // A binary node with no operands at all.
        ("1 + 2;", "additive_expression", |n| n.children.clear()),
        // A unary node without its operand.
        ("-1;", "unary_expression", |n| n.children.truncate(1)),
        // A suffix chain whose `.` lost its property name.
        ("console.log(1);", "member_expression", |n| {
            n.children.truncate(2);
        }),
        // A call chain whose base is a token.
        ("console.log(1);", "call_expression", |n| {
            let ASTNodeOrToken::Node(arguments) = &n.children[1] else {
                unreachable!("call_expression [callee, arguments]");
            };
            n.children[0] = arguments.children[0].clone(); // the `(` token
        }),
        // A computed member without its closing bracket.
        ("o[1];", "member_expression", |n| {
            n.children.pop();
        }),
        // A declarator with no name token.
        ("let x = 1;", "lexical_binding", |n| {
            n.children.retain(|c| matches!(c, ASTNodeOrToken::Node(_)));
        }),
        // A statement wrapper with two statements in it.
        ("1;", "statement", |n| {
            let copy = n.children[0].clone();
            n.children.push(copy);
        }),
        // A primary expression with nothing in it.
        ("1;", "primary_expression", |n| n.children.clear()),
    ];
    for (source, rule, edit) in edits {
        let mut cst = regex_aware(source);
        edit(find_mut(&mut cst, rule));
        let result = std::panic::catch_unwind(|| compile_ast(&cst, "forged"));
        let error = result
            .unwrap_or_else(|_| panic!("compile_ast panicked on a forged {rule} in {source:?}"))
            .unwrap_err();
        assert!(!error.is_empty(), "{source}");
    }
}

#[test]
fn a_tower_of_wrappers_does_not_exhaust_a_small_stack() {
    // 20,000 nested `( … )` around `1`: far past the parser's own depth
    // guard, so only a hand-built tree can look like this. Building and
    // dropping it recurses, so that happens on a big stack; compiling it
    // happens on a 256 KiB thread, which proves `compile_ast` walks
    // wrappers in a loop.
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            let mut cst = regex_aware("(1);");
            let paren = find_mut(&mut cst, "primary_expression").clone();
            // `( expression )`: keep the `expression` for `1` as the core.
            let ASTNodeOrToken::Node(one) = &paren.children[1] else {
                unreachable!("primary_expression [(, expression, )]");
            };
            let mut node = one.clone();
            for _ in 0..20_000 {
                let mut wrapper = paren.clone();
                wrapper.children[1] = ASTNodeOrToken::Node(node);
                node = wrapper;
            }
            *find_mut(&mut cst, "primary_expression") = node;
            let module = std::thread::scope(|scope| {
                std::thread::Builder::new()
                    .stack_size(256 << 10)
                    .spawn_scoped(scope, || compile_ast(&cst, "tower"))
                    .unwrap()
                    .join()
                    .expect("compile_ast must not overflow a small stack")
            })
            .unwrap();
            let ops: Vec<_> = module.functions[0]
                .instructions
                .iter()
                .map(|i| i.op.as_str())
                .collect();
            assert_eq!(ops, ["const", "ret_void"]);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn cst_spellings_the_typed_ast_normalised_still_run() {
    for (source, expected) in [
        // Parentheses are transparent, even around the callee.
        ("(console).log(1);", "1\n"),
        ("(console.log)(2);", "2\n"),
        ("((console)).log(((3)));", "3\n"),
        // Left associativity is read off the flat operator chain.
        (
            "console.log(8 - 2 + 1, 8 / 2 / 2, 2 * 3 - 4 / 2);",
            "7 2 4\n",
        ),
        ("console.log((1 - 2) - 3, 1 - (2 - 3));", "-4 2\n"),
        // Trailing argument comma, comments, ASI.
        ("console.log(1, 2,);", "1 2\n"),
        ("console.log(1) /* c */ // d\nconsole.log(2)", "1\n2\n"),
        ("let a = 1\nlet b = a + 1\nconsole.log(a, b)", "1 2\n"),
        // Number spellings.
        ("console.log(0x10, 0b11, 0o17);", "16 3 15\n"),
        ("console.log(017, 019, 08);", "15 19 8\n"),
        ("console.log(.5, 5., 5.e1);", "0.5 5 50\n"),
        // Context keywords are ordinary names.
        ("let of = 2; let get = 3; console.log(of, -get);", "2 -3\n"),
    ] {
        assert_eq!(run_source(source).unwrap(), expected, "{source}");
    }
}

#[test]
fn near_miss_spellings_are_still_rejected() {
    for (source, needle) in [
        ("console['log'](1);", "unsupported JavaScript expression"),
        ("console?.log(1);", "unsupported JavaScript expression"),
        ("console.log(1)(2);", "unsupported JavaScript expression"),
        ("console.log(...[1]);", "unsupported JavaScript expression"),
        (
            "console.log(1 % 2);",
            "unsupported JavaScript numeric operator",
        ),
        (
            "console.log(a in b);",
            "unsupported JavaScript numeric operator",
        ),
        ("console.log(+1);", "unsupported JavaScript expression"),
        (
            "console.log(typeof 1);",
            "unsupported JavaScript expression",
        ),
        ("console.log(x++);", "unsupported JavaScript expression"),
        ("console.log(1 && 2);", "unsupported JavaScript expression"),
        ("console.log((1, 2));", "unsupported JavaScript expression"),
        ("console.log(1n);", "unsupported JavaScript expression"),
        ("console.log(this);", "unsupported JavaScript expression"),
        (
            "console.log(new.target);",
            "unsupported JavaScript expression",
        ),
        ("console.log`x`;", "unsupported JavaScript expression"),
        (
            "console.log(1)`x`;",
            "UnsupportedSyntax: grammar rule 'call_expression'",
        ),
        (";", "unsupported JavaScript statement"),
        ("{ console.log(1); }", "unsupported JavaScript statement"),
    ] {
        let error = compile_source(source, "m").unwrap_err();
        assert!(error.contains(needle), "{source}: {error}");
    }
}
