//! Hostile input tests for CWE-20, CWE-190, and CWE-400 bounds (VER15).

use audio_device_sink::{
    checked_frame_count, checked_pcm_byte_len, checked_sample_count_for_duration,
    validate_duration_seconds, validate_finite_samples, validate_format, validate_sample_rate_f64,
    AudioSinkError, PcmFormat, PcmPlaybackBuffer, MAX_BLOCKING_DURATION_SECONDS, MAX_CHANNELS,
    MAX_ERROR_MESSAGE_BYTES, MAX_PCM_BYTES, MAX_SAMPLES, MAX_SAMPLE_RATE_HZ, MIN_SAMPLE_RATE_HZ,
    SUPPORTED_BIT_DEPTH,
};

#[test]
fn test_hostile_cwe_20_format_rejections() {
    assert!(matches!(
        validate_format(0, 1, 16),
        Err(AudioSinkError::InvalidFormat(_))
    ));
    assert!(matches!(
        validate_format(MAX_SAMPLE_RATE_HZ + 1, 1, 16),
        Err(AudioSinkError::InvalidFormat(_))
    ));
    assert!(matches!(
        validate_format(u32::MAX, 1, 16),
        Err(AudioSinkError::InvalidFormat(_))
    ));
    assert!(matches!(
        validate_format(44_100, 0, 16),
        Err(AudioSinkError::InvalidFormat(_))
    ));
    assert!(matches!(
        validate_format(44_100, MAX_CHANNELS + 1, 16),
        Err(AudioSinkError::InvalidFormat(_))
    ));
    assert!(matches!(
        validate_format(44_100, u16::MAX, 16),
        Err(AudioSinkError::InvalidFormat(_))
    ));
    for bad_depth in [0, 8, 24, 32, u16::MAX] {
        assert!(matches!(
            validate_format(44_100, 1, bad_depth),
            Err(AudioSinkError::InvalidFormat(_))
        ));
    }
    assert!(validate_format(MIN_SAMPLE_RATE_HZ, 1, SUPPORTED_BIT_DEPTH).is_ok());
    assert!(validate_format(MAX_SAMPLE_RATE_HZ, 1, SUPPORTED_BIT_DEPTH).is_ok());
}

#[test]
fn test_hostile_cwe_20_non_finite_rates_durations_and_samples() {
    for bad_rate in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        0.0,
        -1.0,
        -44_100.0,
        44_100.5,
        (MAX_SAMPLE_RATE_HZ as f64) + 1.0,
        1e20,
    ] {
        assert!(
            matches!(
                validate_sample_rate_f64(bad_rate),
                Err(AudioSinkError::InvalidFormat(_))
            ),
            "expected InvalidFormat for rate {bad_rate:?}"
        );
    }

    for bad_duration in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -0.001,
        -100.0,
        MAX_BLOCKING_DURATION_SECONDS + 0.001,
        1e12,
    ] {
        assert!(
            validate_duration_seconds(bad_duration).is_err(),
            "expected error for duration {bad_duration:?}"
        );
    }

    for bad_sample in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(matches!(
            validate_finite_samples(&[0.0, bad_sample, 0.5]),
            Err(AudioSinkError::InvalidSamples(_))
        ));
    }
}

#[test]
fn test_hostile_cwe_190_and_400_checked_math_and_caps() {
    let format = PcmFormat::new(44_100, 1, 16).expect("valid format");

    // Pre-flight integer overflow / resource exhaustion checks without allocating
    assert!(matches!(
        checked_pcm_byte_len(usize::MAX, format),
        Err(AudioSinkError::InvalidSamples(_))
    ));
    assert!(matches!(
        checked_pcm_byte_len(MAX_SAMPLES + 1, format),
        Err(AudioSinkError::InvalidSamples(_))
    ));
    assert!(matches!(
        PcmPlaybackBuffer::validate_sample_count(usize::MAX, format),
        Err(AudioSinkError::InvalidSamples(_))
    ));
    assert!(matches!(
        checked_frame_count(usize::MAX, 1),
        Err(AudioSinkError::InvalidSamples(_))
    ));
    assert!(matches!(
        checked_frame_count(MAX_SAMPLES + 1, 1),
        Err(AudioSinkError::InvalidSamples(_))
    ));
    assert!(matches!(
        checked_frame_count(10, 0),
        Err(AudioSinkError::InvalidFormat(_))
    ));
    assert!(matches!(
        checked_frame_count(10, 2),
        Err(AudioSinkError::InvalidFormat(_))
    ));

    // Exact boundary at sample_rate_hz = 1 (max 600 samples)
    let slow = PcmFormat::new(MIN_SAMPLE_RATE_HZ, 1, 16).expect("valid slow format");
    assert_eq!(slow.max_samples().expect("max_samples"), 600);
    let max_ok = PcmPlaybackBuffer::new(vec![0; 600], slow).expect("600 samples at 1 Hz is valid");
    assert_eq!(max_ok.byte_len().expect("byte_len"), 1200);
    assert!(matches!(
        PcmPlaybackBuffer::new(vec![0; 601], slow),
        Err(AudioSinkError::InvalidSamples(_))
    ));

    // Duration-to-sample conversion bounds
    assert_eq!(
        checked_sample_count_for_duration(format, 0.0).expect("zero duration"),
        0
    );
    assert_eq!(
        checked_sample_count_for_duration(format, MAX_BLOCKING_DURATION_SECONDS)
            .expect("max duration"),
        44_100 * 600
    );
    assert!(
        checked_sample_count_for_duration(format, MAX_BLOCKING_DURATION_SECONDS + 1.0).is_err()
    );
    assert!(checked_sample_count_for_duration(format, f64::NAN).is_err());

    // Max rate boundary
    let fast = PcmFormat::new(MAX_SAMPLE_RATE_HZ, 1, 16).expect("valid max format");
    assert_eq!(fast.max_samples().expect("max_samples"), MAX_SAMPLES);
    assert_eq!(
        checked_pcm_byte_len(MAX_SAMPLES, fast).expect("max byte len"),
        MAX_PCM_BYTES
    );
    assert_eq!(fast.frame_width_bytes().expect("frame width"), 2);
}

#[test]
fn test_hostile_error_messages_are_o1_bounded() {
    let huge = "x".repeat(10_000);
    let err = AudioSinkError::invalid_samples(huge);
    let AudioSinkError::InvalidSamples(msg) = err else {
        panic!("expected InvalidSamples");
    };
    assert!(msg.len() <= MAX_ERROR_MESSAGE_BYTES);
    assert!(msg.ends_with("..."));

    // Multi-byte UTF-8 boundary truncation must never split a codepoint or panic
    let emoji = "🎵".repeat(200);
    let utf8_err = AudioSinkError::invalid_format(emoji);
    let AudioSinkError::InvalidFormat(utf8_msg) = utf8_err else {
        panic!("expected InvalidFormat");
    };
    assert!(utf8_msg.len() <= MAX_ERROR_MESSAGE_BYTES);
    assert!(utf8_msg.ends_with("..."));
}
