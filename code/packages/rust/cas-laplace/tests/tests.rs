use cas_laplace::{
    build_laplace_handler_table, dirac_delta_handler, ilt_handler, inverse_laplace,
    laplace_handler, laplace_transform, unit_step_handler, DIRAC_DELTA, ILT, LAPLACE, UNIT_STEP,
};
use symbolic_ir::{apply, int, rat, sym, ADD, COS, COSH, DIV, EXP, MUL, NEG, POW, SIN, SINH, SUB};

fn t() -> symbolic_ir::IRNode {
    sym("t")
}

fn s() -> symbolic_ir::IRNode {
    sym("s")
}

// ─── helpers ─────────────────────────────────────────────────────────────────

fn mul(a: symbolic_ir::IRNode, b: symbolic_ir::IRNode) -> symbolic_ir::IRNode {
    apply(sym(MUL), vec![a, b])
}
fn add(a: symbolic_ir::IRNode, b: symbolic_ir::IRNode) -> symbolic_ir::IRNode {
    apply(sym(ADD), vec![a, b])
}
fn sub(a: symbolic_ir::IRNode, b: symbolic_ir::IRNode) -> symbolic_ir::IRNode {
    apply(sym(SUB), vec![a, b])
}
fn div(a: symbolic_ir::IRNode, b: symbolic_ir::IRNode) -> symbolic_ir::IRNode {
    apply(sym(DIV), vec![a, b])
}
fn pow(base: symbolic_ir::IRNode, exp: symbolic_ir::IRNode) -> symbolic_ir::IRNode {
    apply(sym(POW), vec![base, exp])
}

// ─── Existing forward transform tests ────────────────────────────────────────

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
    let id = |node| node;
    let table = build_laplace_handler_table();
    assert!(table.contains_key(LAPLACE));
    assert!(table.contains_key(ILT));
    assert!(table.contains_key(DIRAC_DELTA));
    assert!(table.contains_key(UNIT_STEP));
    assert_eq!(
        table[DIRAC_DELTA](&apply(sym(DIRAC_DELTA), vec![int(0)]), &id),
        int(1)
    );
    assert_eq!(
        table[UNIT_STEP](&apply(sym(UNIT_STEP), vec![int(0)]), &id),
        rat(1, 2)
    );
}

// ─── Existing inverse table tests ────────────────────────────────────────────

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

// ─── Forward: t^n·trig for n = 2, 3 ─────────────────────────────────────────

#[test]
fn forward_tn_sin_cos_n2() {
    // L{t²·sin(2t)} = 2ω(3s²−ω²)/(s²+ω²)³  with ω=2
    // Expected: Div(Mul(Mul(2,2), Sub(Mul(3,Pow(s,2)), Pow(2,2))), Pow(Add(Pow(s,2),Pow(2,2)),3))
    let f = mul(pow(t(), int(2)), apply(sym(SIN), vec![mul(int(2), t())]));
    let result = laplace_transform(f, t(), s());
    let s2 = pow(s(), int(2));
    let w2 = pow(int(2), int(2));
    let s2pw2 = add(s2.clone(), w2.clone());
    let expected_num = mul(mul(int(2), int(2)), sub(mul(int(3), s2), w2));
    let expected = div(expected_num, pow(s2pw2, int(3)));
    assert_eq!(result, expected);

    // L{t²·cos(2t)} = 2s(s²−3ω²)/(s²+ω²)³
    let f2 = mul(pow(t(), int(2)), apply(sym(COS), vec![mul(int(2), t())]));
    let result2 = laplace_transform(f2, t(), s());
    let s2b = pow(s(), int(2));
    let w2b = pow(int(2), int(2));
    let s2pw2b = add(s2b.clone(), w2b.clone());
    let expected_num2 = mul(mul(int(2), s()), sub(s2b, mul(int(3), w2b)));
    let expected2 = div(expected_num2, pow(s2pw2b, int(3)));
    assert_eq!(result2, expected2);
}

#[test]
fn forward_tn_sin_cos_n3() {
    // L{t³·sin(t)} = 24ωs(s²−ω²)/(s²+ω²)⁴  with ω=1
    let f = mul(pow(t(), int(3)), apply(sym(SIN), vec![t()]));
    let result = laplace_transform(f, t(), s());
    let s2 = pow(s(), int(2));
    let w2 = pow(int(1), int(2));
    let s2pw2 = add(s2.clone(), w2.clone());
    let expected_num = mul(mul(int(24), int(1)), mul(s(), sub(s2, w2)));
    let expected = div(expected_num, pow(s2pw2, int(4)));
    assert_eq!(result, expected);

    // L{t³·cos(t)} = 6(s⁴−6s²ω²+ω⁴)/(s²+ω²)⁴
    let f2 = mul(pow(t(), int(3)), apply(sym(COS), vec![t()]));
    let result2 = laplace_transform(f2, t(), s());
    let s2c = pow(s(), int(2));
    let w2c = pow(int(1), int(2));
    let s2pw2c = add(s2c.clone(), w2c.clone());
    let s4 = pow(s(), int(4));
    let w4 = pow(int(1), int(4));
    let inner = add(sub(s4, mul(int(6), mul(s2c, w2c))), w4);
    let expected2 = div(mul(int(6), inner), pow(s2pw2c, int(4)));
    assert_eq!(result2, expected2);
}

#[test]
fn forward_tn_trig_n4_falls_through() {
    // n=4 is unsupported; falls through to unevaluated Laplace(...)
    let f = mul(pow(t(), int(4)), apply(sym(SIN), vec![t()]));
    let result = laplace_transform(f, t(), s());
    assert!(
        matches!(result, symbolic_ir::IRNode::Apply(ref app) if app.head == sym(LAPLACE)),
        "expected unevaluated Laplace, got {:?}",
        result
    );
}

// ─── Inverse: irreducible quadratic (complex conjugate poles) ────────────────

#[test]
fn inverse_complex_poles_1_over_s2_2s_2() {
    // 1/(s²+2s+2) → exp(−t)·sin(t) = Mul(Exp(Neg(t)), Sin(t))
    let denom = add(add(pow(s(), int(2)), mul(int(2), s())), int(2));
    let f = div(int(1), denom);
    let result = inverse_laplace(f, s(), t());
    let expected = mul(
        apply(sym(EXP), vec![apply(sym(NEG), vec![t()])]),
        apply(sym(SIN), vec![t()]),
    );
    assert_eq!(result, expected);
}

#[test]
fn inverse_complex_poles_s_over_s2_2s_2() {
    // s/(s²+2s+2) → exp(−t)·cos(t) + (−1)·exp(−t)·sin(t)
    let denom = add(add(pow(s(), int(2)), mul(int(2), s())), int(2));
    let f = div(s(), denom);
    let result = inverse_laplace(f, s(), t());
    let exp_neg_t = apply(sym(EXP), vec![apply(sym(NEG), vec![t()])]);
    let t1 = mul(exp_neg_t.clone(), apply(sym(COS), vec![t()]));
    let t2 = mul(int(-1), mul(exp_neg_t, apply(sym(SIN), vec![t()])));
    let expected = add(t1, t2);
    assert_eq!(result, expected);
}

#[test]
fn inverse_complex_poles_mixed_1_over_s_s2_1() {
    // 1/(s·(s²+1)) → Add(UnitStep(t), Mul(-1, Cos(t)))
    let f = div(int(1), mul(s(), add(pow(s(), int(2)), int(1))));
    let result = inverse_laplace(f, s(), t());
    let expected = add(
        apply(sym(UNIT_STEP), vec![t()]),
        mul(int(-1), apply(sym(COS), vec![t()])),
    );
    assert_eq!(result, expected);
}

// ─── Inverse: repeated poles ─────────────────────────────────────────────────

#[test]
fn inverse_repeated_pole_1_over_s_minus_2_sq() {
    // 1/(s−2)² → t·exp(2t) = Mul(t, Exp(Mul(2, t)))
    let f = div(int(1), pow(sub(s(), int(2)), int(2)));
    let result = inverse_laplace(f, s(), t());
    let expected = mul(t(), apply(sym(EXP), vec![mul(int(2), t())]));
    assert_eq!(result, expected);
}

#[test]
fn inverse_repeated_pole_s_over_s_minus_1_sq() {
    // s/(s−1)² → t·exp(t) + exp(t)
    // iltSimplePole/iltRepeatedPole with a=1 produce exp(t) not exp(1*t)
    let f = div(s(), pow(sub(s(), int(1)), int(2)));
    let result = inverse_laplace(f, s(), t());
    let exp_t = apply(sym(EXP), vec![t()]);
    let expected = add(mul(t(), exp_t.clone()), exp_t);
    assert_eq!(result, expected);
}

// ─── Inverse: improper fractions ─────────────────────────────────────────────

#[test]
fn inverse_improper_s2_over_s2_plus_1() {
    // s²/(s²+1) → DiracDelta(t) + (−1)·Sin(t)
    let f = div(pow(s(), int(2)), add(pow(s(), int(2)), int(1)));
    let result = inverse_laplace(f, s(), t());
    let expected = add(
        apply(sym(DIRAC_DELTA), vec![t()]),
        mul(int(-1), apply(sym(SIN), vec![t()])),
    );
    assert_eq!(result, expected);
}
