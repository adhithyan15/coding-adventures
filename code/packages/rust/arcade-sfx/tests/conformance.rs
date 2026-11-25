//! # Conformance and Security Verification Test Suite for `arcade-sfx` (GAME12)
//!
//! Validates:
//! - Waveform synthesis: Square (duty cycles), Triangle, Sawtooth, and LFSR Noise.
//! - Deterministic LFSR noise generation: 15-bit long mode vs 7-bit short mode with seed guard (CWE-20).
//! - Sample-counted ADSR envelope stages: Attack, Decay, Sustain, Release, and dynamic release.
//! - Arcade sound presets: Pong (PaddleHit, WallBounce, Score), Breakout (BrickHit rows, LoseLife),
//!   CoinInsert arpeggio, and LaserSweep pitch sweep.
//! - SfxVoice single-voice synthesis and SfxBank multi-voice polyphony with priority voice stealing.
//! - CWE-369: Divide-by-zero protection (sample rate validation and Nyquist frequency limits).
//! - CWE-190: Integer overflow protection (polyphonic accumulation hard saturation clamping to `[-32768, 32767]`).
//! - Integration: `audio-mixer-realtime` PCM ring buffer routing and `pcm_audio::PcmAudio` export.

use arcade_sfx::{
    clamp_sample, export_preset_to_pcm_audio, export_sfx_to_pcm_audio, lfsr_step_15, lfsr_step_7,
    mix_bank_into, render_sfx_into, scale_gain, validate_frequency, validate_sample_rate,
    AdsrEnvelope, SfxBank, SfxError, SfxPatch, SfxPreset, SfxVoice, Waveform, DEFAULT_LFSR_SEED,
    DEFAULT_SAMPLE_RATE_HZ, DUTY_50_PERCENT, MAX_GAIN_Q16, MAX_SAMPLE_RATE_HZ, MIN_SAMPLE_RATE_HZ,
    PCM16_MAX, PCM16_MIN, UNITY_GAIN_Q16,
};
use audio_mixer_realtime::PcmRingBuffer;

#[test]
fn test_cwe_369_sample_rate_validation() {
    assert!(matches!(
        validate_sample_rate(0),
        Err(SfxError::InvalidSampleRate { rate: 0 })
    ));
    assert!(validate_sample_rate(MIN_SAMPLE_RATE_HZ).is_ok());
    assert!(validate_sample_rate(DEFAULT_SAMPLE_RATE_HZ).is_ok());
    assert!(validate_sample_rate(MAX_SAMPLE_RATE_HZ).is_ok());
    assert!(matches!(
        validate_sample_rate(MAX_SAMPLE_RATE_HZ + 1),
        Err(SfxError::InvalidSampleRate { .. })
    ));

    // Constructing voice or bank with invalid sample rate fails
    assert!(SfxVoice::new(0).is_err());
    assert!(SfxBank::<4>::empty(0).is_err());
}

#[test]
fn test_cwe_20_frequency_nyquist_validation() {
    let sr = 44_100;
    assert!(validate_frequency(1000, sr).is_ok());
    assert!(validate_frequency(22_050, sr).is_ok());
    assert!(matches!(
        validate_frequency(22_051, sr),
        Err(SfxError::InvalidFrequency {
            freq: 22_051,
            max: 22_050
        })
    ));
}

#[test]
fn test_cwe_20_lfsr_seed_guard_and_determinism() {
    // Non-zero seed guard restores 0 to DEFAULT_LFSR_SEED
    assert_eq!(lfsr_step_15(0), lfsr_step_15(DEFAULT_LFSR_SEED));
    assert_eq!(lfsr_step_7(0), lfsr_step_7(DEFAULT_LFSR_SEED));

    // Deterministic progression
    let s1 = lfsr_step_15(1);
    let s2 = lfsr_step_15(s1);
    let s3 = lfsr_step_15(s2);
    assert_ne!(s1, s2);
    assert_ne!(s2, s3);

    // Sequence reproduces identically
    assert_eq!(lfsr_step_15(lfsr_step_15(1)), s2);
}

#[test]
fn test_cwe_190_saturation_clamping() {
    assert_eq!(clamp_sample(0), 0);
    assert_eq!(clamp_sample(PCM16_MAX as i64), PCM16_MAX);
    assert_eq!(clamp_sample(PCM16_MIN as i64), PCM16_MIN);
    assert_eq!(clamp_sample(500_000), PCM16_MAX);
    assert_eq!(clamp_sample(-500_000), PCM16_MIN);
    assert_eq!(clamp_sample(i64::MAX), PCM16_MAX);
    assert_eq!(clamp_sample(i64::MIN), PCM16_MIN);
}

#[test]
fn test_cwe_190_polyphonic_bank_saturation() {
    let mut bank = SfxBank::<16>::empty(44_100).expect("valid sample rate");
    bank.master_gain_q16 = UNITY_GAIN_Q16;

    // Trigger all 16 voices with full-scale positive square waves
    let env = AdsrEnvelope::from_samples(0, 0, UNITY_GAIN_Q16, 0).expect("valid env");
    let patch = SfxPatch::create_patch(
        Waveform::Square {
            duty_q16: DUTY_50_PERCENT,
        },
        1000,
        1000,
        0,
        env,
        1000,
        UNITY_GAIN_Q16,
    )
    .expect("valid patch");

    for _ in 0..16 {
        bank.trigger_bank_patch(patch).expect("trigger voice");
    }
    assert_eq!(bank.active_bank_voices(), 16);

    let mut out = [0i16; 64];
    mix_bank_into(&mut bank, &mut out).expect("mix succeeds");

    // 16 simultaneous full-scale square waves saturate to PCM16_MAX or PCM16_MIN
    // and NEVER wrap around (CWE-190 guard)
    assert_eq!(out[0], PCM16_MAX);
    assert_eq!(out[30], PCM16_MIN);
    for &sample in &out {
        assert!(sample == PCM16_MAX || sample == PCM16_MIN);
    }
}

#[test]
fn test_scale_gain() {
    assert_eq!(scale_gain(1000, UNITY_GAIN_Q16), 1000);
    assert_eq!(scale_gain(1000, 0), 0);
    assert_eq!(scale_gain(1000, MAX_GAIN_Q16), 4000);
}

#[test]
fn test_adsr_envelope_stages() {
    // 10 attack, 10 decay to 50% (32768), 20 sustain, 10 release
    let env = AdsrEnvelope::from_samples(10, 10, 32_768, 10).expect("valid env");
    assert_eq!(env.total_envelope_samples(20), 50);

    // Attack stage
    assert_eq!(env.compute_level(0, 20), 0);
    assert_eq!(env.compute_level(5, 20), 32_768);
    assert_eq!(env.compute_level(9, 20), 58_982);

    // Decay stage
    assert_eq!(env.compute_level(10, 20), UNITY_GAIN_Q16);
    assert_eq!(env.compute_level(15, 20), 49_152);

    // Sustain stage
    assert_eq!(env.compute_level(20, 20), 32_768);
    assert_eq!(env.compute_level(30, 20), 32_768);
    assert_eq!(env.compute_level(39, 20), 32_768);

    // Release stage
    assert_eq!(env.compute_level(40, 20), 32_768);
    assert_eq!(env.compute_level(45, 20), 16_384);
    assert_eq!(env.compute_level(50, 20), 0);

    // Beyond envelope
    assert_eq!(env.compute_level(100, 20), 0);
}

#[test]
fn test_adsr_from_ms() {
    let env = AdsrEnvelope::from_ms(10, 20, 32_768, 30, 44_100).expect("valid env");
    assert_eq!(env.attack_samples, 441);
    assert_eq!(env.decay_samples, 882);
    assert_eq!(env.sustain_level_q16, 32_768);
    assert_eq!(env.release_samples, 1323);
}

#[test]
fn test_waveforms_synthesis() {
    let mut voice = SfxVoice::new(44_100).expect("valid sample rate");
    let env = AdsrEnvelope::from_samples(0, 0, UNITY_GAIN_Q16, 0).expect("env");

    // 1. Square wave
    let square_patch = SfxPatch::create_patch(
        Waveform::Square {
            duty_q16: DUTY_50_PERCENT,
        },
        441,
        441,
        0,
        env,
        100,
        UNITY_GAIN_Q16,
    )
    .expect("square patch");
    voice.trigger_patch(square_patch).expect("trigger square");
    let mut buf = [0i16; 100];
    let written = render_sfx_into(&mut voice, &mut buf).expect("render square");
    assert_eq!(written, 100);
    assert!(buf.contains(&PCM16_MAX));
    assert!(buf.contains(&-PCM16_MAX));

    // 2. Triangle wave
    voice.reset_voice();
    let tri_patch =
        SfxPatch::create_patch(Waveform::Triangle, 441, 441, 0, env, 100, UNITY_GAIN_Q16)
            .expect("tri patch");
    voice.trigger_patch(tri_patch).expect("trigger tri");
    let mut tri_buf = [0i16; 100];
    render_sfx_into(&mut voice, &mut tri_buf).expect("render tri");
    assert!(tri_buf.iter().any(|&s| s > 20_000));
    assert!(tri_buf.iter().any(|&s| s < -20_000));

    // 3. Sawtooth wave
    voice.reset_voice();
    let saw_patch =
        SfxPatch::create_patch(Waveform::Sawtooth, 441, 441, 0, env, 100, UNITY_GAIN_Q16)
            .expect("saw patch");
    voice.trigger_patch(saw_patch).expect("trigger saw");
    let mut saw_buf = [0i16; 100];
    render_sfx_into(&mut voice, &mut saw_buf).expect("render saw");
    assert!(saw_buf.iter().any(|&s| s > 20_000));
    assert!(saw_buf.iter().any(|&s| s < -20_000));

    // 4. LFSR noise
    voice.reset_voice();
    let noise_patch = SfxPatch::create_patch(
        Waveform::LfsrNoise { short_mode: false },
        10_000,
        10_000,
        0,
        env,
        500,
        UNITY_GAIN_Q16,
    )
    .expect("noise patch");
    voice.trigger_patch(noise_patch).expect("trigger noise");
    let mut noise_buf = [0i16; 500];
    render_sfx_into(&mut voice, &mut noise_buf).expect("render noise");
    assert!(noise_buf.contains(&PCM16_MAX));
    assert!(noise_buf.contains(&-PCM16_MAX));
}

#[test]
fn test_pong_presets() {
    let mut voice = SfxVoice::new(44_100).expect("valid voice");

    // Paddle Hit
    voice
        .trigger_preset(SfxPreset::PongPaddleHit)
        .expect("paddle hit");
    assert!(voice.is_active());
    let mut buf = [0i16; 256];
    let written = render_sfx_into(&mut voice, &mut buf).expect("render paddle");
    assert_eq!(written, 256);
    assert!(buf.iter().any(|&s| s != 0));

    // Wall Bounce
    voice.reset_voice();
    voice
        .trigger_preset(SfxPreset::PongWallBounce)
        .expect("wall bounce");
    let mut bounce_buf = [0i16; 256];
    render_sfx_into(&mut voice, &mut bounce_buf).expect("render bounce");
    assert!(bounce_buf.iter().any(|&s| s != 0));

    // Score
    voice.reset_voice();
    voice.trigger_preset(SfxPreset::PongScore).expect("score");
    let mut score_buf = [0i16; 256];
    render_sfx_into(&mut voice, &mut score_buf).expect("render score");
    assert!(score_buf.iter().any(|&s| s != 0));
}

#[test]
fn test_breakout_brick_hit_frequency_progression() {
    let p0 = SfxPreset::BreakoutBrickHit { row: 0 }
        .to_patch(44_100)
        .expect("patch 0");
    let p1 = SfxPreset::BreakoutBrickHit { row: 1 }
        .to_patch(44_100)
        .expect("patch 1");
    let p5 = SfxPreset::BreakoutBrickHit { row: 5 }
        .to_patch(44_100)
        .expect("patch 5");

    assert!(p0.start_freq_hz < p1.start_freq_hz);
    assert!(p1.start_freq_hz < p5.start_freq_hz);
}

#[test]
fn test_coin_insert_arpeggio() {
    let patch = SfxPreset::CoinInsert.to_patch(44_100).expect("coin patch");
    assert_eq!(patch.start_freq_hz, 988);
    assert_eq!(patch.arpeggio_freq_hz, 1319);
    assert!(patch.arpeggio_step_samples > 0);

    let mut voice = SfxVoice::new(44_100).expect("voice");
    voice.trigger_patch(patch).expect("trigger coin");

    let total = voice.total_voice_samples() as usize;
    let mut full_buf = vec![0i16; total];
    let written = render_sfx_into(&mut voice, &mut full_buf).expect("render coin");
    assert_eq!(written, total);
    assert!(!voice.is_active());
}

#[test]
fn test_laser_sweep_pitch_modulation() {
    let patch = SfxPreset::LaserSweep.to_patch(44_100).expect("laser patch");
    assert_eq!(patch.start_freq_hz, 1600);
    assert_eq!(patch.end_freq_hz, 100);
    assert!(patch.freq_sweep_samples > 0);

    let mut voice = SfxVoice::new(44_100).expect("voice");
    voice.trigger_patch(patch).expect("trigger laser");
    let mut buf = [0i16; 512];
    render_sfx_into(&mut voice, &mut buf).expect("render laser");
    assert!(buf.iter().any(|&s| s != 0));
}

#[test]
fn test_early_note_release() {
    let mut voice = SfxVoice::new(44_100).expect("voice");
    let env = AdsrEnvelope::from_samples(10, 10, UNITY_GAIN_Q16, 20).expect("env");
    let patch = SfxPatch::create_patch(
        Waveform::Square {
            duty_q16: DUTY_50_PERCENT,
        },
        440,
        440,
        0,
        env,
        1000,
        UNITY_GAIN_Q16,
    )
    .expect("patch");

    voice.trigger_patch(patch).expect("trigger");
    let mut buf1 = [0i16; 30];
    render_sfx_into(&mut voice, &mut buf1).expect("render initial");
    assert!(voice.is_active());

    // Premature release during sustain
    voice.trigger_release();
    assert_eq!(voice.released_at_sample, 30);
    assert_eq!(voice.total_voice_samples(), 50);

    let mut buf2 = [0i16; 30];
    let written = render_sfx_into(&mut voice, &mut buf2).expect("render release");
    assert_eq!(written, 20);
    assert!(!voice.is_active());
}

#[test]
fn test_sfx_bank_voice_stealing() {
    let mut bank = SfxBank::<2>::empty(44_100).expect("bank");

    let env = AdsrEnvelope::from_samples(10, 10, UNITY_GAIN_Q16, 10).expect("env");
    let patch = SfxPatch::create_patch(
        Waveform::Square {
            duty_q16: DUTY_50_PERCENT,
        },
        440,
        440,
        0,
        env,
        1000,
        UNITY_GAIN_Q16,
    )
    .expect("patch");

    let v0 = bank.trigger_bank_patch(patch).expect("voice 0");
    assert_eq!(v0, 0);
    bank.voices[0].priority = 10;

    let v1 = bank.trigger_bank_patch(patch).expect("voice 1");
    assert_eq!(v1, 1);
    bank.voices[1].priority = 5;

    // Bank is full: triggers voice stealing on lowest priority (voice 1)
    let v2 = bank.trigger_bank_patch(patch).expect("voice 2 steals");
    assert_eq!(v2, 1);
}

#[test]
fn test_pcm_ring_buffer_integration() {
    let mut voice = SfxVoice::new(44_100).expect("voice");
    voice
        .trigger_preset(SfxPreset::PongPaddleHit)
        .expect("preset");

    let mut ring = PcmRingBuffer::<256>::empty();
    let mut scratch = [0i16; 64];

    let routed = voice
        .route_voice_to_ring_buffer(&mut ring, &mut scratch)
        .expect("route to ring");
    assert_eq!(routed, 64);
    assert_eq!(ring.len(), 64);

    let mut bank = SfxBank::<4>::empty(44_100).expect("bank");
    bank.trigger_bank_preset(SfxPreset::CoinInsert)
        .expect("coin");
    let mut bank_ring = PcmRingBuffer::<256>::empty();
    let bank_routed = bank
        .route_bank_to_ring_buffer(&mut bank_ring, &mut scratch)
        .expect("route bank");
    assert_eq!(bank_routed, 64);
    assert_eq!(bank_ring.len(), 64);
}

#[test]
fn test_export_to_pcm_audio() {
    let pcm = export_preset_to_pcm_audio(SfxPreset::PongPaddleHit, 44_100).expect("export preset");
    assert_eq!(pcm.format().sample_rate_hz(), 44_100.0);
    assert_eq!(pcm.format().channel_count(), 1);
    assert_eq!(pcm.format().bit_depth(), 16);
    assert!(pcm.sample_count() > 0);

    let patch = SfxPatch::default_patch();
    let patch_pcm = export_sfx_to_pcm_audio(&patch, 44_100).expect("export patch");
    assert_eq!(patch_pcm.format().sample_rate_hz(), 44_100.0);
    assert!(patch_pcm.sample_count() > 0);
}
