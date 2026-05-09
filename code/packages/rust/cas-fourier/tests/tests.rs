use cas_fourier::{
    build_fourier_handler_table, fourier_handler, fourier_transform, ifourier_handler,
    ifourier_transform, DIRAC_DELTA, FOURIER, IFOURIER, IMAGINARY_UNIT, PI, UNIT_STEP,
};
use symbolic_ir::{apply, int, sym, ADD, COS, DIV, EXP, MUL, NEG, POW, SIN};

fn t() -> symbolic_ir::IRNode {
    sym("t")
}

fn w() -> symbolic_ir::IRNode {
    sym("omega")
}

#[test]
fn forward_delta_and_constant() {
    assert_eq!(
        fourier_transform(apply(sym(DIRAC_DELTA), vec![t()]), t(), w()),
        int(1)
    );
    let result = fourier_transform(int(1), t(), w());
    assert!(contains_head(&result, DIRAC_DELTA));
    assert!(contains_symbol(&result, PI));
}

#[test]
fn forward_causal_complex_trig_and_gaussian() {
    let causal = apply(
        sym(EXP),
        vec![apply(sym(NEG), vec![apply(sym(MUL), vec![int(2), t()])])],
    );
    let result = fourier_transform(causal, t(), w());
    assert!(matches!(&result, symbolic_ir::IRNode::Apply(app) if app.head == sym(DIV)));
    assert!(contains_symbol(&result, IMAGINARY_UNIT));

    let complex = apply(
        sym(EXP),
        vec![apply(
            sym(MUL),
            vec![apply(sym(MUL), vec![sym(IMAGINARY_UNIT), int(3)]), t()],
        )],
    );
    assert!(contains_head(
        &fourier_transform(complex, t(), w()),
        DIRAC_DELTA
    ));

    assert!(contains_head(
        &fourier_transform(apply(sym(SIN), vec![t()]), t(), w()),
        DIRAC_DELTA
    ));
    assert!(contains_symbol(
        &fourier_transform(apply(sym(COS), vec![t()]), t(), w()),
        PI
    ));

    let gaussian = apply(
        sym(EXP),
        vec![apply(sym(NEG), vec![apply(sym(POW), vec![t(), int(2)])])],
    );
    assert!(contains_head(
        &fourier_transform(gaussian, t(), w()),
        "Sqrt"
    ));
}

#[test]
fn linearity_and_fallback() {
    let sum = apply(sym(ADD), vec![apply(sym(DIRAC_DELTA), vec![t()]), int(1)]);
    assert!(
        matches!(fourier_transform(sum, t(), w()), symbolic_ir::IRNode::Apply(app) if app.head == sym(ADD))
    );
    let scaled = apply(sym(MUL), vec![int(4), apply(sym(DIRAC_DELTA), vec![t()])]);
    assert!(
        matches!(fourier_transform(scaled, t(), w()), symbolic_ir::IRNode::Apply(app) if app.head == sym(MUL))
    );
    let unknown = apply(sym("Mystery"), vec![t()]);
    assert_eq!(
        fourier_transform(unknown.clone(), t(), w()),
        apply(sym(FOURIER), vec![unknown, t(), w()])
    );
}

#[test]
fn inverse_table_entries() {
    assert_eq!(
        ifourier_transform(int(1), w(), t()),
        apply(sym(DIRAC_DELTA), vec![t()])
    );
    assert_eq!(
        ifourier_transform(apply(sym(DIRAC_DELTA), vec![w()]), w(), t()),
        apply(
            sym(DIV),
            vec![int(1), apply(sym(MUL), vec![int(2), sym(PI)])]
        )
    );
    let two_pi_delta = apply(
        sym(MUL),
        vec![
            apply(sym(MUL), vec![int(2), sym(PI)]),
            apply(sym(DIRAC_DELTA), vec![w()]),
        ],
    );
    assert_eq!(ifourier_transform(two_pi_delta, w(), t()), int(1));
    let causal = apply(
        sym(DIV),
        vec![
            int(1),
            apply(
                sym(ADD),
                vec![int(2), apply(sym(MUL), vec![sym(IMAGINARY_UNIT), w()])],
            ),
        ],
    );
    let inverse = ifourier_transform(causal, w(), t());
    assert!(contains_head(&inverse, UNIT_STEP));
    assert!(contains_head(&inverse, EXP));
}

#[test]
fn handlers() {
    let id = |node| node;
    assert_eq!(
        fourier_handler(
            &apply(
                sym(FOURIER),
                vec![apply(sym(DIRAC_DELTA), vec![t()]), t(), w()]
            ),
            &id
        ),
        int(1)
    );
    assert_eq!(
        ifourier_handler(&apply(sym(IFOURIER), vec![int(1), w(), t()]), &id),
        apply(sym(DIRAC_DELTA), vec![t()])
    );
    assert_eq!(build_fourier_handler_table(), vec![FOURIER, IFOURIER]);
}

fn contains_head(node: &symbolic_ir::IRNode, head: &str) -> bool {
    match node {
        symbolic_ir::IRNode::Apply(app) => {
            app.head == sym(head) || app.args.iter().any(|arg| contains_head(arg, head))
        }
        _ => false,
    }
}

fn contains_symbol(node: &symbolic_ir::IRNode, name: &str) -> bool {
    match node {
        symbolic_ir::IRNode::Symbol(symbol) => symbol == name,
        symbolic_ir::IRNode::Apply(app) => {
            contains_symbol(&app.head, name)
                || app.args.iter().any(|arg| contains_symbol(arg, name))
        }
        _ => false,
    }
}
