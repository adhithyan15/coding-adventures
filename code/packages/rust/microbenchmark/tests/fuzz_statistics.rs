//! Deterministic VER06 fuzz campaign over arbitrary sample encodings.

use coding_adventures_rank_statistics::{median, median_interval};
use coding_adventures_test_bootstrap::fuzz::CoverageBitmap;
use coding_adventures_test_bootstrap::fuzz::{fuzz_target, FuzzOptions, FuzzOutcome};

#[test]
fn seeded_mutations_preserve_rank_statistic_invariants() {
    let corpus = vec![vec![], vec![0], vec![1; 8], (0..64).collect()];
    let report = fuzz_target(
        &corpus,
        &FuzzOptions {
            max_iterations: 256,
            seed: 0x5241_4e4b,
            max_input_len: 128,
            base_energy: 4,
        },
        |bytes, coverage: &mut CoverageBitmap| {
            coverage.record_edge(bytes.len());
            let values: Vec<f64> = bytes.iter().map(|byte| f64::from(*byte) - 127.0).collect();
            if values.is_empty() {
                return if median(&values).is_err() && median_interval(&values, 95).is_err() {
                    FuzzOutcome::Pass
                } else {
                    FuzzOutcome::Crash {
                        kind: "empty-sample-accepted".into(),
                        frames: vec!["fuzz_statistics".into()],
                    }
                };
            }
            let estimate = match median(&values) {
                Ok(value) => value,
                Err(error) => {
                    return FuzzOutcome::Crash {
                        kind: error.to_string(),
                        frames: vec!["median".into()],
                    }
                }
            };
            let min = values.iter().copied().fold(f64::INFINITY, f64::min);
            let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            if estimate < min || estimate > max {
                return FuzzOutcome::Crash {
                    kind: "median-outside-sample".into(),
                    frames: vec!["median".into()],
                };
            }
            if let Some(interval) = match median_interval(&values, 95) {
                Ok(value) => value,
                Err(error) => {
                    return FuzzOutcome::Crash {
                        kind: error.to_string(),
                        frames: vec!["interval".into()],
                    }
                }
            } {
                coverage.record_edge(256 ^ interval.low.to_bits() as usize);
                if interval.low > interval.high
                    || !values.contains(&interval.low)
                    || !values.contains(&interval.high)
                {
                    return FuzzOutcome::Crash {
                        kind: "interval-not-order-statistic".into(),
                        frames: vec!["median_interval".into()],
                    };
                }
            }
            FuzzOutcome::Pass
        },
    );
    assert!(
        report.is_clean(),
        "fuzz campaign found an invariant violation: {report:?}"
    );
}
