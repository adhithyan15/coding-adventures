//! Hostile-input and deterministic fuzz test suite for `audio-mixer-realtime`.

#![forbid(unsafe_code)]

use audio_mixer_realtime::{
    clamp_sample, export_to_pcm_audio, export_to_sink_buffer, mix_channels_into,
    scale_voice_sample, validate_sample_rate, MixerError, PcmRingBuffer, RealtimeMixerState,
    MAX_MIXER_CHANNELS,
};

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

    fn next_i16(&mut self) -> i16 {
        (self.next_u64() & 0xFFFF) as i16
    }
}

#[test]
fn hostile_ring_buffer_zero_cap_and_atomic_overrun_and_extreme_mix() {
    // PcmRingBuffer<0> must never divide/mod by zero
    let mut zero_ring = PcmRingBuffer::<0>::empty();
    assert_eq!(zero_ring.push(123), Err(MixerError::RingBufferOverrun));
    assert_eq!(zero_ring.pop(), None);
    assert_eq!(
        zero_ring.push_slice(&[1, 2]),
        Err(MixerError::RingBufferOverrun)
    );
    let mut pop_buf = [99i16; 4];
    assert_eq!(zero_ring.pop_slice(&mut pop_buf), 0);

    // Atomic push_slice on overrun
    let mut ring = PcmRingBuffer::<4>::empty();
    assert_eq!(ring.push_slice(&[10, 20]), Ok(2));
    let before = ring;
    assert_eq!(
        ring.push_slice(&[30, 40, 50]),
        Err(MixerError::RingBufferOverrun)
    );
    assert_eq!(ring.len(), 2);
    assert_eq!(ring.overruns, before.overruns + 1);
    let mut drained = [0i16; 2];
    assert_eq!(ring.pop_slice(&mut drained), 2);
    assert_eq!(drained, [10, 20]);

    // Invalid sample rates & OOB slot indices
    assert!(RealtimeMixerState::new(0).is_err());
    assert!(RealtimeMixerState::new(u32::MAX).is_err());
    assert!(validate_sample_rate(0).is_err());
    assert!(validate_sample_rate(u32::MAX).is_err());
    assert!(export_to_pcm_audio(&[0], 0).is_err());
    assert!(export_to_sink_buffer(&[0], u32::MAX).is_err());

    let mut mixer = RealtimeMixerState::new(44_100).unwrap();
    assert!(matches!(
        mixer.set_voice_gain(usize::MAX, 65_536),
        Err(MixerError::ChannelIndexOutOfBounds { .. })
    ));
    assert!(matches!(
        mixer.set_voice_pan(usize::MAX, 0),
        Err(MixerError::ChannelIndexOutOfBounds { .. })
    ));
    assert!(matches!(
        mixer.set_voice_pitch(usize::MAX, 65_536),
        Err(MixerError::ChannelIndexOutOfBounds { .. })
    ));
    assert!(matches!(
        mixer.stop_slot(usize::MAX),
        Err(MixerError::ChannelIndexOutOfBounds { .. })
    ));

    // Mutated u64::MAX phase_q16 & u32::MAX gains across 16 voices at i16::MIN
    mixer.master_gain_q16 = u32::MAX;
    for slot in &mut mixer.slots {
        slot.active = true;
        slot.gain_q16 = u32::MAX;
        slot.pitch_step_q16 = u32::MAX;
        slot.phase_q16 = u64::MAX;
        slot.looping = true;
    }
    let min_src = [i16::MIN; 8];
    let sources: Vec<&[i16]> = (0..MAX_MIXER_CHANNELS)
        .map(|_| min_src.as_slice())
        .collect();
    let mut out = [0i16; 16];
    assert_eq!(mix_channels_into(&mut mixer, &sources, &mut out), Ok(16));

    // Empty / short sources must deactivate cleanly without OOB
    let empty_src: &[i16] = &[];
    assert!(mix_channels_into(&mut mixer, &[empty_src], &mut out).is_ok());
    assert_eq!(clamp_sample(i64::MIN), i16::MIN);
    assert_eq!(clamp_sample(i64::MAX), i16::MAX);
    let _ = scale_voice_sample(i16::MIN, u32::MAX, u32::MAX);
}

#[test]
fn hostile_fuzz_campaign_1500_cases() {
    let mut rng = SplitMix64(0x8E41_71AE_0000_0001);
    let mut mixer = RealtimeMixerState::new(48_000).unwrap();
    let mut ring = PcmRingBuffer::<32>::empty();
    let src_data: Vec<i16> = (0..64).map(|_| rng.next_i16()).collect();

    for i in 0..1_500 {
        let sr = match i % 6 {
            0 => 0,
            1 => u32::MAX,
            _ => rng.next_u32(),
        };
        let _ = RealtimeMixerState::new(sr);
        let _ = validate_sample_rate(sr);

        let slot_idx = if i % 9 == 0 {
            usize::MAX
        } else {
            (rng.next_u32() as usize) % (MAX_MIXER_CHANNELS + 4)
        };
        let _ = mixer.allocate_voice(
            rng.next_u32(),
            (rng.next_u32() & 0xFF) as u8,
            rng.next_u32(),
            (i & 1) == 0,
        );
        let _ = mixer.set_voice_gain(slot_idx, rng.next_u32());
        let _ = mixer.set_voice_pitch(slot_idx, rng.next_u32());
        let _ = mixer.set_voice_pan(slot_idx, rng.next_u32() as i32);

        if i % 11 == 0 {
            let s = &mut mixer.slots[i % MAX_MIXER_CHANNELS];
            s.gain_q16 = rng.next_u32();
            s.pitch_step_q16 = rng.next_u32();
            s.phase_q16 = rng.next_u64();
            mixer.master_gain_q16 = rng.next_u32();
        }

        let n_sources = (rng.next_u32() as usize) % (MAX_MIXER_CHANNELS + 2);
        let sources: Vec<&[i16]> = (0..n_sources)
            .map(|k| {
                if k % 5 == 0 {
                    &[][..]
                } else {
                    let len = 1 + ((rng.next_u32() as usize) % src_data.len());
                    &src_data[..len]
                }
            })
            .collect();
        let mut out = vec![0i16; (rng.next_u32() as usize) % 32];
        let _ = mix_channels_into(&mut mixer, &sources, &mut out);

        let _ = ring.push(rng.next_i16());
        let _ = ring.push_slice(&out);
        let mut pop_dst = vec![0i16; (rng.next_u32() as usize) % 16];
        let _ = ring.pop_slice(&mut pop_dst);
        if i % 10 == 0 {
            let _ = ring.pop();
        }
    }
}
