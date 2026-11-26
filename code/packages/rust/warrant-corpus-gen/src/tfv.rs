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
//!                strings, bytes lowercase hex, Result is {"ok":v} / {"err":"CASE"},
//!                a tuple is a JSON array of its components
//!   cases.tfv    tfv 1
//!                <id> R<n> Kop S<hex op> K<param> <v> .. [Kwidth I<w>] Kexpect <v>
//!                T/F, I<decimal>, X<hex>, Result as `M1 S6f6b v` / `M1 S657272 S<hex case>`,
//!                a tuple as `L<n> v1 .. vn` (nested the same way)
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
use crate::stream::encode_in;
use crate::types::{Decls, Ty};
use crate::value::Value;

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// The containers TFV nests at most this deep inside one field value, the outermost being depth 1
/// (TF01 2.2, VER04j 4.9, R99).  The line's own `R<n>` is the record header and does not count.
pub const MAX_TFV_DEPTH: usize = 64;

/// One container deeper: `depth` counts the `L` and `M` open around the value being written.  A variant
/// is two containers (`L2` holding the name and an `L<k>` of the fields), a `Result` one (`M1`, for `Ok`
/// and for `Err`), an `Option` one (`L0` as well as `L1`).  Past 64 is a `limit` (R99), before anything
/// is written for the value.
fn enter(depth: usize) -> Result<usize, GenError> {
    if depth >= MAX_TFV_DEPTH {
        return Err(GenError::new(
            code::LIMIT,
            "",
            "a TFV value nests deeper than 64 containers (TF01 2.2, VER04j 4.9)",
        ));
    }
    Ok(depth + 1)
}

fn wrong_shape() -> GenError {
    GenError::internal("an example value has the wrong shape")
}

/// The JSON of a value of type `ty` as `corpus.json` writes it (VER04r 2.3): integers are decimal strings,
/// bytes lowercase hex, a `Result` `{"ok":v}` / `{"err":"CASE"}`, a tuple or a record an array, an `Option`
/// `{"none":true}` / `{"some":v}`, a variant `{"ctor":C,"fields":[..]}`.
///
/// # Errors
///
/// `W0701` for a value that does not have its declared shape.
pub fn value_json(decls: &Decls, ty: &Ty, v: &Value) -> Result<Json, GenError> {
    json_value(decls, ty, v)
}

/// The TFV of a value of type `ty` as `cases.tfv` writes it (VER04r 2.5), counting containers from depth 0
/// (R99): a tuple or record `L<n> ..`, an `Option` `L0` / `L1 x`, a `Result` `M1 ..`, a variant
/// `L2 S<hex ctor> L<k> ..`.
///
/// # Errors
///
/// `limit` for a value that would open a container at depth 65 (R99), `W0701` for a wrong shape.
pub fn value_tfv(decls: &Decls, ty: &Ty, v: &Value) -> Result<String, GenError> {
    tfv_value(decls, ty, v, 0)
}

fn json_value(decls: &Decls, ty: &Ty, v: &Value) -> Result<Json, GenError> {
    Ok(match (ty, v) {
        (_, Value::Bool(b)) => Json::Bool(*b),
        (_, Value::Int(i)) => Json::str(i.to_string()),
        (_, Value::Bytes(b)) => Json::str(hex(b)),
        (Ty::Result(ok, _), Value::Ok(inner)) => Json::obj([("ok", json_value(decls, ok, inner)?)]),
        (_, Value::Err(case)) => Json::obj([("err", Json::str(&**case))]),
        (Ty::Tuple(parts), Value::Tuple(items)) if parts.len() == items.len() => Json::arr(
            parts
                .iter()
                .zip(items)
                .map(|(t, v)| json_value(decls, t, v))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        // `ir.data`: `{"none":true}` / `{"some":v}`; a record is the array of its fields; a variant is
        // `{"ctor":C,"fields":[..]}` (VER04r 2.3)
        (Ty::Option(_), Value::Opt(None)) => Json::obj([("none", Json::Bool(true))]),
        (Ty::Option(t), Value::Opt(Some(x))) => Json::obj([("some", json_value(decls, t, x)?)]),
        (Ty::Named(name), Value::Con(ctor, items)) => {
            let fields = decls.fields(name, ctor).filter(|f| f.len() == items.len()).ok_or_else(wrong_shape)?;
            let parts = fields
                .iter()
                .zip(items)
                .map(|(t, v)| json_value(decls, t, v))
                .collect::<Result<Vec<_>, _>>()?;
            if decls.is_record(name) {
                Json::arr(parts)
            } else {
                Json::obj([("ctor", Json::str(&**ctor)), ("fields", Json::arr(parts))])
            }
        }
        _ => return Err(wrong_shape()),
    })
}

fn tfv_value(decls: &Decls, ty: &Ty, v: &Value, depth: usize) -> Result<String, GenError> {
    Ok(match (ty, v) {
        (_, Value::Bool(true)) => "T".to_string(),
        (_, Value::Bool(false)) => "F".to_string(),
        (_, Value::Int(i)) => format!("I{i}"),
        (_, Value::Bytes(b)) => format!("X{}", hex(b)),
        (Ty::Result(ok, _), Value::Ok(inner)) => {
            let d = enter(depth)?;
            format!("M1 S{} {}", hex(b"ok"), tfv_value(decls, ok, inner, d)?)
        }
        (_, Value::Err(case)) => {
            enter(depth)?;
            format!("M1 S{} S{}", hex(b"err"), hex(case.as_bytes()))
        }
        (Ty::Tuple(parts), Value::Tuple(items)) if parts.len() == items.len() => {
            let d = enter(depth)?;
            let mut out = format!("L{}", items.len());
            for (t, v) in parts.iter().zip(items) {
                out.push(' ');
                out.push_str(&tfv_value(decls, t, v, d)?);
            }
            out
        }
        // `ir.data` (VER04j 4.9, VER04r 2.5): an `Option` is `L0` / `L1 x`; a record is `L<n> f1 .. fn`; a
        // variant is `L2 S<hex ctor> L<k> f1 .. fk`, by constructor *name*.  The declared type decides what
        // an `L1` is, never the text.
        (Ty::Option(_), Value::Opt(None)) => {
            enter(depth)?;
            "L0".to_string()
        }
        (Ty::Option(t), Value::Opt(Some(x))) => {
            let d = enter(depth)?;
            format!("L1 {}", tfv_value(decls, t, x, d)?)
        }
        (Ty::Named(name), Value::Con(ctor, items)) => {
            let fields = decls.fields(name, ctor).filter(|f| f.len() == items.len()).ok_or_else(wrong_shape)?;
            if decls.is_record(name) {
                let d = enter(depth)?;
                let mut out = format!("L{}", items.len());
                for (t, v) in fields.iter().zip(items) {
                    out.push(' ');
                    out.push_str(&tfv_value(decls, t, v, d)?);
                }
                out
            } else {
                let d = enter(enter(depth)?)?;
                let mut out = format!("L2 S{} L{}", hex(ctor.as_bytes()), items.len());
                for (t, v) in fields.iter().zip(items) {
                    out.push(' ');
                    out.push_str(&tfv_value(decls, t, v, d)?);
                }
                out
            }
        }
        _ => return Err(wrong_shape()),
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

/// Is a `Bytes` inside `v` (itself, or under an `Ok`, an `Option`, a tuple, a record or a variant) longer than the 4,096-byte limit?
fn over_limit(decls: &Decls, ty: &Ty, v: &Value) -> bool {
    match (ty, v) {
        (_, Value::Bytes(b)) => b.len() > MAX_EXAMPLE_BYTES,
        (Ty::Result(ok, _), Value::Ok(inner)) | (Ty::Option(ok), Value::Opt(Some(inner))) => {
            over_limit(decls, ok, inner)
        }
        (Ty::Tuple(parts), Value::Tuple(items)) => {
            parts.iter().zip(items).any(|(t, v)| over_limit(decls, t, v))
        }
        (Ty::Named(name), Value::Con(ctor, items)) => decls
            .fields(name, ctor)
            .is_some_and(|f| f.iter().zip(items).any(|(t, v)| over_limit(decls, t, v))),
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
    decls: &'l Decls,
}

impl Evaluated<'_> {
    /// Is any argument, or the pure value, over the 4,096-byte limit?
    fn any_over_limit(&self) -> bool {
        self.args.iter().any(|(_, t, v)| over_limit(self.decls, t, v))
            || over_limit(self.decls, &self.lop.ret, &self.expected)
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
    program: &'l Program<'_>,
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
        decls: program.decls(),
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
        // VER04r 2.3, 14: the 4,096-byte cap is enforced at emission, after the D18/D19 checks
        // `evaluate` has made.  The omit decision above is unchanged.
        if ev.any_over_limit() {
            return Err(at(GenError::new(
                code::LIMIT,
                "",
                "an example argument or pure value is a Bytes over 4,096 bytes (VER04r 2.3); \
                 omit it with an `omit_example` line",
            )));
        }
        let mut arg_json = Vec::new();
        let mut fields = vec![("op".to_string(), format!("S{}", hex(ev.lop.name.as_bytes())))];
        for (pname, concrete, v) in &ev.args {
            encode_in(ev.decls, concrete, v, &mut Vec::new()).map_err(at)?;
            arg_json.push(json_value(ev.decls, concrete, v).map_err(at)?);
            // a field value starts at depth 0: the line's `R<n>` does not count (R99)
            fields.push(((*pname).to_string(), tfv_value(ev.decls, concrete, v, 0).map_err(at)?));
        }
        encode_in(ev.decls, &ev.lop.ret, &ev.expected, &mut Vec::new()).map_err(at)?;
        let mut row = vec![
            ("id".to_string(), Json::str(id.as_str())),
            ("op".to_string(), Json::str(ev.lop.name.as_str())),
            ("args".to_string(), Json::Arr(arg_json)),
            (
                "expected".to_string(),
                json_value(ev.decls, &ev.lop.ret, &ev.expected).map_err(at)?,
            ),
            ("result_type".to_string(), Json::str(ev.lop.ret.name())),
        ];
        if let Some(w) = ev.widths.first() {
            row.push(("width".to_string(), Json::Int(*w)));
            fields.push(("width".to_string(), format!("I{w}")));
        }
        fields.push((
            "expect".to_string(),
            tfv_value(ev.decls, &ev.lop.ret, &ev.expected, 0).map_err(at)?,
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
