//! # Fuzz Conformance Corpus Test (VER06 §6)
//!
//! Validates the Rust fuzz engine against the canonical `fuzz-v1` conformance corpus.
//! Every record in `specs/fixtures/fuzz-v1/cases.tfv` is executed as a test in a `Suite`.

use coding_adventures_test_bootstrap::fuzz;
use coding_adventures_test_bootstrap::run::{self, Ctx, Suite};
use coding_adventures_test_bootstrap::tfv::{self, Record, Value};
use coding_adventures_test_bootstrap::Verdict;

const CORPUS: &str = include_str!("../../../../specs/fixtures/fuzz-v1/cases.tfv");

fn main() {
    let records = match tfv::parse_file(CORPUS) {
        Ok(records) => records,
        Err(error) => {
            eprintln!("fuzz-v1/cases.tfv does not parse: {error}");
            std::process::exit(1);
        }
    };
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

fn check(ctx: &Ctx, record: &Record) -> Result<Verdict, String> {
    let op = text(record, "op")?;
    Ok(match op {
        "mutate_bit_flip" => {
            let input = bytes(record, "input")?;
            let bit_offset = int_usize(record, "bit_offset")?;
            let count = int_usize(record, "count")?;
            let expected = bytes(record, "output")?;
            let got = fuzz::bit_flip(input, bit_offset, count);
            ctx.deep_equal(&Value::Bytes(got), &Value::Bytes(expected.to_vec()))
        }
        "mutate_byte_flip" => {
            let input = bytes(record, "input")?;
            let byte_offset = int_usize(record, "byte_offset")?;
            let count = int_usize(record, "count")?;
            let expected = bytes(record, "output")?;
            let got = fuzz::byte_flip(input, byte_offset, count);
            ctx.deep_equal(&Value::Bytes(got), &Value::Bytes(expected.to_vec()))
        }
        "mutate_arithmetic" => {
            let input = bytes(record, "input")?;
            let byte_offset = int_usize(record, "byte_offset")?;
            let delta = int_i64(record, "delta")?;
            let width = int_usize(record, "width")?;
            let endian = text(record, "endian")?;
            let expected = bytes(record, "output")?;
            let got = fuzz::byte_arithmetic(input, byte_offset, delta, width, endian);
            ctx.deep_equal(&Value::Bytes(got), &Value::Bytes(expected.to_vec()))
        }
        "mutate_boundary" => {
            let input = bytes(record, "input")?;
            let byte_offset = int_usize(record, "byte_offset")?;
            let width = int_usize(record, "width")?;
            let val = int_u64(record, "val")?;
            let endian = text(record, "endian")?;
            let expected = bytes(record, "output")?;
            let got = fuzz::boundary_insert(input, byte_offset, width, val, endian);
            ctx.deep_equal(&Value::Bytes(got), &Value::Bytes(expected.to_vec()))
        }
        "mutate_block_delete" => {
            let input = bytes(record, "input")?;
            let offset = int_usize(record, "offset")?;
            let length = int_usize(record, "length")?;
            let expected = bytes(record, "output")?;
            let got = fuzz::block_delete(input, offset, length);
            ctx.deep_equal(&Value::Bytes(got), &Value::Bytes(expected.to_vec()))
        }
        "mutate_block_insert" => {
            let input = bytes(record, "input")?;
            let offset = int_usize(record, "offset")?;
            let insert_bytes = bytes(record, "insert_bytes")?;
            let expected = bytes(record, "output")?;
            let got = fuzz::block_insert(input, offset, insert_bytes);
            ctx.deep_equal(&Value::Bytes(got), &Value::Bytes(expected.to_vec()))
        }
        "mutate_block_duplicate" => {
            let input = bytes(record, "input")?;
            let src_offset = int_usize(record, "src_offset")?;
            let length = int_usize(record, "length")?;
            let dst_offset = int_usize(record, "dst_offset")?;
            let expected = bytes(record, "output")?;
            let got = fuzz::block_duplicate(input, src_offset, length, dst_offset);
            ctx.deep_equal(&Value::Bytes(got), &Value::Bytes(expected.to_vec()))
        }
        "mutate_splice" => {
            let target = bytes(record, "target")?;
            let donor = bytes(record, "donor")?;
            let target_split = int_usize(record, "target_split")?;
            let donor_split = int_usize(record, "donor_split")?;
            let expected = bytes(record, "output")?;
            let got = fuzz::splice(target, donor, target_split, donor_split);
            ctx.deep_equal(&Value::Bytes(got), &Value::Bytes(expected.to_vec()))
        }
        "mutate_sequence" => {
            let input = bytes(record, "input")?;
            let seed = int_u64(record, "seed")?;
            let steps = int_usize(record, "steps")?;
            let expected = bytes(record, "output")?;
            let got = fuzz::mutate_sequence(input, seed, steps);
            ctx.deep_equal(&Value::Bytes(got), &Value::Bytes(expected.to_vec()))
        }
        "corpus_admit" => {
            let existing_edges = ints_usize(list(record, "existing_edges")?)?;
            let trace_edges = ints_usize(list(record, "trace_edges")?)?;
            let expected_admitted = match field(record, "admitted")? {
                Value::Bool(b) => *b,
                _ => return Err("admitted must be bool".into()),
            };
            let expected_new = int_usize(record, "new_edges")?;
            let (got_admitted, got_new) = fuzz::corpus_admit(&existing_edges, &trace_edges);

            if got_admitted != expected_admitted {
                Verdict::fail(
                    "corpus-mismatch",
                    format!("admitted mismatch: got {got_admitted}, want {expected_admitted}"),
                )
            } else if got_new != expected_new {
                Verdict::fail(
                    "corpus-mismatch",
                    format!("new_edges mismatch: got {got_new}, want {expected_new}"),
                )
            } else {
                Verdict::Pass
            }
        }
        "crash_dedup" => {
            let kind_a = text(record, "crash_a_kind")?;
            let frames_a = texts(list(record, "crash_a_frames")?)?;
            let kind_b = text(record, "crash_b_kind")?;
            let frames_b = texts(list(record, "crash_b_frames")?)?;
            let expected_dup = match field(record, "is_duplicate")? {
                Value::Bool(b) => *b,
                _ => return Err("is_duplicate must be bool".into()),
            };

            let frames_a_refs: Vec<&str> = frames_a.iter().map(String::as_str).collect();
            let frames_b_refs: Vec<&str> = frames_b.iter().map(String::as_str).collect();
            let got_dup = fuzz::is_duplicate_crash(kind_a, &frames_a_refs, kind_b, &frames_b_refs);

            if got_dup != expected_dup {
                Verdict::fail(
                    "corpus-mismatch",
                    format!("duplicate crash mismatch: got {got_dup}, want {expected_dup}"),
                )
            } else {
                Verdict::Pass
            }
        }
        "failure_id_valid" => {
            let candidate = text(record, "input")?;
            let got = fuzz::is_registered_failure_id(candidate);
            match field(record, "valid")? {
                Value::Bool(want) => {
                    if got == *want {
                        Verdict::Pass
                    } else {
                        Verdict::fail(
                            "corpus-mismatch",
                            format!("failure_id_valid mismatch for {candidate:?}"),
                        )
                    }
                }
                _ => return Err("valid must be bool".into()),
            }
        }
        "private_reporting" => {
            let target = text(record, "target")?;
            let failure_id = text(record, "failure_id")?;
            let raw_payload = bytes(record, "raw_payload")?;
            let sig_hash = text(record, "sig_hash")?;
            let expected_tap = text(record, "tap_line")?;
            let expected_contains = match field(record, "contains_raw_payload")? {
                Value::Bool(b) => *b,
                _ => return Err("contains_raw_payload must be bool".into()),
            };

            let got_tap = fuzz::format_tap_failure(1, target, failure_id, sig_hash);
            if got_tap != expected_tap {
                return Ok(Verdict::fail(
                    "corpus-mismatch",
                    format!("tap_line mismatch: got {got_tap:?}, want {expected_tap:?}"),
                ));
            }

            let hex_raw = raw_payload
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            let contains_raw = got_tap.contains(&hex_raw);
            if contains_raw != expected_contains {
                return Ok(Verdict::fail(
                    "corpus-mismatch",
                    format!(
                        "contains_raw_payload mismatch: got {contains_raw}, want {expected_contains}"
                    ),
                ));
            }
            Verdict::Pass
        }
        "corpus_reject" => {
            let raw_text = text(record, "text")?;
            match tfv::parse_file(raw_text) {
                Err(_) => Verdict::Pass,
                Ok(_) => Verdict::fail(
                    "corpus-mismatch",
                    "the parser accepted a text it must reject",
                ),
            }
        }
        other => return Err(format!("unknown op {other:?}")),
    })
}

// Field accessors

fn field<'r>(record: &'r Record, name: &str) -> Result<&'r Value, String> {
    record
        .field(name)
        .ok_or_else(|| format!("missing field {name:?}"))
}

fn text<'r>(record: &'r Record, name: &str) -> Result<&'r str, String> {
    field(record, name)?
        .as_text()
        .ok_or_else(|| format!("{name} must be text"))
}

fn bytes<'r>(record: &'r Record, name: &str) -> Result<&'r [u8], String> {
    field(record, name)?
        .as_bytes()
        .ok_or_else(|| format!("{name} must be bytes"))
}

fn int_u64(record: &Record, name: &str) -> Result<u64, String> {
    field(record, name)?
        .as_u64()
        .ok_or_else(|| format!("{name} must be a u64"))
}

fn int_usize(record: &Record, name: &str) -> Result<usize, String> {
    let u = int_u64(record, name)?;
    usize::try_from(u).map_err(|_| format!("{name} does not fit in usize"))
}

fn int_i64(record: &Record, name: &str) -> Result<i64, String> {
    match field(record, name)? {
        Value::Int(s) => s
            .parse::<i64>()
            .map_err(|_| format!("{name} is not a valid i64: {s}")),
        _ => Err(format!("{name} must be an integer")),
    }
}

fn list<'r>(record: &'r Record, name: &str) -> Result<&'r [Value], String> {
    field(record, name)?
        .as_list()
        .ok_or_else(|| format!("{name} must be a list"))
}

fn ints_usize(values: &[Value]) -> Result<Vec<usize>, String> {
    values
        .iter()
        .map(|v| match v {
            Value::Int(s) => s
                .parse::<usize>()
                .map_err(|_| format!("item is not a valid usize: {s}")),
            _ => Err("expected int item".to_owned()),
        })
        .collect()
}

fn texts(values: &[Value]) -> Result<Vec<String>, String> {
    values
        .iter()
        .map(|v| match v {
            Value::Text(s) => Ok(s.clone()),
            _ => Err("expected text item".to_owned()),
        })
        .collect()
}
