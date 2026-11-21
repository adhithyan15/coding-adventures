//! # VER08b runtime capability conformance for `flash-audio-bridge`.
//!
//! `required_capabilities.json` declares `"capabilities": []` (pure
//! computation). This test proves it at run time: the full public API is
//! exercised inside a process that has been sandboxed, mechanically from that
//! manifest, to the minimal armed baseline (allocator + exit + TAP output).
//!
//! On non-Linux hosts the gate reports `NOT-LIVE` (RCG010); it never reports
//! a pass it did not earn. Set `RCG_STRICT=1` to turn not-live into failure.

#![deny(unsafe_code)]

use std::path::PathBuf;

use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};
use flash_audio_bridge::{
    bridge_render_to_ring_buffer, decode_adpcm_into, decode_pcm16_le_into, parse_define_sound,
    parse_mp3_frame_header, parse_start_sound, render_flash_audio_into, validate_adpcm_bit_width,
    validate_envelope_point_count, validate_sound_rate_code, validate_sound_sample_count,
    FlashAudioState, PcmRingBuffer, SwfAudioChannels,
};

fn check(name: &str, ok: bool) -> Check {
    Check {
        name: name.to_string(),
        ok,
    }
}

fn prepare() -> Exercise {
    std::panic::set_hook(Box::new(|_| {}));

    Box::new(move || {
        // 1. Validation entry points
        let mut checks = vec![
            check(
                "validate_sound_rate_code",
                validate_sound_rate_code(3).is_ok(),
            ),
            check(
                "validate_adpcm_bit_width",
                validate_adpcm_bit_width(4).is_ok(),
            ),
            check(
                "validate_sound_sample_count",
                validate_sound_sample_count(44_100).is_ok(),
            ),
            check(
                "validate_envelope_point_count",
                validate_envelope_point_count(4).is_ok(),
            ),
        ];

        // 2. Tag parsing
        let mut ds_bytes = Vec::new();
        ds_bytes.extend_from_slice(&1u16.to_le_bytes());
        ds_bytes.push(0x3E); // UncompressedLe (3<<4) | 44100 (3<<2) | 16-bit (1<<1) | Mono (0)
        ds_bytes.extend_from_slice(&2u32.to_le_bytes());
        ds_bytes.extend_from_slice(&1000i16.to_le_bytes());
        ds_bytes.extend_from_slice(&2000i16.to_le_bytes());
        let ds_res = parse_define_sound(&ds_bytes);
        checks.push(check("parse_define_sound", ds_res.is_ok()));

        let ss_bytes = [0x01, 0x00, 0x00];
        checks.push(check(
            "parse_start_sound",
            parse_start_sound(&ss_bytes).is_ok(),
        ));

        // 3. Decoders & MP3 header
        let mut pcm_out = [0i16; 2];
        checks.push(check(
            "decode_pcm16_le_into",
            decode_pcm16_le_into(&ds_bytes[7..], &mut pcm_out) == Ok(2),
        ));

        let adpcm_bytes = [0x40, 0x00, 0x0A, 0x12];
        let mut adpcm_out = [0i16; 2];
        checks.push(check(
            "decode_adpcm_into",
            decode_adpcm_into(&adpcm_bytes, SwfAudioChannels::Mono, 2, &mut adpcm_out) == Ok(2),
        ));

        checks.push(check(
            "parse_mp3_frame_header",
            parse_mp3_frame_header(&[0xFF, 0xFB, 0x90, 0x00]).is_ok(),
        ));

        // 4. Mixer & RingBuffer bridge
        if let Ok(mut engine) = FlashAudioState::create_engine(44_100, 24) {
            let reg_ok = ds_res
                .as_ref()
                .ok()
                .and_then(|t| engine.register_define_sound(t).ok())
                .is_some();
            checks.push(check("register_define_sound", reg_ok));
            checks.push(check(
                "as_sound_start",
                engine.as_sound_start(1, 0, 1).is_ok(),
            ));
            let mut mix_out = [0i16; 8];
            checks.push(check(
                "render_flash_audio_into",
                render_flash_audio_into(&mut engine, &mut mix_out).is_ok(),
            ));
            let mut ring: PcmRingBuffer<16> = PcmRingBuffer::empty();
            checks.push(check(
                "bridge_render_to_ring_buffer",
                bridge_render_to_ring_buffer(&mut engine, &mut ring, 4).is_ok(),
            ));
        } else {
            checks.push(check("create_engine", false));
        }

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/flash-audio-bridge",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
