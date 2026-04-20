use audio_device_sink::{AudioSink, AudioSinkError, NoopAudioSink, PcmFormat, PcmPlaybackBuffer};

#[test]
fn valid_format_records_pcm_metadata() {
    let format = PcmFormat::new(44_100, 1, 16).expect("format should be valid");

    assert_eq!(format.sample_rate_hz, 44_100);
    assert_eq!(format.channel_count, 1);
    assert_eq!(format.bit_depth, 16);
    assert_eq!(format.sample_width_bytes(), 2);
}

#[test]
fn format_rejects_zero_sample_rate() {
    let error = PcmFormat::new(0, 1, 16).expect_err("zero rate should fail");

    assert!(matches!(error, AudioSinkError::InvalidFormat(_)));
    assert!(error.to_string().contains("sample_rate_hz"));
}

#[test]
fn format_rejects_unsupported_channels_and_bit_depths() {
    let channels = PcmFormat::new(44_100, 2, 16).expect_err("stereo is later work");
    let bits = PcmFormat::new(44_100, 1, 24).expect_err("24-bit is later work");

    assert!(channels.to_string().contains("mono"));
    assert!(bits.to_string().contains("16-bit"));
}

#[test]
fn buffer_reports_count_and_duration() {
    let format = PcmFormat::new(4, 1, 16).expect("format should be valid");
    let buffer =
        PcmPlaybackBuffer::new(vec![0, 1000, 0, -1000], format).expect("buffer should be valid");

    assert_eq!(buffer.sample_count(), 4);
    assert_eq!(buffer.frame_count(), 4);
    assert_eq!(buffer.duration_seconds(), 1.0);
    assert!(!buffer.is_empty());
}

#[test]
fn noop_sink_returns_a_report_without_touching_hardware() {
    let format = PcmFormat::new(44_100, 1, 16).expect("format should be valid");
    let buffer = PcmPlaybackBuffer::new(vec![0, 1000], format).expect("buffer should be valid");
    let sink = NoopAudioSink::new("test-noop");

    let report = sink
        .play_blocking(&buffer)
        .expect("noop playback should succeed");

    assert_eq!(report.frames_played, 2);
    assert_eq!(report.sample_rate_hz, 44_100);
    assert_eq!(report.channel_count, 1);
    assert_eq!(report.backend_name, "test-noop");
}

#[test]
fn empty_buffers_are_valid_noop_payloads() {
    let format = PcmFormat::new(44_100, 1, 16).expect("format should be valid");
    let buffer = PcmPlaybackBuffer::new(Vec::new(), format).expect("empty buffer is valid");
    let report = NoopAudioSink::new("empty").play_blocking(&buffer).unwrap();

    assert!(buffer.is_empty());
    assert_eq!(report.frames_played, 0);
    assert_eq!(report.duration_seconds, 0.0);
}

#[test]
fn display_strings_name_error_categories() {
    assert_eq!(
        AudioSinkError::backend_unavailable("default device missing").to_string(),
        "audio backend unavailable: default device missing"
    );
    assert_eq!(
        AudioSinkError::unsupported_platform("Core Audio requires macOS").to_string(),
        "unsupported platform: Core Audio requires macOS"
    );
}
