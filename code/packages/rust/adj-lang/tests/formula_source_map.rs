use adj_lang::ast::{ArithOp, ExprAst};
use std::fs;
use std::path::{Path, PathBuf};

macro_rules! span_text {
    ($source:expr, $span:expr) => {
        &$source[$span.start..$span.end]
    };
}

#[test]
fn simple_formula_maps_exact_declaration_and_body_bytes() {
    let src = "formulabook arithmetic {\n    formula add(a, b) = a + b\n}\n";

    let formulas = adj_lang::formula_source_map(src).expect("valid formula book");

    assert_eq!(formulas.len(), 1);
    let mapped = &formulas[0];
    assert_eq!(mapped.formulabook, "arithmetic");
    assert_eq!(mapped.formula.name, "add");
    assert_eq!(mapped.formula.params, ["a", "b"]);
    assert_eq!(
        span_text!(src, mapped.declaration_span),
        "formula add(a, b) = a + b"
    );
    assert_eq!(span_text!(src, mapped.body_span), "a + b");
    assert!(matches!(
        &mapped.formula.body,
        ExprAst::Bin(ArithOp::Add, lhs, rhs)
            if matches!(lhs.as_ref(), ExprAst::Ref(name) if name == "a")
                && matches!(rhs.as_ref(), ExprAst::Ref(name) if name == "b")
    ));
}

#[test]
fn multiple_formulas_preserve_source_order_and_exact_spans() {
    let src = "formulabook arithmetic {\n\
    formula subtract(a, b) = a - b\n\
    formula divide(numerator, denominator) = numerator / denominator\n\
}\n";

    let formulas = adj_lang::formula_source_map(src).expect("valid formula book");

    assert_eq!(formulas.len(), 2);
    assert_eq!(formulas[0].formula.name, "subtract");
    assert_eq!(formulas[1].formula.name, "divide");
    assert!(formulas[0].declaration_span.start < formulas[1].declaration_span.start);
    assert_eq!(
        span_text!(src, formulas[0].declaration_span),
        "formula subtract(a, b) = a - b"
    );
    assert_eq!(span_text!(src, formulas[0].body_span), "a - b");
    assert_eq!(
        span_text!(src, formulas[1].declaration_span),
        "formula divide(numerator, denominator) = numerator / denominator"
    );
    assert_eq!(
        span_text!(src, formulas[1].body_span),
        "numerator / denominator"
    );
}

#[test]
fn multi_step_formula_body_span_selects_only_the_final_expression() {
    let src = "formulabook staged {\n\
    formula scaled_sum(a, b, scale) {\n\
        let sum = a + b\n\
        let scaled = sum * scale\n\
        scaled / 2\n\
    }\n\
}\n";

    let formulas = adj_lang::formula_source_map(src).expect("valid block formula");

    assert_eq!(formulas.len(), 1);
    let mapped = &formulas[0];
    assert_eq!(mapped.formula.steps.len(), 2);
    assert_eq!(
        span_text!(src, mapped.declaration_span),
        "formula scaled_sum(a, b, scale) {\n\
        let sum = a + b\n\
        let scaled = sum * scale\n\
        scaled / 2\n\
    }"
    );
    assert_eq!(span_text!(src, mapped.body_span), "scaled / 2");
    assert!(matches!(
        &mapped.formula.body,
        ExprAst::Bin(ArithOp::Div, lhs, rhs)
            if matches!(lhs.as_ref(), ExprAst::Ref(name) if name == "scaled")
                && matches!(rhs.as_ref(), ExprAst::Lit(value) if *value == 2.0)
    ));
}

#[test]
fn unicode_before_formula_uses_utf8_byte_offsets() {
    let src = "formulabook unicode_offsets {\n    % pi: \u{03c0}; snowman: \u{2603}\n    formula identity(x) = x\n}\n";

    let formulas = adj_lang::formula_source_map(src).expect("valid Unicode source");

    let mapped = &formulas[0];
    let expected_start = src.find("formula identity").expect("formula declaration");
    assert_eq!(mapped.declaration_span.start, expected_start);
    assert_eq!(
        span_text!(src, mapped.declaration_span),
        "formula identity(x) = x"
    );
    assert_eq!(span_text!(src, mapped.body_span), "x");
}

#[test]
fn escaped_quoted_latex_body_has_an_exact_span() {
    let src =
        "formulabook latex_math {\n    formula product(x, y) = latex \"$x \\\\times y$\"\n}\n";

    let formulas = adj_lang::formula_source_map(src).expect("valid LaTeX formula");

    let mapped = &formulas[0];
    assert_eq!(
        span_text!(src, mapped.declaration_span),
        "formula product(x, y) = latex \"$x \\\\times y$\""
    );
    assert_eq!(
        span_text!(src, mapped.body_span),
        "latex \"$x \\\\times y$\""
    );
    assert!(matches!(
        &mapped.formula.body,
        ExprAst::Bin(ArithOp::Mul, lhs, rhs)
            if matches!(lhs.as_ref(), ExprAst::Ref(name) if name == "x")
                && matches!(rhs.as_ref(), ExprAst::Ref(name) if name == "y")
    ));
}

#[test]
fn malformed_input_returns_an_error() {
    let src = "formulabook broken { formula missing_body(x) = }";

    assert!(adj_lang::formula_source_map(src).is_err());
}

#[test]
fn rulebook_contained_formulabook_is_mapped_like_the_lowerer_maps_it() {
    let src = "rulebook outer {\n\
    formulabook nested {\n\
        formula identity(x) = x\n\
            source \"identity definition\"\n\
            trust authoritative\n\
    }\n\
}\n";

    adj_lang::compile(src).expect("nested formulabook is executable after flattening");
    let formulas = adj_lang::formula_source_map(src).expect("nested formulabook is mapped");

    assert_eq!(formulas.len(), 1);
    assert_eq!(formulas[0].formulabook, "nested");
    assert_eq!(formulas[0].formula.name, "identity");
    assert_eq!(span_text!(src, formulas[0].body_span), "x");
}

#[test]
fn shipped_formula_corpus_has_exact_nested_spans() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../specs/data/adj-formula-stdlib");
    let mut paths = Vec::new();
    collect_adj_files(&root, &mut paths);
    paths.sort();
    let mut formula_count = 0;

    for path in paths {
        let src = fs::read_to_string(&path).expect("shipped ADJ source is UTF-8");
        let mapped = adj_lang::formula_source_map(&src)
            .unwrap_or_else(|error| panic!("{} did not map: {error:?}", path.display()));
        for formula in mapped {
            formula_count += 1;
            assert!(formula.declaration_span.start <= formula.body_span.start);
            assert!(formula.body_span.end <= formula.declaration_span.end);
            assert!(!span_text!(src, formula.declaration_span).is_empty());
            assert!(!span_text!(src, formula.body_span).is_empty());
            assert!(span_text!(src, formula.declaration_span)
                .starts_with(&format!("formula {}(", formula.formula.name)));
        }
    }

    assert!(
        formula_count >= 150,
        "expected the shipped formula corpus, found {formula_count} formulas"
    );
}

fn collect_adj_files(directory: &Path, paths: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).expect("formula stdlib directory is readable") {
        let path = entry.expect("valid directory entry").path();
        if path.is_dir() {
            collect_adj_files(&path, paths);
        } else if path.extension().is_some_and(|extension| extension == "adj") {
            paths.push(path);
        }
    }
}
