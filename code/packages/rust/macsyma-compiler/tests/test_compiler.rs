use coding_adventures_macsyma_compiler::{
    compile_macsyma, compile_macsyma_with_options, CompileOptions, DISPLAY, SUPPRESS,
};
use symbolic_ir::{
    apply, int, sym, ADD, ASSIGN, D, DEFINE, DIV, EQUAL, GREATER, IF, LESS, LIST, MUL, NEG, POW,
    SIN, SUB,
};

fn one(source: &str) -> symbolic_ir::IRNode {
    let statements = compile_macsyma(source).unwrap();
    assert_eq!(statements.len(), 1);
    statements.into_iter().next().unwrap()
}

#[test]
fn compiles_atoms() {
    assert_eq!(one("42;"), int(42));
    assert_eq!(one("x;"), sym("x"));
    assert_eq!(one("%pi;"), sym("%pi"));
}

#[test]
fn compiles_arithmetic_precedence_and_associativity() {
    assert_eq!(
        one("1 + 2 * 3;"),
        apply(
            sym(ADD),
            vec![int(1), apply(sym(MUL), vec![int(2), int(3)])]
        )
    );
    assert_eq!(
        one("a - b - c;"),
        apply(
            sym(SUB),
            vec![apply(sym(SUB), vec![sym("a"), sym("b")]), sym("c")]
        )
    );
    assert_eq!(
        one("a / b * c;"),
        apply(
            sym(MUL),
            vec![apply(sym(DIV), vec![sym("a"), sym("b")]), sym("c")]
        )
    );
    assert_eq!(
        one("a^b^c;"),
        apply(
            sym(POW),
            vec![sym("a"), apply(sym(POW), vec![sym("b"), sym("c")])]
        )
    );
    assert_eq!(one("-x;"), apply(sym(NEG), vec![sym("x")]));
}

#[test]
fn compiles_function_calls_and_standard_function_names() {
    assert_eq!(one("f(x, y);"), apply(sym("f"), vec![sym("x"), sym("y")]));
    assert_eq!(
        one("diff(x^2, x);"),
        apply(
            sym(D),
            vec![apply(sym(POW), vec![sym("x"), int(2)]), sym("x")]
        )
    );
    assert_eq!(one("sin(x);"), apply(sym(SIN), vec![sym("x")]));
}

#[test]
fn compiles_comparisons_and_logic() {
    assert_eq!(one("x = 4;"), apply(sym(EQUAL), vec![sym("x"), int(4)]));
    assert_eq!(one("a < b;"), apply(sym(LESS), vec![sym("a"), sym("b")]));
    assert_eq!(one("a > b;"), apply(sym(GREATER), vec![sym("a"), sym("b")]));
    assert_eq!(one("a and b and c;").to_string(), "And(a, b, c)");
}

#[test]
fn compiles_assignment_and_function_definition() {
    assert_eq!(one("a : 5;"), apply(sym(ASSIGN), vec![sym("a"), int(5)]));
    assert_eq!(
        one("f(x) := x^2;"),
        apply(
            sym(DEFINE),
            vec![
                sym("f"),
                apply(sym(LIST), vec![sym("x")]),
                apply(sym(POW), vec![sym("x"), int(2)]),
            ]
        )
    );
}

#[test]
fn compiles_lists_and_optional_terminator_wrappers() {
    assert_eq!(
        one("[1, 2, 3];"),
        apply(sym(LIST), vec![int(1), int(2), int(3)])
    );
    assert_eq!(
        compile_macsyma_with_options(
            "x; y$",
            CompileOptions {
                wrap_terminators: true,
            },
        )
        .unwrap(),
        vec![
            apply(sym(DISPLAY), vec![sym("x")]),
            apply(sym(SUPPRESS), vec![sym("y")]),
        ]
    );
}

#[test]
fn compiles_if_expressions_to_symbolic_if() {
    assert_eq!(
        one("if x < 0 then -x else x;"),
        apply(
            sym(IF),
            vec![
                apply(sym(LESS), vec![sym("x"), int(0)]),
                apply(sym(NEG), vec![sym("x")]),
                sym("x"),
            ]
        )
    );
}
