use std::error::Error;
use std::fmt;

use symbolic_ir::IRNode;

pub const RK4_SOLVE: &str = "RK4Solve";

#[derive(Debug, Clone, PartialEq)]
pub struct Rk4Options {
    pub state_names: Option<Vec<String>>,
    pub t_name: String,
}

impl Default for Rk4Options {
    fn default() -> Self {
        Self {
            state_names: None,
            t_name: "t".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Binding<'a> {
    pub name: &'a str,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rk4Point {
    pub t: f64,
    pub state: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Rk4Error {
    Y0LengthMismatch {
        f_components: usize,
        y0_entries: usize,
    },
    StateNamesLengthMismatch {
        state_names: usize,
        f_components: usize,
    },
    NonPositiveDt {
        dt: f64,
    },
    Eval {
        index: usize,
        message: String,
    },
}

impl fmt::Display for Rk4Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Y0LengthMismatch {
                f_components,
                y0_entries,
            } => write!(
                f,
                "f_ir has {f_components} components but y0 has {y0_entries} entries."
            ),
            Self::StateNamesLengthMismatch {
                state_names,
                f_components,
            } => write!(
                f,
                "state_names has {state_names} entries but f_ir has {f_components}."
            ),
            Self::NonPositiveDt { dt } => write!(f, "dt must be positive, got {dt:?}."),
            Self::Eval { index, message } => {
                write!(
                    f,
                    "RK4: failed to evaluate RHS component {index}: {message}"
                )
            }
        }
    }
}

impl Error for Rk4Error {}

pub fn ir_to_float(node: &IRNode) -> Option<f64> {
    match node {
        IRNode::Integer(value) => Some(*value as f64),
        IRNode::Rational(numer, denom) => Some(*numer as f64 / *denom as f64),
        IRNode::Float(value) => Some(*value),
        _ => None,
    }
}

pub fn default_state_names(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("y{i}")).collect()
}

pub fn rk4_solve<E>(
    f_ir: &[IRNode],
    y0: &[f64],
    t_span: (f64, f64),
    dt: f64,
    mut eval_fn: E,
    options: Rk4Options,
) -> Result<Vec<Rk4Point>, Rk4Error>
where
    E: FnMut(&IRNode, &[Binding<'_>]) -> Result<f64, String>,
{
    let n = f_ir.len();
    if y0.len() != n {
        return Err(Rk4Error::Y0LengthMismatch {
            f_components: n,
            y0_entries: y0.len(),
        });
    }
    if dt <= 0.0 {
        return Err(Rk4Error::NonPositiveDt { dt });
    }

    let state_names = options
        .state_names
        .unwrap_or_else(|| default_state_names(n));
    if state_names.len() != n {
        return Err(Rk4Error::StateNamesLengthMismatch {
            state_names: state_names.len(),
            f_components: n,
        });
    }

    let (t_start, t_end) = t_span;
    let mut trajectory = Vec::new();
    let mut t_cur = t_start;
    let mut y_cur = y0.to_vec();
    trajectory.push(Rk4Point {
        t: t_cur,
        state: y_cur.clone(),
    });

    while t_cur < t_end - dt * 1e-10 {
        let h = dt.min(t_end - t_cur);
        let k1 = eval_rhs(
            f_ir,
            t_cur,
            &y_cur,
            &state_names,
            &options.t_name,
            &mut eval_fn,
        )?;

        let y_mid1: Vec<f64> = (0..n).map(|i| y_cur[i] + 0.5 * h * k1[i]).collect();
        let k2 = eval_rhs(
            f_ir,
            t_cur + 0.5 * h,
            &y_mid1,
            &state_names,
            &options.t_name,
            &mut eval_fn,
        )?;

        let y_mid2: Vec<f64> = (0..n).map(|i| y_cur[i] + 0.5 * h * k2[i]).collect();
        let k3 = eval_rhs(
            f_ir,
            t_cur + 0.5 * h,
            &y_mid2,
            &state_names,
            &options.t_name,
            &mut eval_fn,
        )?;

        let y_end_stage: Vec<f64> = (0..n).map(|i| y_cur[i] + h * k3[i]).collect();
        let k4 = eval_rhs(
            f_ir,
            t_cur + h,
            &y_end_stage,
            &state_names,
            &options.t_name,
            &mut eval_fn,
        )?;

        let y_next: Vec<f64> = (0..n)
            .map(|i| y_cur[i] + (h / 6.0) * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]))
            .collect();

        t_cur += h;
        y_cur = y_next;
        trajectory.push(Rk4Point {
            t: t_cur,
            state: y_cur.clone(),
        });
    }

    Ok(trajectory)
}

fn eval_rhs<E>(
    f_ir: &[IRNode],
    t_value: f64,
    y_values: &[f64],
    state_names: &[String],
    t_name: &str,
    eval_fn: &mut E,
) -> Result<Vec<f64>, Rk4Error>
where
    E: FnMut(&IRNode, &[Binding<'_>]) -> Result<f64, String>,
{
    let mut bindings = Vec::with_capacity(1 + state_names.len());
    bindings.push(Binding {
        name: t_name,
        value: t_value,
    });
    for (name, value) in state_names.iter().zip(y_values.iter()) {
        bindings.push(Binding {
            name,
            value: *value,
        });
    }

    let mut result = Vec::with_capacity(f_ir.len());
    for (index, node) in f_ir.iter().enumerate() {
        let value =
            eval_fn(node, &bindings).map_err(|message| Rk4Error::Eval { index, message })?;
        result.push(value);
    }
    Ok(result)
}
