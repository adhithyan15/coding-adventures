use coding_adventures_test_bootstrap::prng::SplitMix64;
use oscillator::{SineOscillator, UniformSampler};
use pcm_audio::{
    from_wav_bytes, to_wav_bytes, PcmAudio, PcmError, PcmFormat, MAX_WAV_BYTES,
};

#[test]
fn test_wav_round_trip_mono_16bit() {
    let osc = SineOscillator::new(440.0).unwrap();
    let sampler = UniformSampler::new(44_100.0).unwrap();
    let sample_buffer = sampler.sample(&osc, 0.05).unwrap();

    let audio = PcmAudio::from_sample_buffer(&sample_buffer).unwrap();
    assert_eq!(audio.sample_count(), 2205);

    let wav_bytes = to_wav_bytes(&audio).unwrap();
    // 44-byte header + 2205 * 2 bytes = 4454 bytes
    assert_eq!(wav_bytes.len(), 44 + 2205 * 2);

    let decoded = from_wav_bytes(&wav_bytes).unwrap();
    assert_eq!(decoded.sample_count(), audio.sample_count());
    assert_eq!(decoded.format().sample_rate_hz(), 44_100.0);
    assert_eq!(decoded.format().channel_count(), 1);
    assert_eq!(decoded.samples(), audio.samples());
}

#[test]
fn test_wav_round_trip_stereo_16bit() {
    let format = PcmFormat::new(48_000.0, 2, 16, 1.0).unwrap();
    let mut stereo_samples = Vec::new();
    for i in 0..100 {
        stereo_samples.push(i as i16 * 100); // Left channel
        stereo_samples.push(-(i as i16 * 100)); // Right channel
    }

    let audio = PcmAudio::from_i16(stereo_samples.clone(), format, 0.0).unwrap();
    let wav_bytes = to_wav_bytes(&audio).unwrap();

    let decoded = from_wav_bytes(&wav_bytes).unwrap();
    assert_eq!(decoded.sample_count(), 200);
    assert_eq!(decoded.format().channel_count(), 2);
    assert_eq!(decoded.format().sample_rate_hz(), 48_000.0);
    assert_eq!(decoded.samples(), &stereo_samples[..]);
}

// --- Hostile Inputs & Security Bounds (CWE-400, CWE-190, CWE-20) ---

#[test]
fn test_cwe20_empty_and_truncated_wav() {
    assert!(matches!(
        from_wav_bytes(&[]).unwrap_err(),
        PcmError::MalformedWav(..)
    ));

    // Less than 44 bytes
    let truncated = [0u8; 40];
    assert!(matches!(
        from_wav_bytes(&truncated).unwrap_err(),
        PcmError::MalformedWav(..)
    ));
}

#[test]
fn test_cwe20_invalid_riff_or_wave_magic() {
    let audio = PcmAudio::from_i16(vec![0; 10], PcmFormat::default(), 0.0).unwrap();
    let mut bytes = to_wav_bytes(&audio).unwrap();

    // Corrupt RIFF
    bytes[0] = b'X';
    bytes[1] = b'X';
    assert!(matches!(
        from_wav_bytes(&bytes).unwrap_err(),
        PcmError::MalformedWav(..)
    ));

    // Restore RIFF, corrupt WAVE
    bytes[0] = b'R';
    bytes[1] = b'I';
    bytes[8] = b'X';
    assert!(matches!(
        from_wav_bytes(&bytes).unwrap_err(),
        PcmError::MalformedWav(..)
    ));
}

#[test]
fn test_cwe400_allocation_bomb_rejected_without_allocating() {
    // Construct a malicious header claiming a 4 GB data chunk, with only 44 bytes provided
    let mut bomb = vec![0u8; 44];
    bomb[0..4].copy_from_slice(b"RIFF");
    bomb[4..8].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
    bomb[8..12].copy_from_slice(b"WAVE");

    bomb[12..16].copy_from_slice(b"fmt ");
    bomb[16..20].copy_from_slice(&16u32.to_le_bytes()); // fmt size
    bomb[20..22].copy_from_slice(&1u16.to_le_bytes()); // PCM format
    bomb[22..24].copy_from_slice(&1u16.to_le_bytes()); // 1 channel
    bomb[24..28].copy_from_slice(&44100u32.to_le_bytes()); // 44100 Hz
    bomb[28..32].copy_from_slice(&(44100u32 * 2).to_le_bytes()); // byte rate
    bomb[32..34].copy_from_slice(&2u16.to_le_bytes()); // block align
    bomb[34..36].copy_from_slice(&16u16.to_le_bytes()); // 16 bits

    bomb[36..40].copy_from_slice(b"data");
    bomb[40..44].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes()); // Declares 4 GB!

    // Must be rejected cleanly without panic, OOM, or allocating 4 GB
    let err = from_wav_bytes(&bomb).unwrap_err();
    assert!(matches!(err, PcmError::MalformedWav(..)));
}

#[test]
fn test_cwe20_unsupported_audio_formats() {
    let audio = PcmAudio::from_i16(vec![0; 10], PcmFormat::default(), 0.0).unwrap();
    let mut bytes = to_wav_bytes(&audio).unwrap();

    // Set audio format tag to 2 (ADPCM) or 6 (A-law)
    bytes[20] = 2;
    bytes[21] = 0;
    assert!(matches!(
        from_wav_bytes(&bytes).unwrap_err(),
        PcmError::UnsupportedFormat { format_code: 2 }
    ));
}

#[test]
fn test_cwe20_unsupported_bit_depth() {
    let audio = PcmAudio::from_i16(vec![0; 10], PcmFormat::default(), 0.0).unwrap();
    let mut bytes = to_wav_bytes(&audio).unwrap();

    // Change bit depth in fmt chunk to 24 bits
    bytes[34] = 24;
    bytes[35] = 0;
    // Update block align accordingly so bit depth check fails cleanly
    bytes[32] = 3;
    bytes[33] = 0;
    bytes[28..32].copy_from_slice(&(44100u32 * 3).to_le_bytes());

    assert!(matches!(
        from_wav_bytes(&bytes).unwrap_err(),
        PcmError::UnsupportedBitDepth { bit_depth: 24 }
    ));
}

#[test]
fn test_cwe20_incomplete_sample_block() {
    let audio = PcmAudio::from_i16(vec![0; 10], PcmFormat::default(), 0.0).unwrap();
    let mut bytes = to_wav_bytes(&audio).unwrap();

    // Append 1 extra orphan byte making data chunk length odd
    bytes.push(0x42);
    // Update data chunk size in header
    let old_size = u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]);
    bytes[40..44].copy_from_slice(&(old_size + 1).to_le_bytes());

    assert!(matches!(
        from_wav_bytes(&bytes).unwrap_err(),
        PcmError::MalformedWav(..)
    ));
}

#[test]
fn test_cwe400_oversized_wav_byte_limit() {
    let too_large_slice = vec![0u8; MAX_WAV_BYTES + 1];
    let err = from_wav_bytes(&too_large_slice).unwrap_err();
    assert!(matches!(err, PcmError::OversizedWav { .. }));
}

#[test]
fn test_ieee_float_wav_parsing() {
    // Build a valid 32-bit float mono WAV with 4 samples: [0.0, 0.5, -0.5, 1.0]
    let float_samples: [f32; 4] = [0.0, 0.5, -0.5, 1.0];
    let data_len = 4 * 4; // 16 bytes
    let total_len = 44 + data_len;
    let riff_size = (total_len - 8) as u32;

    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&riff_size.to_le_bytes());
    wav.extend_from_slice(b"WAVE");

    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&3u16.to_le_bytes()); // 3 = IEEE float
    wav.extend_from_slice(&1u16.to_le_bytes()); // 1 channel
    wav.extend_from_slice(&48000u32.to_le_bytes());
    wav.extend_from_slice(&(48000u32 * 4).to_le_bytes()); // byte rate
    wav.extend_from_slice(&4u16.to_le_bytes()); // block align = 4
    wav.extend_from_slice(&32u16.to_le_bytes()); // 32 bits

    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data_len as u32).to_le_bytes());
    for &sample in &float_samples {
        wav.extend_from_slice(&sample.to_le_bytes());
    }

    let audio = from_wav_bytes(&wav).unwrap();
    assert_eq!(audio.sample_count(), 4);
    assert_eq!(audio.format().sample_rate_hz(), 48_000.0);
    assert_eq!(audio.samples()[0], 0);
    assert_eq!(audio.samples()[1], 16384);
    assert_eq!(audio.samples()[2], -16384);
    assert_eq!(audio.samples()[3], 32767);
}

// --- SplitMix64 PBT Round-Trip ---

#[test]
fn test_pbt_splitmix64_random_wav_round_trips() {
    let mut rng = SplitMix64::new(0x2026_1115_0102_0304);

    for _ in 0..100 {
        let sample_rate = match rng.next_u64() % 4 {
            0 => 22_050.0,
            1 => 44_100.0,
            2 => 48_000.0,
            _ => 96_000.0,
        };
        let channels = (rng.next_u64() % 2 + 1) as u16; // 1 or 2
        let format = PcmFormat::new(sample_rate, channels, 16, 1.0).unwrap();

        let count = ((rng.next_u64() % 200 + 1) * channels as u64) as usize;
        let mut samples = Vec::with_capacity(count);
        for _ in 0..count {
            let s = ((rng.next_u64() % 65536) as i32 - 32768) as i16;
            samples.push(s);
        }

        let audio = PcmAudio::from_i16(samples.clone(), format, 0.0).unwrap();
        let bytes = to_wav_bytes(&audio).unwrap();
        let decoded = from_wav_bytes(&bytes).unwrap();

        assert_eq!(decoded.sample_count(), count);
        assert_eq!(decoded.format().channel_count(), channels);
        assert_eq!(decoded.format().sample_rate_hz(), sample_rate);
        assert_eq!(decoded.samples(), &samples[..]);
    }
}
