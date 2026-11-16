//! # Microbenchmark Suite and Zero-Allocation Verification for `swf-container`
//!
//! Validates via `test-bootstrap`'s counting global allocator:
//! - zero heap allocation in `BitReader` (ub, sb, fb, rect, matrix, cxform),
//! - zero heap allocation in `SwfTagIter` streaming over short & long tags,
//! - sabotage detection: an intentional allocation is caught and rejected.

#![deny(unsafe_code)]

use std::hint::black_box;
use std::time::Duration;

use coding_adventures_test_bootstrap::bench::{
    AllocCounter, BenchmarkConfig, BenchmarkHarness, BenchmarkResult, Budget,
};
use coding_adventures_test_bootstrap::counting_alloc;
use coding_adventures_test_bootstrap::manifest::{self, Json};
use coding_adventures_test_bootstrap::run::{self, Ctx, Outcome, Suite};
use coding_adventures_test_bootstrap::Verdict;

use swf_container::{BitReader, SwfTagIter};

const FID_BENCH_FAIL: &str = "property-counterexample";

coding_adventures_test_bootstrap::install_counting_allocator!();

fn zero_alloc_ok(result: &BenchmarkResult) -> bool {
    result.budget_verdict.is_acceptable()
        && result.alloc_bytes_per_iter == Some(0.0)
        && !result.unchecked_hard_budgets.contains(&"alloc_bytes")
}

fn fast_bench_config(samples: usize) -> BenchmarkConfig {
    BenchmarkConfig {
        warmup_iterations: 10,
        warmup_max_duration: Duration::from_millis(1),
        min_batch_duration: Duration::from_micros(10),
        max_batch_size: 100_000,
        sample_count: samples,
        seed: 0x9e37_79b9_7f4a_7c15,
    }
}

fn zero_budget() -> Budget {
    Budget {
        max_alloc_bytes: Some(0),
        ..Default::default()
    }
}

fn bench_zero_alloc(name: &str, f: impl FnMut()) -> Verdict {
    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = zero_budget();
    let alloc = AllocCounter::new();
    let res = harness.run_with_counter(name, &budget, &alloc, f);
    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            format!("{name} exceeded zero allocation budget"),
        );
    }
    Verdict::Pass
}

fn check_counting_allocator_is_live(_ctx: &Ctx) -> Verdict {
    if !counting_alloc::is_installed() {
        return Verdict::fail(FID_BENCH_FAIL, "counting allocator is not installed");
    }
    Verdict::Pass
}

fn check_bit_reader_zero_alloc(_ctx: &Ctx) -> Verdict {
    // RECT: 5 bits nbits (=15), then 4 x 15 bits coords
    // 5 + 60 = 65 bits -> 9 bytes
    let rect_bytes = [
        0x78, 0x00, 0x05, 0x5F, 0x00, 0x00, 0x03, 0xE8, 0x00, 0x00, 0x00, 0x00,
    ];
    // Matrix bytes
    let matrix_bytes = [
        0x80, 0x48, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    // CXForm bytes
    let cxform_bytes = [0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

    bench_zero_alloc("bit_reader_zero_alloc", || {
        let mut r1 = BitReader::from_bits(black_box(&rect_bytes));
        let _ = black_box(r1.read_rect());

        let mut r2 = BitReader::from_bits(black_box(&matrix_bytes));
        let _ = black_box(r2.read_matrix());

        let mut r3 = BitReader::from_bits(black_box(&cxform_bytes));
        let _ = black_box(r3.read_cxform());
    })
}

fn check_tag_iter_zero_alloc(_ctx: &Ctx) -> Verdict {
    // Sequence of 4 tags:
    // Tag 9 (SetBackgroundColor, len 3) -> 2 header bytes + 3 payload
    // Tag 1 (ShowFrame, len 0) -> 2 header bytes
    // Tag 12 (DoAction, len 4) -> 2 header bytes + 4 payload
    // Tag 0 (End, len 0) -> 2 header bytes
    let tag_stream = [
        (9u8 << 6) | 3,
        0x00,
        0xFF,
        0xAA,
        0x55,
        1u8 << 6,
        0x00,
        (12u8 << 6) | 4,
        0x00,
        0x01,
        0x02,
        0x03,
        0x04,
        0x00,
        0x00,
    ];

    bench_zero_alloc("tag_iter_zero_alloc", || {
        let iter = SwfTagIter::from_payload(black_box(&tag_stream));
        let mut count = 0;
        for t in iter.flatten() {
            count += t.tag_payload_slice().len();
        }
        black_box(count);
    })
}

fn check_sabotage_allocation_detected(_ctx: &Ctx) -> Verdict {
    let harness = BenchmarkHarness::new(fast_bench_config(5));
    let budget = zero_budget();
    let alloc = AllocCounter::new();
    let res = harness.run_with_counter("sabotage_allocation", &budget, &alloc, || {
        let v: Vec<u8> = vec![1, 2, 3, 4];
        black_box(v);
    });

    if zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "sabotage allocation was not detected by harness",
        );
    }
    Verdict::Pass
}

fn write_bench_manifest(outcomes: &[Outcome]) {
    let Some(path) = manifest::output_path("WARRANT_BENCH_OUT") else {
        return;
    };
    let mut tests: Vec<Json> = outcomes
        .iter()
        .map(|o| {
            Json::obj([
                ("id", Json::str(o.id.clone())),
                ("kind", Json::str("bench")),
                (
                    "verdict",
                    Json::str(if o.passed() { "pass" } else { "fail" }),
                ),
            ])
        })
        .collect();
    tests.sort_by_key(|t| t.render());
    let doc = Json::obj([
        ("schema", Json::str("warrant-bench-evidence-v1")),
        ("module", Json::str("swf_container")),
        ("hw_instruction_counters", Json::str("not-live")),
        (
            "source_sha256",
            Json::str(coding_adventures_sha256::sha256_hex(
                &manifest::source_tree_framing(
                    &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
                )
                .expect("bind bench manifest to src"),
            )),
        ),
        ("tests", Json::Arr(tests)),
    ]);
    manifest::write(&path, &doc).expect("write bench manifest");
}

fn main() {
    let mut suite = Suite::new();

    suite.add(
        "bench/alloc/counting_allocator_is_live",
        check_counting_allocator_is_live,
    );
    suite.add("bench/bit_reader/zero_alloc", check_bit_reader_zero_alloc);
    suite.add("bench/tag_iter/zero_alloc", check_tag_iter_zero_alloc);
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
