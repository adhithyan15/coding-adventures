use cas_laplace::{
    build_laplace_handler_table, dirac_delta_handler, ilt_handler, inverse_laplace,
    laplace_handler, laplace_transform, unit_step_handler, DIRAC_DELTA, ILT, LAPLACE, UNIT_STEP,
};
use symbolic_ir::{apply, int, rat, sym, ADD, COS, COSH, DIV, EXP, MUL, POW, SIN, SINH, SUB};

fn t() -> symbolic_ir::IRNode {
    sym("t")
}

fn s() -> symbolic_ir::IRNode {
    sym("s")
}

#[test]
fn forward_constant_and_powers() {
    assert_eq!(
        laplace_transform(int(1), t(), s()),
        apply(sym(DIV), vec![int(1), s()])
    );
    assert_eq!(
        laplace_transform(t(), t(), s()),
        apply(sym(DIV), vec![int(1), apply(sym(POW), vec![s(), int(2)])])
    );
    assert_eq!(
        laplace_transform(apply(sym(POW), vec![t(), int(3)]), t(), s()),
        apply(sym(DIV), vec![int(6), apply(sym(POW), vec![s(), int(4)])])
    );
}

#[test]
fn forward_exp_trig_hyperbolic() {
    assert_eq!(
        laplace_transform(
            apply(sym(EXP), vec![apply(sym(MUL), vec![int(3), t()])]),
            t(),
            s()
        ),
        apply(sym(DIV), vec![int(1), apply(sym(SUB), vec![s(), int(3)])])
    );
    assert_eq!(
        laplace_transform(
            apply(sym(SIN), vec![apply(sym(MUL), vec![int(2), t()])]),
            t(),
            s()
        ),
        apply(
            sym(DIV),
            vec![
                int(2),
                apply(
                    sym(ADD),
                    vec![
                        apply(sym(POW), vec![s(), int(2)]),
                        apply(sym(POW), vec![int(2), int(2)])
                    ]
                )
            ],
        )
    );
    assert!(matches!(
        laplace_transform(apply(sym(COSH), vec![t()]), t(), s()),
        symbolic_ir::IRNode::Apply(_)
    ));
    assert!(matches!(
        laplace_transform(apply(sym(SINH), vec![t()]), t(), s()),
        symbolic_ir::IRNode::Apply(_)
    ));
}

#[test]
fn forward_linearity_and_products() {
    let expr = apply(
        sym(ADD),
        vec![apply(sym(SIN), vec![t()]), apply(sym(COS), vec![t()])],
    );
    let result = laplace_transform(expr, t(), s());
    assert!(matches!(result, symbolic_ir::IRNode::Apply(app) if app.head == sym(ADD)));

    let scaled = apply(sym(MUL), vec![int(5), apply(sym(SIN), vec![t()])]);
    let scaled_result = laplace_transform(scaled, t(), s());
    assert!(matches!(scaled_result, symbolic_ir::IRNode::Apply(app) if app.head == sym(MUL)));

    let exp_sin = apply(
        sym(MUL),
        vec![
            apply(sym(EXP), vec![t()]),
            apply(sym(SIN), vec![apply(sym(MUL), vec![int(2), t()])]),
        ],
    );
    let result = laplace_transform(exp_sin, t(), s());
    assert!(matches!(result, symbolic_ir::IRNode::Apply(app) if app.head == sym(DIV)));
}

#[test]
fn special_heads_and_handlers() {
    let delta_t = apply(sym(DIRAC_DELTA), vec![t()]);
    let step_t = apply(sym(UNIT_STEP), vec![t()]);
    assert_eq!(laplace_transform(delta_t, t(), s()), int(1));
    assert_eq!(
        laplace_transform(step_t, t(), s()),
        apply(sym(DIV), vec![int(1), s()])
    );
    assert_eq!(
        dirac_delta_handler(&apply(sym(DIRAC_DELTA), vec![int(0)])),
        int(1)
    );
    assert_eq!(
        unit_step_handler(&apply(sym(UNIT_STEP), vec![int(-1)])),
        int(0)
    );
    assert_eq!(
        unit_step_handler(&apply(sym(UNIT_STEP), vec![int(0)])),
        rat(1, 2)
    );
    assert_eq!(
        unit_step_handler(&apply(sym(UNIT_STEP), vec![int(4)])),
        int(1)
    );
    assert_eq!(
        build_laplace_handler_table(),
        vec![LAPLACE, ILT, DIRAC_DELTA, UNIT_STEP]
    );
}

#[test]
fn inverse_table_entries() {
    assert_eq!(
        inverse_laplace(apply(sym(DIV), vec![int(1), s()]), s(), t()),
        apply(sym(UNIT_STEP), vec![t()])
    );
    assert_eq!(
        inverse_laplace(
            apply(sym(DIV), vec![int(1), apply(sym(SUB), vec![s(), int(3)])]),
            s(),
            t()
        ),
        apply(sym(EXP), vec![apply(sym(MUL), vec![int(3), t()])])
    );
    assert_eq!(
        inverse_laplace(
            apply(
                sym(DIV),
                vec![
                    int(2),
                    apply(sym(ADD), vec![apply(sym(POW), vec![s(), int(2)]), int(4)])
                ]
            ),
            s(),
            t(),
        ),
        apply(sym(SIN), vec![apply(sym(MUL), vec![int(2), t()])])
    );
    assert_eq!(
        inverse_laplace(
            apply(
                sym(DIV),
                vec![
                    s(),
                    apply(sym(SUB), vec![apply(sym(POW), vec![s(), int(2)]), int(1)])
                ]
            ),
            s(),
            t(),
        ),
        apply(sym(COSH), vec![apply(sym(MUL), vec![int(1), t()])])
    );
}

#[test]
fn handler_eval_and_fallback() {
    let id = |node| node;
    let expr = apply(sym(LAPLACE), vec![int(1), t(), s()]);
    assert_eq!(
        laplace_handler(&expr, &id),
        apply(sym(DIV), vec![int(1), s()])
    );
    let ilt_expr = apply(sym(ILT), vec![apply(sym(DIV), vec![int(1), s()]), s(), t()]);
    assert_eq!(
        ilt_handler(&ilt_expr, &id),
        apply(sym(UNIT_STEP), vec![t()])
    );
    let unknown = apply(sym("Mystery"), vec![t()]);
    assert_eq!(
        laplace_transform(unknown.clone(), t(), s()),
        apply(sym(LAPLACE), vec![unknown, t(), s()])
    );
}
