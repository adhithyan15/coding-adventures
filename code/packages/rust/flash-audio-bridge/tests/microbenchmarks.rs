//! # Microbenchmark Suite and Zero-Allocation Verification for `flash-audio-bridge` (`VER07`)
//!
//! Validates via `test-bootstrap`'s counting global allocator:
//! - Zero heap allocation in `decode_pcm16_le_into` and `decode_adpcm_into`,
//! - Zero heap allocation in `Mp3FrameIter`,
//! - Zero heap allocation in `render_flash_audio_into` and `bridge_render_to_ring_buffer`,
//! - Sabotage detection: an intentional allocation is caught and rejected.

#![deny(unsafe_code)]

use std::hint::black_box;
use std::time::Duration;

use coding_adventures_test_bootstrap::bench::{
    AllocCounter, BenchmarkConfig, BenchmarkHarness, BenchmarkResult, Budget, BudgetVerdict,
};
use coding_adventures_test_bootstrap::counting_alloc;
use coding_adventures_test_bootstrap::manifest::{self, Json};
use coding_adventures_test_bootstrap::run::{self, Ctx, Outcome, Suite};
use coding_adventures_test_bootstrap::Verdict;

use flash_audio_bridge::{
    bridge_render_to_ring_buffer, decode_adpcm_into, decode_pcm16_le_into, render_flash_audio_into,
    FlashAudioState, Mp3FrameIter, PcmRingBuffer, SwfAudioChannels,
};

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

fn check_decode_pcm16_zero_alloc(_ctx: &Ctx) -> Verdict {
    let payload = [0x34u8; 64]; // 64 bytes -> 32 samples
    let mut out = [0i16; 32];

    bench_zero_alloc("decode_pcm16_le_zero_alloc", || {
        let n =
            decode_pcm16_le_into(black_box(&payload), black_box(&mut out)).expect("decode pcm16");
        black_box(n);
    })
}

fn check_decode_adpcm_zero_alloc(_ctx: &Ctx) -> Verdict {
    // 2-bit header (0b01 << 6 = 4-bit code size) + InitialSample (0) + InitialIndex (10)
    // Total bits for 16 mono 4-bit samples: 2 + 16 + 6 + 15*4 = 84 bits = 11 bytes.
    let adpcm_bytes = [
        0x40, 0x00, 0x0A, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0,
    ];
    let mut out = [0i16; 16];

    bench_zero_alloc("decode_adpcm_zero_alloc", || {
        let n = decode_adpcm_into(
            black_box(&adpcm_bytes),
            SwfAudioChannels::Mono,
            16,
            black_box(&mut out),
        )
        .expect("decode adpcm");
        black_box(n);
    })
}

fn check_mp3_frame_iter_zero_alloc(_ctx: &Ctx) -> Verdict {
    // 1 MPEG-1 Layer III 128 kbps 44.1kHz frame = 417 bytes
    let mut frame_buf = [0u8; 417];
    frame_buf[0] = 0xFF;
    frame_buf[1] = 0xFB;
    frame_buf[2] = 0x90;
    frame_buf[3] = 0x00;

    bench_zero_alloc("mp3_frame_iter_zero_alloc", || {
        let mut count = 0usize;
        for f in Mp3FrameIter::mp3_iter_from_bytes(black_box(&frame_buf)) {
            let frame = f.expect("valid mp3 frame");
            count += frame.bytes.len();
        }
        black_box(count);
    })
}

fn check_render_flash_audio_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut state = FlashAudioState::create_engine(44_100, 24).expect("engine");
    let src = [1500i16; 256];
    state
        .register_pcm_sound(1, 44_100, SwfAudioChannels::Mono, &src, None)
        .expect("register");
    let stream_chunk = [250i16; 64];
    let mut out = [0i16; 64];

    bench_zero_alloc("render_flash_audio_zero_alloc", || {
        if state.active_voice_count() == 0 {
            let _ = state.as_sound_start(1, 0, 1);
        }
        if state.stream_fifo_len() < 64 {
            let _ = state.push_stream_pcm_into_fifo(black_box(&stream_chunk));
        }
        let n = render_flash_audio_into(&mut state, black_box(&mut out)).expect("render");
        black_box(n);
    })
}

fn check_ring_buffer_bridge_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut state = FlashAudioState::create_engine(44_100, 24).expect("engine");
    let src = [1000i16; 128];
    state
        .register_pcm_sound(1, 44_100, SwfAudioChannels::Mono, &src, None)
        .expect("register");
    let mut ring: PcmRingBuffer<128> = PcmRingBuffer::empty();

    bench_zero_alloc("ring_buffer_bridge_zero_alloc", || {
        if state.active_voice_count() == 0 {
            let _ = state.as_sound_start(1, 0, 1);
        }
        ring.clear();
        let n = bridge_render_to_ring_buffer(&mut state, &mut ring, 32).expect("bridge");
        black_box(n);
    })
}

fn check_sabotage_allocation_detected(_ctx: &Ctx) -> Verdict {
    let harness = BenchmarkHarness::new(fast_bench_config(5));
    let budget = zero_budget();
    let counter = AllocCounter::new();
    let leaky = harness.run_with_counter("sabotage_alloc", &budget, &counter, || {
        black_box(Vec::<u8>::with_capacity(1))
    });
    if !matches!(
        leaky.budget_verdict,
        BudgetVerdict::Failed {
            observed: 1,
            limit: 0,
            ..
        }
    ) {
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
        ("module", Json::str("flash_audio_bridge")),
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
    suite.add(
        "bench/pcm/decode_pcm16_zero_alloc",
        check_decode_pcm16_zero_alloc,
    );
    suite.add(
        "bench/adpcm/decode_adpcm_4bit_zero_alloc",
        check_decode_adpcm_zero_alloc,
    );
    suite.add(
        "bench/mp3/frame_iter_zero_alloc",
        check_mp3_frame_iter_zero_alloc,
    );
    suite.add(
        "bench/mixer/render_flash_audio_zero_alloc",
        check_render_flash_audio_zero_alloc,
    );
    suite.add(
        "bench/bridge/ring_buffer_bridge_zero_alloc",
        check_ring_buffer_bridge_zero_alloc,
    );
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
