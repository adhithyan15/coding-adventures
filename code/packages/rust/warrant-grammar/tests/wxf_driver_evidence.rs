//! WX-F slice 4: driver obligations, tested by *running* what the generator
//! emits. The generated harness (`tests/generated/wxf_driver_evidence.rs`,
//! checked in and drift-tested by `tests/wxf_driver_regen.rs`) is included
//! as a module and run against a mode-switched subject: honest by default,
//! with one faulty mode per obligation the spec says must fail.

use std::collections::BTreeSet;

use coding_adventures_test_bootstrap::run::Options;
use coding_adventures_test_bootstrap::Verdict;

#[allow(dead_code, unused_variables, unused_parens, clippy::all)]
mod harness {
    /// The subject under test, with one behavior mode per obligation under
    /// test. The mode is thread-local: `cargo test` runs tests in parallel.
    pub mod wxf_subject {
        use std::cell::Cell;

        pub const MODE_HONEST: u32 = 0;
        /// A hidden global counter leaks into results: identical drives differ.
        pub const MODE_COUNTER: u32 = 1;
        /// The subject ignores its input entirely.
        pub const MODE_CONSTANT: u32 = 2;
        /// `finish` pads in place through interior mutability.
        pub const MODE_FINISH_MUT: u32 = 3;
        /// `finish` ignores the state.
        pub const MODE_FINISH_CONST: u32 = 4;
        /// Empty drives and empty chunks panic.
        pub const MODE_PANIC_EMPTY: u32 = 5;

        thread_local! {
            pub static MODE: Cell<u32> = const { Cell::new(0) };
            static COUNTER: Cell<u64> = const { Cell::new(0) };
            static LOG: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
        }

        pub fn set_mode(mode: u32) {
            MODE.with(|m| m.set(mode));
            COUNTER.with(|c| c.set(0));
            LOG.with(|l| l.borrow_mut().clear());
        }

        fn log(entry: String) {
            LOG.with(|l| l.borrow_mut().push(entry));
        }

        pub fn take_log() -> Vec<String> {
            LOG.with(|l| l.borrow_mut().split_off(0))
        }

        fn hex(bytes: &[u8]) -> String {
            bytes.iter().map(|b| format!("{b:02x}")).collect()
        }

        fn mix(acc: &[u8], salt: u64) -> [u8; 20] {
            let mut out = [0u8; 20];
            for (i, b) in acc.iter().enumerate() {
                out[i % 20] = out[i % 20].wrapping_add(*b);
            }
            for (i, b) in salt.to_le_bytes().iter().enumerate() {
                out[i % 20] = out[i % 20].wrapping_add(*b);
            }
            out
        }

        #[derive(Debug)]
        pub struct Digest {
            acc: Vec<u8>,
            steps: usize,
            pad: Cell<u8>,
        }

        impl Digest {
            pub fn new() -> Self {
                log("new".to_string());
                Digest {
                    acc: Vec::new(),
                    steps: 0,
                    pad: Cell::new(0),
                }
            }

            pub fn update(&mut self, chunk: &[u8]) {
                if MODE.with(|m| m.get()) == MODE_PANIC_EMPTY && chunk.is_empty() {
                    panic!("empty chunk");
                }
                log(format!("update({})", hex(chunk)));
                self.steps += 1;
                self.acc.extend_from_slice(chunk);
            }

            pub fn sum1(&self) -> [u8; 20] {
                log("finish".to_string());
                match MODE.with(|m| m.get()) {
                    MODE_COUNTER => {
                        let c = COUNTER.with(|c| {
                            let v = c.get();
                            c.set(v + 1);
                            v
                        });
                        mix(&self.acc, c)
                    }
                    MODE_CONSTANT => [0xcc; 20],
                    MODE_FINISH_MUT => {
                        self.pad.set(self.pad.get().wrapping_add(1));
                        mix(&self.acc, self.pad.get() as u64)
                    }
                    MODE_FINISH_CONST => [0xcf; 20],
                    MODE_PANIC_EMPTY if self.steps == 0 => panic!("empty drive"),
                    _ => mix(&self.acc, 0),
                }
            }
        }
    }

    include!("generated/wxf_driver_evidence.rs");
}

use harness::wxf_subject::*;

#[test]
fn honest_subject_passes_every_obligation() {
    set_mode(MODE_HONEST);
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| ()));
    for (name, f, runtimes) in [
        ("shape", harness::obl_digest_chunks_shape as fn(harness::Fault) -> _, 0u64),
        ("empty", harness::obl_digest_chunks_empty, 1),
        ("empty-element", harness::obl_digest_chunks_empty_element, 14),
        ("deterministic", harness::obl_digest_chunks_deterministic, 32),
        (
            "finish-observational",
            harness::obl_digest_chunks_finish_observational,
            16,
        ),
        ("steps-cap", harness::obl_digest_chunks_steps_cap, 2),
    ] {
        let (v, s) = f(harness::Fault::None);
        assert_eq!(v, Verdict::Pass, "{name} failed: {v:?}");
        // `shape` is compile-time (the pins); the rest count their drives.
        assert_eq!(s.cases, runtimes, "{name} ran an unexpected case count");
    }
    std::panic::set_hook(previous);
}

#[test]
fn hidden_counter_fails_deterministic() {
    set_mode(MODE_COUNTER);
    let (v, _) = harness::obl_digest_chunks_deterministic(harness::Fault::None);
    match v {
        Verdict::Fail { failure_id, .. } => assert_eq!(failure_id, "driver-nondeterministic"),
        Verdict::Pass => panic!("nondeterministic subject passed deterministic"),
    }
}

#[test]
fn constant_subject_fails_the_deterministic_control() {
    set_mode(MODE_CONSTANT);
    let (v, _) = harness::obl_digest_chunks_deterministic(harness::Fault::None);
    match v {
        Verdict::Fail { failure_id, .. } => assert_eq!(failure_id, "driver-control-insensitive"),
        Verdict::Pass => panic!("constant subject passed the control"),
    }
}

#[test]
fn mutating_finish_fails_finish_observational() {
    set_mode(MODE_FINISH_MUT);
    let (v, _) = harness::obl_digest_chunks_finish_observational(harness::Fault::None);
    match v {
        Verdict::Fail { failure_id, .. } => assert_eq!(failure_id, "driver-finish-mutating"),
        Verdict::Pass => panic!("mutating finish passed finish-observational"),
    }
}

#[test]
fn constant_finish_fails_the_finish_control() {
    set_mode(MODE_FINISH_CONST);
    let (v, _) = harness::obl_digest_chunks_finish_observational(harness::Fault::None);
    match v {
        Verdict::Fail { failure_id, .. } => assert_eq!(failure_id, "driver-control-dead"),
        Verdict::Pass => panic!("constant finish passed the control"),
    }
}

#[test]
fn empty_drive_panics_fail_empty() {
    set_mode(MODE_PANIC_EMPTY);
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| ()));
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        harness::obl_digest_chunks_empty(harness::Fault::None)
    }));
    std::panic::set_hook(previous);
    assert!(r.is_err(), "panicking subject passed empty");
}

#[test]
fn empty_chunk_panics_fail_empty_element() {
    set_mode(MODE_PANIC_EMPTY);
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| ()));
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        harness::obl_digest_chunks_empty_element(harness::Fault::None)
    }));
    std::panic::set_hook(previous);
    assert!(r.is_err(), "panicking subject passed empty-element");
}

#[test]
fn empty_drive_calls_new_then_finish() {
    set_mode(MODE_HONEST);
    let (v, _) = harness::obl_digest_chunks_empty(harness::Fault::None);
    assert_eq!(v, Verdict::Pass);
    assert_eq!(take_log(), vec!["new".to_string(), "finish".to_string()]);
}

#[test]
fn element_drives_call_in_order_with_exact_bytes() {
    set_mode(MODE_HONEST);
    let (v, _) = harness::obl_digest_chunks_empty_element(harness::Fault::None);
    assert_eq!(v, Verdict::Pass);
    let log = take_log();
    // First drive is [b""]: new, one empty update, finish.
    assert_eq!(&log[..3], ["new", "update()", "finish"]);
    // 2 one-element + 4 two-element + 8 three-element drives.
    let news = log.iter().filter(|e| e.as_str() == "new").count();
    let finishes = log.iter().filter(|e| e.as_str() == "finish").count();
    let updates: Vec<_> = log
        .iter()
        .filter(|e| e.starts_with("update("))
        .collect();
    assert_eq!((news, finishes), (14, 14));
    assert_eq!(updates.len(), 2 + 8 + 24);
    // Every update carries exactly the chunk bytes the drive lists.
    assert!(updates.iter().all(|u| *u == "update()" || *u == "update(00)"));
}

#[test]
fn the_full_generated_suite_passes_and_manifest_names_drivers() {
    set_mode(MODE_HONEST);
    // The steps-cap obligation catches its own refusal panic; silence the
    // hook like `run_main` does.
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| ()));
    let suite = harness::build_suite(harness::Fault::None);
    let mut tap = Vec::new();
    let (summary, outcomes) = suite
        .run_recorded(&Options::default(), &mut tap)
        .expect("run");
    std::panic::set_hook(previous);
    assert!(!summary.bailed);
    let tap = String::from_utf8(tap).unwrap();
    // Slice boundary, pinned: driver faults plan as identity until slice 5,
    // so their negctl entries correctly report not-caught. Nothing else may
    // fail; slice 5 flips this assertion to all-caught.
    const DRIVER_FAULTS: [&str; 6] = [
        "drop-last-chunk",
        "reorder-chunks",
        "duplicate-last-chunk",
        "early-finish",
        "reset-per-step",
        "drop-last-output",
    ];
    let not_ok: Vec<_> = tap
        .lines()
        .filter(|l| l.starts_with("not ok"))
        .collect();
    assert!(!not_ok.is_empty(), "expected driver-fault negctl entries");
    for l in &not_ok {
        assert!(l.contains("/negctl/"), "non-negctl failure: {l}");
        assert!(
            DRIVER_FAULTS.iter().any(|f| l.contains(f)),
            "unexpected failure: {l}"
        );
    }
    let negctl_driver_total = outcomes
        .iter()
        .filter(|o| {
            o.id.contains("/negctl/") && DRIVER_FAULTS.iter().any(|f| o.id.contains(f))
        })
        .count();
    assert_eq!(
        not_ok.len(),
        negctl_driver_total,
        "some driver-fault negctl unexpectedly passed"
    );
    let driver: Vec<_> = outcomes
        .iter()
        .filter(|o| o.id.contains("/driver/"))
        .collect();
    assert_eq!(driver.len(), 6, "six obligations run");
    assert!(driver.iter().all(|o| o.passed()));

    let out = std::env::temp_dir().join(format!("wxf_manifest_{}.json", std::process::id()));
    std::env::set_var("WARRANT_EVIDENCE_OUT", &out);
    harness::write_manifest_for(&outcomes);
    std::env::remove_var("WARRANT_EVIDENCE_OUT");
    let manifest = std::fs::read_to_string(&out).expect("manifest written");
    let _ = std::fs::remove_file(&out);
    let flat = manifest.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flat.contains("\"warrant/wxf_driver/driver/digest_chunks/empty\""),
        "{manifest}"
    );
    assert!(flat.contains("\"kind\": \"driver\""), "{manifest}");
    assert!(flat.contains("\"steps_max_observed\":"), "{manifest}");
    assert!(flat.contains("\"obligations\":"), "{manifest}");
    let kinds: BTreeSet<&str> = driver
        .iter()
        .map(|o| o.id.rsplit('/').next().unwrap())
        .collect();
    assert_eq!(
        kinds,
        BTreeSet::from([
            "shape",
            "empty",
            "empty-element",
            "deterministic",
            "finish-observational",
            "steps-cap"
        ]),
        "{kinds:?}"
    );
}
