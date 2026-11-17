//! # Conformance and Security Verification Test Suite (MUS08)
//!
//! Validates:
//! - CWE-190: 16-channel full-scale saturation without integer wrap-around.
//! - CWE-400: Ring buffer overrun handling without unbounded dynamic allocation.
//! - CWE-125: Out-of-bounds source slice and cursor safety.
//! - Q16.16 fixed-point gain, pitch stepping, panning, and voice loop cycling.
//! - Voice allocation and lowest-priority voice stealing.
//! - Device sink (`audio_device_sink::PcmPlaybackBuffer`) and container (`pcm_audio::PcmAudio`) bridges.
//! - Pure mathematical kernel functions and error representations.

use audio_mixer_realtime::{
    clamp_sample, export_to_pcm_audio, export_to_sink_buffer, mix_channels_into,
    scale_voice_sample, validate_sample_rate, MixerError, PcmRingBuffer, RealtimeMixerState,
    VoiceSlot, MAX_GAIN_Q16, MAX_MIXER_CHANNELS, MAX_PITCH_STEP_Q16, MAX_SAMPLE_RATE_HZ,
    MIN_PITCH_STEP_Q16, MIN_SAMPLE_RATE_HZ, PAN_CENTER_Q16, PAN_LEFT_Q16, PAN_RIGHT_Q16,
    PCM16_MAX, PCM16_MIN, UNITY_GAIN_Q16,
};

#[test]
fn test_cwe_190_sixteen_channel_full_scale_positive_saturation() {
    let mut state = RealtimeMixerState::new(44_100).expect("valid sample rate");

    // Allocate all 16 slots with maximum positive amplitude (32,767)
    let max_pos = [PCM16_MAX; 32];
    let mut sources: Vec<&[i16]> = Vec::new();

    for i in 0..MAX_MIXER_CHANNELS {
        let slot_idx = state
            .allocate_voice(i as u32 + 1, 10, UNITY_GAIN_Q16, true)
            .expect("should allocate slot");
        assert_eq!(slot_idx, i);
        sources.push(&max_pos);
    }
    assert_eq!(state.active_slots(), 16);

    let mut out = [0i16; 16];
    let written = mix_channels_into(&mut state, &sources, &mut out).expect("mix succeeds");
    assert_eq!(written, 16);

    // Theoretical sum: 16 * 32,767 = 524,272.
    // Must clamp to PCM16_MAX (32,767) and NEVER wrap to negative.
    for &sample in &out {
        assert_eq!(sample, PCM16_MAX);
    }
    assert_eq!(state.clipped_samples, 16);
    assert_eq!(state.mixed_frames, 16);
}

#[test]
fn test_cwe_190_sixteen_channel_full_scale_negative_saturation() {
    let mut state = RealtimeMixerState::new(44_100).expect("valid sample rate");

    // Allocate all 16 slots with maximum negative amplitude (-32,768)
    let max_neg = [PCM16_MIN; 32];
    let mut sources: Vec<&[i16]> = Vec::new();

    for i in 0..MAX_MIXER_CHANNELS {
        state
            .allocate_voice(i as u32 + 1, 10, UNITY_GAIN_Q16, true)
            .expect("should allocate slot");
        sources.push(&max_neg);
    }

    let mut out = [0i16; 16];
    let written = mix_channels_into(&mut state, &sources, &mut out).expect("mix succeeds");
    assert_eq!(written, 16);

    // Theoretical sum: 16 * (-32,768) = -524,288.
    // Must clamp to PCM16_MIN (-32,768) and NEVER wrap to positive.
    for &sample in &out {
        assert_eq!(sample, PCM16_MIN);
    }
    assert_eq!(state.clipped_samples, 16);
}

#[test]
fn test_cwe_400_ring_buffer_overrun_and_underrun() {
    let mut ring: PcmRingBuffer<8> = PcmRingBuffer::empty();
    assert_eq!(ring.capacity(), 8);
    assert_eq!(ring.len(), 0);
    assert!(ring.is_empty());
    assert!(!ring.is_full());

    // Push up to capacity
    for i in 0..8 {
        assert!(ring.push(i as i16 * 100).is_ok());
    }
    assert_eq!(ring.len(), 8);
    assert!(ring.is_full());
    assert_eq!(ring.overruns, 0);

    // Overrun attempts must be rejected fail-safe without resizing
    for _ in 0..5 {
        let err = ring.push(9999).unwrap_err();
        assert_eq!(err, MixerError::RingBufferOverrun);
    }
    assert_eq!(ring.len(), 8);
    assert_eq!(ring.capacity(), 8);
    assert_eq!(ring.overruns, 5);

    // Pop all items in FIFO order
    for i in 0..8 {
        assert_eq!(ring.pop(), Some(i as i16 * 100));
    }
    assert_eq!(ring.len(), 0);
    assert!(ring.is_empty());

    // Underrun attempts return None and increment underruns counter
    assert_eq!(ring.pop(), None);
    assert_eq!(ring.pop(), None);
    assert_eq!(ring.underruns, 2);
}

#[test]
fn test_ring_buffer_slice_operations_and_clear() {
    let mut ring: PcmRingBuffer<16> = PcmRingBuffer::empty();

    let input = [10, 20, 30, 40, 50];
    let pushed = ring.push_slice(&input).expect("push_slice succeeds");
    assert_eq!(pushed, 5);
    assert_eq!(ring.len(), 5);

    let mut out = [0i16; 3];
    let popped = ring.pop_slice(&mut out);
    assert_eq!(popped, 3);
    assert_eq!(out, [10, 20, 30]);
    assert_eq!(ring.len(), 2);

    ring.clear();
    assert_eq!(ring.len(), 0);
    assert!(ring.is_empty());
}

#[test]
fn test_cwe_125_source_slice_out_of_bounds_safety() {
    let mut state = RealtimeMixerState::new(44_100).expect("valid sample rate");

    // Activate voice 0 and voice 1
    state
        .allocate_voice(101, 1, UNITY_GAIN_Q16, false)
        .expect("allocate slot 0");
    state
        .allocate_voice(102, 1, UNITY_GAIN_Q16, false)
        .expect("allocate slot 1");

    // Provide only 1 source slice when 2 are active (sources.len() < active slots)
    let source0 = [1000i16; 8];
    let sources = [&source0[..]];

    let mut out = [0i16; 4];
    let res = mix_channels_into(&mut state, &sources, &mut out);
    assert!(res.is_ok());

    // Slot 1 had no corresponding source slice, so it was cleanly deactivated
    assert!(state.slots[0].active);
    assert!(!state.slots[1].active);
}

#[test]
fn test_cwe_125_empty_source_slice_safety() {
    let mut state = RealtimeMixerState::new(44_100).expect("valid sample rate");
    state
        .allocate_voice(101, 1, UNITY_GAIN_Q16, false)
        .expect("allocate slot 0");

    let empty: [i16; 0] = [];
    let sources = [&empty[..]];

    let mut out = [0i16; 4];
    let res = mix_channels_into(&mut state, &sources, &mut out);
    assert!(res.is_ok());
    assert!(!state.slots[0].active);
}

#[test]
fn test_pitch_stepping_and_phase_accumulation() {
    let mut state = RealtimeMixerState::new(44_100).expect("valid sample rate");
    let slot_idx = state
        .allocate_voice(1, 10, UNITY_GAIN_Q16, false)
        .expect("allocate voice");

    // 2.0x pitch speed = pitch_step_q16 = 131,072
    state
        .set_voice_pitch(slot_idx, UNITY_GAIN_Q16 * 2)
        .expect("set pitch");

    let source = [100, 200, 300, 400, 500, 600, 700, 800];
    let sources = [&source[..]];

    let mut out = [0i16; 3];
    mix_channels_into(&mut state, &sources, &mut out).expect("mix succeeds");

    // At 2.0x speed, phase advances by 2 samples per output frame:
    // Frame 0: index 0 -> 100
    // Frame 1: index 2 -> 300
    // Frame 2: index 4 -> 500
    assert_eq!(out, [100, 300, 500]);
    assert_eq!(state.slots[slot_idx].cursor, 4);
}

#[test]
fn test_pitch_stepping_half_speed() {
    let mut state = RealtimeMixerState::new(44_100).expect("valid sample rate");
    let slot_idx = state
        .allocate_voice(1, 10, UNITY_GAIN_Q16, false)
        .expect("allocate voice");

    // 0.5x speed = pitch_step_q16 = 32,768
    state
        .set_voice_pitch(slot_idx, UNITY_GAIN_Q16 / 2)
        .expect("set pitch");

    let source = [100, 200, 300, 400];
    let sources = [&source[..]];

    let mut out = [0i16; 4];
    mix_channels_into(&mut state, &sources, &mut out).expect("mix succeeds");

    // Frame 0: phase 0 -> index 0 -> 100
    // Frame 1: phase 0.5 -> index 0 -> 100
    // Frame 2: phase 1.0 -> index 1 -> 200
    // Frame 3: phase 1.5 -> index 1 -> 200
    assert_eq!(out, [100, 100, 200, 200]);
}

#[test]
fn test_looping_voice_wraps_phase() {
    let mut state = RealtimeMixerState::new(44_100).expect("valid sample rate");
    let slot_idx = state
        .allocate_voice(1, 10, UNITY_GAIN_Q16, true)
        .expect("allocate looping voice");

    let source = [10, 20];
    let sources = [&source[..]];

    let mut out = [0i16; 6];
    mix_channels_into(&mut state, &sources, &mut out).expect("mix succeeds");

    // Looping over 2-sample buffer:
    assert_eq!(out, [10, 20, 10, 20, 10, 20]);
    assert!(state.slots[slot_idx].active);
}

#[test]
fn test_non_looping_voice_terminates() {
    let mut state = RealtimeMixerState::new(44_100).expect("valid sample rate");
    let slot_idx = state
        .allocate_voice(1, 10, UNITY_GAIN_Q16, false)
        .expect("allocate non-looping voice");

    let source = [10, 20];
    let sources = [&source[..]];

    let mut out = [0i16; 4];
    mix_channels_into(&mut state, &sources, &mut out).expect("mix succeeds");

    assert_eq!(out[0], 10);
    assert_eq!(out[1], 20);
    assert_eq!(out[2], 0);
    assert_eq!(out[3], 0);
    assert!(!state.slots[slot_idx].active);
}

#[test]
fn test_voice_gain_attenuation_and_master_gain() {
    let mut state = RealtimeMixerState::new(44_100).expect("valid sample rate");
    let slot_idx = state
        .allocate_voice(1, 10, UNITY_GAIN_Q16 / 2, false)
        .expect("allocate voice at 50% gain");

    let source = [1000, 2000];
    let sources = [&source[..]];

    let mut out = [0i16; 2];
    mix_channels_into(&mut state, &sources, &mut out).expect("mix succeeds");
    assert_eq!(out, [500, 1000]);

    // Master gain attenuation to 50% (overall 25%)
    state.slots[slot_idx].active = true;
    state.slots[slot_idx].phase_q16 = 0;
    state.master_gain_q16 = UNITY_GAIN_Q16 / 2;

    mix_channels_into(&mut state, &sources, &mut out).expect("mix succeeds");
    assert_eq!(out, [250, 500]);
}

#[test]
fn test_voice_stealing_by_priority() {
    let mut state = RealtimeMixerState::new(44_100).expect("valid sample rate");

    // Fill all 16 slots with priority 5
    for i in 0..MAX_MIXER_CHANNELS {
        state
            .allocate_voice(i as u32, 5, UNITY_GAIN_Q16, false)
            .expect("allocate");
    }

    // Set slot 3 to lower priority (2)
    state.slots[3].priority = 2;

    // Allocate with priority 4: should steal slot 3 (priority 2 < 4)
    let stolen_idx = state
        .allocate_voice(999, 4, UNITY_GAIN_Q16, false)
        .expect("should steal");
    assert_eq!(stolen_idx, 3);
    assert_eq!(state.slots[3].voice_id, 999);
    assert_eq!(state.slots[3].priority, 4);

    // Attempt to allocate with priority 1 (lower than all slots): rejected
    let err = state
        .allocate_voice(1000, 1, UNITY_GAIN_Q16, false)
        .unwrap_err();
    assert_eq!(err, MixerError::ChannelLimitExceeded);
}

#[test]
fn test_stop_voice_and_reset() {
    let mut state = RealtimeMixerState::new(44_100).expect("valid sample rate");
    state
        .allocate_voice(42, 10, UNITY_GAIN_Q16, true)
        .expect("allocate");
    state
        .allocate_voice(42, 10, UNITY_GAIN_Q16, true)
        .expect("allocate");
    state
        .allocate_voice(99, 10, UNITY_GAIN_Q16, true)
        .expect("allocate");
    assert_eq!(state.active_slots(), 3);

    state.stop_voice(42);
    assert_eq!(state.active_slots(), 1);
    assert_eq!(state.slots[2].voice_id, 99);

    state.reset();
    assert_eq!(state.active_slots(), 0);
    assert_eq!(state.clipped_samples, 0);
    assert_eq!(state.mixed_frames, 0);
}

#[test]
fn test_export_to_sink_buffer_roundtrip() {
    let samples = [0i16, 1000, -2000, 32767, -32768];
    let sink_buf = export_to_sink_buffer(&samples, 48_000).expect("export to sink succeeds");

    assert_eq!(sink_buf.sample_count(), 5);
    assert_eq!(sink_buf.frame_count(), 5);
    assert_eq!(sink_buf.format().sample_rate_hz, 48_000);
    assert_eq!(sink_buf.format().channel_count, 1);
    assert_eq!(sink_buf.format().bit_depth, 16);
    assert_eq!(sink_buf.samples(), &samples);
    assert!(!sink_buf.is_empty());
}

#[test]
fn test_export_to_pcm_audio_roundtrip() {
    let samples = [500i16, -500, 1500, -1500];
    let pcm = export_to_pcm_audio(&samples, 44_100).expect("export to pcm audio succeeds");

    assert_eq!(pcm.sample_count(), 4);
    assert_eq!(pcm.samples(), &samples);
    assert_eq!(pcm.format().sample_rate_hz(), 44_100.0);
    assert_eq!(pcm.format().channel_count(), 1);
}

#[test]
fn test_sample_rate_validation_boundaries() {
    assert!(validate_sample_rate(MIN_SAMPLE_RATE_HZ).is_ok());
    assert!(validate_sample_rate(44_100).is_ok());
    assert!(validate_sample_rate(MAX_SAMPLE_RATE_HZ).is_ok());

    assert_eq!(
        validate_sample_rate(0),
        Err(MixerError::InvalidSampleRate { rate: 0 })
    );
    assert_eq!(
        validate_sample_rate(MAX_SAMPLE_RATE_HZ + 1),
        Err(MixerError::InvalidSampleRate {
            rate: MAX_SAMPLE_RATE_HZ + 1
        })
    );
}

#[test]
fn test_pure_kernel_functions() {
    assert_eq!(clamp_sample(0), 0);
    assert_eq!(clamp_sample(32767), 32767);
    assert_eq!(clamp_sample(50000), 32767);
    assert_eq!(clamp_sample(-32768), -32768);
    assert_eq!(clamp_sample(-60000), -32768);

    assert_eq!(
        scale_voice_sample(1000, UNITY_GAIN_Q16, UNITY_GAIN_Q16),
        1000
    );
    assert_eq!(scale_voice_sample(1000, UNITY_GAIN_Q16 / 2, UNITY_GAIN_Q16), 500);
}

#[test]
fn test_voice_slot_controls_and_defaults() {
    let mut state = RealtimeMixerState::new(44_100).expect("valid sample rate");
    let slot_idx = state
        .allocate_voice(1, 10, UNITY_GAIN_Q16, false)
        .expect("allocate slot");

    // Test gain clamps to MAX_GAIN_Q16
    assert!(state.set_voice_gain(slot_idx, MAX_GAIN_Q16 * 2).is_ok());
    assert_eq!(state.slots[slot_idx].gain_q16, MAX_GAIN_Q16);

    // Test pitch clamps to MIN/MAX
    assert!(state.set_voice_pitch(slot_idx, 1).is_ok());
    assert_eq!(state.slots[slot_idx].pitch_step_q16, MIN_PITCH_STEP_Q16);
    assert!(state.set_voice_pitch(slot_idx, 1_000_000).is_ok());
    assert_eq!(state.slots[slot_idx].pitch_step_q16, MAX_PITCH_STEP_Q16);

    // Test pan clamps to LEFT/RIGHT
    assert!(state.set_voice_pan(slot_idx, PAN_LEFT_Q16 - 100).is_ok());
    assert_eq!(state.slots[slot_idx].pan_q16, PAN_LEFT_Q16);
    assert!(state.set_voice_pan(slot_idx, PAN_RIGHT_Q16 + 100).is_ok());
    assert_eq!(state.slots[slot_idx].pan_q16, PAN_RIGHT_Q16);
    assert!(state.set_voice_pan(slot_idx, PAN_CENTER_Q16).is_ok());
    assert_eq!(state.slots[slot_idx].pan_q16, PAN_CENTER_Q16);

    // Test out of bounds slot index
    assert_eq!(
        state.set_voice_gain(MAX_MIXER_CHANNELS, 100),
        Err(MixerError::ChannelIndexOutOfBounds {
            index: MAX_MIXER_CHANNELS,
            max: MAX_MIXER_CHANNELS,
        })
    );
    assert_eq!(
        state.set_voice_pitch(MAX_MIXER_CHANNELS, 100),
        Err(MixerError::ChannelIndexOutOfBounds {
            index: MAX_MIXER_CHANNELS,
            max: MAX_MIXER_CHANNELS,
        })
    );
    assert_eq!(
        state.set_voice_pan(MAX_MIXER_CHANNELS, 0),
        Err(MixerError::ChannelIndexOutOfBounds {
            index: MAX_MIXER_CHANNELS,
            max: MAX_MIXER_CHANNELS,
        })
    );

    // Test stop_slot
    assert!(state.stop_slot(slot_idx).is_ok());
    assert!(!state.slots[slot_idx].active);
    assert_eq!(
        state.stop_slot(MAX_MIXER_CHANNELS),
        Err(MixerError::ChannelIndexOutOfBounds {
            index: MAX_MIXER_CHANNELS,
            max: MAX_MIXER_CHANNELS,
        })
    );

    // Test VoiceSlot defaults
    let empty_slot = VoiceSlot::empty();
    assert!(!empty_slot.active);
    let default_slot = VoiceSlot::default();
    assert_eq!(empty_slot, default_slot);
}

#[test]
fn test_mixer_error_formatting() {
    let errors = [
        MixerError::InvalidSampleRate { rate: 42 },
        MixerError::InvalidParameter { reason: "bad" },
        MixerError::ChannelLimitExceeded,
        MixerError::ChannelIndexOutOfBounds { index: 16, max: 16 },
        MixerError::RingBufferOverrun,
        MixerError::RingBufferUnderrun,
        MixerError::EmptyBuffer,
        MixerError::BufferTooSmall,
        MixerError::SinkBufferError,
    ];
    for err in &errors {
        let msg = format!("{err}");
        assert!(!msg.is_empty());
    }
}
