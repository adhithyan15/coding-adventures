//! Hostile-input and deterministic fuzz test suite for `arcade-sfx`.

#![forbid(unsafe_code)]

use arcade_sfx::{
    clamp_sample, export_sfx_to_pcm_audio, lfsr_step_15, lfsr_step_7, mix_bank_into,
    render_sfx_into, scale_gain, validate_frequency, validate_sample_rate, AdsrEnvelope, SfxBank,
    SfxError, SfxPatch, SfxPreset, SfxVoice, Waveform, MAX_EXPORT_SAMPLES,
};
use audio_mixer_realtime::PcmRingBuffer;

struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    fn next_u16(&mut self) -> u16 {
        (self.next_u64() & 0xFFFF) as u16
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
}

#[test]
fn hostile_export_oom_guard_and_atomic_ring_routing() {
    // u32::MAX sustain / ADSR durations must be rejected before allocating gigabytes
    let mut huge_patch = SfxPatch::default_patch();
    huge_patch.envelope.attack_samples = u32::MAX;
    huge_patch.envelope.decay_samples = u32::MAX;
    huge_patch.sustain_samples = u32::MAX;
    huge_patch.envelope.release_samples = u32::MAX;
    assert_eq!(huge_patch.patch_total_samples(), u32::MAX);
    assert!(matches!(
        export_sfx_to_pcm_audio(&huge_patch, 44_100),
        Err(SfxError::InvalidParameter { .. })
    ));

    let mut over_limit = SfxPatch::default_patch();
    over_limit.envelope.attack_samples = 0;
    over_limit.envelope.decay_samples = 0;
    over_limit.envelope.release_samples = 0;
    over_limit.sustain_samples = MAX_EXPORT_SAMPLES + 1;
    assert!(matches!(
        export_sfx_to_pcm_audio(&over_limit, 44_100),
        Err(SfxError::InvalidParameter { .. })
    ));

    // Atomic voice routing on ring overrun: voice, scratch, and ring remain untouched
    let mut ring = PcmRingBuffer::<8>::empty();
    ring.push_slice(&[1, 2, 3, 4, 5, 6]).unwrap(); // only 2 slots free
    let mut voice = SfxVoice::new(44_100).unwrap();
    voice.trigger_preset(SfxPreset::LaserSweep).unwrap();
    let voice_snapshot = voice;
    let mut scratch = [0x5A5Ai16; 4];
    let scratch_snapshot = scratch;
    let ring_snapshot = ring;
    assert_eq!(
        voice.route_voice_to_ring_buffer(&mut ring, &mut scratch),
        Err(SfxError::MixerIntegrationError)
    );
    assert_eq!(voice, voice_snapshot);
    assert_eq!(scratch, scratch_snapshot);
    assert_eq!(ring, ring_snapshot);

    // Atomic bank routing on ring overrun & invalid sample rate
    let mut bank = SfxBank::<4>::empty(44_100).unwrap();
    bank.trigger_bank_preset(SfxPreset::PongScore).unwrap();
    let bank_snapshot = bank;
    assert_eq!(
        bank.route_bank_to_ring_buffer(&mut ring, &mut scratch),
        Err(SfxError::MixerIntegrationError)
    );
    assert_eq!(bank, bank_snapshot);
    assert_eq!(scratch, scratch_snapshot);

    // Mutated sample_rate_hz = 0 in mix_bank_into must fail before touching out or voices
    bank.voices[0].sample_rate_hz = 0;
    let mut out = [777i16; 8];
    assert_eq!(
        mix_bank_into(&mut bank, &mut out),
        Err(SfxError::InvalidSampleRate { rate: 0 })
    );
    assert_eq!(out, [777i16; 8]);
    assert_eq!(bank.voices[0].current_sample, 0);

    // Master gain u32::MAX clamping in mix_bank_into must not overflow
    let mut loud_bank = SfxBank::<4>::empty(44_100).unwrap();
    loud_bank.master_gain_q16 = u32::MAX;
    for _ in 0..4 {
        loud_bank.trigger_bank_preset(SfxPreset::PongScore).unwrap();
    }
    let _ = mix_bank_into(&mut loud_bank, &mut out).unwrap();
}

#[test]
fn hostile_fuzz_campaign_1500_cases() {
    let mut rng = SplitMix64(0xA4CA_DE5F_0000_0001);
    let mut voice = SfxVoice::new(44_100).unwrap();
    let mut bank = SfxBank::<4>::empty(44_100).unwrap();
    let mut scratch = [0i16; 32];
    let mut ring = PcmRingBuffer::<16>::empty();

    for i in 0..1_500 {
        let sr = match i % 6 {
            0 => 0,
            1 => u32::MAX,
            _ => rng.next_u32(),
        };
        let _ = validate_sample_rate(sr);
        let _ = validate_frequency(rng.next_u32(), sr);
        let _ = clamp_sample(rng.next_u64() as i64);
        let _ = scale_gain(rng.next_i32(), rng.next_u32());
        let _ = lfsr_step_15(rng.next_u16());
        let _ = lfsr_step_7(rng.next_u16());

        let wf = match i % 4 {
            0 => Waveform::Square {
                duty_q16: rng.next_u16(),
            },
            1 => Waveform::Triangle,
            2 => Waveform::Sawtooth,
            _ => Waveform::LfsrNoise {
                short_mode: (i & 1) == 0,
            },
        };
        let env = AdsrEnvelope {
            attack_samples: rng.next_u32(),
            decay_samples: rng.next_u32(),
            sustain_level_q16: rng.next_u32(),
            release_samples: rng.next_u32(),
        };
        let _ = env.compute_level(rng.next_u32(), rng.next_u32());
        let _ = env.compute_release_level(rng.next_u32(), rng.next_u32());

        let patch = SfxPatch {
            waveform: wf,
            start_freq_hz: rng.next_u32(),
            end_freq_hz: rng.next_u32(),
            freq_sweep_samples: rng.next_u32(),
            arpeggio_step_samples: rng.next_u32(),
            arpeggio_freq_hz: rng.next_u32(),
            envelope: env,
            sustain_samples: rng.next_u32(),
            gain_q16: rng.next_u32(),
        };
        let _ = voice.trigger_patch(patch);
        voice.lfsr_state = 0;
        voice.patch.gain_q16 = rng.next_u32();
        let _ = render_sfx_into(&mut voice, &mut scratch);

        let _ = bank.trigger_bank_patch(patch);
        bank.master_gain_q16 = rng.next_u32();
        let _ = mix_bank_into(&mut bank, &mut scratch);
        let _ = voice.route_voice_to_ring_buffer(&mut ring, &mut scratch);
        let _ = bank.route_bank_to_ring_buffer(&mut ring, &mut scratch);
        if i % 7 == 0 {
            ring.clear();
        }
    }
}
