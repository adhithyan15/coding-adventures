use cas_ode_numeric::{ir_to_float, rk4_solve, Binding, Rk4Error, Rk4Options};
use symbolic_ir::{apply, int, rat, sym, IRNode, ADD, MUL, NEG, POW, SUB};

fn eval_numeric(node: &IRNode, bindings: &[Binding<'_>]) -> Result<f64, String> {
    let out = eval_node(node, bindings);
    ir_to_float(&out).ok_or_else(|| format!("expected numeric IR node, got {out:?}"))
}

fn eval_node(node: &IRNode, bindings: &[Binding<'_>]) -> IRNode {
    match node {
        IRNode::Symbol(name) => bindings
            .iter()
            .find(|binding| binding.name == name)
            .map(|binding| IRNode::Float(binding.value))
            .unwrap_or_else(|| node.clone()),
        IRNode::Apply(apply_node) => {
            let head = apply_node.head.clone();
            let args: Vec<IRNode> = apply_node
                .args
                .iter()
                .map(|arg| eval_node(arg, bindings))
                .collect();
            let name = match &head {
                IRNode::Symbol(name) => name.as_str(),
                _ => return IRNode::Apply(Box::new(symbolic_ir::IRApply { head, args })),
            };
            match (name, args.as_slice()) {
                (ADD, [a, b]) => numeric_binary(a, b, |x, y| x + y),
                (SUB, [a, b]) => numeric_binary(a, b, |x, y| x - y),
                (MUL, [a, b]) => numeric_binary(a, b, |x, y| x * y),
                (POW, [a, b]) => numeric_binary(a, b, |x, y| x.powf(y)),
                (NEG, [a]) => ir_to_float(a).map(|v| IRNode::Float(-v)),
                _ => None,
            }
            .unwrap_or_else(|| IRNode::Apply(Box::new(symbolic_ir::IRApply { head, args })))
        }
        other => other.clone(),
    }
}

fn numeric_binary(a: &IRNode, b: &IRNode, op: impl FnOnce(f64, f64) -> f64) -> Option<IRNode> {
    Some(IRNode::Float(op(ir_to_float(a)?, ir_to_float(b)?)))
}

fn solve(
    f_ir: &[IRNode],
    y0: &[f64],
    t_span: (f64, f64),
    dt: f64,
    state_names: &[&str],
) -> Vec<cas_ode_numeric::Rk4Point> {
    rk4_solve(
        f_ir,
        y0,
        t_span,
        dt,
        eval_numeric,
        Rk4Options {
            state_names: Some(state_names.iter().map(|name| name.to_string()).collect()),
            ..Rk4Options::default()
        },
    )
    .unwrap()
}

#[test]
fn ir_to_float_accepts_numeric_literals() {
    assert_eq!(ir_to_float(&int(3)), Some(3.0));
    assert_eq!(ir_to_float(&IRNode::Float(1.5)), Some(1.5));
    assert_eq!(ir_to_float(&rat(1, 2)), Some(0.5));
    assert_eq!(ir_to_float(&sym("x")), None);
}

#[test]
fn integrates_scalar_decay() {
    let y = sym("y");
    let f = apply(sym(MUL), vec![int(-2), y]);
    let traj = solve(&[f], &[1.0], (0.0, 1.0), 0.001, &["y"]);
    let end = traj.last().unwrap();
    assert!((end.t - 1.0).abs() < 1e-10);
    assert!((end.state[0] - (-2.0_f64).exp()).abs() < 1e-4);
}

#[test]
fn records_initial_condition_and_expected_length() {
    let y = sym("y");
    let f = apply(sym(MUL), vec![int(-1), y]);
    let traj = solve(&[f], &[2.5], (0.0, 1.0), 0.1, &["y"]);
    assert_eq!(traj.len(), 11);
    assert!((traj[0].t - 0.0).abs() < 1e-12);
    assert!((traj[0].state[0] - 2.5).abs() < 1e-12);
}

#[test]
fn keeps_zero_rhs_constant() {
    let traj = solve(&[int(0)], &[3.7], (0.0, 2.0), 0.5, &["y"]);
    assert!(traj
        .iter()
        .all(|point| (point.state[0] - 3.7).abs() < 1e-12));
}

#[test]
fn integrates_coupled_oscillator() {
    let y = sym("y");
    let v = sym("v");
    let f_y = v;
    let f_v = apply(sym(NEG), vec![y]);
    let traj = solve(
        &[f_y, f_v],
        &[1.0, 0.0],
        (0.0, 2.0 * std::f64::consts::PI),
        0.001,
        &["y", "v"],
    );
    let state = &traj.last().unwrap().state;
    assert!((state[0] - 1.0).abs() < 0.01);
    assert!(state[1].abs() < 0.01);
}

#[test]
fn uses_time_binding_and_clamps_final_step() {
    let traj = rk4_solve(
        &[sym("time")],
        &[0.0],
        (0.0, 1.0),
        0.3,
        eval_numeric,
        Rk4Options {
            state_names: Some(vec!["y".to_string()]),
            t_name: "time".to_string(),
        },
    )
    .unwrap();

    let end = traj.last().unwrap();
    assert!((end.t - 1.0).abs() < 1e-12);
    assert_eq!(traj.len(), 5);
    assert!((end.state[0] - 0.5).abs() < 1e-8);
}

#[test]
fn smaller_dt_gives_lower_error() {
    let y = sym("y");
    let f = apply(sym(MUL), vec![int(-1), y]);
    let exact = (-1.0_f64).exp();
    let coarse = solve(std::slice::from_ref(&f), &[1.0], (0.0, 1.0), 0.1, &["y"]);
    let fine = solve(&[f], &[1.0], (0.0, 1.0), 0.05, &["y"]);
    let coarse_err = (coarse.last().unwrap().state[0] - exact).abs();
    let fine_err = (fine.last().unwrap().state[0] - exact).abs();
    assert!(coarse_err > fine_err);
}

#[test]
fn reports_argument_errors() {
    assert_eq!(
        rk4_solve(
            &[int(0)],
            &[1.0],
            (0.0, 1.0),
            0.0,
            eval_numeric,
            Rk4Options::default()
        )
        .unwrap_err(),
        Rk4Error::NonPositiveDt { dt: 0.0 }
    );
    assert_eq!(
        rk4_solve(
            &[int(0)],
            &[1.0, 2.0],
            (0.0, 1.0),
            0.1,
            eval_numeric,
            Rk4Options::default()
        )
        .unwrap_err(),
        Rk4Error::Y0LengthMismatch {
            f_components: 1,
            y0_entries: 2
        }
    );
    assert_eq!(
        rk4_solve(
            &[int(0)],
            &[1.0],
            (0.0, 1.0),
            0.1,
            eval_numeric,
            Rk4Options {
                state_names: Some(vec!["a".to_string(), "b".to_string()]),
                ..Rk4Options::default()
            },
        )
        .unwrap_err(),
        Rk4Error::StateNamesLengthMismatch {
            state_names: 2,
            f_components: 1
        }
    );
}

#[test]
fn reports_non_numeric_rhs() {
    let err = rk4_solve(
        &[sym("unbound")],
        &[1.0],
        (0.0, 0.1),
        0.05,
        eval_numeric,
        Rk4Options {
            state_names: Some(vec!["y".to_string()]),
            ..Rk4Options::default()
        },
    )
    .unwrap_err();
    assert!(matches!(err, Rk4Error::Eval { index: 0, .. }));
}

#[test]
fn simulates_underdamped_rlc_transient() {
    let q = sym("q");
    let i = sym("i");
    let half_i = apply(sym(MUL), vec![IRNode::Float(0.5), i.clone()]);
    let di_dt = apply(sym(SUB), vec![apply(sym(SUB), vec![int(1), half_i]), q]);
    let traj = solve(&[i, di_dt], &[0.0, 0.0], (0.0, 20.0), 0.01, &["q", "i"]);
    let q_values: Vec<f64> = traj.iter().map(|point| point.state[0]).collect();
    assert!(q_values.iter().copied().fold(f64::NEG_INFINITY, f64::max) < 3.0);
    assert!(q_values.iter().copied().fold(f64::INFINITY, f64::min) > -1.0);
    assert!((traj.last().unwrap().state[0] - 1.0).abs() < 0.05);
}
