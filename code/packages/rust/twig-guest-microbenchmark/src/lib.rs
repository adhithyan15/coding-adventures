//! Host-clocked measurement of programs dispatched by the real Twig VM.
//!
//! Compilation is a preparation step. A measured operation executes one
//! already-compiled IIR module and returns the guest's sum. This boundary is
//! part of the result: it includes the host-to-dispatch call and interpreter
//! execution, and it does not represent simulated processor cycles.

use coding_adventures_microbenchmark::{collect, Batch, Clock, Config, Error, Report};
use interpreter_ir::IIRModule;
use twig_vm::{dispatch, LispyValue, TwigVM};

/// Sum of the integers from one through the guest's fixed upper bound (32).
pub const GUEST_CHECKSUM: i64 = 528;
/// Each logical operation executes one compiled Twig guest entry.
pub const WORKLOAD_ID: &str = "twig-recursive-sum-1-through-32";

/// Source for a recursive sum and an accumulator-style recursive sum.
///
/// Both programs visit the same 32 integers. Their recursion shapes differ,
/// while the returned checksum keeps the collector's correctness criterion
/// identical.
const BASELINE_SOURCE: &str = "(define (sum n) (if (= n 0) 0 (+ n (sum (- n 1))))) (sum 32)";
const CANDIDATE_SOURCE: &str =
    "(define (sum n acc) (if (= n 0) acc (sum (- n 1) (+ acc n)))) (sum 32 0)";
const EMPTY_SOURCE: &str = "0";

/// Prepared programs for repeated guest dispatch. Construct before timing.
pub struct TwigGuestBenchmark {
    baseline: IIRModule,
    candidate: IIRModule,
    empty: IIRModule,
}

impl TwigGuestBenchmark {
    /// Compile both variants and the empty-entry probe once, then verify them.
    pub fn prepare() -> Result<Self, Error> {
        let vm = TwigVM::new();
        let baseline = vm
            .compile(BASELINE_SOURCE)
            .map_err(|e| Error(e.to_string()))?;
        let candidate = vm
            .compile(CANDIDATE_SOURCE)
            .map_err(|e| Error(e.to_string()))?;
        let empty = vm.compile(EMPTY_SOURCE).map_err(|e| Error(e.to_string()))?;
        verify_result(&baseline, GUEST_CHECKSUM)?;
        verify_result(&candidate, GUEST_CHECKSUM)?;
        verify_result(&empty, 0)?;
        Ok(Self {
            baseline,
            candidate,
            empty,
        })
    }

    /// Execute `entries` baseline guest calls and sum their returned checksums.
    pub fn baseline_batch(&self, entries: usize) -> Result<i64, Error> {
        run_batch(&self.baseline, entries)
    }

    /// Execute `entries` candidate guest calls and sum their returned checksums.
    pub fn candidate_batch(&self, entries: usize) -> Result<i64, Error> {
        run_batch(&self.candidate, entries)
    }

    /// Execute `entries` empty guest calls; each must return zero.
    pub fn empty_batch(&self, entries: usize) -> Result<i64, Error> {
        run_batch(&self.empty, entries)
    }

    /// Collect a paired report using the caller's monotonic host clock.
    pub fn collect(&self, config: Config, clock: &mut Clock<'_>) -> Result<Report, Error> {
        let mut baseline = |entries| self.baseline_batch(entries);
        let mut candidate = |entries| self.candidate_batch(entries);
        collect(config, GUEST_CHECKSUM, &mut baseline, &mut candidate, clock)
    }
}

fn verify_result(module: &IIRModule, expected: i64) -> Result<(), Error> {
    let value = dispatch::run(module).map_err(|e| Error(e.to_string()))?;
    let actual = value
        .as_int()
        .ok_or_else(|| Error("Twig guest result is not an integer".to_owned()))?;
    if actual != expected {
        return Err(Error(format!(
            "Twig guest checksum mismatch: got {actual}, want {expected}"
        )));
    }
    Ok(())
}

fn run_batch(module: &IIRModule, entries: usize) -> Result<i64, Error> {
    if entries == 0 || entries as u128 > i64::MAX as u128 {
        return Err(Error(
            "guest entry count must be a positive signed 64-bit value".to_owned(),
        ));
    }
    let operation_count = i64::try_from(entries)
        .map_err(|_| Error("guest entry count exceeds signed 64-bit range".to_owned()))?;
    GUEST_CHECKSUM
        .checked_mul(operation_count)
        .ok_or_else(|| Error("guest batch checksum overflow".to_owned()))?;
    let mut checksum = 0_i64;
    for _ in 0..entries {
        let value: LispyValue = dispatch::run(module).map_err(|e| Error(e.to_string()))?;
        let result = value
            .as_int()
            .ok_or_else(|| Error("Twig guest result is not an integer".to_owned()))?;
        checksum = checksum
            .checked_add(result)
            .ok_or_else(|| Error("guest batch checksum overflow".to_owned()))?;
    }
    Ok(checksum)
}

/// Construct the default collector configuration (40 pairs, 95% confidence).
pub fn default_config() -> Config {
    Config::default()
}

/// Collect caller-prepared guest entry functions using the shared VER07 contract.
pub fn collect_batches(
    config: Config,
    expected_checksum: i64,
    baseline: &mut Batch<'_>,
    candidate: &mut Batch<'_>,
    clock: &mut Clock<'_>,
) -> Result<Report, Error> {
    collect(config, expected_checksum, baseline, candidate, clock)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prepared_guest_variants_return_the_same_checksum() {
        let guest = TwigGuestBenchmark::prepare().unwrap();
        for entries in 1..=100 {
            assert_eq!(
                guest.baseline_batch(entries).unwrap(),
                GUEST_CHECKSUM * entries as i64
            );
            assert_eq!(
                guest.candidate_batch(entries).unwrap(),
                GUEST_CHECKSUM * entries as i64
            );
        }
        assert!(guest.baseline_batch(0).is_err());
        assert!(guest.candidate_batch(usize::MAX).is_err());
    }

    #[test]
    fn copied_fixture_matches_canonical_source_bytes() {
        let local = include_bytes!("../testdata/microbenchmark-pairs-v1.json");
        let canonical = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../specs/fixtures/microbenchmark-pairs-v1.json");
        if canonical.is_file() {
            assert_eq!(local.as_slice(), std::fs::read(canonical).unwrap());
        }
        let fixture: serde_json::Value = serde_json::from_slice(local).unwrap();
        let cases = fixture["cases"].as_array().unwrap();
        assert!(cases.len() >= 6);
        for case in cases {
            let candidates = case["candidate_ns"].as_array().unwrap();
            let pairs: Vec<_> = candidates.iter().enumerate().map(|(index, candidate)| {
                serde_json::json!({
                    "baseline_ns": case["baseline_ns"],
                    "candidate_ns": candidate,
                    "order": if index < candidates.len().div_ceil(2) { "baseline-first" } else { "candidate-first" },
                })
            }).collect();
            let core = serde_json::json!({
                "config": {"pairs": pairs.len(), "confidence_percent": fixture["confidence_percent"],
                    "minimum_factor": fixture["minimum_factor"], "seed": 0, "target_ns": 100000, "max_batch": 65536},
                "batch_size": 1, "pairs": pairs,
                "analysis": {"ratio": case["ratio"], "interval": case["interval"], "verdict": case["verdict"]},
            }).to_string();
            assert!(
                coding_adventures_guest_microbenchmark_support::replay_core_report(&core).is_ok(),
                "fixture case {}",
                case["name"]
            );
        }
        assert!(Report::from_json("{").is_err());
    }

    #[test]
    fn collector_uses_guest_programs_and_rejects_clock_failure() {
        let guest = TwigGuestBenchmark::prepare().unwrap();
        let config = Config {
            pairs: 2,
            target_ns: 1,
            max_batch: 1,
            ..Config::default()
        };
        let mut ticks = (0..40).map(|tick| tick * 100);
        let report = guest
            .collect(config.clone(), &mut || ticks.next().unwrap())
            .unwrap();
        assert_eq!(report.pairs.len(), 2);
        assert_eq!(report.batch_size, 1);
        let mut altered: serde_json::Value =
            serde_json::from_str(&report.to_json().unwrap()).unwrap();
        altered["analysis"]["ratio"] = serde_json::json!(123.0);
        assert!(Report::from_json(&altered.to_string()).is_err());
        let mut broken = || -1;
        assert!(guest.collect(config, &mut broken).is_err());
    }

    #[test]
    fn caller_guest_callback_failure_discards_report() {
        let mut baseline = |_entries| Err(Error("guest trapped".to_owned()));
        let mut candidate = |_entries| Ok(GUEST_CHECKSUM);
        let mut clock = || 0;
        assert!(collect_batches(
            Config::default(),
            GUEST_CHECKSUM,
            &mut baseline,
            &mut candidate,
            &mut clock
        )
        .is_err());
    }
}
