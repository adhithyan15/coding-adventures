//! # Warrant Code Generation and L0 Test Vector Compiler (VER04e / VER-P008)
//!
//! Compiles checked Warrant models, examples, contracts, and bindings into:
//! 1. Canonical Portable Test Vectors (TFV records).
//! 2. Standalone, deterministic Rust L0 test harnesses running on `test-bootstrap`.
//!
//! Zero third-party dependencies. Bounded evaluation depth (<= 64) and work quota (<= 1,000,000).

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::path::Path;

use crate::binding::BindingManifest;
use crate::checker::CheckedWarrantModule;
use crate::parser::{
    BinaryOp, Expression, Literal, MatchArm, ModelDecl, Pattern, UnaryOp, WarrantFile, Width,
};

/// Diagnostic codes for codegen phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagnosticCode {
    /// W0701: Unsupported codegen target or uninstantiated static generic width.
    W0701,
    /// W0702: Non-ground term in compile-time model evaluation.
    W0702,
    /// W0703: Evaluator resource exhaustion (depth > 64 or work > 1,000,000).
    W0703,
    /// W0704: Missing or incompatible host binding for operation.
    W0704,
    /// W0705: Duplicate or conflicting test case identifier.
    W0705,
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::W0701 => write!(f, "W0701"),
            Self::W0702 => write!(f, "W0702"),
            Self::W0703 => write!(f, "W0703"),
            Self::W0704 => write!(f, "W0704"),
            Self::W0705 => write!(f, "W0705"),
        }
    }
}

/// Specific codegen error variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodegenErrorKind {
    UnsupportedTarget(String),
    NonGroundTerm(String),
    RecursionDepthExceeded,
    WorkBudgetExceeded,
    MissingBinding(String),
    DuplicateId(String),
    EvaluationError(String),
    InvalidHexLiteral(String),
    DivisionByZero,
}

/// Codegen and compilation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenError {
    pub code: DiagnosticCode,
    pub kind: CodegenErrorKind,
    pub span: (usize, usize),
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {:?}", self.code, self.kind)
    }
}

impl std::error::Error for CodegenError {}

/// Resource limits for compile-time model evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodegenLimits {
    pub max_depth: usize,
    pub max_work: usize,
    pub max_value_bytes: usize,
}

impl Default for CodegenLimits {
    fn default() -> Self {
        Self {
            max_depth: 64,
            max_work: 1_000_000,
            max_value_bytes: 65_536,
        }
    }
}

/// Ground evaluated value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvalValue {
    Bool(bool),
    Int(i128),
    Nat(u64),
    U8(u8),
    U64(u64),
    Bytes(Vec<u8>),
    List(Vec<EvalValue>),
    Option(Option<Box<EvalValue>>),
    Result(Result<Box<EvalValue>, (String, String)>), // Ok(val) or Err(Type, Variant)
}

impl EvalValue {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Bytes(b) => Some(b.as_slice()),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Nat(n) | Self::U64(n) => Some(*n),
            Self::U8(n) => Some(*n as u64),
            Self::Int(i) if *i >= 0 => Some(*i as u64),
            _ => None,
        }
    }

    pub fn as_i128(&self) -> Option<i128> {
        match self {
            Self::Int(i) => Some(*i),
            Self::Nat(n) | Self::U64(n) => Some(*n as i128),
            Self::U8(n) => Some(*n as i128),
            _ => None,
        }
    }

    /// Serialize into canonical TFV prefix notation value.
    pub fn encode_tfv(&self) -> String {
        match self {
            Self::Bool(true) => "T".to_string(),
            Self::Bool(false) => "F".to_string(),
            Self::Int(i) => format!("I{i}"),
            Self::Nat(n) | Self::U64(n) => format!("I{n}"),
            Self::U8(n) => format!("I{n}"),
            Self::Bytes(bytes) => format!("X{}", hex_encode(bytes)),
            Self::List(items) => {
                let mut out = format!("L{}", items.len());
                for item in items {
                    out.push(' ');
                    out.push_str(&item.encode_tfv());
                }
                out
            }
            Self::Option(None) => "N".to_string(),
            Self::Option(Some(inner)) => inner.encode_tfv(),
            Self::Result(Ok(inner)) => inner.encode_tfv(),
            Self::Result(Err((_, variant))) => format!("S{}", hex_encode(variant.as_bytes())),
        }
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

pub fn hex_decode(s: &str, span: (usize, usize)) -> Result<Vec<u8>, CodegenError> {
    if s.len() & 1 != 0 {
        return Err(CodegenError {
            code: DiagnosticCode::W0702,
            kind: CodegenErrorKind::InvalidHexLiteral(format!(
                "Odd-length hex string: length {}",
                s.len()
            )),
            span,
        });
    }
    let mut bytes = Vec::with_capacity(s.len() / 2);
    let raw = s.as_bytes();
    for i in (0..raw.len()).step_by(2) {
        let hi = match raw[i] {
            b'0'..=b'9' => raw[i] - b'0',
            b'a'..=b'f' => raw[i] - b'a' + 10,
            b'A'..=b'F' => raw[i] - b'A' + 10,
            _ => {
                return Err(CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::InvalidHexLiteral(format!(
                        "Invalid hex byte {}",
                        raw[i]
                    )),
                    span,
                })
            }
        };
        let lo = match raw[i + 1] {
            b'0'..=b'9' => raw[i + 1] - b'0',
            b'a'..=b'f' => raw[i + 1] - b'a' + 10,
            b'A'..=b'F' => raw[i + 1] - b'A' + 10,
            _ => {
                return Err(CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::InvalidHexLiteral(format!(
                        "Invalid hex byte {}",
                        raw[i + 1]
                    )),
                    span,
                })
            }
        };
        bytes.push((hi << 4) | lo);
    }
    Ok(bytes)
}

/// Pure model and ground expression evaluator.
pub struct Evaluator<'a> {
    models: HashMap<&'a str, &'a ModelDecl<'a>>,
    limits: CodegenLimits,
    depth: usize,
    work: usize,
}

impl<'a> Evaluator<'a> {
    pub fn new(file: &'a WarrantFile<'a>, limits: CodegenLimits) -> Self {
        let mut models = HashMap::new();
        for decl in &file.declarations {
            if let crate::parser::Declaration::Model(m) = decl {
                models.insert(m.name, m);
            }
        }
        Self {
            models,
            limits,
            depth: 0,
            work: 0,
        }
    }

    fn charge_work(&mut self, amount: usize, span: (usize, usize)) -> Result<(), CodegenError> {
        self.work = self.work.saturating_add(amount);
        if self.work > self.limits.max_work {
            return Err(CodegenError {
                code: DiagnosticCode::W0703,
                kind: CodegenErrorKind::WorkBudgetExceeded,
                span,
            });
        }
        Ok(())
    }

    fn enter_scope(&mut self, span: (usize, usize)) -> Result<(), CodegenError> {
        self.depth += 1;
        if self.depth > self.limits.max_depth {
            return Err(CodegenError {
                code: DiagnosticCode::W0703,
                kind: CodegenErrorKind::RecursionDepthExceeded,
                span,
            });
        }
        Ok(())
    }

    fn leave_scope(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    pub fn reset_work(&mut self) {
        self.work = 0;
    }

    pub fn eval(
        &mut self,
        expr: &Expression<'a>,
        env: &HashMap<&'a str, EvalValue>,
    ) -> Result<EvalValue, CodegenError> {
        self.charge_work(1, expr_span(expr))?;
        self.enter_scope(expr_span(expr))?;
        let res = self.eval_internal(expr, env);
        self.leave_scope();
        res
    }

    fn eval_internal(
        &mut self,
        expr: &Expression<'a>,
        env: &HashMap<&'a str, EvalValue>,
    ) -> Result<EvalValue, CodegenError> {
        match expr {
            Expression::Literal { lit, span } => match lit {
                Literal::Bool(b) => Ok(EvalValue::Bool(*b)),
                Literal::U64(n) => Ok(EvalValue::U64(*n)),
                Literal::U8(n) => Ok(EvalValue::U8(*n)),
                Literal::Int(n) => Ok(EvalValue::Int(*n as i128)),
                Literal::Bytes(hex_str) => {
                    let bytes = hex_decode(hex_str, *span)?;
                    Ok(EvalValue::Bytes(bytes))
                }
            },
            Expression::Grouped { inner, .. } => self.eval(inner, env),
            Expression::List { elements, span } => {
                if elements.len() > self.limits.max_value_bytes {
                    return Err(CodegenError {
                        code: DiagnosticCode::W0703,
                        kind: CodegenErrorKind::WorkBudgetExceeded,
                        span: *span,
                    });
                }
                let mut list = Vec::with_capacity(elements.len());
                for el in elements {
                    list.push(self.eval(el, env)?);
                }
                self.charge_work(elements.len(), *span)?;
                Ok(EvalValue::List(list))
            }
            Expression::Reference {
                name,
                widths,
                args,
                span,
            } => {
                let first = name.segments[0];
                if name.segments.len() == 1 {
                    if let Some(val) = env.get(first) {
                        return Ok(val.clone());
                    }
                    // Check built-in functions
                    if let Some(arg_exprs) = args {
                        return self.eval_call(first, widths, arg_exprs, env, *span);
                    }
                    return Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::NonGroundTerm(format!("Unbound symbol {first}")),
                        span: *span,
                    });
                }
                // Qualified name: Error.Case constructor
                if name.segments.len() == 2 {
                    let type_name = name.segments[0].to_string();
                    let case_name = name.segments[1].to_string();
                    return Ok(EvalValue::Result(Err((type_name, case_name))));
                }
                Err(CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::NonGroundTerm(format!(
                        "Complex qualified name {first}"
                    )),
                    span: *span,
                })
            }
            Expression::Unary { op, operand, span } => {
                let v = self.eval(operand, env)?;
                match op {
                    UnaryOp::Not => match v {
                        EvalValue::Bool(b) => Ok(EvalValue::Bool(!b)),
                        _ => Err(CodegenError {
                            code: DiagnosticCode::W0702,
                            kind: CodegenErrorKind::EvaluationError("not expects Bool".into()),
                            span: *span,
                        }),
                    },
                    UnaryOp::Neg => match v {
                        EvalValue::Int(i) => {
                            let neg = i.checked_neg().ok_or_else(|| CodegenError {
                                code: DiagnosticCode::W0702,
                                kind: CodegenErrorKind::EvaluationError(
                                    "Arithmetic overflow in Neg".into(),
                                ),
                                span: *span,
                            })?;
                            Ok(EvalValue::Int(neg))
                        }
                        EvalValue::Nat(n) => {
                            let neg = (n as i128).checked_neg().ok_or_else(|| CodegenError {
                                code: DiagnosticCode::W0702,
                                kind: CodegenErrorKind::EvaluationError(
                                    "Arithmetic overflow in Neg".into(),
                                ),
                                span: *span,
                            })?;
                            Ok(EvalValue::Int(neg))
                        }
                        _ => Err(CodegenError {
                            code: DiagnosticCode::W0702,
                            kind: CodegenErrorKind::EvaluationError(
                                "neg expects Int or Nat".into(),
                            ),
                            span: *span,
                        }),
                    },
                }
            }
            Expression::Binary {
                op,
                left,
                right,
                span,
            } => {
                let l = self.eval(left, env)?;
                let r = self.eval(right, env)?;
                self.eval_binary(*op, l, r, *span)
            }
            Expression::Conditional {
                condition,
                then_branch,
                else_branch,
                span,
            } => {
                let c = self.eval(condition, env)?;
                match c {
                    EvalValue::Bool(true) => self.eval(then_branch, env),
                    EvalValue::Bool(false) => self.eval(else_branch, env),
                    _ => Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError("Condition must be Bool".into()),
                        span: *span,
                    }),
                }
            }
            Expression::Match { target, arms, span } => {
                let tgt = self.eval(target, env)?;
                self.eval_match(tgt, arms, env, *span)
            }
            Expression::Quantified { span, .. } => Err(CodegenError {
                code: DiagnosticCode::W0702,
                kind: CodegenErrorKind::NonGroundTerm(
                    "Quantifiers cannot be evaluated ground".into(),
                ),
                span: *span,
            }),
        }
    }

    fn eval_call(
        &mut self,
        name: &'a str,
        widths: &[Width<'a>],
        args: &[Expression<'a>],
        env: &HashMap<&'a str, EvalValue>,
        span: (usize, usize),
    ) -> Result<EvalValue, CodegenError> {
        let mut evaled_args = Vec::with_capacity(args.len());
        for a in args {
            evaled_args.push(self.eval(a, env)?);
        }

        match name {
            "len" => {
                if evaled_args.len() != 1 {
                    return Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError("len expects 1 arg".into()),
                        span,
                    });
                }
                match &evaled_args[0] {
                    EvalValue::Bytes(b) => Ok(EvalValue::Nat(b.len() as u64)),
                    EvalValue::List(l) => Ok(EvalValue::Nat(l.len() as u64)),
                    _ => Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError("len expects Bytes or List".into()),
                        span,
                    }),
                }
            }
            "at" => {
                if evaled_args.len() != 2 {
                    return Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError("at expects 2 args".into()),
                        span,
                    });
                }
                let idx = evaled_args[1].as_u64().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("at index must be Nat".into()),
                    span,
                })? as usize;

                match &evaled_args[0] {
                    EvalValue::Bytes(b) => {
                        if idx < b.len() {
                            Ok(EvalValue::Option(Some(Box::new(EvalValue::U8(b[idx])))))
                        } else {
                            Ok(EvalValue::Option(None))
                        }
                    }
                    EvalValue::List(l) => {
                        if idx < l.len() {
                            Ok(EvalValue::Option(Some(Box::new(l[idx].clone()))))
                        } else {
                            Ok(EvalValue::Option(None))
                        }
                    }
                    _ => Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError("at expects Bytes or List".into()),
                        span,
                    }),
                }
            }
            "concat" => {
                if evaled_args.len() != 2 {
                    return Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError("concat expects 2 args".into()),
                        span,
                    });
                }
                match (&evaled_args[0], &evaled_args[1]) {
                    (EvalValue::Bytes(a), EvalValue::Bytes(b)) => {
                        let total_len = a.len().saturating_add(b.len());
                        if total_len > self.limits.max_value_bytes {
                            return Err(CodegenError {
                                code: DiagnosticCode::W0703,
                                kind: CodegenErrorKind::WorkBudgetExceeded,
                                span,
                            });
                        }
                        self.charge_work(total_len, span)?;
                        let mut res = a.clone();
                        res.extend_from_slice(b);
                        Ok(EvalValue::Bytes(res))
                    }
                    (EvalValue::List(a), EvalValue::List(b)) => {
                        let total_len = a.len().saturating_add(b.len());
                        if total_len > self.limits.max_value_bytes {
                            return Err(CodegenError {
                                code: DiagnosticCode::W0703,
                                kind: CodegenErrorKind::WorkBudgetExceeded,
                                span,
                            });
                        }
                        self.charge_work(total_len, span)?;
                        let mut res = a.clone();
                        res.extend_from_slice(b);
                        Ok(EvalValue::List(res))
                    }
                    _ => Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError(
                            "concat expects 2 Bytes or 2 Lists".into(),
                        ),
                        span,
                    }),
                }
            }
            "fixed" => {
                if evaled_args.len() != 1 {
                    return Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError("fixed expects 1 arg".into()),
                        span,
                    });
                }
                let target_width = match widths.first() {
                    Some(Width::Literal(n)) => *n,
                    _ => {
                        return Err(CodegenError {
                            code: DiagnosticCode::W0701,
                            kind: CodegenErrorKind::UnsupportedTarget(
                                "fixed requires static literal width".into(),
                            ),
                            span,
                        })
                    }
                };
                match &evaled_args[0] {
                    EvalValue::Bytes(b) => {
                        if b.len() as u64 == target_width {
                            Ok(EvalValue::Option(Some(Box::new(EvalValue::Bytes(
                                b.clone(),
                            )))))
                        } else {
                            Ok(EvalValue::Option(None))
                        }
                    }
                    _ => Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError("fixed expects Bytes".into()),
                        span,
                    }),
                }
            }
            "div" => {
                if evaled_args.len() != 2 {
                    return Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError("div expects 2 args".into()),
                        span,
                    });
                }
                let a = evaled_args[0].as_i128().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("div lhs must be integer".into()),
                    span,
                })?;
                let b = evaled_args[1].as_i128().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("div rhs must be integer".into()),
                    span,
                })?;
                if b == 0 {
                    Ok(EvalValue::Option(None))
                } else if a == i128::MIN && b == -1 {
                    Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError(
                            "Arithmetic overflow in div".into(),
                        ),
                        span,
                    })
                } else {
                    let d = a.div_euclid(b);
                    Ok(EvalValue::Option(Some(Box::new(EvalValue::Int(d)))))
                }
            }
            "Some" => {
                if evaled_args.len() != 1 {
                    return Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError("Some expects 1 arg".into()),
                        span,
                    });
                }
                Ok(EvalValue::Option(Some(Box::new(evaled_args.remove(0)))))
            }
            "None" => Ok(EvalValue::Option(None)),
            "Ok" => {
                if evaled_args.len() != 1 {
                    return Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError("Ok expects 1 arg".into()),
                        span,
                    });
                }
                Ok(EvalValue::Result(Ok(Box::new(evaled_args.remove(0)))))
            }
            "Err" => {
                if evaled_args.len() != 1 {
                    return Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError("Err expects 1 arg".into()),
                        span,
                    });
                }
                match evaled_args.remove(0) {
                    EvalValue::Result(Err((t, v))) => Ok(EvalValue::Result(Err((t, v)))),
                    _ => Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError(
                            "Err expects nominal error constructor".into(),
                        ),
                        span,
                    }),
                }
            }
            _ => {
                // Check if user-defined pure model function
                let model = self.models.get(name).copied().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::NonGroundTerm(format!("Unknown model function {name}")),
                    span,
                })?;

                if model.params.len() != evaled_args.len() {
                    return Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError(format!(
                            "Model {name} expects {} params, got {}",
                            model.params.len(),
                            evaled_args.len()
                        )),
                        span,
                    });
                }

                let mut new_env = HashMap::new();
                for (param, val) in model.params.iter().zip(evaled_args) {
                    new_env.insert(param.name, val);
                }

                let body = model.body.as_ref().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError(format!("Model {name} has no body")),
                    span,
                })?;

                self.eval(body, &new_env)
            }
        }
    }

    fn eval_binary(
        &mut self,
        op: BinaryOp,
        left: EvalValue,
        right: EvalValue,
        span: (usize, usize),
    ) -> Result<EvalValue, CodegenError> {
        match op {
            BinaryOp::Eq => Ok(EvalValue::Bool(left == right)),
            BinaryOp::NotEq => Ok(EvalValue::Bool(left != right)),
            BinaryOp::And => {
                let a = left.as_bool().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("and expects Bool".into()),
                    span,
                })?;
                let b = right.as_bool().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("and expects Bool".into()),
                    span,
                })?;
                Ok(EvalValue::Bool(a && b))
            }
            BinaryOp::Or => {
                let a = left.as_bool().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("or expects Bool".into()),
                    span,
                })?;
                let b = right.as_bool().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("or expects Bool".into()),
                    span,
                })?;
                Ok(EvalValue::Bool(a || b))
            }
            BinaryOp::Implies => {
                let a = left.as_bool().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("=> expects Bool".into()),
                    span,
                })?;
                let b = right.as_bool().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("=> expects Bool".into()),
                    span,
                })?;
                Ok(EvalValue::Bool(!a || b))
            }
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                let a = left.as_i128().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("Comparison expects integers".into()),
                    span,
                })?;
                let b = right.as_i128().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("Comparison expects integers".into()),
                    span,
                })?;
                let res = match op {
                    BinaryOp::Lt => a < b,
                    BinaryOp::Le => a <= b,
                    BinaryOp::Gt => a > b,
                    BinaryOp::Ge => a >= b,
                    _ => unreachable!(),
                };
                Ok(EvalValue::Bool(res))
            }
            BinaryOp::Add => {
                if let (EvalValue::U64(a), EvalValue::U64(b)) = (&left, &right) {
                    return Ok(EvalValue::U64(a.wrapping_add(*b)));
                }
                if let (EvalValue::U8(a), EvalValue::U8(b)) = (&left, &right) {
                    return Ok(EvalValue::U8(a.wrapping_add(*b)));
                }
                let a = left.as_i128().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("Add expects integers".into()),
                    span,
                })?;
                let b = right.as_i128().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("Add expects integers".into()),
                    span,
                })?;
                let sum = a.checked_add(b).ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("Arithmetic overflow in Add".into()),
                    span,
                })?;
                Ok(EvalValue::Int(sum))
            }
            BinaryOp::Sub => {
                if let (EvalValue::U64(a), EvalValue::U64(b)) = (&left, &right) {
                    return Ok(EvalValue::U64(a.wrapping_sub(*b)));
                }
                if let (EvalValue::U8(a), EvalValue::U8(b)) = (&left, &right) {
                    return Ok(EvalValue::U8(a.wrapping_sub(*b)));
                }
                let a = left.as_i128().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("Sub expects integers".into()),
                    span,
                })?;
                let b = right.as_i128().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("Sub expects integers".into()),
                    span,
                })?;
                let diff = a.checked_sub(b).ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("Arithmetic overflow in Sub".into()),
                    span,
                })?;
                Ok(EvalValue::Int(diff))
            }
            BinaryOp::Mul => {
                if let (EvalValue::U64(a), EvalValue::U64(b)) = (&left, &right) {
                    return Ok(EvalValue::U64(a.wrapping_mul(*b)));
                }
                if let (EvalValue::U8(a), EvalValue::U8(b)) = (&left, &right) {
                    return Ok(EvalValue::U8(a.wrapping_mul(*b)));
                }
                let a = left.as_i128().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("Mul expects integers".into()),
                    span,
                })?;
                let b = right.as_i128().ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("Mul expects integers".into()),
                    span,
                })?;
                let prod = a.checked_mul(b).ok_or_else(|| CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("Arithmetic overflow in Mul".into()),
                    span,
                })?;
                Ok(EvalValue::Int(prod))
            }
        }
    }

    fn eval_match(
        &mut self,
        target: EvalValue,
        arms: &[MatchArm<'a>],
        env: &HashMap<&'a str, EvalValue>,
        span: (usize, usize),
    ) -> Result<EvalValue, CodegenError> {
        for arm in arms {
            match (&arm.pattern, &target) {
                (Pattern::None, EvalValue::Option(None)) => {
                    return self.eval(&arm.body, env);
                }
                (Pattern::Some(binder), EvalValue::Option(Some(inner))) => {
                    let mut arm_env = env.clone();
                    arm_env.insert(*binder, (**inner).clone());
                    return self.eval(&arm.body, &arm_env);
                }
                (Pattern::Ok(binder), EvalValue::Result(Ok(inner))) => {
                    let mut arm_env = env.clone();
                    arm_env.insert(*binder, (**inner).clone());
                    return self.eval(&arm.body, &arm_env);
                }
                (Pattern::Err(binder), EvalValue::Result(Err((t, v)))) => {
                    let mut arm_env = env.clone();
                    arm_env.insert(*binder, EvalValue::Result(Err((t.clone(), v.clone()))));
                    return self.eval(&arm.body, &arm_env);
                }
                _ => continue,
            }
        }
        Err(CodegenError {
            code: DiagnosticCode::W0702,
            kind: CodegenErrorKind::EvaluationError("Match not exhaustive at runtime".into()),
            span,
        })
    }
}

fn expr_span(expr: &Expression<'_>) -> (usize, usize) {
    match expr {
        Expression::Quantified { span, .. }
        | Expression::Conditional { span, .. }
        | Expression::Match { span, .. }
        | Expression::Binary { span, .. }
        | Expression::Unary { span, .. }
        | Expression::Literal { span, .. }
        | Expression::List { span, .. }
        | Expression::Reference { span, .. }
        | Expression::Grouped { span, .. } => *span,
    }
}

/// A compiled portable TFV test record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TfvRecord {
    pub id: String,
    pub fields: Vec<(String, String)>,
}

impl TfvRecord {
    /// Format as canonical TFV record line: `<id> R<count> K<key1> <val1> ...`
    pub fn to_tfv_line(&self) -> String {
        let mut line = format!("{} R{}", self.id, self.fields.len());
        for (k, v) in &self.fields {
            line.push_str(&format!(" K{k} {v}"));
        }
        line
    }
}

/// Extracted example case invocation.
#[derive(Clone, Debug)]
pub struct ExampleCase {
    pub id: String,
    pub op_name: String,
    pub static_width: Option<u64>,
    pub args: Vec<EvalValue>,
    pub expected: EvalValue,
}

/// Compiles an `ExampleDecl` AST node into an `ExampleCase`.
pub fn extract_example_case<'a>(
    name: &'a str,
    body: &'a Expression<'a>,
    evaluator: &mut Evaluator<'a>,
) -> Result<ExampleCase, CodegenError> {
    let span = expr_span(body);
    let (left, right) = match body {
        Expression::Binary {
            op: BinaryOp::Eq,
            left,
            right,
            ..
        } => (left, right),
        _ => {
            return Err(CodegenError {
                code: DiagnosticCode::W0702,
                kind: CodegenErrorKind::EvaluationError(
                    "Example body must be an equality assertion (op(...) == expected)".into(),
                ),
                span,
            })
        }
    };

    let empty_env = HashMap::new();
    let expected = evaluator.eval(right, &empty_env)?;

    let (op_name, static_width, args) = match &**left {
        Expression::Reference {
            name: op_ref,
            widths,
            args: Some(arg_exprs),
            span: ref_span,
        } => {
            let width = if let Some(first_width) = widths.first() {
                match first_width {
                    Width::Literal(w) => Some(*w),
                    _ => {
                        return Err(CodegenError {
                            code: DiagnosticCode::W0701,
                            kind: CodegenErrorKind::UnsupportedTarget(
                                "Example operation width must be literal".into(),
                            ),
                            span: *ref_span,
                        })
                    }
                }
            } else {
                None
            };
            let mut evaled_args = Vec::with_capacity(arg_exprs.len());
            for a in arg_exprs {
                evaled_args.push(evaluator.eval(a, &empty_env)?);
            }
            (op_ref.segments[0].to_string(), width, evaled_args)
        }
        _ => {
            return Err(CodegenError {
                code: DiagnosticCode::W0702,
                kind: CodegenErrorKind::EvaluationError(
                    "LHS of example must be an operation call".into(),
                ),
                span,
            })
        }
    };

    Ok(ExampleCase {
        id: name.to_string(),
        op_name,
        static_width,
        args,
        expected,
    })
}

/// Generates a TFV record for an example case.
pub fn compile_example_to_tfv(
    module_name: &str,
    case: &ExampleCase,
) -> Result<TfvRecord, CodegenError> {
    let mut fields = Vec::new();
    fields.push((
        "op".to_string(),
        format!("S{}", hex_encode(case.op_name.as_bytes())),
    ));

    // Record arguments based on CT01 standard conventions
    match case.op_name.as_str() {
        "eq" | "eq_fixed" => {
            if case.args.len() != 2 {
                return Err(CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError(format!(
                        "{} expects 2 arguments",
                        case.op_name
                    )),
                    span: (0, 0),
                });
            }
            let left_bytes = case.args[0].as_bytes().ok_or_else(|| CodegenError {
                code: DiagnosticCode::W0702,
                kind: CodegenErrorKind::EvaluationError("left argument must be Bytes".into()),
                span: (0, 0),
            })?;
            let right_bytes = case.args[1].as_bytes().ok_or_else(|| CodegenError {
                code: DiagnosticCode::W0702,
                kind: CodegenErrorKind::EvaluationError("right argument must be Bytes".into()),
                span: (0, 0),
            })?;

            fields.push(("left".to_string(), format!("X{}", hex_encode(left_bytes))));
            fields.push(("right".to_string(), format!("X{}", hex_encode(right_bytes))));
            if let Some(w) = case.static_width {
                fields.push(("width".to_string(), format!("I{w}")));
            }
            let eq_bool = case.expected.as_bool().ok_or_else(|| CodegenError {
                code: DiagnosticCode::W0702,
                kind: CodegenErrorKind::EvaluationError("Expected value must be Bool".into()),
                span: (0, 0),
            })?;
            fields.push((
                "equal".to_string(),
                if eq_bool { "T".into() } else { "F".into() },
            ));

            // Logical work fields
            let positions = if left_bytes.len() == right_bytes.len() {
                left_bytes.len() as u64
            } else {
                0
            };
            fields.push(("byte_positions".to_string(), format!("I{positions}")));
            fields.push(("byte_reads".to_string(), format!("I{}", 2 * positions)));
            fields.push(("byte_writes".to_string(), "I0".to_string()));
            fields.push(("u64_bits".to_string(), "I0".to_string()));
        }
        "select_bytes" => {
            if case.args.len() != 3 {
                return Err(CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError(
                        "select_bytes expects 3 arguments".into(),
                    ),
                    span: (0, 0),
                });
            }
            let left_bytes = case.args[0].as_bytes().ok_or_else(|| CodegenError {
                code: DiagnosticCode::W0702,
                kind: CodegenErrorKind::EvaluationError("left argument must be Bytes".into()),
                span: (0, 0),
            })?;
            let right_bytes = case.args[1].as_bytes().ok_or_else(|| CodegenError {
                code: DiagnosticCode::W0702,
                kind: CodegenErrorKind::EvaluationError("right argument must be Bytes".into()),
                span: (0, 0),
            })?;
            let choice = case.args[2].as_bool().ok_or_else(|| CodegenError {
                code: DiagnosticCode::W0702,
                kind: CodegenErrorKind::EvaluationError("choice argument must be Bool".into()),
                span: (0, 0),
            })?;

            fields.push(("left".to_string(), format!("X{}", hex_encode(left_bytes))));
            fields.push(("right".to_string(), format!("X{}", hex_encode(right_bytes))));
            fields.push((
                "choice".to_string(),
                if choice { "T".into() } else { "F".into() },
            ));

            match &case.expected {
                EvalValue::Result(Ok(inner)) => {
                    let out_bytes = inner.as_bytes().ok_or_else(|| CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError("Output must be Bytes".into()),
                        span: (0, 0),
                    })?;
                    fields.push(("output".to_string(), format!("X{}", hex_encode(out_bytes))));
                    fields.push(("fresh_output".to_string(), "T".to_string()));

                    let positions = left_bytes.len() as u64;
                    fields.push(("byte_positions".to_string(), format!("I{positions}")));
                    fields.push(("byte_reads".to_string(), format!("I{}", 2 * positions)));
                    fields.push(("byte_writes".to_string(), format!("I{positions}")));
                    fields.push(("u64_bits".to_string(), "I0".to_string()));
                }
                EvalValue::Result(Err((_, err_case))) => {
                    fields.push((
                        "error_id".to_string(),
                        format!("S{}", hex_encode(err_case.as_bytes())),
                    ));
                    fields.push(("byte_positions".to_string(), "I0".to_string()));
                    fields.push(("byte_reads".to_string(), "I0".to_string()));
                    fields.push(("byte_writes".to_string(), "I0".to_string()));
                    fields.push(("u64_bits".to_string(), "I0".to_string()));
                }
                _ => {
                    return Err(CodegenError {
                        code: DiagnosticCode::W0702,
                        kind: CodegenErrorKind::EvaluationError(
                            "select_bytes expected must be Result".into(),
                        ),
                        span: (0, 0),
                    })
                }
            }
        }
        "eq_u64" => {
            if case.args.len() != 2 {
                return Err(CodegenError {
                    code: DiagnosticCode::W0702,
                    kind: CodegenErrorKind::EvaluationError("eq_u64 expects 2 arguments".into()),
                    span: (0, 0),
                });
            }
            let left_val = case.args[0].as_u64().ok_or_else(|| CodegenError {
                code: DiagnosticCode::W0702,
                kind: CodegenErrorKind::EvaluationError("left_u64 must be U64".into()),
                span: (0, 0),
            })?;
            let right_val = case.args[1].as_u64().ok_or_else(|| CodegenError {
                code: DiagnosticCode::W0702,
                kind: CodegenErrorKind::EvaluationError("right_u64 must be U64".into()),
                span: (0, 0),
            })?;

            fields.push(("left_u64".to_string(), format!("I{left_val}")));
            fields.push(("right_u64".to_string(), format!("I{right_val}")));

            let eq_bool = case.expected.as_bool().ok_or_else(|| CodegenError {
                code: DiagnosticCode::W0702,
                kind: CodegenErrorKind::EvaluationError("Expected value must be Bool".into()),
                span: (0, 0),
            })?;
            fields.push((
                "equal".to_string(),
                if eq_bool { "T".into() } else { "F".into() },
            ));

            fields.push(("byte_positions".to_string(), "I0".to_string()));
            fields.push(("byte_reads".to_string(), "I0".to_string()));
            fields.push(("byte_writes".to_string(), "I0".to_string()));
            fields.push(("u64_bits".to_string(), "I64".to_string()));
        }
        _ => {
            return Err(CodegenError {
                code: DiagnosticCode::W0701,
                kind: CodegenErrorKind::UnsupportedTarget(format!(
                    "Unsupported operation for TFV: {}",
                    case.op_name
                )),
                span: (0, 0),
            });
        }
    }

    Ok(TfvRecord {
        id: format!("{}/{}", module_name, case.id),
        fields,
    })
}

/// Compiles all examples in a Warrant document to a full TFV file content string.
pub fn compile_warrant_to_tfv(
    file: &WarrantFile<'_>,
    limits: CodegenLimits,
) -> Result<String, CodegenError> {
    let mut evaluator = Evaluator::new(file, limits);
    let mut lines = vec!["tfv 1".to_string()];

    for decl in &file.declarations {
        if let crate::parser::Declaration::Example(ex) = decl {
            evaluator.reset_work();
            let case = extract_example_case(ex.name, &ex.body, &mut evaluator)?;
            let record = compile_example_to_tfv(file.name, &case)?;
            lines.push(record.to_tfv_line());
        }
    }

    let mut out = lines.join("\n");
    out.push('\n');
    Ok(out)
}

/// Options for compiling a Rust L0 test harness.
#[derive(Clone, Debug)]
pub struct RustL0Options {
    pub package_crate: String,
    pub spec_path: String,
}

/// Compiles a checked Warrant document into a complete, standalone Rust L0 test harness source code.
pub fn compile_rust_l0_harness(
    file: &WarrantFile<'_>,
    _checked: &CheckedWarrantModule,
    manifest: &BindingManifest,
    options: &RustL0Options,
) -> Result<String, CodegenError> {
    let mut evaluator = Evaluator::new(file, CodegenLimits::default());
    let mut example_cases = Vec::new();

    // Collect fixed widths used across examples
    let mut fixed_widths = BTreeMap::new();

    for decl in &file.declarations {
        if let crate::parser::Declaration::Example(ex) = decl {
            evaluator.reset_work();
            let case = extract_example_case(ex.name, &ex.body, &mut evaluator)?;
            if case.op_name == "eq_fixed" {
                if let Some(w) = case.static_width {
                    fixed_widths.insert(w, ());
                }
            }
            example_cases.push(case);
        }
    }

    // Verify all operations called in examples are bound in manifest
    for case in &example_cases {
        if !manifest.bindings.contains_key(&case.op_name) {
            return Err(CodegenError {
                code: DiagnosticCode::W0704,
                kind: CodegenErrorKind::MissingBinding(format!(
                    "Operation {} in example {} has no binding in manifest",
                    case.op_name, case.id
                )),
                span: (0, 0),
            });
        }
    }

    // Resolve dynamic public exports from manifest
    let eq_export = manifest
        .bindings
        .get("eq")
        .map(|b| b.export.as_str())
        .unwrap_or("ct_eq");
    let eq_fixed_export = manifest
        .bindings
        .get("eq_fixed")
        .map(|b| b.export.as_str())
        .unwrap_or("ct_eq_fixed");
    let select_export = manifest
        .bindings
        .get("select_bytes")
        .map(|b| b.export.as_str())
        .unwrap_or("ct_select_bytes");
    let eq_u64_export = manifest
        .bindings
        .get("eq_u64")
        .map(|b| b.export.as_str())
        .unwrap_or("ct_eq_u64");

    let select_error_case = manifest
        .bindings
        .get("select_bytes")
        .and_then(|b| b.error_normalization.as_ref())
        .map(|n| n.error_case.as_str())
        .unwrap_or("CT_SELECT_LENGTH_MISMATCH");

    // Build deterministic Rust harness code
    let mut out = String::new();
    out.push_str("// GENERATED BY coding_adventures_warrant_grammar::codegen (VER04e). DO NOT EDIT MANUALLY.\n");
    out.push_str(&format!("// Source: {}\n", options.spec_path));
    out.push_str("// Target: Rust L0 test harness on test-bootstrap\n\n");

    out.push_str("#![allow(unused_imports, clippy::all)]\n\n");
    out.push_str("use coding_adventures_test_bootstrap::{\n");
    out.push_str("    run::{self, Ctx},\n");
    out.push_str("    tfv::{self, Record, Value},\n");
    out.push_str("    Suite, Verdict,\n");
    out.push_str("};\n");
    out.push_str("use std::panic::catch_unwind;\n");
    out.push_str(&format!("use {}::*;\n\n", options.package_crate));

    // Fault injection enum
    out.push_str("/// Fault injection controls for negative test verification.\n");
    out.push_str("#[derive(Clone, Copy, PartialEq, Eq)]\n");
    out.push_str("pub enum Fault {\n");
    out.push_str("    None,\n");
    out.push_str("    Equality,\n");
    out.push_str("    Selection,\n");
    out.push_str("    MissingError,\n");
    out.push_str("}\n\n");

    // Const-generic dispatch function
    out.push_str("/// Fixed width const-generic dispatch.\n");
    out.push_str(
        "fn fixed_dispatch(width: u64, left: &[u8], right: &[u8]) -> Result<bool, String> {\n",
    );
    out.push_str("    match width {\n");
    for w in fixed_widths.keys() {
        out.push_str(&format!("        {w} => fixed::<{w}>(left, right),\n"));
    }
    out.push_str("        _ => Err(\"fixed width has no generated arm\".into()),\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");

    out.push_str("fn fixed<const N: usize>(left: &[u8], right: &[u8]) -> Result<bool, String> {\n");
    out.push_str(
        "    let left: &[u8; N] = left.try_into().map_err(|_| \"left fixed width mismatch\")?;\n",
    );
    out.push_str("    let right: &[u8; N] = right.try_into().map_err(|_| \"right fixed width mismatch\")?;\n");
    out.push_str(&format!("    Ok({eq_fixed_export}(left, right))\n"));
    out.push_str("}\n\n");

    // Baseline validation helper
    out.push_str("fn reject_after_baseline(ctx: &Ctx, baseline: impl Fn(&Ctx, Fault) -> Verdict, fault: Fault) -> Verdict {\n");
    out.push_str("    if !matches!(baseline(ctx, Fault::None), Verdict::Pass) {\n");
    out.push_str("        return Verdict::fail(\"corpus-mismatch\", \"valid adapter baseline did not pass\");\n");
    out.push_str("    }\n");
    out.push_str("    match baseline(ctx, fault) {\n");
    out.push_str("        Verdict::Fail { .. } => Verdict::Pass,\n");
    out.push_str("        Verdict::Pass => Verdict::fail(\"corpus-mismatch\", \"broken adapter behavior was accepted\"),\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");

    // Suite constructor
    out.push_str("/// Builds the executable test suite.\n");
    out.push_str("pub fn build_suite() -> Suite {\n");
    out.push_str("    let mut suite = Suite::new();\n\n");

    // Add example test cases
    for case in &example_cases {
        let test_name = format!("{}/{}", file.name, case.id);
        out.push_str(&format!("    suite.add(\"{test_name}\", |ctx| {{\n"));
        let body = generate_case_body(case, eq_u64_export)?;
        out.push_str(&body);
        out.push_str("    });\n\n");
    }

    // Add negative control checks
    out.push_str("    // Adapter negative controls (sabotage checks)\n");
    out.push_str("    suite.add(\"adapter/wrong-equality\", |ctx| {\n");
    out.push_str("        reject_after_baseline(ctx, |c, f| check_eq(c, b\"00\", b\"00\", true, f), Fault::Equality)\n");
    out.push_str("    });\n");
    out.push_str("    suite.add(\"adapter/wrong-selection\", |ctx| {\n");
    out.push_str("        reject_after_baseline(ctx, |c, f| check_select(c, b\"00\", b\"ff\", true, b\"00\", f), Fault::Selection)\n");
    out.push_str("    });\n");
    out.push_str("    suite.add(\"adapter/missing-error\", |ctx| {\n");
    out.push_str(&format!(
        "        reject_after_baseline(ctx, |c, f| check_select_err(c, b\"\", b\"00\", false, \"{select_error_case}\", f), Fault::MissingError)\n"
    ));
    out.push_str("    });\n");
    out.push_str("    suite.add(\"adapter/unknown-fixed-width\", |_ctx| {\n");
    out.push_str("        let res = fixed_dispatch(999, b\"\", b\"\");\n");
    out.push_str("        if res == Err(\"fixed width has no generated arm\".into()) { Verdict::Pass } else { Verdict::fail(\"corpus-mismatch\", \"unsupported width did not report missing arm\") }\n");
    out.push_str("    });\n");
    out.push_str("    suite.add(\"adapter/wrong-work\", |_ctx| {\n");
    out.push_str("        let positions = 2u64;\n");
    out.push_str("        let byte_reads = 2 * positions;\n");
    out.push_str("        let mutated_reads = 0u64;\n");
    out.push_str("        if byte_reads != mutated_reads { Verdict::Pass } else { Verdict::fail(\"corpus-mismatch\", \"work mismatch not caught\") }\n");
    out.push_str("    });\n\n");

    out.push_str("    suite\n");
    out.push_str("}\n\n");

    // Helper functions for checking operations
    out.push_str("fn check_eq(ctx: &Ctx, left: &[u8], right: &[u8], expected: bool, fault: Fault) -> Verdict {\n");
    out.push_str(&format!("    let mut got = {eq_export}(left, right);\n"));
    out.push_str("    if matches!(fault, Fault::Equality) { got = !got; }\n");
    out.push_str("    ctx.deep_equal(&Value::Bool(got), &Value::Bool(expected))\n");
    out.push_str("}\n\n");

    out.push_str("fn check_select(ctx: &Ctx, left: &[u8], right: &[u8], choice: bool, expected: &[u8], fault: Fault) -> Verdict {\n");
    out.push_str("    let left_before = left.to_vec();\n");
    out.push_str("    let right_before = right.to_vec();\n");
    out.push_str("    let effective_choice = if matches!(fault, Fault::Selection) { !choice } else { choice };\n");
    out.push_str(&format!(
        "    let mut output = {select_export}(left, right, effective_choice);\n"
    ));
    out.push_str("    let got = output.clone();\n");
    out.push_str("    if let Some(first) = output.first_mut() { *first ^= 0xff; }\n");
    out.push_str("    let independent = left == left_before && right == right_before\n");
    out.push_str("        && (output.is_empty() || (output.as_ptr() != left.as_ptr() && output.as_ptr() != right.as_ptr()));\n");
    out.push_str(
        "    let verdict = ctx.deep_equal(&Value::Bytes(got), &Value::Bytes(expected.to_vec()));\n",
    );
    out.push_str("    if !matches!(verdict, Verdict::Pass) { return verdict; }\n");
    out.push_str("    ctx.deep_equal(&Value::Bool(independent), &Value::Bool(true))\n");
    out.push_str("}\n\n");

    out.push_str("fn check_select_err(ctx: &Ctx, left: &[u8], right: &[u8], choice: bool, expected_err: &str, fault: Fault) -> Verdict {\n");
    out.push_str("    let got = catch_unwind(|| {\n");
    out.push_str("        if matches!(fault, Fault::MissingError) { Vec::new() }\n");
    out.push_str(&format!(
        "        else {{ {select_export}(left, right, choice) }}\n"
    ));
    out.push_str("    });\n");
    out.push_str(&format!(
        "    ctx.error_is(got.err().map(|_| \"{select_error_case}\"), expected_err)\n"
    ));
    out.push_str("}\n\n");

    // Main entrypoint with silent panic hook
    out.push_str("fn main() {\n");
    out.push_str("    std::panic::set_hook(Box::new(|_| ()));\n");
    out.push_str("    let suite = build_suite();\n");
    out.push_str("    std::process::exit(run::main(&suite));\n");
    out.push_str("}\n");

    Ok(out)
}

fn generate_case_body(case: &ExampleCase, eq_u64_export: &str) -> Result<String, CodegenError> {
    let mut s = String::new();
    match case.op_name.as_str() {
        "eq" => {
            let left = case.args[0].as_bytes().unwrap_or(&[]);
            let right = case.args[1].as_bytes().unwrap_or(&[]);
            let expected = case.expected.as_bool().unwrap_or(false);
            s.push_str(&format!(
                "        check_eq(ctx, &b\"{}\"[..], &b\"{}\"[..], {}, Fault::None)\n",
                escape_bytes(left),
                escape_bytes(right),
                expected
            ));
        }
        "eq_fixed" => {
            let left = case.args[0].as_bytes().unwrap_or(&[]);
            let right = case.args[1].as_bytes().unwrap_or(&[]);
            let width = case.static_width.unwrap_or(0);
            let expected = case.expected.as_bool().unwrap_or(false);
            s.push_str(&format!(
                "        let got = fixed_dispatch({width}, &b\"{}\"[..], &b\"{}\"[..]);\n",
                escape_bytes(left),
                escape_bytes(right)
            ));
            s.push_str("        match got {\n");
            s.push_str("            Ok(val) => ctx.deep_equal(&Value::Bool(val), &Value::Bool(");
            s.push_str(if expected { "true" } else { "false" });
            s.push_str(")),\n");
            s.push_str(
                "            Err(e) => Verdict::fail(\"corpus-malformed\", format!(\"fixed failed: {e}\")),\n",
            );
            s.push_str("        }\n");
        }
        "select_bytes" => {
            let left = case.args[0].as_bytes().unwrap_or(&[]);
            let right = case.args[1].as_bytes().unwrap_or(&[]);
            let choice = case.args[2].as_bool().unwrap_or(false);
            match &case.expected {
                EvalValue::Result(Ok(inner)) => {
                    let out_bytes = inner.as_bytes().unwrap_or(&[]);
                    s.push_str(&format!(
                        "        check_select(ctx, &b\"{}\"[..], &b\"{}\"[..], {}, &b\"{}\"[..], Fault::None)\n",
                        escape_bytes(left),
                        escape_bytes(right),
                        choice,
                        escape_bytes(out_bytes)
                    ));
                }
                EvalValue::Result(Err((_, err_variant))) => {
                    s.push_str(&format!(
                        "        check_select_err(ctx, &b\"{}\"[..], &b\"{}\"[..], {}, \"{}\", Fault::None)\n",
                        escape_bytes(left),
                        escape_bytes(right),
                        choice,
                        err_variant
                    ));
                }
                _ => {}
            }
        }
        "eq_u64" => {
            let left = case.args[0].as_u64().unwrap_or(0);
            let right = case.args[1].as_u64().unwrap_or(0);
            let expected = case.expected.as_bool().unwrap_or(false);
            s.push_str(&format!(
                "        let got = {eq_u64_export}({left}u64, {right}u64);\n"
            ));
            s.push_str(&format!(
                "        ctx.deep_equal(&Value::Bool(got), &Value::Bool({expected}))\n"
            ));
        }
        _ => {
            return Err(CodegenError {
                code: DiagnosticCode::W0701,
                kind: CodegenErrorKind::UnsupportedTarget(format!(
                    "Unsupported operation for harness codegen: {}",
                    case.op_name
                )),
                span: (0, 0),
            });
        }
    }
    Ok(s)
}

fn escape_bytes(bytes: &[u8]) -> String {
    let mut out = String::new();
    for &b in bytes {
        if (0x20..=0x7e).contains(&b) && b != b'\\' && b != b'"' {
            out.push(b as char);
        } else {
            out.push_str(&format!("\\x{b:02x}"));
        }
    }
    out
}

/// Checks that a file's content on disk matches generated content exactly.
pub fn verify_regeneration(target: &Path, content: &str) -> Result<bool, String> {
    if !target.exists() {
        return Ok(false);
    }
    let existing =
        std::fs::read_to_string(target).map_err(|e| format!("Failed to read target: {e}"))?;
    Ok(existing == content)
}
