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

/// The rows of `examples` and the lines of `cases.tfv` (without the `tfv 1` header).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Examples {
    /// The `examples` rows, in declaration order.
    pub rows: Vec<Json>,
    /// One TFV record line per example.
    pub lines: Vec<String>,
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

/// Evaluate every example of `lowered`.
///
/// # Errors
///
/// `W0701` for a value outside its declared type or an example whose subject is not an
/// operation call; evaluator errors, attributed to the example.
pub fn examples(lowered: &Lowered, program: &Program<'_>) -> Result<Examples, GenError> {
    let mut out = Examples {
        rows: Vec::new(),
        lines: Vec::new(),
    };
    // The caller's program may not know the operations; this one does ([D19]).
    let checker = program
        .clone()
        .with_ops(&lowered.ops, lowered.version == Version::V0);
    for x in &lowered.examples {
        let at = |e: GenError| e.at(&x.name);
        let Ir::Subject { op, args, widths } = &x.reference.subject else {
            return Err(at(GenError::new(
                code::W0701,
                "",
                "an example is one operation call compared with a pure value",
            )));
        };
        let lop = lowered
            .op(op)
            .ok_or_else(|| at(GenError::internal("an example calls an undeclared op")))?;
        let mut st = Stats::default();
        let id = format!("warrant/{}/example/{}", lowered.module, x.name);
        let mut arg_json = Vec::new();
        let mut fields = vec![("op".to_string(), format!("S{}", hex(op.as_bytes())))];
        for (a, (pname, pty)) in args.iter().zip(&lop.params) {
            let v = program.eval(a, &[], &mut st).map_err(at)?;
            let concrete = pty.substitute(&lop.statics.iter().cloned().zip(widths.iter().copied()).collect::<Vec<_>>());
            encode(&concrete, &v, &mut Vec::new()).map_err(at)?;
            arg_json.push(json_value(&concrete, &v).map_err(at)?);
            fields.push((pname.clone(), tfv_value(&concrete, &v).map_err(at)?));
        }
        // [D19]: an example is a call, so its arguments must be inside the op's `admit`.
        if lop.admit.is_some() {
            let mut vals = Vec::new();
            for a in args {
                vals.push(Some(program.eval(a, &[], &mut Stats::default()).map_err(at)?));
            }
            if !checker.admits_call(op, &vals).map_err(at)? {
                return Err(at(outside_admit(op)));
            }
        }
        let expected = program.eval(&x.reference.expected, &[], &mut st).map_err(at)?;
        // [D18]: v0 rule 2 applies to the evaluation of an example, its arguments and its
        // pure side, as it does to a claim: a wrap is a generation error in a v0 document.
        check_v0_wraps(lowered.version, st.wraps).map_err(at)?;
        encode(&lop.ret, &expected, &mut Vec::new()).map_err(at)?;
        let mut row = vec![
            ("id".to_string(), Json::str(id.as_str())),
            ("op".to_string(), Json::str(op.as_str())),
            ("args".to_string(), Json::Arr(arg_json)),
            (
                "expected".to_string(),
                json_value(&lop.ret, &expected).map_err(at)?,
            ),
            ("result_type".to_string(), Json::str(lop.ret.name())),
        ];
        if let Some(w) = widths.first() {
            row.push(("width".to_string(), Json::Int(*w)));
            fields.push(("width".to_string(), format!("I{w}")));
        }
        fields.push(("expect".to_string(), tfv_value(&lop.ret, &expected).map_err(at)?));
        let mut line = format!("{id} R{}", fields.len());
        for (k, v) in &fields {
            line.push_str(&format!(" K{k} {v}"));
        }
        out.rows.push(Json::Obj(row));
        out.lines.push(line);
    }
    Ok(out)
}
