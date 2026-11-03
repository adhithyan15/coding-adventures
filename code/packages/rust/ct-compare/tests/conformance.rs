//! CT01 through the TF01 bootstrap harness (VER26 step 4).
//!
//! The transport is generated from the independent JSON oracle. Every record
//! calls the real public API. `expected_work` is checked as logical public
//! shape, not advertised as runtime instrumentation or a timing proof. The
//! separate optimized-IR test retains compiler evidence for equality scans.

use coding_adventures_ct_compare::{ct_eq, ct_eq_fixed, ct_eq_u64, ct_select_bytes};
use coding_adventures_test_bootstrap::{
    run::{self, Ctx},
    tfv::{self, Record, Value},
    Suite, Verdict,
};
use std::panic::catch_unwind;

const CORPUS: &str = include_str!("../../../../specs/fixtures/ct-compare-v1/cases.tfv");
include!("generated/fixed_widths.rs");

/// Fault injection belongs only to this test binary. The negative checks below
/// prove the adapter notices three kinds of broken public behavior.
#[derive(Clone, Copy)]
enum Fault {
    None,
    Equality,
    Selection,
    MissingError,
}

fn main() {
    let records = tfv::parse_file(CORPUS).expect("embedded CT01 transport must parse");
    assert!(
        !records.is_empty(),
        "an empty corpus cannot validate a package"
    );
    let mut suite = Suite::new();
    for record in &records {
        let record = record.clone();
        suite.add(format!("ct01/{}", record.id), move |ctx| {
            verdict(ctx, &record, Fault::None)
        });
    }
    for (id, case_id, fault) in [
        ("wrong-equality", "eq-one-equal", Fault::Equality),
        ("wrong-selection", "select-left-fresh", Fault::Selection),
        (
            "missing-error",
            "select-public-length-mismatch",
            Fault::MissingError,
        ),
    ] {
        let record = records
            .iter()
            .find(|r| r.id == case_id)
            .expect("negative-check case required")
            .clone();
        suite.add(format!("adapter/{id}"), move |ctx| {
            reject_after_baseline(ctx, &record, &record, fault)
        });
    }
    for (id, case_id, field, value) in [
        ("wrong-work", "eq-one-equal", "byte_reads", 0),
        ("unknown-fixed-width", "fixed-16-equal", "width", 1),
    ] {
        let record = records
            .iter()
            .find(|r| r.id == case_id)
            .expect("negative-check case required")
            .clone();
        let mut changed = record.clone();
        let (_, old) = changed
            .fields
            .iter_mut()
            .find(|(key, _)| key == field)
            .expect("field required");
        *old = Value::from_u64(value);
        suite.add(format!("adapter/{id}"), move |ctx| {
            reject_after_baseline(ctx, &record, &changed, Fault::None)
        });
    }
    std::process::exit(run::main(&suite));
}

fn reject_after_baseline(ctx: &Ctx, baseline: &Record, changed: &Record, fault: Fault) -> Verdict {
    // A checker that rejects everything must not pass its negative checks.
    if !matches!(verdict(ctx, baseline, Fault::None), Verdict::Pass) {
        return Verdict::fail("corpus-mismatch", "valid adapter baseline did not pass");
    }
    match verdict(ctx, changed, fault) {
        Verdict::Fail { .. } => Verdict::Pass,
        Verdict::Pass => Verdict::fail("corpus-mismatch", "broken adapter behavior was accepted"),
    }
}

fn verdict(ctx: &Ctx, record: &Record, fault: Fault) -> Verdict {
    check(ctx, record, fault).unwrap_or_else(|problem| Verdict::fail("corpus-malformed", problem))
}

fn check(ctx: &Ctx, record: &Record, fault: Fault) -> Result<Verdict, String> {
    let op = field(record, "op")?.as_text().ok_or("op must be text")?;
    if op == "eq_u64" {
        work(record, 0, 0, 64)?;
        let mut got = ct_eq_u64(integer(record, "left_u64")?, integer(record, "right_u64")?);
        if matches!(fault, Fault::Equality) {
            got = !got;
        }
        return Ok(ctx.deep_equal(&Value::Bool(got), field(record, "equal")?));
    }
    let left = bytes(record, "left")?;
    let right = bytes(record, "right")?;
    let same_length = left.len() == right.len();
    let positions = if same_length { left.len() as u64 } else { 0 };
    work(
        record,
        positions,
        if op == "select_bytes" { positions } else { 0 },
        0,
    )?;
    match op {
        "eq" | "eq_fixed" => {
            let mut got = if op == "eq" {
                ct_eq(left, right)
            } else {
                fixed_dispatch(integer(record, "width")?, left, right)?
            };
            if matches!(fault, Fault::Equality) {
                got = !got;
            }
            Ok(ctx.deep_equal(&Value::Bool(got), field(record, "equal")?))
        }
        "select_bytes" => {
            let Value::Bool(choice) = field(record, "choice")? else {
                return Err("choice must be bool".into());
            };
            if !same_length {
                let expected = field(record, "error_id")?
                    .as_text()
                    .ok_or("error_id must be text")?;
                if expected != "CT_SELECT_LENGTH_MISMATCH" {
                    return Err("unknown error ID".into());
                }
                // The public API intentionally still panics on public length
                // mismatch. A successful return cannot be mapped to an error.
                let got = catch_unwind(|| {
                    if matches!(fault, Fault::MissingError) {
                        Vec::new()
                    } else {
                        ct_select_bytes(left, right, *choice)
                    }
                });
                return Ok(ctx.error_is(got.err().map(|_| "CT_SELECT_LENGTH_MISMATCH"), expected));
            }
            let choice = if matches!(fault, Fault::Selection) {
                !choice
            } else {
                *choice
            };
            let mut output = ct_select_bytes(left, right, choice);
            let got = output.clone();
            let left_before = left.to_vec();
            let right_before = right.to_vec();
            if let Some(first) = output.first_mut() {
                *first ^= 0xff;
            }
            // Empty Vecs may share a dangling sentinel: ownership is about
            // mutable contents, not a fabricated allocation for zero bytes.
            let independent = left == left_before
                && right == right_before
                && (output.is_empty()
                    || (output.as_ptr() != left.as_ptr() && output.as_ptr() != right.as_ptr()));
            let bytes_verdict = ctx.deep_equal(&Value::Bytes(got), field(record, "output")?);
            if !matches!(bytes_verdict, Verdict::Pass) {
                return Ok(bytes_verdict);
            }
            Ok(ctx.deep_equal(&Value::Bool(independent), field(record, "fresh_output")?))
        }
        _ => Err("unknown CT01 operation".into()),
    }
}

/// Keep all four logical work fields armed, including zeros at public rejects.
fn work(record: &Record, positions: u64, writes: u64, bits: u64) -> Result<(), String> {
    for (key, want) in [
        ("byte_positions", positions),
        ("byte_reads", 2 * positions),
        ("byte_writes", writes),
        ("u64_bits", bits),
    ] {
        if integer(record, key)? != want {
            return Err(format!("{key} disagrees with public shape"));
        }
    }
    Ok(())
}

fn fixed<const N: usize>(left: &[u8], right: &[u8]) -> Result<bool, String> {
    let left: &[u8; N] = left.try_into().map_err(|_| "left fixed width mismatch")?;
    let right: &[u8; N] = right.try_into().map_err(|_| "right fixed width mismatch")?;
    Ok(ct_eq_fixed(left, right))
}

fn field<'a>(record: &'a Record, name: &str) -> Result<&'a Value, String> {
    record
        .field(name)
        .ok_or_else(|| format!("missing field {name}"))
}
fn bytes<'a>(record: &'a Record, name: &str) -> Result<&'a [u8], String> {
    field(record, name)?
        .as_bytes()
        .ok_or_else(|| format!("{name} must be bytes"))
}
fn integer(record: &Record, name: &str) -> Result<u64, String> {
    field(record, name)?
        .as_u64()
        .ok_or_else(|| format!("{name} must be u64"))
}
