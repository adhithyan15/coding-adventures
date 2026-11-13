//! # Fixture binary for `l0_contracts.rs`: a real `fn main()` suite
//!
//! `run::main` reads the process arguments and turns a `Summary` into an
//! exit status, which cannot be observed from inside the process. This
//! target is a tiny `harness = false` suite whose contents are chosen by the
//! `TFB_FIXTURE` environment variable, so `l0_contracts.rs` can spawn it and
//! look at the real exit status and the real bytes on stdout.
//!
//! Run by itself (no variable set) it is two passing tests and exits 0, so
//! `cargo test` stays green.
//!
//! | `TFB_FIXTURE`    | suite                                                    |
//! |------------------|----------------------------------------------------------|
//! | unset / `pass`   | two tests that pass                                      |
//! | `fail`           | one pass, one `deep_equal` failure                       |
//! | `panic`          | one pass, one test that panics                           |
//! | `vacuous`        | one test that returns `Pass` without asserting anything  |
//! | `bad-failure-id` | one test that fabricates an unregistered failure id      |
//! | `duplicate`      | two tests with the same id                               |
//! | `empty`          | no tests at all                                          |
//! | `exit-zero`      | one pass, then a test that calls `process::exit(0)`      |

use coding_adventures_test_bootstrap::run::{self, Suite};
use coding_adventures_test_bootstrap::tfv::Value;
use coding_adventures_test_bootstrap::Verdict;

pub(crate) fn main() {
    let mode = std::env::var("TFB_FIXTURE").unwrap_or_else(|_| "pass".into());
    let mut suite = Suite::new();
    match mode.as_str() {
        "pass" => {
            suite.add("fixture/one", |ctx| {
                ctx.deep_equal(&Value::Null, &Value::Null)
            });
            suite.add("fixture/two", |ctx| ctx.error_is(Some("E"), "E"));
        }
        "fail" => {
            suite.add("fixture/good", |ctx| {
                ctx.deep_equal(&Value::Null, &Value::Null)
            });
            suite.add("fixture/bad", |ctx| {
                ctx.deep_equal(&Value::Bool(true), &Value::Bool(false))
            });
        }
        "panic" => {
            suite.add("fixture/good", |_| Verdict::Pass);
            suite.add("fixture/boom", |_| panic!("boom"));
        }
        "vacuous" => {
            suite.add("fixture/vacuous", |_| Verdict::Pass);
        }
        "bad-failure-id" => {
            suite.add("fixture/forged", |_| Verdict::Fail {
                failure_id: "made-up",
                message: "x".into(),
            });
        }
        "duplicate" => {
            suite.add("fixture/dup", |_| Verdict::Pass);
            suite.add("fixture/dup", |_| Verdict::Pass);
        }
        "empty" => {}
        "exit-zero" => {
            suite.add("fixture/a-first", |_| Verdict::Pass);
            suite.add("fixture/z-quitter", |_| std::process::exit(0));
        }
        other => panic!("unknown TFB_FIXTURE {other:?}"),
    }
    std::process::exit(run::main(&suite));
}
