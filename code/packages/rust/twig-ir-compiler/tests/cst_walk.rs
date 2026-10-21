//! The compiler walks the parse tree (LANG84 decision 6).
//!
//! Since v0.46 the only input of the Twig IIR compiler is the generic
//! `GrammarASTNode` CST that `twig_parser::parse_to_ast` builds from
//! `twig.grammar`.  This file pins that down four ways:
//!
//! 1. **A source check** — no file under `src/` names the parser crate's
//!    typed-AST API (`Program`, `extract_program`, `parse`, the `Expr`
//!    family, …), so no future edit can quietly put the typed lift back on
//!    the IIR path.  Only an allow-list of CST-level items may be imported
//!    from `twig_parser`, and only the CST entry point of the type checker.
//! 2. **The public CST entry points** — `compile_ast` and its extern /
//!    globals siblings accept a tree from `parse_to_ast` and agree with
//!    `compile_source`.
//! 3. **`check_cst` reproduces the extractor's errors** on malformed
//!    hand-built trees, message for message and position for position,
//!    including the depth guard.
//! 4. **Sugar and the type gate** — the spots where the old typed AST hid
//!    an expansion the compiler must now perform itself (`'x` vs
//!    `(quote x)`, function-sugar vs `(define f (lambda …))`), and the
//!    TW05-B gate now fed from the CST.

use std::collections::HashMap;

use interpreter_ir::IIRModule;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use twig_ir_compiler::{
    check_cst, compile_ast, compile_ast_with_externs, compile_ast_with_externs_and_globals,
    compile_source,
};
use twig_parser::{extract_program, parse_to_ast, MAX_AST_DEPTH};

// ============================================================
// 1. Source check: no typed lift on the IIR path
// ============================================================

/// Every source file of the compiler, as compiled into this test binary.
const SOURCES: &[(&str, &str)] = &[
    ("src/lib.rs", include_str!("../src/lib.rs")),
    ("src/compiler.rs", include_str!("../src/compiler.rs")),
    ("src/cst.rs", include_str!("../src/cst.rs")),
    ("src/check_cst.rs", include_str!("../src/check_cst.rs")),
    ("src/free_vars.rs", include_str!("../src/free_vars.rs")),
    ("src/errors.rs", include_str!("../src/errors.rs")),
];

/// The *code* lines of a source file: everything outside `//` comments
/// (doc comments included).  Comments may — and do — talk about the old
/// typed-AST pipeline; only code matters here.
fn code_lines(src: &'static str) -> impl Iterator<Item = &'static str> {
    src.lines()
        .map(|l| match l.find("//") {
            Some(i) => &l[..i],
            None => l,
        })
        .filter(|l| !l.trim().is_empty())
}

/// The item names a line uses through `prefix` (`twig_parser::`, …):
/// either a single path segment or the members of a `{a, b, c}` group.
fn items_after<'a>(line: &'a str, prefix: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(i) = rest.find(prefix) {
        rest = &rest[i + prefix.len()..];
        if let Some(group) = rest.strip_prefix('{') {
            let end = group.find('}').expect("closing brace on the same line");
            out.extend(group[..end].split(',').map(str::trim).filter(|s| !s.is_empty()));
        } else {
            let end = rest
                .find(|c: char| !(c.is_alphanumeric() || c == '_'))
                .unwrap_or(rest.len());
            out.push(&rest[..end]);
        }
    }
    out
}

#[test]
fn compiler_source_never_names_the_typed_ast() {
    // Built with `concat!` so this test's own source does not contain the
    // forbidden words as plain literals.
    let forbidden = [
        concat!("extract", "_program"),
        concat!("ast", "_nodes"),
        concat!("ast", "_extract"),
        concat!("check", "_program"),
        concat!("Typed", "Program"),
    ];
    for (file, src) in SOURCES {
        for line in code_lines(src) {
            for word in forbidden {
                assert!(
                    !line.contains(word),
                    "{file} code mentions `{word}` — the IIR compiler must walk the \
                     GrammarASTNode, not the typed AST:\n{line}"
                );
            }
        }
    }
}

#[test]
fn compiler_source_imports_only_cst_level_items_from_the_parser_crate() {
    // Every item the compiler names from `twig_parser` must be on this
    // list.  `parse`, `Program`, `Expr`, `Form`, `Lambda`, `TypeAnnotation`
    // and the rest of the typed AST are deliberately absent.
    let allowed = ["parse_to_ast", "TwigParseError", "MAX_AST_DEPTH", "check_membership_int_count"];
    let mut seen = 0;
    for (file, src) in SOURCES {
        for line in code_lines(src) {
            for name in items_after(line, "twig_parser::") {
                seen += 1;
                assert!(
                    allowed.contains(&name),
                    "{file} uses `twig_parser::{name}`; only {allowed:?} are allowed on the \
                     IIR path (LANG84: the compiler's only input is the parse tree)"
                );
            }
        }
    }
    assert!(seen > 0, "expected the compiler to import parse_to_ast from the parser crate");
}

#[test]
fn compiler_source_feeds_the_type_checker_only_the_cst() {
    // The TW05-B gate hands the checker the CST and reads back diagnostics.
    // `type_check_source` is the LANG50 source-text entry used by
    // `compile_typed_source`; its output is keyed by source span.
    let allowed = ["check_cst_with_globals", "TwigKind", "type_check_source"];
    for (file, src) in SOURCES {
        for line in code_lines(src) {
            for name in items_after(line, "twig_type_checker::") {
                assert!(
                    allowed.contains(&name),
                    "{file} uses `twig_type_checker::{name}`; only {allowed:?} are allowed"
                );
            }
        }
    }
}

// ============================================================
// 2. compile_ast — the CST entry points
// ============================================================

fn dump(m: &IIRModule) -> String {
    format!("{m:?}")
}

const PROGRAMS: &[&str] = &[
    "",
    "(+ 1 2)",
    "(define (square x) (* x x)) (square 7)",
    "(define (fact n) (if (= n 0) 1 (* n (fact (- n 1))))) (fact 5)",
    "(let ((x 1) (y 2)) (let* ((z (+ x y))) (begin z)))",
    "((lambda (x) (lambda (y) (+ x y))) 1)",
    "(define s \"HE\") (string-length (string-append s \"LLO\"))",
    "(union T (A (x : int)) (B)) (match (A 1) ((A v) v) ((B) 0) (_ 9))",
    "(record P (x : int) (y : int)) (p-x (P 1 2))",
    "(define (f (x : (Int 0 10)) (y : (Member int (1 2 3))) -> int) (+ x y)) (f 1 2)",
    "(module m (export f) (typed lenient)) (define (f x) x) (f 'a)",
];

#[test]
fn compile_ast_agrees_with_compile_source() {
    for src in PROGRAMS {
        let cst = parse_to_ast(src).expect("parse");
        let a = compile_ast(&cst, "m").expect("compile_ast");
        let b = compile_source(src, "m").expect("compile_source");
        assert_eq!(dump(&a), dump(&b), "IIR differs for {src:?}");
    }
}

#[test]
fn compile_ast_with_externs_resolves_cross_module_calls() {
    let cst = parse_to_ast("(define (sq x) (* x x)) (helper (sq 5))").unwrap();
    // Without the extern, `helper` is unbound …
    let err = compile_ast(&cst, "m").unwrap_err();
    assert!(err.message.contains("unbound name \"helper\""), "{}", err.message);
    // … with it, the call lowers to a direct `call`.
    let m = compile_ast_with_externs(&cst, "m", &["helper"]).unwrap();
    let main = m.get_function("main").unwrap();
    assert!(main.instructions.iter().any(|i| i.op == "call"
        && matches!(i.srcs.first(), Some(interpreter_ir::Operand::Var(n)) if n == "helper")));
}

#[test]
fn module_exports_are_read_from_the_cst() {
    let cst = parse_to_ast("(module m (export f g f)) (define (f) 1) (define g 2) (f)").unwrap();
    let m = compile_ast(&cst, "m").unwrap();
    // `g` is a value define (not exported as a function); `f` appears as
    // often as it is listed, exactly as before.
    let names: Vec<&str> = m.exports.iter().map(|e| e.public_name()).collect();
    assert_eq!(names, ["f", "f"]);
}

// ============================================================
// 3. check_cst reproduces the extractor's errors
// ============================================================

/// The first nested node of `n` whose rule is `rule` (depth-first).
fn find_mut<'a>(n: &'a mut GrammarASTNode, rule: &str) -> Option<&'a mut GrammarASTNode> {
    if n.rule_name == rule {
        return Some(n);
    }
    for c in n.children.iter_mut() {
        if let ASTNodeOrToken::Node(m) = c {
            if let Some(found) = find_mut(m, rule) {
                return Some(found);
            }
        }
    }
    None
}

/// `check_cst` must fail exactly where `extract_program` fails, and
/// `compile_ast` must report it as `parse error: …` at the same position.
fn assert_same_error(tree: &GrammarASTNode) -> String {
    let expected = extract_program(tree).expect_err("the extractor should reject this tree");
    let got = check_cst(tree).expect_err("check_cst should reject this tree");
    assert_eq!(got, expected);
    let compiled = compile_ast(tree, "m").expect_err("compile_ast should reject this tree");
    assert_eq!(compiled.message, format!("parse error: {}", expected.message));
    assert_eq!((compiled.line, compiled.column), (expected.line, expected.column));
    expected.message
}

#[test]
fn rejects_a_non_program_root() {
    let cst = parse_to_ast("(+ 1 2)").unwrap();
    let ASTNodeOrToken::Node(form) = &cst.children[0] else { panic!("expected a node") };
    let msg = assert_same_error(form);
    assert!(msg.starts_with("expected 'program' root"), "{msg}");
}

#[test]
fn rejects_an_integer_that_overflows_i64() {
    // The lexer accepts any digit run; the range check is semantic.
    let msg = assert_same_error(&parse_to_ast("(+ 1 99999999999999999999)").unwrap());
    assert!(msg.contains("does not fit in i64"), "{msg}");
    let msg = assert_same_error(
        &parse_to_ast("(define (f (x : (Int 0 99999999999999999999))) x)").unwrap(),
    );
    assert!(msg.starts_with("type annotation error: "), "{msg}");
}

#[test]
fn rejects_semantic_shape_errors_the_grammar_allows() {
    for (src, needle) in [
        ("(define x 1 2)", "takes exactly one body expression"),
        ("(module m (typed bogus))", "expects strict, lenient, or off"),
    ] {
        let msg = assert_same_error(&parse_to_ast(src).unwrap());
        assert!(msg.contains(needle), "{src}: {msg}");
    }
}

#[test]
fn rejects_malformed_hand_built_nodes() {
    // An `if` with a branch removed.
    let mut t = parse_to_ast("(if #t 1 2)").unwrap();
    let iff = find_mut(&mut t, "if_form").unwrap();
    iff.children.remove(3);
    assert_eq!(assert_same_error(&t), "(if ...) takes exactly 3 expressions");

    // A `begin` with no body.
    let mut t = parse_to_ast("(begin 1)").unwrap();
    let b = find_mut(&mut t, "begin_form").unwrap();
    b.children.retain(|c| !matches!(c, ASTNodeOrToken::Node(_)));
    assert_eq!(assert_same_error(&t), "(begin ...) needs at least one expression");

    // An unknown compound rule.
    let mut t = parse_to_ast("(f 1)").unwrap();
    find_mut(&mut t, "apply").unwrap().rule_name = "bogus".into();
    assert!(assert_same_error(&t).starts_with("unexpected compound child"));

    // A `match` without arms.
    let mut t = parse_to_ast("(match 1 (_ 2))").unwrap();
    let m = find_mut(&mut t, "match_form").unwrap();
    m.children.retain(|c| !matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "match_arm"));
    assert_eq!(assert_same_error(&t), "(match ...) needs at least one arm");
}

#[test]
fn rejects_a_hand_built_tree_deeper_than_max_ast_depth() {
    // Build `(begin (begin … 1))` far deeper than the parser would ever
    // allow, by hand.  The depth guard must fire — no stack overflow.
    let leaf = parse_to_ast("1").unwrap();
    let mut expr = match &leaf.children[0] {
        ASTNodeOrToken::Node(form) => match &form.children[0] {
            ASTNodeOrToken::Node(e) => e.clone(),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    };
    for _ in 0..(MAX_AST_DEPTH * 2) {
        let begin = GrammarASTNode {
            rule_name: "begin_form".into(),
            children: vec![ASTNodeOrToken::Node(expr)],
            start_line: Some(1),
            start_column: Some(1),
            end_line: None,
            end_column: None,
        };
        let compound = GrammarASTNode { rule_name: "compound".into(), children: vec![ASTNodeOrToken::Node(begin)], ..leaf.clone() };
        expr = GrammarASTNode { rule_name: "expr".into(), children: vec![ASTNodeOrToken::Node(compound)], ..leaf.clone() };
    }
    let form = GrammarASTNode { rule_name: "form".into(), children: vec![ASTNodeOrToken::Node(expr)], ..leaf.clone() };
    let program = GrammarASTNode { children: vec![ASTNodeOrToken::Node(form)], ..leaf.clone() };
    let msg = assert_same_error(&program);
    assert!(msg.contains("MAX_AST_DEPTH"), "{msg}");
}

#[test]
fn accepts_every_valid_program_the_extractor_accepts() {
    for src in PROGRAMS {
        let cst = parse_to_ast(src).unwrap();
        assert!(extract_program(&cst).is_ok());
        assert!(check_cst(&cst).is_ok(), "{src}");
    }
}

// ============================================================
// 4. Sugar and the type gate
// ============================================================

/// `(op, dest, srcs, type_hint)` of every instruction — the IIR minus its
/// source positions, which legitimately differ between two spellings.
fn shape(m: &IIRModule) -> Vec<(String, Vec<String>)> {
    m.functions
        .iter()
        .map(|f| {
            let instrs = f
                .instructions
                .iter()
                .map(|i| format!("{} {:?} {:?} {}", i.op, i.dest, i.srcs, i.type_hint))
                .collect();
            (f.name.clone(), instrs)
        })
        .collect()
}

#[test]
fn quote_sugar_and_quote_form_lower_identically() {
    let a = compile_source("(cons 'a (quote b))", "m").unwrap();
    let b = compile_source("(cons (quote a) 'b)", "m").unwrap();
    assert_eq!(shape(&a), shape(&b));
}

#[test]
fn function_sugar_and_define_lambda_lower_identically() {
    let a = compile_source("(define (f x) (+ x 1)) (f 2)", "m").unwrap();
    let b = compile_source("(define f (lambda (x) (+ x 1))) (f 2)", "m").unwrap();
    assert_eq!(shape(&a), shape(&b));
}

#[test]
fn strict_type_errors_still_stop_compilation() {
    let cst = parse_to_ast("(module m (typed strict)) (define (f x) x) (f 1 2)").unwrap();
    let err = compile_ast(&cst, "m").unwrap_err();
    assert!(err.message.starts_with("type error: "), "{}", err.message);
    // Lenient mode only warns.
    let cst = parse_to_ast("(module m (typed lenient)) (define (f x) x) (f 1 2)").unwrap();
    assert!(compile_ast(&cst, "m").is_ok());
}

#[test]
fn imported_globals_reach_the_type_gate() {
    let cst = parse_to_ast("(module m (typed strict)) (define r (add-one 41)) r").unwrap();
    // Unknown to the checker → strict error.
    assert!(compile_ast_with_externs(&cst, "m", &["add-one"]).is_err());
    // Seeded as an imported global → accepted.
    let mut globals = HashMap::new();
    globals.insert("add-one".to_string(), twig_type_checker::TwigKind::Function { arity: 1 });
    assert!(compile_ast_with_externs_and_globals(&cst, "m", &["add-one"], &globals).is_ok());
}

// ============================================================
// Wide `and` / `or` (security review of the CST rewrite)
// ============================================================

/// A wide `(and 1 1 … 1)` used to recurse once per argument inside the
/// compiler — argument count is bounded by neither the parser's nesting
/// limit nor `MAX_COMPILE_DEPTH` — and overflowed the stack at about
/// 20,000 arguments.  The lowering is now a loop: 50,000 arguments compile
/// on a 1 MiB thread.
#[test]
fn wide_and_or_compile_in_constant_stack() {
    std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(|| {
            for form in ["and", "or"] {
                let src = format!("({form}{})", " 1".repeat(50_000));
                twig_ir_compiler::compile_source(&src, "m")
                    .unwrap_or_else(|e| panic!("({form} …×50000): {e:?}"));
            }
        })
        .unwrap()
        .join()
        .unwrap();
}
