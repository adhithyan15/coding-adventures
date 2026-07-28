//! Harness tests for `sir-bench`.  The pure formatting/lowering paths run
//! everywhere; the one path that spawns a toolchain uses the interpreted Ruby
//! target (no compile, no fresh binary) and a trivial program, and tolerates a
//! `Skipped` when `ruby` is not on `PATH`.

use sir_bench::{corpus, lower, markdown_report, measure, Bench, Sample, Target};
use std::time::Duration;

#[test]
fn every_corpus_program_lowers() {
    // The frontend must accept every shipped program (else a whole table is a
    // "frontend failed" row that never measures anything).
    for b in corpus() {
        assert!(
            lower(b.name, b.ruby).is_ok(),
            "corpus program `{}` failed to lower",
            b.name
        );
    }
}

#[test]
fn report_ranks_fastest_first_and_marks_skips() {
    let bench = Bench {
        name: "demo",
        ruby: "puts 1\n",
        iters: 3,
        warmup: 1,
        note: "demo",
    };
    let results = vec![
        (
            Target::Ruby,
            Sample::Ran {
                emit: Duration::from_micros(300),
                compile: None,
                run: Duration::from_millis(200), // slow
                stdout: "1".into(),
            },
        ),
        (
            Target::C,
            Sample::Ran {
                emit: Duration::from_micros(400),
                compile: Some(Duration::from_millis(120)),
                run: Duration::from_millis(6), // fastest
                stdout: "1".into(),
            },
        ),
        (Target::Go, Sample::Skipped("go not on PATH".into())),
    ];
    let report = markdown_report(&bench, Some("1"), &results);

    // The fastest (C, 6ms) must appear before the slow (Ruby, 200ms).
    let c_at = report.find("| c |").expect("c row present");
    let ruby_at = report.find("| ruby |").expect("ruby row present");
    assert!(c_at < ruby_at, "fastest backend must be listed first:\n{report}");

    // The skip is surfaced, not rendered as a zero.
    assert!(report.contains("_skip_"), "skip is marked:\n{report}");
    assert!(report.contains("go not on PATH"), "skip reason shown:\n{report}");

    // The compiled cell shows a compile time; the interpreted one is blank.
    assert!(report.contains("120.00"), "C carries a compile time:\n{report}");

    // The `vs fastest` ratio ranks Ruby well above 1×.
    assert!(report.contains('×'), "ratio column present:\n{report}");
}

#[test]
fn measure_ruby_runs_or_skips_but_never_panics() {
    // Interpreted Ruby: no compile, no fresh binary (so no first-exec scan) —
    // a fast, deterministic exercise of the real emit+run path.
    let bench = Bench {
        name: "tiny",
        ruby: "puts 40 + 2\n",
        iters: 1,
        warmup: 0,
        note: "tiny",
    };
    let module = lower(bench.name, bench.ruby).expect("lower tiny");
    match measure(&bench, &module, Target::Ruby) {
        Sample::Ran { stdout, .. } => assert_eq!(stdout, "42"),
        Sample::Skipped(_) => { /* ruby not installed on this host — fine */ }
        Sample::Failed(e) => panic!("ruby measure failed: {e}"),
    }
}
