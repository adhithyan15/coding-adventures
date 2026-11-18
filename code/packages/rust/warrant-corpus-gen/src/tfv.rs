//! # Examples: the `examples` rows of `corpus.json` and `cases.tfv` (VER04r 1.4, 2.3, 2.5)
//!
//! An `example` is one subject call compared with a pure value:
//! `eq_fixed<16>(b"00..", b"00..") == true`.  The generator evaluates the pure arguments and
//! the pure side with its own evaluator (the same one that makes `expected_sha256`) and
//! writes the result twice:
//!
//! ```text
//!   corpus.json  {"id","op","args":[..],"expected":..,"result_type"[,"width"]}
//!                values untagged, decoded by declared type (2.3): integers are decimal
//!                strings, bytes lowercase hex, Result is {"ok":v} / {"err":"CASE"}
//!   cases.tfv    tfv 1
//!                <id> R<n> Kop S<hex op> K<param> <v> .. [Kwidth I<w>] Kexpect <v>
//!                T/F, I<decimal>, X<hex>, Result as `M1 S6f6b v` / `M1 S657272 S<hex case>`
//! ```
//!
//! Every value is checked against its declared type with the stream encoder first, so an
//! example whose value does not fit (a `Nat` past `2^64-1`) is `W0701`, exactly as for a
//! suite.  For ct-compare `cases.tfv` is byte-identical to the v1 file.

use crate::arith::check_v0_wraps;
use crate::error::{code, GenError};
use crate::eval::{outside_admit, Program, Stats};
use crate::ir::Ir;
use crate::json::Json;
use crate::lower::Lowered;
use coding_adventures_warrant_grammar::parser::Version;
use crate::stream::encode;
use crate::types::Ty;
use crate::value::Value;

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn json_value(ty: &Ty, v: &Value) -> Result<Json, GenError> {
    Ok(match (ty, v) {
        (_, Value::Bool(b)) => Json::Bool(*b),
        (_, Value::Int(i)) => Json::str(i.to_string()),
        (_, Value::Bytes(b)) => Json::str(hex(b)),
        (Ty::Result(ok, _), Value::Ok(inner)) => Json::obj([("ok", json_value(ok, inner)?)]),
        (_, Value::Err(case)) => Json::obj([("err", Json::str(&**case))]),
        _ => return Err(GenError::internal("an example value has the wrong shape")),
    })
}

fn tfv_value(ty: &Ty, v: &Value) -> Result<String, GenError> {
    Ok(match (ty, v) {
        (_, Value::Bool(true)) => "T".to_string(),
        (_, Value::Bool(false)) => "F".to_string(),
        (_, Value::Int(i)) => format!("I{i}"),
        (_, Value::Bytes(b)) => format!("X{}", hex(b)),
        (Ty::Result(ok, _), Value::Ok(inner)) => {
            format!("M1 S{} {}", hex(b"ok"), tfv_value(ok, inner)?)
        }
        (_, Value::Err(case)) => format!("M1 S{} S{}", hex(b"err"), hex(case.as_bytes())),
        _ => return Err(GenError::internal("an example value has the wrong shape")),
    })
}

/// The closed vocabulary of the reasons of an omitted example (VER04r 8.4, [D34]).
pub mod reason {
    /// The example is not `op(args) == pure`.
    pub const NOT_OP_EQ_PURE: &str = "not-op-eq-pure";
    /// A value the example needs is over the limit.
    pub const VALUE_OVER_LIMIT: &str = "value-over-limit";
}

/// The 4,096-byte limit of a `Bytes` value in `corpus.json` (VER04r 2.3).
pub const MAX_EXAMPLE_BYTES: usize = 4096;

/// The rows of `examples` and the lines of `cases.tfv` (without the `tfv 1` header).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Examples {
    /// The `examples` rows, in declaration order.
    pub rows: Vec<Json>,
    /// One TFV record line per example.
    pub lines: Vec<String>,
    /// The omitted examples as `(id, reason)`, sorted by id bytes (VER04r 1.4).
    pub omitted: Vec<(String, &'static str)>,
}

impl Examples {
    /// `cases.tfv`: `tfv 1`, one record per line, a final LF.
    pub fn cases_tfv(&self) -> String {
        let mut out = String::from("tfv 1\n");
        for l in &self.lines {
            out.push_str(l);
            out.push('\n');
        }
        out
    }
}

/// Is a `Bytes` inside `v` (itself, or under an `Ok`) longer than the 4,096-byte limit?
fn over_limit(ty: &Ty, v: &Value) -> bool {
    match (ty, v) {
        (_, Value::Bytes(b)) => b.len() > MAX_EXAMPLE_BYTES,
        (Ty::Result(ok, _), Value::Ok(inner)) => over_limit(ok, inner),
        _ => false,
    }
}

/// One example, evaluated: the call, its arguments as `(parameter, concrete type, value)`
/// and the pure side's value.
struct Evaluated<'l> {
    lop: &'l crate::lower::LoweredOp,
    widths: &'l [u64],
    args: Vec<(&'l str, Ty, Value)>,
    expected: Value,
}

impl Evaluated<'_> {
    /// Is any argument, or the pure value, over the 4,096-byte limit?
    fn any_over_limit(&self) -> bool {
        self.args.iter().any(|(_, t, v)| over_limit(t, v)) || over_limit(&self.lop.ret, &self.expected)
    }
}

/// Evaluate one example in the normative order of VER04r 8.4:
///
/// 1. the arguments, left to right, then the pure side; **the first error ends the run**
///    (so a size error in an earlier argument is the first error, whatever a later argument
///    would have done);
/// 2. the [D19] check: the arguments are inside the op's `admit`;
/// 3. the [D18] check: a v0 document's evaluation did not wrap.
///
/// Encoding against the declared types is **not** part of this: it is the emission of a row,
/// which an omitted example never reaches.
fn evaluate<'l>(
    lowered: &'l Lowered,
    program: &Program<'_>,
    checker: &Program<'_>,
    reference: &'l crate::lower::ExampleRef,
) -> Result<Evaluated<'l>, GenError> {
    let Ir::Subject { op, args, widths } = &reference.subject else {
        return Err(GenError::new(
            code::W0701,
            "",
            "an example is one operation call compared with a pure value",
        ));
    };
    let lop = lowered
        .op(op)
        .ok_or_else(|| GenError::internal("an example calls an undeclared op"))?;
    let statics: Vec<(String, u64)> = lop.statics.iter().cloned().zip(widths.iter().copied()).collect();
    let mut st = Stats::default();
    let mut vals = Vec::with_capacity(args.len());
    for a in args {
        vals.push(program.eval(a, &[], &mut st)?);
    }
    let expected = program.eval(&reference.expected, &[], &mut st)?;
    // [D19]: an example is a call, so its arguments must be inside the op's `admit`.
    if lop.admit.is_some() {
        let some: Vec<Option<Value>> = vals.iter().cloned().map(Some).collect();
        if !checker.admits_call(op, &some)? {
            return Err(outside_admit(op));
        }
    }
    // [D18]: v0 rule 2 applies to the evaluation of an example, its arguments and its
    // pure side, as it does to a claim: a wrap is a generation error in a v0 document.
    check_v0_wraps(lowered.version, st.wraps)?;
    Ok(Evaluated {
        lop,
        widths,
        args: lop
            .params
            .iter()
            .zip(vals)
            .map(|((n, t), v)| (n.as_str(), t.substitute(&statics), v))
            .collect(),
        expected,
    })
}

/// [`examples_omitting`] with no `omit_example` line.
///
/// # Errors
///
/// As [`examples_omitting`].
pub fn examples(lowered: &Lowered, program: &Program<'_>) -> Result<Examples, GenError> {
    examples_omitting(lowered, program, &[])
}

/// Evaluate every example of `lowered`, except the ones `omit` names: those are only
/// *classified* (VER04r 8.4, [D34]).
///
/// An omitted example is not written to `cases.tfv`, not stream-encoded and not
/// admit-checked beyond the decision.  Its reason is `not-op-eq-pure` if its shape is not
/// `op(args) == pure` (decided first, at lowering).  Otherwise it is evaluated as for
/// emission and the first outcome decides: a size error (a `Bytes` past `MAX_IR_VALUE`)
/// is `value-over-limit`; any other error refuses generation; a completed evaluation that
/// passed the D19 and D18 checks is `value-over-limit` only if an argument or the pure value
/// is over 4,096 bytes.  Nothing else qualifies, and an omitted example that qualifies for
/// nothing is a stale profile.
///
/// # Errors
///
/// `profile` for an `omit` id that names no example, or an example that is emittable (the
/// message names the example, never a value); `W0701` for a value outside its declared type
/// or an example whose subject is not an operation call; evaluator errors, attributed to the
/// example.
pub fn examples_omitting(
    lowered: &Lowered,
    program: &Program<'_>,
    omit: &[String],
) -> Result<Examples, GenError> {
    let mut out = Examples {
        rows: Vec::new(),
        lines: Vec::new(),
        omitted: Vec::new(),
    };
    let id_of = |name: &str| format!("warrant/{}/example/{name}", lowered.module);
    for want in omit {
        if !lowered.examples.iter().any(|x| id_of(&x.name) == *want) {
            return Err(GenError::new(
                code::PROFILE,
                "corpus.profile",
                "an omit_example line names no example of the spec",
            ));
        }
    }
    // The caller's program may not know the operations; this one does ([D19]).
    let checker = program
        .clone()
        .with_ops(&lowered.ops, lowered.version == Version::V0);
    for x in &lowered.examples {
        let at = |e: GenError| e.at(&x.name);
        let id = id_of(&x.name);
        let omitted = omit.contains(&id);
        let evaluated = match &x.reference {
            Some(r) => evaluate(lowered, program, &checker, r),
            None if omitted => {
                out.omitted.push((id, reason::NOT_OP_EQ_PURE));
                continue;
            }
            // lowering refuses a non-omitted example of another shape
            None => return Err(at(GenError::internal("an example has no reference"))),
        };
        if omitted {
            match evaluated {
                Err(e) if e.size => out.omitted.push((id, reason::VALUE_OVER_LIMIT)),
                Err(e) => return Err(at(e)),
                Ok(ev) if ev.any_over_limit() => out.omitted.push((id, reason::VALUE_OVER_LIMIT)),
                Ok(_) => {
                    return Err(GenError::new(
                        code::PROFILE,
                        "corpus.profile",
                        format!("an omit_example line names an example that is emittable: `{id}`"),
                    ))
                }
            }
            continue;
        }
        let ev = evaluated.map_err(at)?;
        let mut arg_json = Vec::new();
        let mut fields = vec![("op".to_string(), format!("S{}", hex(ev.lop.name.as_bytes())))];
        for (pname, concrete, v) in &ev.args {
            encode(concrete, v, &mut Vec::new()).map_err(at)?;
            arg_json.push(json_value(concrete, v).map_err(at)?);
            fields.push(((*pname).to_string(), tfv_value(concrete, v).map_err(at)?));
        }
        encode(&ev.lop.ret, &ev.expected, &mut Vec::new()).map_err(at)?;
        let mut row = vec![
            ("id".to_string(), Json::str(id.as_str())),
            ("op".to_string(), Json::str(ev.lop.name.as_str())),
            ("args".to_string(), Json::Arr(arg_json)),
            (
                "expected".to_string(),
                json_value(&ev.lop.ret, &ev.expected).map_err(at)?,
            ),
            ("result_type".to_string(), Json::str(ev.lop.ret.name())),
        ];
        if let Some(w) = ev.widths.first() {
            row.push(("width".to_string(), Json::Int(*w)));
            fields.push(("width".to_string(), format!("I{w}")));
        }
        fields.push((
            "expect".to_string(),
            tfv_value(&ev.lop.ret, &ev.expected).map_err(at)?,
        ));
        let mut line = format!("{id} R{}", fields.len());
        for (k, v) in &fields {
            line.push_str(&format!(" K{k} {v}"));
        }
        out.rows.push(Json::Obj(row));
        out.lines.push(line);
    }
    out.omitted.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    Ok(out)
}
