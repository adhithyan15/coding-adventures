//! Property-based testing for audio-device-sink using SplitMix64 PRNG (MUS02, VER15).

use audio_device_sink::{
    checked_frame_count, checked_pcm_byte_len, validate_sample_rate_f64, AudioSink, AudioSinkError,
    NoopAudioSink, PcmFormat, PcmPlaybackBuffer, MAX_BLOCKING_DURATION_SECONDS, MAX_CHANNELS,
    MAX_SAMPLE_RATE_HZ, MIN_SAMPLE_RATE_HZ, SUPPORTED_BIT_DEPTH,
};
use coding_adventures_test_bootstrap::prng::SplitMix64;

#[test]
fn test_pbt_valid_formats_and_buffers_preserve_invariants() {
    let mut prng = SplitMix64::new(0x0A0D_10D3_71CE_51A4);
    let sink = NoopAudioSink::new("pbt-noop");

    for _ in 0..200 {
        let rate = MIN_SAMPLE_RATE_HZ + (prng.next_u64() as u32 % MAX_SAMPLE_RATE_HZ);
        let format = PcmFormat::new(rate, MAX_CHANNELS, SUPPORTED_BIT_DEPTH)
            .expect("generated rate must be valid");
        assert_eq!(
            validate_sample_rate_f64(rate as f64).expect("f64 rate"),
            rate
        );

        let max_samples = format.max_samples().expect("max_samples");
        let len = (prng.next_u64() as usize) % (max_samples.min(4096) + 1);
        let samples: Vec<i16> = (0..len).map(|_| prng.next_u64() as i16).collect();

        let buffer = PcmPlaybackBuffer::new(samples.clone(), format).expect("valid buffer");
        assert_eq!(buffer.sample_count(), len);
        assert_eq!(
            buffer.frame_count(),
            checked_frame_count(len, format.channel_count).expect("frame_count")
        );
        assert_eq!(
            buffer.byte_len().expect("byte_len"),
            checked_pcm_byte_len(len, format).expect("checked_pcm_byte_len")
        );
        assert_eq!(buffer.byte_len().expect("byte_len"), len * 2);

        let dur = buffer.duration_seconds();
        assert!(dur.is_finite());
        assert!((0.0..=MAX_BLOCKING_DURATION_SECONDS).contains(&dur));

        let r1 = sink.play_blocking(&buffer).expect("noop play 1");
        let r2 = sink.play_blocking(&buffer).expect("noop play 2");
        assert_eq!(r1, r2);
        assert_eq!(r1.frames_played, len);
        assert_eq!(r1.sample_rate_hz, rate);
        assert_eq!(r1.channel_count, 1);
    }
}

#[test]
fn test_pbt_hostile_formats_and_oversized_counts_fail_without_panic() {
    let mut prng = SplitMix64::new(0x0BAD_F00D_1234_5678);

    for _ in 0..200 {
        let rate = (MAX_SAMPLE_RATE_HZ + 1) + (prng.next_u64() as u32 % 1_000_000);
        assert!(matches!(
            PcmFormat::new(rate, 1, 16),
            Err(AudioSinkError::InvalidFormat(_))
        ));

        let bad_ch = 2 + (prng.next_u64() as u16 % 100);
        assert!(matches!(
            PcmFormat::new(44_100, bad_ch, 16),
            Err(AudioSinkError::InvalidFormat(_))
        ));

        let bad_bits = match prng.next_u64() % 4 {
            0 => 0,
            1 => 8,
            2 => 24,
            _ => 32,
        };
        assert!(matches!(
            PcmFormat::new(44_100, 1, bad_bits),
            Err(AudioSinkError::InvalidFormat(_))
        ));

        let valid_rate = 1 + (prng.next_u64() as u32 % 192_000);
        let fmt = PcmFormat::new(valid_rate, 1, 16).expect("valid rate");
        let over =
            fmt.max_samples().expect("max_samples") + 1 + (prng.next_u64() as usize % 10_000);
        assert!(matches!(
            PcmPlaybackBuffer::validate_sample_count(over, fmt),
            Err(AudioSinkError::InvalidSamples(_))
        ));
    }
}
