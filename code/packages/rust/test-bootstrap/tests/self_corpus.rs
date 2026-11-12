//! # The self-corpus: the harness checking itself against data (TF01 §4)
//!
//! Every record in `cases.tfv` becomes one test. The record says what to
//! compute and what the answer must be; this file computes it with the
//! crate and compares. The expected answers were derived by an independent
//! Python reference, so a pass here means two implementations agree.
//!
//! ```text
//!   deep_equal / error_is   run the verdict core (through Ctx, so sabotage
//!                           applies) and compare with the expected verdict
//!   splitmix64 / fnv1a64 /  run the protocol primitive and compare outputs
//!   shuffle / sub_seed
//!   tfv_reject              the parser must reject the text
//! ```
//!
//! Run it with `--sabotage always-pass` and every expected *fail* case
//! disagrees with the (sabotaged) core, so the run must print `not ok`.
//!
//! The corpus is embedded at compile time, so this test reads no files.

use coding_adventures_test_bootstrap::prng::{fnv1a64, shuffled, sub_seed, SplitMix64};
use coding_adventures_test_bootstrap::run::{self, Ctx, Suite};
use coding_adventures_test_bootstrap::tfv::{self, Record, Value};
use coding_adventures_test_bootstrap::Verdict;

const CORPUS: &str = include_str!("../../../../specs/fixtures/test-bootstrap-v1/cases.tfv");

pub(crate) fn main() {
    let records = match tfv::parse_file(CORPUS) {
        Ok(records) => records,
        Err(error) => {
            eprintln!("test-bootstrap-v1/cases.tfv does not parse: {error}");
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

/// One record's test. `Err` means the record itself is not shaped the way
/// its `op` requires (the validator should have caught that first).
fn check(ctx: &Ctx, record: &Record) -> Result<Verdict, String> {
    let op = text(record, "op")?;
    Ok(match op {
        "deep_equal" => {
            let got = ctx.deep_equal(field(record, "actual")?, field(record, "expected")?);
            expect_verdict(record, got)?
        }
        "error_is" => {
            let raised = match field(record, "raised")? {
                Value::Null => None,
                Value::Text(id) => Some(id.as_str()),
                _ => return Err("raised must be null or text".into()),
            };
            let got = ctx.error_is(raised, text(record, "expected")?);
            expect_verdict(record, got)?
        }
        "failure_id_valid" => {
            let got = coding_adventures_test_bootstrap::verdict::is_registered_failure_id(text(
                record, "input",
            )?);
            match field(record, "valid")? {
                Value::Bool(want) => {
                    agree(got == *want, || "failure-ID membership disagrees".into())
                }
                _ => return Err("valid must be bool".into()),
            }
        }
        "splitmix64" => {
            let mut rng = SplitMix64::new(int(record, "seed")?);
            let want = ints(list(record, "outputs")?)?;
            let got: Vec<u64> = want.iter().map(|_| rng.next_u64()).collect();
            agree(got == want, || format!("outputs {got:?}"))
        }
        "fnv1a64" => {
            let input = field(record, "input")?
                .as_bytes()
                .ok_or("input must be bytes")?;
            let got = fnv1a64(input);
            agree(got == int(record, "hash")?, || format!("hash {got}"))
        }
        "shuffle" => {
            let ids = texts(list(record, "ids")?)?;
            let got = shuffled(int(record, "seed")?, &ids);
            agree(got == texts(list(record, "order")?)?, || {
                format!("order {got:?}")
            })
        }
        "sub_seed" => {
            let got = sub_seed(int(record, "seed")?, text(record, "test_id")?);
            agree(got == int(record, "sub_seed")?, || {
                format!("sub-seed {got}")
            })
        }
        "tfv_reject" => match tfv::parse_file(text(record, "text")?) {
            Err(_) => Verdict::Pass,
            Ok(_) => Verdict::fail(
                "corpus-mismatch",
                "the parser accepted a text it must reject",
            ),
        },
        other => return Err(format!("unknown op {other:?}")),
    })
}

/// Compare the core's verdict with the record's `verdict` and `failure_id`.
fn expect_verdict(record: &Record, got: Verdict) -> Result<Verdict, String> {
    let want = text(record, "verdict")?;
    let matches = match (&got, want) {
        (Verdict::Pass, "pass") => true,
        (Verdict::Fail { failure_id, .. }, "fail") => *failure_id == text(record, "failure_id")?,
        (_, "pass" | "fail") => false,
        _ => return Err(format!("verdict must be pass or fail, got {want:?}")),
    };
    Ok(agree(matches, || {
        let said = match &got {
            Verdict::Pass => "pass".to_owned(),
            Verdict::Fail { failure_id, .. } => format!("fail ({failure_id})"),
        };
        format!("corpus expects {want}, the core said {said}")
    }))
}

fn agree(ok: bool, got: impl FnOnce() -> String) -> Verdict {
    if ok {
        Verdict::Pass
    } else {
        Verdict::fail(
            "corpus-mismatch",
            format!("disagrees with the corpus: {}", got()),
        )
    }
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
        .ok_or_else(|| format!("{name} must be text"))
}

fn int(record: &Record, name: &str) -> Result<u64, String> {
    field(record, name)?
        .as_u64()
        .ok_or_else(|| format!("{name} must be a u64"))
}

fn list<'r>(record: &'r Record, name: &str) -> Result<&'r [Value], String> {
    field(record, name)?
        .as_list()
        .ok_or_else(|| format!("{name} must be a list"))
}

fn ints(values: &[Value]) -> Result<Vec<u64>, String> {
    values
        .iter()
        .map(|v| v.as_u64().ok_or_else(|| "expected u64 items".to_owned()))
        .collect()
}

fn texts(values: &[Value]) -> Result<Vec<String>, String> {
    values
        .iter()
        .map(|v| {
            v.as_text()
                .map(str::to_owned)
                .ok_or_else(|| "expected text items".to_owned())
        })
        .collect()
}
