//! # Portable Warrant corpus compiler (VER04i WD-003)
//!
//! This module compiles the supported Warrant v0 `ct_compare` profile into a
//! small descriptor corpus. It deliberately keeps subject calls and pure
//! model calls as different IR tags: expected values are evaluated only by the
//! Warrant model evaluator, never by an operation binding.

use std::collections::{BTreeMap, HashMap};

use coding_adventures_sha256::sha256_hex;

use crate::codegen::{CodegenLimits, EvalValue, Evaluator, TfvRecord};
use crate::parser::{
    BinaryOp, Declaration, Expression, Literal, Parameter, TypeExpr, UnaryOp, Version, WarrantFile,
};

const EXHAUSTIVE_BUDGET: usize = 400_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusError(pub String);

impl std::fmt::Display for CorpusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CorpusError {}

fn err(message: impl Into<String>) -> CorpusError {
    CorpusError(message.into())
}

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn canonical_type(ty: &TypeExpr<'_>) -> Result<String, CorpusError> {
    match ty {
        TypeExpr::Named { name, .. } if matches!(*name, "Bool" | "U8" | "U64" | "Nat") => {
            Ok((*name).to_string())
        }
        TypeExpr::Named { name: "Bytes", .. } => Ok("Bytes".to_string()),
        TypeExpr::Bytes { width: None, .. } => Ok("Bytes".to_string()),
        TypeExpr::Bytes {
            width: Some(crate::parser::Width::Literal(n)),
            ..
        } => Ok(format!("Bytes<{n}>")),
        TypeExpr::Bytes {
            width: Some(crate::parser::Width::Named(name)),
            ..
        } => Ok(format!("Bytes<{name}>")),
        TypeExpr::Result {
            ok_type, err_name, ..
        } => Ok(format!("Result<{},{}>", canonical_type(ok_type)?, err_name)),
        _ => Err(err(
            "W0701: unsupported type in warrant-corpus-v1 v0 profile",
        )),
    }
}

fn bytes_values(len: usize, alphabet: &[u8]) -> Vec<Vec<u8>> {
    fn products(n: usize, alphabet: &[u8], prefix: &mut Vec<u8>, out: &mut Vec<Vec<u8>>) {
        if prefix.len() == n {
            out.push(prefix.clone());
            return;
        }
        for byte in alphabet {
            prefix.push(*byte);
            products(n, alphabet, prefix, out);
            prefix.pop();
        }
    }
    let mut values = Vec::new();
    products(len, alphabet, &mut Vec::new(), &mut values);
    values
}

fn bytes_d1() -> Vec<EvalValue> {
    let mut out = vec![EvalValue::Bytes(Vec::new())];
    out.extend((0u16..=255).map(|b| EvalValue::Bytes(vec![b as u8])));
    let alphabet = [0x00, 0x01, 0x80, 0xff];
    out.extend(bytes_values(2, &alphabet).into_iter().map(EvalValue::Bytes));
    out.extend(bytes_values(3, &alphabet).into_iter().map(EvalValue::Bytes));
    for n in 4..=8 {
        out.push(EvalValue::Bytes(vec![0; n]));
        out.push(EvalValue::Bytes(vec![0xa5; n]));
    }
    out
}

fn domain(ty: &TypeExpr<'_>) -> Result<(String, Vec<EvalValue>), CorpusError> {
    match ty {
        TypeExpr::Bytes { width: None, .. } | TypeExpr::Named { name: "Bytes", .. } => {
            Ok(("bytes_d1".into(), bytes_d1()))
        }
        TypeExpr::Bytes {
            width: Some(crate::parser::Width::Literal(n)),
            ..
        } => {
            if !matches!(*n, 0 | 1 | 2 | 4 | 16) {
                return Err(err(
                    "W0701: Bytes<N> width is outside the closed ct_compare v0 corpus profile",
                ));
            }
            let n = usize::try_from(*n).map_err(|_| err("W0707: width overflow"))?;
            let values = if n <= 3 {
                bytes_d1()
                    .into_iter()
                    .filter(|v| matches!(v, EvalValue::Bytes(b) if b.len() == n))
                    .collect()
            } else {
                let alphabet = [0x00, 0x01, 0x80, 0xff];
                let mut values: Vec<EvalValue> = alphabet
                    .iter()
                    .map(|b| EvalValue::Bytes(vec![*b; n]))
                    .collect();
                for pos in 0..n {
                    for byte in [0x01, 0x80, 0xff] {
                        let mut value = vec![0; n];
                        value[pos] = byte;
                        values.push(EvalValue::Bytes(value));
                    }
                }
                values
            };
            Ok((format!("bytes_n({n})"), values))
        }
        TypeExpr::Named { name: "Bool", .. } => Ok((
            "bool".into(),
            vec![EvalValue::Bool(false), EvalValue::Bool(true)],
        )),
        TypeExpr::Named { name: "U8", .. } => Ok((
            "u8_all".into(),
            (0u16..=255).map(|x| EvalValue::U8(x as u8)).collect(),
        )),
        TypeExpr::Named { name: "U64", .. } => {
            let mut values: Vec<EvalValue> = (0u64..=255).map(EvalValue::U64).collect();
            values.extend((8..=63).map(|i| EvalValue::U64(1u64 << i)));
            values.push(EvalValue::U64(u64::MAX));
            values.extend((0..=63).map(|i| EvalValue::U64(u64::MAX ^ (1u64 << i))));
            Ok(("u64_edge".into(), values))
        }
        TypeExpr::Named { name: "Nat", .. } => {
            Ok(("nat_0_64".into(), (0u64..=64).map(EvalValue::Nat).collect()))
        }
        _ => Err(err(
            "W0701: unsupported binder in warrant-corpus-v1 v0 profile",
        )),
    }
}

fn encode_stream_value(value: &EvalValue, out: &mut Vec<u8>) -> Result<(), CorpusError> {
    match value {
        EvalValue::Bool(false) => out.push(0x00),
        EvalValue::Bool(true) => out.push(0x01),
        EvalValue::U8(v) => {
            out.push(0x10);
            out.extend_from_slice(&u64::from(*v).to_le_bytes());
        }
        EvalValue::U64(v) | EvalValue::Nat(v) => {
            out.push(0x10);
            out.extend_from_slice(&v.to_le_bytes());
        }
        EvalValue::Bytes(v) => {
            out.push(0x20);
            out.extend_from_slice(&(v.len() as u32).to_le_bytes());
            out.extend_from_slice(v);
        }
        EvalValue::Result(Ok(value)) => {
            out.push(0x50);
            encode_stream_value(value, out)?;
        }
        EvalValue::Result(Err((_, variant))) => {
            out.push(0x51);
            put_string(out, variant)?;
        }
        _ => return Err(err("W0701: unsupported stream value in v0 profile")),
    }
    Ok(())
}

fn put_string(out: &mut Vec<u8>, value: &str) -> Result<(), CorpusError> {
    let len = u32::try_from(value.len()).map_err(|_| err("corpus string too long"))?;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

fn stream_context(
    id: &str,
    binders: &[(String, String)],
    result_type: &str,
) -> Result<Vec<u8>, CorpusError> {
    let mut out = b"warrant-stream-v1\0".to_vec();
    put_string(&mut out, id)?;
    let count = u32::try_from(binders.len()).map_err(|_| err("too many binders"))?;
    out.extend_from_slice(&count.to_le_bytes());
    for (_, ty) in binders {
        put_string(&mut out, ty)?;
    }
    put_string(&mut out, result_type)?;
    Ok(out)
}

fn typed_json(value: &EvalValue, ty: &str) -> Result<String, CorpusError> {
    match value {
        EvalValue::Bool(v) => Ok(v.to_string()),
        EvalValue::U8(v) => Ok(format!("{{\"type\":\"U8\",\"value\":\"{v}\"}}")),
        EvalValue::U64(v) => Ok(format!("{{\"type\":\"U64\",\"value\":\"{v}\"}}")),
        EvalValue::Nat(v) => Ok(format!("{{\"type\":\"Nat\",\"value\":\"{v}\"}}")),
        EvalValue::Bytes(v) => Ok(format!(
            "{{\"type\":{},\"value\":{}}}",
            json_string(ty),
            json_string(&hex(v))
        )),
        EvalValue::Result(Ok(v)) => Ok(format!(
            "{{\"type\":{},\"value\":{{\"ok\":{}}}}}",
            json_string(ty),
            typed_json(v, "Bytes")?
        )),
        EvalValue::Result(Err((_, variant))) => Ok(format!(
            "{{\"type\":{},\"value\":{{\"err\":{}}}}}",
            json_string(ty),
            json_string(variant)
        )),
        _ => Err(err("W0701: unsupported example value in v0 profile")),
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(DIGITS[(b >> 4) as usize] as char);
        out.push(DIGITS[(b & 15) as usize] as char);
    }
    out
}

fn expression_ir(
    expr: &Expression<'_>,
    ops: &[String],
    models: &[String],
    vars: &[String],
) -> Result<String, CorpusError> {
    let arr = |tag: &str, parts: Vec<String>| {
        let mut values = vec![json_string(tag)];
        values.extend(parts);
        format!("[{}]", values.join(","))
    };
    match expr {
        Expression::Grouped { inner, .. } => expression_ir(inner, ops, models, vars),
        Expression::Literal { lit, .. } => match lit {
            Literal::Bool(v) => Ok(arr("lit", vec![v.to_string()])),
            Literal::U8(v) => Ok(arr(
                "lit",
                vec![format!("{{\"type\":\"U8\",\"value\":\"{v}\"}}")],
            )),
            Literal::U64(v) => Ok(arr(
                "lit",
                vec![format!("{{\"type\":\"U64\",\"value\":\"{v}\"}}")],
            )),
            Literal::Bytes(v) => Ok(arr(
                "lit",
                vec![format!(
                    "{{\"type\":\"Bytes\",\"value\":{}}}",
                    json_string(v)
                )],
            )),
            _ => Err(err("W0701: unsupported literal in v0 corpus IR")),
        },
        Expression::Reference {
            name, widths, args, ..
        } if name.segments.len() == 1 => {
            let name = name.segments[0];
            if let Some(args) = args {
                let encoded_args = if name == "Err" {
                    let variant = args
                        .first()
                        .and_then(|arg| match arg {
                            Expression::Reference { name, .. } => name.segments.last().copied(),
                            _ => None,
                        })
                        .ok_or_else(|| err("W0701: malformed Err constructor in corpus IR"))?;
                    vec![json_string(variant)]
                } else {
                    args.iter()
                        .map(|arg| expression_ir(arg, ops, models, vars))
                        .collect::<Result<Vec<_>, _>>()?
                };
                let tag = if ops.iter().any(|op| op == name) {
                    "subject_op"
                } else if models.iter().any(|model| model == name) {
                    "model_call"
                } else {
                    match name {
                        "len" => "len",
                        "Ok" => "ok",
                        "Err" => "err",
                        _ => {
                            return Err(err(format!(
                                "W0701: unsupported reference `{name}` in corpus IR"
                            )))
                        }
                    }
                };
                if tag == "subject_op" {
                    let mut parts =
                        vec![json_string(name), format!("[{}]", encoded_args.join(","))];
                    if !widths.is_empty() {
                        let widths = widths
                            .iter()
                            .map(|w| match w {
                                crate::parser::Width::Literal(n) => Ok(n.to_string()),
                                crate::parser::Width::Named(_) => {
                                    Err(err("W0701: named static width in corpus IR"))
                                }
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        parts.push(format!("[{}]", widths.join(",")));
                    }
                    Ok(arr(tag, parts))
                } else if tag == "model_call" {
                    Ok(arr(
                        tag,
                        vec![json_string(name), format!("[{}]", encoded_args.join(","))],
                    ))
                } else {
                    Ok(arr(tag, encoded_args))
                }
            } else if vars.iter().any(|var| var == name) {
                Ok(arr("var", vec![json_string(name)]))
            } else {
                Err(err(format!(
                    "W0701: unsupported bare reference `{name}` in corpus IR"
                )))
            }
        }
        Expression::Binary {
            op, left, right, ..
        } => {
            let tag = match op {
                BinaryOp::Eq => "eq",
                BinaryOp::NotEq => "ne",
                BinaryOp::Lt => "lt",
                BinaryOp::Gt => "gt",
                BinaryOp::Le => "le",
                BinaryOp::Ge => "ge",
                BinaryOp::Add => "add",
                BinaryOp::Sub => "sub",
                BinaryOp::Mul => "mul",
                BinaryOp::And => "and",
                BinaryOp::Or => "or",
                BinaryOp::Implies => "implies",
            };
            Ok(arr(
                tag,
                vec![
                    expression_ir(left, ops, models, vars)?,
                    expression_ir(right, ops, models, vars)?,
                ],
            ))
        }
        Expression::Unary {
            op: UnaryOp::Not,
            operand,
            ..
        } => Ok(arr("not", vec![expression_ir(operand, ops, models, vars)?])),
        Expression::Unary {
            op: UnaryOp::Neg, ..
        } => Err(err(
            "W0701: numeric negation is unsupported in warrant-corpus-v1 v0 IR",
        )),
        Expression::Conditional {
            condition,
            then_branch,
            else_branch,
            ..
        } => Ok(arr(
            "if",
            vec![
                expression_ir(condition, ops, models, vars)?,
                expression_ir(then_branch, ops, models, vars)?,
                expression_ir(else_branch, ops, models, vars)?,
            ],
        )),
        _ => Err(err(
            "W0701: unsupported expression in warrant-corpus-v1 v0 profile",
        )),
    }
}

fn set_binder_env<'a>(
    params: &'a [Parameter<'a>],
    domains: &[Vec<EvalValue>],
    at: usize,
    env: &mut HashMap<&'a str, EvalValue>,
    visit: &mut impl FnMut(&HashMap<&'a str, EvalValue>) -> Result<(), CorpusError>,
) -> Result<(), CorpusError> {
    if at == params.len() {
        return visit(env);
    }
    for value in &domains[at] {
        env.insert(params[at].name, value.clone());
        set_binder_env(params, domains, at + 1, env, visit)?;
    }
    env.remove(params[at].name);
    Ok(())
}

fn is_subject(expr: &Expression<'_>, ops: &[String]) -> bool {
    match expr {
        Expression::Reference { name, .. } => name
            .segments
            .first()
            .is_some_and(|n| ops.iter().any(|op| op == n)),
        Expression::Binary { left, right, .. } => is_subject(left, ops) || is_subject(right, ops),
        Expression::Unary { operand, .. } | Expression::Grouped { inner: operand, .. } => {
            is_subject(operand, ops)
        }
        Expression::Conditional {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            is_subject(condition, ops)
                || is_subject(then_branch, ops)
                || is_subject(else_branch, ops)
        }
        _ => false,
    }
}

fn push_corpus_expr(
    e: &Expression<'_>,
    ops: &[String],
    models: &[String],
    vars: &[String],
) -> Result<String, CorpusError> {
    expression_ir(e, ops, models, vars)
}

pub fn generate_corpus(
    file: &WarrantFile<'_>,
    spec_bytes: &[u8],
) -> Result<(String, String), CorpusError> {
    if file.version != Version::V0 || file.name != "ct_compare" {
        return Err(err(
            "W0701: only warrant ct_compare v0 is supported by warrant-corpus-v1 WD-003",
        ));
    }

    let op_names: Vec<String> = file
        .declarations
        .iter()
        .filter_map(|d| match d {
            Declaration::Op(op) => Some(op.name.to_string()),
            _ => None,
        })
        .collect();
    let model_names: Vec<String> = file
        .declarations
        .iter()
        .filter_map(|d| match d {
            Declaration::Model(model) => Some(model.name.to_string()),
            _ => None,
        })
        .collect();

    let mut evaluator = Evaluator::new(file, CodegenLimits::default());
    let mut tfv_records = Vec::new();

    let mut ops_json = Vec::new();
    for op in file.declarations.iter().filter_map(|d| match d {
        Declaration::Op(v) => Some(v),
        _ => None,
    }) {
        let params = op
            .params
            .iter()
            .map(|p| {
                Ok(format!(
                    "[{},{}]",
                    json_string(p.name),
                    json_string(&canonical_type(&p.param_type)?)
                ))
            })
            .collect::<Result<Vec<_>, CorpusError>>()?;
        let width_params = op
            .static_params
            .iter()
            .map(|p| json_string(p.name))
            .collect::<Vec<_>>();
        let mut fields = vec![
            format!("\"params\":[{}]", params.join(",")),
            format!("\"ret\":{}", json_string(&canonical_type(&op.return_type)?)),
        ];
        if !width_params.is_empty() {
            fields.push(format!("\"width_params\":[{}]", width_params.join(",")));
        }
        if op.is_fresh {
            fields.push("\"fresh\":true".to_string());
        }
        ops_json.push(format!("{}:{{{}}}", json_string(op.name), fields.join(",")));
    }
    let mut models_json = Vec::new();
    for model in file.declarations.iter().filter_map(|d| match d {
        Declaration::Model(v) => Some(v),
        _ => None,
    }) {
        let body = model
            .body
            .as_ref()
            .ok_or_else(|| err("W0701: model body is absent"))?;
        let model_params = model
            .params
            .iter()
            .map(|p| p.name.to_string())
            .collect::<Vec<_>>();
        let body_ir = expression_ir(body, &op_names, &model_names, &model_params)?;
        if body_ir.contains("subject_op") {
            return Err(err("W0701: pure model body references subject operation"));
        }
        let params = model
            .params
            .iter()
            .map(|p| json_string(p.name))
            .collect::<Vec<_>>();
        models_json.push(format!(
            "{}:{{\"body\":{},\"params\":[{}]}}",
            json_string(model.name),
            body_ir,
            params.join(",")
        ));
    }

    let mut examples_json = Vec::new();
    for decl in &file.declarations {
        if let Declaration::Example(example) = decl {
            evaluator.reset_work();
            let case =
                crate::codegen::extract_example_case(example.name, &example.body, &mut evaluator)
                    .map_err(|e| err(format!("{}", e)))?;
            let op = file
                .declarations
                .iter()
                .find_map(|d| match d {
                    Declaration::Op(op) if op.name == case.op_name => Some(op),
                    _ => None,
                })
                .ok_or_else(|| err("W0701: example operation is not declared"))?;
            let args = case
                .args
                .iter()
                .zip(&op.params)
                .map(|(value, param)| typed_json(value, &canonical_type(&param.param_type)?))
                .collect::<Result<Vec<_>, CorpusError>>()?;
            let expected_type = canonical_type(&op.return_type)?;
            let mut fields = vec![(
                "op".to_string(),
                format!("S{}", hex(case.op_name.as_bytes())),
            )];
            for (value, param) in case.args.iter().zip(&op.params) {
                fields.push((
                    param.name.to_string(),
                    value.encode_checked().map_err(|e| err(format!("{}", e)))?,
                ));
            }
            if let Some(width) = case.static_width {
                fields.push(("width".to_string(), format!("I{width}")));
            }
            fields.push((
                "expect".to_string(),
                case.expected
                    .encode_checked()
                    .map_err(|e| err(format!("{}", e)))?,
            ));
            tfv_records.push(TfvRecord {
                id: format!("warrant/ct_compare/example/{}", case.id),
                fields,
            });
            examples_json.push(format!(
                "{{\"args\":[{}],\"expected\":{},\"id\":{},\"op\":{},\"result_type\":{},\"width\":{}}}",
                args.join(","), typed_json(&case.expected, &expected_type)?,
                json_string(&format!("warrant/ct_compare/example/{}", case.id)),
                json_string(&case.op_name), json_string(&expected_type), case.static_width.map(|v| v.to_string()).unwrap_or_else(|| "null".into())
            ));
        }
    }

    let mut suites_json = Vec::new();
    let mut controls_json = Vec::new();
    let mut control_claims: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for decl in &file.declarations {
        let Declaration::Claim(claim) = decl else {
            continue;
        };
        let Expression::Quantified {
            is_forall: true,
            binders,
            body,
            ..
        } = &claim.body
        else {
            return Err(err(format!(
                "W0701: claim `{}` must use forall",
                claim.name
            )));
        };
        let Expression::Binary {
            op: BinaryOp::Eq,
            left,
            right,
            ..
        } = body.as_ref()
        else {
            return Err(err(format!(
                "W0701: claim `{}` is not an equality",
                claim.name
            )));
        };
        let (subject, reference) = match (is_subject(left, &op_names), is_subject(right, &op_names))
        {
            (true, false) => (left.as_ref(), right.as_ref()),
            (false, true) => (right.as_ref(), left.as_ref()),
            _ => {
                return Err(err(format!(
                    "W0701: claim `{}` needs exactly one subject side and one pure model side",
                    claim.name
                )))
            }
        };
        let op_name = match subject {
            Expression::Reference { name, .. } => name
                .segments
                .first()
                .ok_or_else(|| err("W0701: empty operation name"))?
                .to_string(),
            _ => return Err(err("W0701: subject side must be one operation call")),
        };
        let op = file
            .declarations
            .iter()
            .find_map(|d| match d {
                Declaration::Op(op) if op.name == op_name => Some(op),
                _ => None,
            })
            .ok_or_else(|| err("W0701: subject operation is absent"))?;
        if binders.len() > 3 {
            return Err(err("W0701: too many binders in v0 profile"));
        }
        let mut binder_domains = Vec::new();
        let mut binder_rows = Vec::new();
        let mut product = 1usize;
        for binder in binders {
            let (descriptor, values) = domain(&binder.param_type)?;
            product = product
                .checked_mul(values.len())
                .ok_or_else(|| err("W0706: domain product overflow"))?;
            if product > EXHAUSTIVE_BUDGET {
                return Err(err("W0706: exhaustive suite exceeds corpus budget"));
            }
            binder_rows.push(format!(
                "[{},{{\"domain\":{},\"type\":{}}}]",
                json_string(binder.name),
                json_string(&descriptor),
                json_string(&canonical_type(&binder.param_type)?)
            ));
            binder_domains.push(values);
        }
        let binders_json = binder_rows.join(",");
        let binder_names = binders
            .iter()
            .map(|p| p.name.to_string())
            .collect::<Vec<_>>();
        let lhs_ir = push_corpus_expr(subject, &op_names, &model_names, &binder_names)?;
        let rhs_ir = push_corpus_expr(reference, &op_names, &model_names, &binder_names)?;
        if rhs_ir.contains("subject_op") {
            return Err(err("W0701: pure reference contains subject operation"));
        }
        let mut admissions = binders
            .iter()
            .filter_map(|p| p.refinement.as_ref())
            .map(|e| push_corpus_expr(e, &op_names, &model_names, &binder_names))
            .collect::<Result<Vec<_>, _>>()?;
        if admissions.is_empty() {
            admissions.push("[\"lit\",true]".into());
        }
        let admit_ir = if admissions.len() == 1 {
            admissions.remove(0)
        } else {
            format!("[\"and\",{}]", admissions.join(","))
        };
        let result_type = canonical_type(&op.return_type)?;
        let id = format!("warrant/ct_compare/exhaustive/{}", claim.name);
        let bind_ctx: Vec<(String, String)> = binders
            .iter()
            .map(|p| Ok((p.name.into(), canonical_type(&p.param_type)?)))
            .collect::<Result<_, CorpusError>>()?;
        let mut input_stream = stream_context(&id, &bind_ctx, &result_type)?;
        let mut expected_stream = input_stream.clone();
        let mut evaluator = Evaluator::new(file, CodegenLimits::default());
        let mut admitted = 0usize;
        let mut discarded = 0usize;
        set_binder_env(
            binders,
            &binder_domains,
            0,
            &mut HashMap::new(),
            &mut |env| {
                evaluator.reset_work();
                for binder in binders {
                    if let Some(refinement) = &binder.refinement {
                        let pass = evaluator
                            .eval(refinement, env)
                            .map_err(|e| err(format!("{}", e)))?;
                        if pass.as_bool() != Some(true) {
                            discarded += 1;
                            return Ok(());
                        }
                    }
                }
                admitted += 1;
                for binder in binders {
                    let value = env
                        .get(binder.name)
                        .ok_or_else(|| err("missing binder value"))?;
                    encode_stream_value(value, &mut input_stream)?;
                }
                let expected = evaluator
                    .eval(reference, env)
                    .map_err(|e| err(format!("{}", e)))?;
                encode_stream_value(&expected, &mut expected_stream)?;
                Ok(())
            },
        )?;
        if admitted == 0 {
            return Err(err("W0706: empty admitted domain"));
        }
        let inputs_sha = sha256_hex(&input_stream);
        let expected_sha = sha256_hex(&expected_stream);
        let domain_text = binders
            .iter()
            .zip(&binder_domains)
            .map(|(p, values)| {
                Ok(format!(
                    "{}[{}]",
                    canonical_type(&p.param_type)?,
                    values.len()
                ))
            })
            .collect::<Result<Vec<_>, CorpusError>>()?
            .join(" x ");
        suites_json.push(format!(
            "{{\"admit\":{},\"binders\":[{}],\"cases\":{},\"claim\":{},\"discarded\":{},\"domain_text\":{},\"expected_calls\":{},\"expected_sha256\":{},\"id\":{},\"inputs_sha256\":{},\"kind\":\"exhaustive\",\"lhs\":{},\"ops\":[{}],\"result_type\":{},\"rhs\":{}}}",
            admit_ir, binders_json, admitted, json_string(claim.name), discarded, json_string(&domain_text),
            admitted, json_string(&expected_sha), json_string(&id), json_string(&inputs_sha), lhs_ir, json_string(&op_name), json_string(&result_type), rhs_ir
        ));
        let faults: &[(&str, &str)] = match op_name.as_str() {
            "eq" | "eq_fixed" => &[("ignore-last-byte", "input"), ("invert", "output")],
            "select_bytes" => &[("missing-error", "output"), ("swap-choice", "input")],
            "eq_u64" => &[("ignore-high-bit", "input"), ("invert", "output")],
            _ => return Err(err("W0701: no fault registry for operation")),
        };
        for (fault, applies) in faults {
            control_claims
                .entry((op_name.clone(), (*fault).to_string()))
                .or_default()
                .push(id.clone());
            // Fault registry rows are collected per operation, below.
            let _ = applies;
        }
    }

    for op_name in &op_names {
        if !control_claims
            .keys()
            .any(|(claim_op, _)| claim_op == op_name)
        {
            return Err(err(format!(
                "W0701: faulted operation `{op_name}` has no claim target"
            )));
        }
    }
    for ((op, fault), claims) in control_claims {
        let control_id = format!("warrant/ct_compare/negctl/exhaustive/{op}/{fault}");
        let claim_ids = claims
            .iter()
            .map(|id| json_string(id))
            .collect::<Vec<_>>()
            .join(",");
        controls_json.push(format!(
            "{{\"fault\":{},\"id\":{},\"op\":{},\"required_claims\":[{}],\"target_kind\":\"exhaustive\"}}",
            json_string(&fault), json_string(&control_id), json_string(&op), claim_ids
        ));
    }

    let spec_sha = sha256_hex(spec_bytes);
    let examples = examples_json.join(",");
    let suites = suites_json.join(",");
    let controls = controls_json.join(",");
    let mut faults = Vec::new();
    for op_name in &op_names {
        let rows: &[(&str, &str)] = match op_name.as_str() {
            "eq" | "eq_fixed" => &[("ignore-last-byte", "input"), ("invert", "output")],
            "select_bytes" => &[("missing-error", "output"), ("swap-choice", "input")],
            "eq_u64" => &[("ignore-high-bit", "input"), ("invert", "output")],
            _ => return Err(err("W0701: no fault registry for operation")),
        };
        for (fault, applies) in rows {
            faults.push(format!(
                "{{\"applies\":{},\"fault\":{},\"op\":{}}}",
                json_string(applies),
                json_string(fault),
                json_string(op_name)
            ));
        }
    }
    let mut tfv = vec!["tfv 1".to_string()];
    tfv.extend(tfv_records.iter().map(TfvRecord::to_tfv_line));
    let cases_tfv = format!("{}\n", tfv.join("\n"));
    let mut types = Vec::new();
    for declaration in &file.declarations {
        if let Declaration::Error(error) = declaration {
            let variants = error
                .variants
                .iter()
                .map(|v| json_string(v))
                .collect::<Vec<_>>()
                .join(",");
            types.push(format!("{}:[{}]", json_string(error.name), variants));
        }
    }
    let corpus = format!(
        "{{\"controls\":[{}],\"examples\":[{}],\"faults\":[{}],\"generator\":{{\"name\":\"warrant-grammar\",\"version\":\"0.4.0\"}},\"models\":{{{}}},\"module\":\"ct_compare\",\"ops\":{{{}}},\"profile\":\"warrant-corpus-v1\",\"schema_version\":1,\"spec\":\"code/specs/ct-compare.warrant\",\"spec_sha256\":{},\"suites\":[{}],\"types\":{{{}}}}}\n",
        controls, examples, faults.join(","), models_json.join(","), ops_json.join(","), json_string(&spec_sha), suites, types.join(",")
    );
    Ok((corpus, cases_tfv))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_framing_golden_vectors_pin_context_and_case_order() {
        let binders = vec![("x".to_string(), "Bool".to_string())];
        let prefix = stream_context("warrant/x", &binders, "Bool").unwrap();
        assert_eq!(
            hex(&prefix),
            "77617272616e742d73747265616d2d7631000900000077617272616e742f780100000004000000426f6f6c04000000426f6f6c"
        );
        assert_eq!(
            sha256_hex(&prefix),
            "c3edd4858a709d95b479823f02b56445e996bed4652838ddf697ff20921a0d35"
        );
        let mut one = prefix.clone();
        encode_stream_value(&EvalValue::Bool(false), &mut one).unwrap();
        assert_eq!(
            sha256_hex(&one),
            "efc37a58bf04b82ca409416f5b4482500d94b5cc0c253d452c615ac781a424bb"
        );
        let mut multiple = one;
        encode_stream_value(&EvalValue::Bool(true), &mut multiple).unwrap();
        assert_eq!(
            sha256_hex(&multiple),
            "33196c746aa4c043a7782c18afc11a6b875a78327be77436be22aa5fd2b0ddda"
        );
        assert_ne!(
            prefix,
            stream_context("warrant/y", &binders, "Bool").unwrap()
        );
        assert_ne!(
            prefix,
            stream_context("warrant/x", &[("x".into(), "Bytes".into())], "Bool").unwrap()
        );
        assert_ne!(
            prefix,
            stream_context(
                "warrant/x",
                &[("x".into(), "Bool".into()), ("y".into(), "Bool".into())],
                "Bool"
            )
            .unwrap()
        );
    }

    #[test]
    fn finite_domains_have_the_documented_cardinalities_and_order() {
        assert_eq!(bytes_d1().len(), 347);
        assert_eq!(
            domain(&TypeExpr::Named {
                name: "U64",
                span: (0, 0)
            })
            .unwrap()
            .1
            .len(),
            377
        );
        let fixed_one = TypeExpr::Bytes {
            width: Some(crate::parser::Width::Literal(1)),
            span: (0, 0),
        };
        assert_eq!(domain(&fixed_one).unwrap().1.len(), 256);
    }

    #[test]
    fn unary_minus_and_unprofiled_fixed_width_fail_closed() {
        let negative = Expression::Unary {
            op: UnaryOp::Neg,
            operand: Box::new(Expression::Literal {
                lit: Literal::Int(1),
                span: (0, 0),
            }),
            span: (0, 0),
        };
        assert!(expression_ir(&negative, &[], &[], &[])
            .unwrap_err()
            .0
            .contains("numeric negation"));
        let wide = TypeExpr::Bytes {
            width: Some(crate::parser::Width::Literal(4096)),
            span: (0, 0),
        };
        assert!(domain(&wide)
            .unwrap_err()
            .0
            .contains("outside the closed ct_compare"));
    }
}
