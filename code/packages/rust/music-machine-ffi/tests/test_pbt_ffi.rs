//! Property-based testing (PBT) and randomized stress testing for `music-machine-ffi`
//! using deterministic SplitMix64 PRNG and bit/byte mutation fuzzers.

use coding_adventures_test_bootstrap::fuzz::{bit_flip, byte_flip};
use coding_adventures_test_bootstrap::prng::SplitMix64;
use music_machine_core::{MAX_SAMPLE_COUNT, MAX_SCORE_BYTES};
use music_machine_ffi::{
    mm_free_buffer, mm_render_portable_json_to_pcm16, mm_render_portable_text_to_pcm16,
    mm_render_score_text_to_pcm16, MM_STATUS_BUDGET_EXCEEDED, MM_STATUS_INVALID_ARGUMENT,
    MM_STATUS_NULL_POINTER, MM_STATUS_OK, MM_STATUS_PANIC,
};

const VALID_V2_TEXT: &str = "\
format: music-machine-score/v2
title: PBT Duet
ppq: 120
sample_rate: 22050
tempo 0 120
meter 0 4/4

instrument flute profile=flute_naive gain=0.8
track lead instrument=flute
event lead 0 120 note C4 velocity=0.7
";

const VALID_V2_JSON: &str = r#"{
  "format_version": "music-machine-score/v2",
  "title": "PBT JSON",
  "ppq": 120,
  "sample_rate_hz": 22050,
  "tempo_events": [{"start_tick": 0, "bpm": 120.0}],
  "meter_events": [{"start_tick": 0, "meter": "4/4"}],
  "instruments": [{"id": "lead", "kind": "preset", "profile_id": "flute_naive", "gain": 0.8}],
  "tracks": [{"id": "mel", "instrument_id": "lead"}],
  "events": [{"track_id": "mel", "start_tick": 0, "duration_tick": 120, "kind": "note", "notes": ["C4"], "velocity": 0.7, "source_order": 0}]
}"#;

#[test]
fn test_pbt_ffi_arbitrary_bytes_and_interior_nul_never_panic() {
    let mut rng = SplitMix64::new(0x5150_FF10_2026_0001);

    for i in 0..800 {
        let len = (rng.next_u64() % 256) as usize;
        let mut bytes = Vec::with_capacity(len);
        for _ in 0..len {
            bytes.push((rng.next_u64() % 256) as u8);
        }
        // Explicitly inject interior NUL on every 4th iteration when non-empty.
        if !bytes.is_empty() && i % 4 == 0 {
            let pos = (rng.next_u64() as usize) % bytes.len();
            bytes[pos] = 0;
        }

        let rate = match i % 5 {
            0 => 0,
            1 => 22050,
            2 => 500,
            3 => 250_000,
            _ => (rng.next_u64() % 300_000) as u32,
        };
        let max_s = match i % 4 {
            0 => 0,
            1 => 10_000,
            2 => MAX_SAMPLE_COUNT + 1,
            _ => (rng.next_u64() % 64_000) as usize,
        };

        let mut out_ptr: *mut i16 = std::ptr::null_mut();
        let mut out_len: usize = 123;
        let mut out_clipped: usize = 456;

        // 1. Beginner score text entry point
        let status1 = unsafe {
            mm_render_score_text_to_pcm16(
                bytes.as_ptr(),
                bytes.len(),
                rate,
                &mut out_ptr,
                &mut out_len,
                &mut out_clipped,
            )
        };
        assert_ne!(status1, MM_STATUS_PANIC);
        if status1 == MM_STATUS_OK {
            unsafe { mm_free_buffer(out_ptr, out_len) };
        } else {
            assert!(out_ptr.is_null());
            assert_eq!(out_len, 0);
            assert_eq!(out_clipped, 0);
        }
        if bytes.contains(&0) && !bytes.is_empty() && (rate == 0 || (1000..=192_000).contains(&rate)) {
            assert_eq!(status1, MM_STATUS_INVALID_ARGUMENT);
        }

        // 2. Portable text entry point
        let status2 = unsafe {
            mm_render_portable_text_to_pcm16(
                bytes.as_ptr(),
                bytes.len(),
                max_s,
                &mut out_ptr,
                &mut out_len,
                &mut out_clipped,
            )
        };
        assert_ne!(status2, MM_STATUS_PANIC);
        if status2 == MM_STATUS_OK {
            unsafe { mm_free_buffer(out_ptr, out_len) };
        } else {
            assert!(out_ptr.is_null());
            assert_eq!(out_len, 0);
            assert_eq!(out_clipped, 0);
        }

        // 3. Portable JSON entry point
        let status3 = unsafe {
            mm_render_portable_json_to_pcm16(
                bytes.as_ptr(),
                bytes.len(),
                max_s,
                &mut out_ptr,
                &mut out_len,
                &mut out_clipped,
            )
        };
        assert_ne!(status3, MM_STATUS_PANIC);
        if status3 == MM_STATUS_OK {
            unsafe { mm_free_buffer(out_ptr, out_len) };
        } else {
            assert!(out_ptr.is_null());
            assert_eq!(out_len, 0);
            assert_eq!(out_clipped, 0);
        }
    }
}

#[test]
fn test_pbt_ffi_mutated_corpora_never_panic() {
    let mut rng = SplitMix64::new(0xCAFE_BABE_FF10_1337);
    let base_v1 = b"title: PBT V1\ntempo: 140\nsample_rate: 22050\nC4/q D4/e E4/e\n";
    let base_v2_txt = VALID_V2_TEXT.as_bytes();
    let base_v2_json = VALID_V2_JSON.as_bytes();

    for i in 0..300 {
        let mut out_ptr: *mut i16 = std::ptr::null_mut();
        let mut out_len: usize = 0;
        let mut out_clipped: usize = 0;

        // Mutate V1 score
        let bit_off = (rng.next_u64() as usize) % (base_v1.len() * 8);
        let cnt = ((rng.next_u64() % 12) + 1) as usize;
        let m1 = if i % 2 == 0 {
            bit_flip(base_v1, bit_off, cnt)
        } else {
            byte_flip(base_v1, (rng.next_u64() as usize) % base_v1.len(), 2)
        };
        let s1 = unsafe {
            mm_render_score_text_to_pcm16(
                m1.as_ptr(),
                m1.len(),
                0,
                &mut out_ptr,
                &mut out_len,
                &mut out_clipped,
            )
        };
        assert_ne!(s1, MM_STATUS_PANIC);
        if s1 == MM_STATUS_OK {
            if out_len > 0 {
                assert!(!out_ptr.is_null());
                let _ = unsafe { std::slice::from_raw_parts(out_ptr, out_len) };
            }
            unsafe { mm_free_buffer(out_ptr, out_len) };
        } else {
            assert!(out_ptr.is_null());
            assert_eq!(out_len, 0);
        }

        // Mutate V2 portable text
        let bit_off2 = (rng.next_u64() as usize) % (base_v2_txt.len() * 8);
        let m2 = byte_flip(base_v2_txt, bit_off2 / 8, 2);
        let s2 = unsafe {
            mm_render_portable_text_to_pcm16(
                m2.as_ptr(),
                m2.len(),
                500_000,
                &mut out_ptr,
                &mut out_len,
                &mut out_clipped,
            )
        };
        assert_ne!(s2, MM_STATUS_PANIC);
        if s2 == MM_STATUS_OK {
            unsafe { mm_free_buffer(out_ptr, out_len) };
        } else {
            assert!(out_ptr.is_null());
            assert_eq!(out_len, 0);
        }

        // Mutate V2 portable JSON
        let bit_off3 = (rng.next_u64() as usize) % (base_v2_json.len() * 8);
        let m3 = byte_flip(base_v2_json, bit_off3 / 8, 2);
        let s3 = unsafe {
            mm_render_portable_json_to_pcm16(
                m3.as_ptr(),
                m3.len(),
                500_000,
                &mut out_ptr,
                &mut out_len,
                &mut out_clipped,
            )
        };
        assert_ne!(s3, MM_STATUS_PANIC);
        if s3 == MM_STATUS_OK {
            unsafe { mm_free_buffer(out_ptr, out_len) };
        } else {
            assert!(out_ptr.is_null());
            assert_eq!(out_len, 0);
        }
    }
}

#[test]
fn test_pbt_ffi_null_oversized_and_aliased_pointer_fuzz() {
    let mut rng = SplitMix64::new(0xDEAD_BEEF_9988_7766);
    let sample = b"C4/q";

    for i in 0..256 {
        let mut out_ptr: *mut i16 = 0x1234usize as *mut i16;
        let mut out_len: usize = 999;
        let mut out_clipped: usize = 777;

        let null_mask = (rng.next_u64() % 15) + 1;
        let in_ptr = if null_mask & 1 != 0 {
            std::ptr::null()
        } else {
            sample.as_ptr()
        };
        let p_out = if null_mask & 2 != 0 {
            std::ptr::null_mut()
        } else {
            &mut out_ptr as *mut *mut i16
        };
        let l_out = if null_mask & 4 != 0 {
            std::ptr::null_mut()
        } else {
            &mut out_len as *mut usize
        };
        let c_out = if null_mask & 8 != 0 {
            std::ptr::null_mut()
        } else {
            &mut out_clipped as *mut usize
        };

        let status = match i % 3 {
            0 => unsafe {
                mm_render_score_text_to_pcm16(in_ptr, sample.len(), 44100, p_out, l_out, c_out)
            },
            1 => unsafe {
                mm_render_portable_json_to_pcm16(in_ptr, sample.len(), 0, p_out, l_out, c_out)
            },
            _ => unsafe {
                mm_render_portable_text_to_pcm16(in_ptr, sample.len(), 0, p_out, l_out, c_out)
            },
        };
        assert_eq!(status, MM_STATUS_NULL_POINTER);

        // Oversized input length must be rejected before reading from a dummy pointer.
        let dummy = b"X";
        let oversized_len = MAX_SCORE_BYTES + 1 + (rng.next_u64() % 1_000_000) as usize;
        let status_over = unsafe {
            mm_render_score_text_to_pcm16(
                dummy.as_ptr(),
                oversized_len,
                44100,
                &mut out_ptr,
                &mut out_len,
                &mut out_clipped,
            )
        };
        assert_eq!(status_over, MM_STATUS_BUDGET_EXCEEDED);
        assert!(out_ptr.is_null());
        assert_eq!(out_len, 0);
        assert_eq!(out_clipped, 0);

        // Oversized max_samples must be rejected before parsing.
        let oversized_samples = MAX_SAMPLE_COUNT + 1 + (rng.next_u64() as usize % 1_000_000);
        let status_max = unsafe {
            mm_render_portable_text_to_pcm16(
                dummy.as_ptr(),
                dummy.len(),
                oversized_samples,
                &mut out_ptr,
                &mut out_len,
                &mut out_clipped,
            )
        };
        assert_eq!(status_max, MM_STATUS_BUDGET_EXCEEDED);
        assert!(out_ptr.is_null());

        // Aliased output pointers must be rejected.
        let status_alias = unsafe {
            mm_render_portable_json_to_pcm16(
                dummy.as_ptr(),
                dummy.len(),
                0,
                &mut out_ptr,
                &mut out_len,
                &mut out_len,
            )
        };
        assert_eq!(status_alias, MM_STATUS_INVALID_ARGUMENT);
    }
}

#[test]
fn test_pbt_ffi_valid_determinism_and_interleaved_alloc_free() {
    let mut rng = SplitMix64::new(0xA110_CA7E_F00D_2026);
    let notes = ["C4", "D4", "E4", "G4", "A4", "C5"];
    let durs = ["q", "e", "s"];
    let rates = [11025u32, 22050, 44100];

    struct LiveAlloc {
        ptr: *mut i16,
        len: usize,
        expected: Vec<i16>,
    }

    let mut pool: Vec<LiveAlloc> = Vec::new();

    for iter in 0..48 {
        let count = (rng.next_u64() % 5 + 1) as usize;
        let rate = rates[(rng.next_u64() as usize) % rates.len()];
        let mut score = format!("title: Interleave {iter}\ntempo: 160\nsample_rate: {rate}\n");
        for _ in 0..count {
            let n = notes[(rng.next_u64() as usize) % notes.len()];
            let d = durs[(rng.next_u64() as usize) % durs.len()];
            score.push_str(&format!("{n}/{d} "));
        }

        let mut ptr1: *mut i16 = std::ptr::null_mut();
        let mut len1: usize = 0;
        let mut clip1: usize = 0;
        let status1 = unsafe {
            mm_render_score_text_to_pcm16(
                score.as_ptr(),
                score.len(),
                0,
                &mut ptr1,
                &mut len1,
                &mut clip1,
            )
        };
        assert_eq!(status1, MM_STATUS_OK);
        assert!(!ptr1.is_null());
        assert!(len1 > 0);
        let expected = unsafe { std::slice::from_raw_parts(ptr1, len1) }.to_vec();

        // Determinism check: second call produces identical samples.
        let mut ptr2: *mut i16 = std::ptr::null_mut();
        let mut len2: usize = 0;
        let mut clip2: usize = 0;
        let status2 = unsafe {
            mm_render_score_text_to_pcm16(
                score.as_ptr(),
                score.len(),
                rate,
                &mut ptr2,
                &mut len2,
                &mut clip2,
            )
        };
        assert_eq!(status2, MM_STATUS_OK);
        assert_eq!(len2, len1);
        assert_eq!(clip2, clip1);
        let second = unsafe { std::slice::from_raw_parts(ptr2, len2) };
        assert_eq!(second, expected.as_slice());
        unsafe { mm_free_buffer(ptr2, len2) };

        pool.push(LiveAlloc {
            ptr: ptr1,
            len: len1,
            expected,
        });

        // Randomly free one earlier allocation while others stay live, interleaved with no-ops.
        if pool.len() > 6 && rng.next_u64().is_multiple_of(2) {
            let idx = (rng.next_u64() as usize) % pool.len();
            let item = pool.swap_remove(idx);
            let observed = unsafe { std::slice::from_raw_parts(item.ptr, item.len) };
            assert_eq!(observed, item.expected.as_slice());
            unsafe {
                mm_free_buffer(std::ptr::null_mut(), (rng.next_u64() % 4096) as usize);
                mm_free_buffer(item.ptr, 0);
                mm_free_buffer(item.ptr, item.len);
            }
        }
    }

    // Drain remaining pool in shuffled order.
    while !pool.is_empty() {
        let idx = (rng.next_u64() as usize) % pool.len();
        let item = pool.swap_remove(idx);
        let observed = unsafe { std::slice::from_raw_parts(item.ptr, item.len) };
        assert_eq!(observed, item.expected.as_slice());
        unsafe { mm_free_buffer(item.ptr, item.len) };
    }
}
