//! Host-clocked measurement of bounded guest functions run by the WASM interpreter.
//!
//! The module is encoded, parsed, validated, and instantiated before timing.
//! Timed batches call its exported functions through `WasmRuntime::call`, so
//! the observation includes host dispatch plus interpreter execution. It is
//! neither native WASM engine throughput nor simulated processor cycles.

use coding_adventures_microbenchmark::{collect, Batch, Clock, Config, Error, Report};
use wasm_module_encoder::encode_module;
use wasm_runtime::{WasmInstance, WasmRuntime};
use wasm_types::{Export, ExternalKind, FuncType, FunctionBody, ValueType, WasmModule};

/// Sum of the integers from one through the guest's fixed upper bound (32).
pub const GUEST_CHECKSUM: i64 = 528;
/// Each logical operation invokes one guest sum function.
pub const WORKLOAD_ID: &str = "wasm-recursive-and-loop-sum-1-through-32";

/// Prepared interpreter and instances for recursion, loop, and empty entry.
pub struct WasmGuestBenchmark {
    runtime: WasmRuntime,
    recursive: WasmInstance,
    looped: WasmInstance,
    empty: WasmInstance,
}

impl WasmGuestBenchmark {
    /// Encode, load, validate, instantiate, and correctness-check the module.
    pub fn prepare() -> Result<Self, Error> {
        let bytes = guest_module_bytes()?;
        let runtime = WasmRuntime::new();
        let module = runtime.load(&bytes).map_err(Error)?;
        let validated = runtime
            .validate(&module)
            .map_err(|e| Error(e.to_string()))?;
        let mut recursive = runtime
            .instantiate(&validated)
            .map_err(|e| Error(e.to_string()))?;
        let mut looped = runtime
            .instantiate(&validated)
            .map_err(|e| Error(e.to_string()))?;
        let mut empty = runtime
            .instantiate(&validated)
            .map_err(|e| Error(e.to_string()))?;
        verify_result(
            &runtime,
            &mut recursive,
            "sum_recursive",
            &[32],
            GUEST_CHECKSUM,
        )?;
        verify_result(&runtime, &mut looped, "sum_loop", &[32], GUEST_CHECKSUM)?;
        verify_result(&runtime, &mut empty, "empty", &[], 0)?;
        Ok(Self {
            runtime,
            recursive,
            looped,
            empty,
        })
    }

    /// Run `entries` recursive guest calls and sum the returned checksums.
    pub fn recursive_batch(&mut self, entries: usize) -> Result<i64, Error> {
        run_batch(&self.runtime, &mut self.recursive, "sum_recursive", entries)
    }

    /// Run `entries` loop guest calls and sum the returned checksums.
    pub fn loop_batch(&mut self, entries: usize) -> Result<i64, Error> {
        run_batch(&self.runtime, &mut self.looped, "sum_loop", entries)
    }

    /// Run `entries` empty guest calls; each must return zero.
    pub fn empty_batch(&mut self, entries: usize) -> Result<i64, Error> {
        run_batch(&self.runtime, &mut self.empty, "empty", entries)
    }

    /// Collect paired raw observations with a caller-supplied monotonic host clock.
    pub fn collect(&mut self, config: Config, clock: &mut Clock<'_>) -> Result<Report, Error> {
        let mut baseline =
            |entries| run_batch(&self.runtime, &mut self.recursive, "sum_recursive", entries);
        let mut candidate =
            |entries| run_batch(&self.runtime, &mut self.looped, "sum_loop", entries);
        collect(config, GUEST_CHECKSUM, &mut baseline, &mut candidate, clock)
    }
}

fn verify_result(
    runtime: &WasmRuntime,
    instance: &mut WasmInstance,
    name: &str,
    args: &[i64],
    expected: i64,
) -> Result<(), Error> {
    let values = runtime
        .call(instance, name, args)
        .map_err(|e| Error(e.to_string()))?;
    if values.as_slice() != [expected] {
        return Err(Error(format!(
            "WASM guest {name} returned {values:?}, want [{expected}]"
        )));
    }
    Ok(())
}

fn run_batch(
    runtime: &WasmRuntime,
    instance: &mut WasmInstance,
    name: &str,
    entries: usize,
) -> Result<i64, Error> {
    if entries == 0 || entries as u128 > i64::MAX as u128 {
        return Err(Error(
            "guest entry count must be a positive signed 64-bit value".to_owned(),
        ));
    }
    let operation_count = i64::try_from(entries)
        .map_err(|_| Error("guest entry count exceeds signed 64-bit range".to_owned()))?;
    if name != "empty" {
        GUEST_CHECKSUM
            .checked_mul(operation_count)
            .ok_or_else(|| Error("guest batch checksum overflow".to_owned()))?;
    }
    let mut checksum = 0_i64;
    for _ in 0..entries {
        let values = runtime
            .call(instance, name, if name == "empty" { &[] } else { &[32] })
            .map_err(|e| Error(e.to_string()))?;
        let value = *values
            .first()
            .ok_or_else(|| Error(format!("WASM guest {name} returned no value")))?;
        if values.len() != 1 {
            return Err(Error(format!(
                "WASM guest {name} returned {} values",
                values.len()
            )));
        }
        checksum = checksum
            .checked_add(value)
            .ok_or_else(|| Error("guest batch checksum overflow".to_owned()))?;
    }
    Ok(checksum)
}

fn guest_module_bytes() -> Result<Vec<u8>, Error> {
    // These instruction streams are WebAssembly 1.0 guest code. The recursive
    // variant uses a recursive call; the candidate uses a guest-side loop.
    let recursive = vec![
        0x20, 0x00, // local.get 0
        0x45, // i32.eqz
        0x04, 0x7f, // if (result i32)
        0x41, 0x00, //   i32.const 0
        0x05, // else
        0x20, 0x00, //   local.get 0
        0x20, 0x00, 0x41, 0x01, 0x6b, // local.get 0; i32.const 1; i32.sub
        0x10, 0x00, //   call 0
        0x6a, //   i32.add
        0x0b, 0x0b, // end if; end function
    ];
    let looped = vec![
        0x41, 0x00, 0x21, 0x01, // local.set 1 = 0 (remaining)
        0x20, 0x00, 0x21, 0x01, // local.set 1 = parameter
        0x41, 0x00, 0x21, 0x02, // local.set 2 = sum 0
        0x02, 0x40, // block
        0x03, 0x40, //   loop
        0x20, 0x01, 0x45, 0x0d, 0x01, // br_if 1 when remaining is zero
        0x20, 0x02, 0x20, 0x01, 0x6a, 0x21, 0x02, // sum += remaining
        0x20, 0x01, 0x41, 0x01, 0x6b, 0x21, 0x01, // remaining -= 1
        0x0c, 0x00, // br loop
        0x0b, 0x0b, // end loop; end block
        0x20, 0x02, 0x0b, // local.get sum; end function
    ];
    let module = WasmModule {
        types: vec![
            FuncType {
                params: vec![ValueType::I32],
                results: vec![ValueType::I32],
            },
            FuncType {
                params: vec![],
                results: vec![ValueType::I32],
            },
        ],
        functions: vec![0, 0, 1],
        exports: vec![
            Export {
                name: "sum_recursive".into(),
                kind: ExternalKind::Function,
                index: 0,
            },
            Export {
                name: "sum_loop".into(),
                kind: ExternalKind::Function,
                index: 1,
            },
            Export {
                name: "empty".into(),
                kind: ExternalKind::Function,
                index: 2,
            },
        ],
        code: vec![
            FunctionBody {
                locals: vec![],
                code: recursive,
            },
            FunctionBody {
                locals: vec![ValueType::I32, ValueType::I32],
                code: looped,
            },
            FunctionBody {
                locals: vec![],
                code: vec![0x41, 0x00, 0x0b],
            },
        ],
        ..Default::default()
    };
    // Runtime setup and validation happen in prepare(), before any clock reads.
    encode_module(&module).map_err(|e| Error(e.to_string()))
}

/// Default VER07 paired collection settings.
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
    fn prepared_guest_variants_have_matching_checksums() {
        let mut guest = WasmGuestBenchmark::prepare().unwrap();
        for entries in 1..=100 {
            assert_eq!(
                guest.recursive_batch(entries).unwrap(),
                GUEST_CHECKSUM * entries as i64
            );
            assert_eq!(
                guest.loop_batch(entries).unwrap(),
                GUEST_CHECKSUM * entries as i64
            );
        }
        assert!(guest.recursive_batch(0).is_err());
        assert!(guest.loop_batch(usize::MAX).is_err());
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
        let wat = include_str!("../guest/sum.wat");
        assert!(wat.contains("sum_recursive"));
        assert!(wat.contains("sum_loop"));
    }

    #[test]
    fn collector_records_real_guest_calls_and_clock_errors() {
        let mut guest = WasmGuestBenchmark::prepare().unwrap();
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
        assert!(guest.collect(config, &mut || -1).is_err());
        assert!(Report::from_json("{").is_err());
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
