//! # The CT01 corpus, run through the L0 bootstrap harness (VER26 step 4)
//!
//! `code/specs/fixtures/ct-compare-v1/` is the neutral oracle every
//! `ct-compare` lane must agree with. Its answers were derived by an
//! independent Python validator (`code/scripts/ct_compare_conformance.py`),
//! never captured from a package, so passing here means this crate agrees
//! with a second implementation, not just with itself.
//!
//! The harness is `test-bootstrap` (TF01), not libtest: it reads the corpus
//! in TFV (no JSON parser needed), runs the cases in a seeded shuffled order
//! and prints TAP v14. Each corpus record becomes one test with the record's
//! id, so a failure names the exact CT01 case.
//!
//! ## What each record checks
//!
//! ```text
//!   op            calls                         compares with
//!   ------------  ----------------------------  ------------------------------
//!   eq            ct_eq(left, right)            equal
//!   eq_fixed      ct_eq_fixed::<width>(..)      equal
//!   select_bytes  ct_select_bytes(left, right,  output + fresh_output, or
//!                   choice)                       error_id (the CT01 error)
//!   eq_u64        ct_eq_u64(left, right)        equal
//!   every op      this lane's work model        work_positions, work_reads,
//!                                                 work_writes, work_u64_bits
//! ```
//!
//! ## The work counts, honestly
//!
//! CT01's `expected_work` counts *logical* input positions, not instructions.
//! Nothing at runtime can count how many bytes a slice comparison read, so
//! [`work_model`] states this lane's loop shape as read off `src/lib.rs` (the
//! loop bound is the public length; a length mismatch returns before any
//! read), and the corpus checks that statement. What ties that statement to
//! the *compiled* code is `tests/optimized_ir.rs`, which inspects optimized
//! LLVM IR for an early exit. Neither is a timing measurement.
//!
//! The corpus is embedded at compile time, so this test reads no files.

use coding_adventures_ct_compare::{
    ct_eq, ct_eq_fixed, ct_eq_u64, ct_select_bytes, SelectLengthMismatch,
};
use coding_adventures_test_bootstrap::run::{self, Ctx, Suite};
use coding_adventures_test_bootstrap::tfv::{self, Record, Value};
use coding_adventures_test_bootstrap::Verdict;

const CORPUS: &str = include_str!("../../../../specs/fixtures/ct-compare-v1/cases.tfv");

/// The corpus has 21 cases today. A truncated or emptied file must not pass
/// vacuously, so the run bails out below this many records.
const MIN_CASES: usize = 21;

fn main() {
    let records = match tfv::parse_file(CORPUS) {
        Ok(records) => records,
        Err(error) => {
            eprintln!("ct-compare-v1/cases.tfv does not parse: {error}");
            std::process::exit(1);
        }
    };
    if records.len() < MIN_CASES {
        eprintln!(
            "ct-compare-v1/cases.tfv has {} records, expected at least {MIN_CASES}",
            records.len()
        );
        std::process::exit(1);
    }
    let mut suite = Suite::new();
    for record in records {
        let id = record.id.clone();
        suite.add(id, move |ctx| match check(ctx, &record) {
            Ok(verdict) => verdict,
            Err(problem) => Verdict::fail("corpus-malformed", problem),
        });
    }
    std::process::exit(run::main(&suite));
}

/// One record's test: the operation's result first, then its work counts.
/// `Err` means the record is not shaped the way its `op` requires.
fn check(ctx: &Ctx, record: &Record) -> Result<Verdict, String> {
    let op = text(record, "op")?;
    let verdict = match op {
        "eq" => {
            let got = ct_eq(bytes(record, "left")?, bytes(record, "right")?);
            ctx.deep_equal(&Value::Bool(got), field(record, "equal")?)
        }
        "eq_fixed" => {
            let width = int(record, "width")?;
            let got = eq_fixed(width, bytes(record, "left")?, bytes(record, "right")?)?;
            ctx.deep_equal(&Value::Bool(got), field(record, "equal")?)
        }
        "select_bytes" => select(ctx, record)?,
        "eq_u64" => {
            let got = ct_eq_u64(int(record, "left")?, int(record, "right")?);
            ctx.deep_equal(&Value::Bool(got), field(record, "equal")?)
        }
        other => return Err(format!("unknown op {other:?}")),
    };
    if verdict != Verdict::Pass {
        return Ok(verdict);
    }
    let expected = Value::List(
        [
            "work_positions",
            "work_reads",
            "work_writes",
            "work_u64_bits",
        ]
        .iter()
        .map(|name| field(record, name).cloned())
        .collect::<Result<_, _>>()?,
    );
    let declared = work_model(op, record)?;
    Ok(ctx.deep_equal(
        &Value::List(declared.iter().map(|&n| Value::from_u64(n)).collect()),
        &expected,
    ))
}

/// `ct_eq_fixed` is generic over a compile-time width, so each width the
/// corpus uses needs its own instantiation. A width with none is a corpus
/// this adapter cannot run, reported rather than silently skipped.
fn eq_fixed(width: u64, left: &[u8], right: &[u8]) -> Result<bool, String> {
    fn at<const N: usize>(left: &[u8], right: &[u8]) -> Result<bool, String> {
        let left: &[u8; N] = left.try_into().map_err(|_| "left is not width bytes")?;
        let right: &[u8; N] = right.try_into().map_err(|_| "right is not width bytes")?;
        Ok(ct_eq_fixed(left, right))
    }
    match width {
        0 => at::<0>(left, right),
        16 => at::<16>(left, right),
        32 => at::<32>(left, right),
        64 => at::<64>(left, right),
        other => Err(format!("no ct_eq_fixed instantiation for width {other}")),
    }
}

/// `select_bytes`: either the selected bytes in fresh storage, or CT01's
/// semantic error.
///
/// Freshness is checked the way CT01 asks: call with mutable inputs, then
/// overwrite both inputs and confirm the result did not change with them.
/// Rust's types already rule out aliasing here (the result is an owned
/// `Vec`), so this check guards against a future signature that returns a
/// borrowed slice.
fn select(ctx: &Ctx, record: &Record) -> Result<Verdict, String> {
    let mut left = bytes(record, "left")?.to_vec();
    let mut right = bytes(record, "right")?.to_vec();
    let choice = match field(record, "choice")? {
        Value::Bool(choice) => *choice,
        _ => return Err("choice must be a bool".into()),
    };
    let result = ct_select_bytes(&left, &right, choice);
    if let Some(expected_id) = record.field("error_id") {
        let expected_id = expected_id.as_text().ok_or("error_id must be text")?;
        let raised = result.as_ref().err().map(|_| SelectLengthMismatch::ID);
        return Ok(ctx.error_is(raised, expected_id));
    }
    let Ok(output) = result else {
        return Ok(Verdict::fail(
            "not-deep-equal",
            format!("expected an output, got error {}", SelectLengthMismatch::ID),
        ));
    };
    if !matches!(field(record, "fresh_output")?, Value::Bool(true)) {
        return Err("fresh_output must be true: CT01 requires fresh storage".into());
    }
    // Overwrite both inputs after the call. Were `output` a view of either,
    // it would change with them and no longer equal the expected bytes.
    for byte in left.iter_mut().chain(right.iter_mut()) {
        *byte = !*byte;
    }
    Ok(ctx.deep_equal(&Value::Bytes(output), field(record, "output")?))
}

/// This lane's public work, as read off `src/lib.rs`:
/// `[positions, reads, writes, u64_bits]`.
///
/// ```text
///   ct_eq            lengths differ → returns before the loop: 0 positions
///                    lengths match  → `for i in 0..len`, two reads each
///   ct_eq_fixed      `for i in 0..N`, two reads each
///   ct_select_bytes  lengths differ → Err before the loop: 0 positions
///                    lengths match  → `for i in 0..len`, two reads and
///                                     one write each
///   ct_eq_u64        one fixed reduction over all 64 bits
/// ```
fn work_model(op: &str, record: &Record) -> Result<[u64; 4], String> {
    let lengths = || -> Result<(u64, u64), String> {
        Ok((
            bytes(record, "left")?.len() as u64,
            bytes(record, "right")?.len() as u64,
        ))
    };
    Ok(match op {
        "eq" => {
            let (left, right) = lengths()?;
            let positions = if left == right { left } else { 0 };
            [positions, 2 * positions, 0, 0]
        }
        "eq_fixed" => {
            let width = int(record, "width")?;
            [width, 2 * width, 0, 0]
        }
        "select_bytes" => {
            let (left, right) = lengths()?;
            let positions = if left == right { left } else { 0 };
            [positions, 2 * positions, positions, 0]
        }
        "eq_u64" => [0, 0, 0, 64],
        other => return Err(format!("unknown op {other:?}")),
    })
}

// Field accessors: each names what was missing or mis-kinded.

fn field<'r>(record: &'r Record, name: &str) -> Result<&'r Value, String> {
    record
        .field(name)
        .ok_or_else(|| format!("missing field {name:?}"))
}

fn text<'r>(record: &'r Record, name: &str) -> Result<&'r str, String> {
    field(record, name)?
        .as_text()
        .ok_or_else(|| format!("field {name:?} must be text"))
}

fn bytes<'r>(record: &'r Record, name: &str) -> Result<&'r [u8], String> {
    field(record, name)?
        .as_bytes()
        .ok_or_else(|| format!("field {name:?} must be bytes"))
}

fn int(record: &Record, name: &str) -> Result<u64, String> {
    field(record, name)?
        .as_u64()
        .ok_or_else(|| format!("field {name:?} must be an integer in 0..2^64"))
}
