//! # PBT Conformance Corpus Test (VER05 §6)
//!
//! Validates the Rust PBT engine against the canonical `pbt-v1` conformance corpus.
//! Every record in `specs/fixtures/pbt-v1/cases.tfv` is executed as a test in a `Suite`.

use coding_adventures_test_bootstrap::pbt::{self, Gen, PbtOptions};
use coding_adventures_test_bootstrap::run::{self, Ctx, Suite};
use coding_adventures_test_bootstrap::tfv::{self, Record, Value};
use coding_adventures_test_bootstrap::Verdict;

const CORPUS: &str = include_str!("../../../../specs/fixtures/pbt-v1/cases.tfv");

fn main() {
    let records = match tfv::parse_file(CORPUS) {
        Ok(records) => records,
        Err(error) => {
            eprintln!("pbt-v1/cases.tfv does not parse: {error}");
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
        "gen_primitive" => {
            let seed = int_u64(record, "seed")?;
            let type_name = text(record, "type")?;
            let count = int_usize(record, "count")?;
            let got = pbt::gen_primitive(seed, type_name, count)?;
            ctx.deep_equal(&Value::List(got), field(record, "outputs")?)
        }
        "gen_bounded" => {
            let seed = int_u64(record, "seed")?;
            let min_val = int_i128(record, "min")?;
            let max_val = int_i128(record, "max")?;
            let count = int_usize(record, "count")?;
            let got = pbt::gen_bounded(seed, min_val, max_val, count)?;
            ctx.deep_equal(&Value::List(got), field(record, "outputs")?)
        }
        "gen_bytes" => {
            let seed = int_u64(record, "seed")?;
            let min_len = int_usize(record, "min_len")?;
            let max_len = int_usize(record, "max_len")?;
            let count = int_usize(record, "count")?;
            let got = pbt::gen_bytes(seed, min_len, max_len, count)?;
            ctx.deep_equal(&Value::List(got), field(record, "outputs")?)
        }
        "shrink_int" | "shrink_refinement" => {
            let initial = int_i128(record, "initial")?;
            let target = int_i128(record, "target")?;
            let min_val = int_i128(record, "min")?;
            let max_val = int_i128(record, "max")?;
            let prop_name = text(record, "property")?;
            let expected_minimal = int_i128(record, "expected_minimal")?;
            let expected_steps = int_usize(record, "expected_steps")?;

            let (got_minimal, got_steps) = pbt::shrink_int(
                initial,
                target,
                min_val,
                max_val,
                |x| eval_int_property(prop_name, x).unwrap_or(false),
                1000,
            );

            if got_minimal != expected_minimal {
                Verdict::fail(
                    "corpus-mismatch",
                    format!("minimal mismatch: got {got_minimal}, want {expected_minimal}"),
                )
            } else if got_steps != expected_steps {
                Verdict::fail(
                    "corpus-mismatch",
                    format!("step count mismatch: got {got_steps}, want {expected_steps}"),
                )
            } else {
                Verdict::Pass
            }
        }
        "shrink_list" => {
            let initial = ints_i128(list(record, "initial")?)?;
            let elem_target = int_i128(record, "elem_target")?;
            let prop_name = text(record, "property")?;
            let expected_minimal = ints_i128(list(record, "expected_minimal")?)?;
            let expected_steps = int_usize(record, "expected_steps")?;

            let (got_minimal, got_steps) = pbt::shrink_list(
                &initial,
                elem_target,
                |slice| eval_list_property(prop_name, slice).unwrap_or(false),
                1000,
            );

            if got_minimal != expected_minimal {
                Verdict::fail(
                    "corpus-mismatch",
                    format!(
                        "minimal list mismatch: got {got_minimal:?}, want {expected_minimal:?}"
                    ),
                )
            } else if got_steps != expected_steps {
                Verdict::fail(
                    "corpus-mismatch",
                    format!("step count mismatch: got {got_steps}, want {expected_steps}"),
                )
            } else {
                Verdict::Pass
            }
        }
        "shrink_bytes" => {
            let initial = field(record, "initial")?
                .as_bytes()
                .ok_or("initial must be bytes")?;
            let prop_name = text(record, "property")?;
            let expected_minimal = field(record, "expected_minimal")?
                .as_bytes()
                .ok_or("expected_minimal must be bytes")?;
            let expected_steps = int_usize(record, "expected_steps")?;

            let (got_minimal, got_steps) = pbt::shrink_bytes(
                initial,
                |b| eval_bytes_property(prop_name, b).unwrap_or(false),
                1000,
            );

            if got_minimal != expected_minimal {
                Verdict::fail(
                    "corpus-mismatch",
                    format!(
                        "minimal bytes mismatch: got {got_minimal:?}, want {expected_minimal:?}"
                    ),
                )
            } else if got_steps != expected_steps {
                Verdict::fail(
                    "corpus-mismatch",
                    format!("step count mismatch: got {got_steps}, want {expected_steps}"),
                )
            } else {
                Verdict::Pass
            }
        }
        "replay_consistency" => {
            let seed = int_u64(record, "seed")?;
            let prop_name = text(record, "property")?;
            let expected_init = int_i128(record, "initial_fail")?;
            let expected_minimal = int_i128(record, "expected_minimal")?;

            let (got_init, got_min) =
                pbt::simulate_replay(seed, |x| eval_int_property(prop_name, x).unwrap_or(false))
                    .map_err(|e| e.to_string())?;

            if got_init != expected_init {
                Verdict::fail(
                    "corpus-mismatch",
                    format!("initial fail mismatch: got {got_init}, want {expected_init}"),
                )
            } else if got_min != expected_minimal {
                Verdict::fail(
                    "corpus-mismatch",
                    format!("minimal mismatch: got {got_min}, want {expected_minimal}"),
                )
            } else {
                Verdict::Pass
            }
        }
        "filter_exhausted" => {
            let max_retries = int_usize(record, "max_retries")?;
            let exhausted_gen = Gen::bounded(1, 10).filter(|_| false, max_retries);
            let opts = PbtOptions {
                num_tests: 10,
                max_retries,
                max_shrink_steps: 100,
                is_ci: false,
            };
            let verdict = pbt::check_property(
                12345,
                &opts,
                |r| exhausted_gen.sample(r),
                |_: &i128| true,
                |cand, _| (cand, 0),
            );

            let want_verdict = text(record, "verdict")?;
            let want_fid = text(record, "failure_id")?;

            match verdict {
                Verdict::Fail { failure_id, .. }
                    if want_verdict == "fail" && failure_id == want_fid =>
                {
                    Verdict::Pass
                }
                Verdict::Pass => Verdict::fail("corpus-mismatch", "expected failure, got pass"),
                Verdict::Fail { failure_id, .. } => Verdict::fail(
                    "corpus-mismatch",
                    format!("expected failure_id {want_fid}, got {failure_id}"),
                ),
            }
        }
        "failure_id_valid" => {
            let candidate = text(record, "input")?;
            let got = pbt::is_registered_failure_id(candidate);
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

fn eval_int_property(name: &str, x: i128) -> Result<bool, String> {
    match name {
        "int_le_100" => Ok(x <= 100),
        "int_in_range_neg10_50" => Ok((-10..=50).contains(&x)),
        "refinement_x_le_50" => Ok(x <= 50),
        other => Err(format!("unknown int property: {other}")),
    }
}

fn eval_list_property(name: &str, lst: &[i128]) -> Result<bool, String> {
    match name {
        "list_no_consecutive_duplicates" => Ok(lst.windows(2).all(|w| w[0] != w[1])),
        "list_sum_le_20" => Ok(lst.iter().sum::<i128>() <= 20),
        other => Err(format!("unknown list property: {other}")),
    }
}

fn eval_bytes_property(name: &str, b: &[u8]) -> Result<bool, String> {
    match name {
        "bytes_len_lt_5" => Ok(b.len() < 5),
        other => Err(format!("unknown bytes property: {other}")),
    }
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

fn int_u64(record: &Record, name: &str) -> Result<u64, String> {
    field(record, name)?
        .as_u64()
        .ok_or_else(|| format!("{name} must be a u64"))
}

fn int_usize(record: &Record, name: &str) -> Result<usize, String> {
    let u = int_u64(record, name)?;
    usize::try_from(u).map_err(|_| format!("{name} does not fit in usize"))
}

fn int_i128(record: &Record, name: &str) -> Result<i128, String> {
    match field(record, name)? {
        Value::Int(s) => s
            .parse::<i128>()
            .map_err(|_| format!("{name} is not a valid i128: {s}")),
        _ => Err(format!("{name} must be an integer")),
    }
}

fn list<'r>(record: &'r Record, name: &str) -> Result<&'r [Value], String> {
    field(record, name)?
        .as_list()
        .ok_or_else(|| format!("{name} must be a list"))
}

fn ints_i128(values: &[Value]) -> Result<Vec<i128>, String> {
    values
        .iter()
        .map(|v| match v {
            Value::Int(s) => s
                .parse::<i128>()
                .map_err(|_| format!("item is not a valid i128: {s}")),
            _ => Err("expected int item".to_owned()),
        })
        .collect()
}
