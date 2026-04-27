use audio_device_coreaudio::{play_default_output_blocking, CoreAudioSink, BACKEND_NAME};
use audio_device_sink::{AudioSink, PcmFormat, PcmPlaybackBuffer};

#[cfg(not(target_os = "macos"))]
use audio_device_sink::AudioSinkError;

#[test]
fn backend_name_is_stable() {
    assert_eq!(BACKEND_NAME, "coreaudio");
    assert_eq!(CoreAudioSink::new().backend_name(), "coreaudio");
}

#[test]
fn empty_buffer_is_a_noop_on_every_platform() {
    let format = PcmFormat::new(44_100, 1, 16).expect("format should be valid");
    let buffer = PcmPlaybackBuffer::new(Vec::new(), format).expect("empty buffer is valid");

    let report = play_default_output_blocking(&buffer).expect("empty playback should succeed");

    assert_eq!(report.frames_played, 0);
    assert_eq!(report.backend_name, "coreaudio");
}

#[cfg(not(target_os = "macos"))]
#[test]
fn non_macos_non_empty_playback_reports_unsupported_platform() {
    let format = PcmFormat::new(44_100, 1, 16).expect("format should be valid");
    let buffer = PcmPlaybackBuffer::new(vec![0], format).expect("buffer should be valid");

    let error = CoreAudioSink::new()
        .play_blocking(&buffer)
        .expect_err("Core Audio is macOS-only");

    assert!(matches!(error, AudioSinkError::UnsupportedPlatform(_)));
    assert!(error.to_string().contains("macOS"));
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "opens the default output device; run manually when audible smoke tests are desired"]
fn macos_can_play_a_tiny_quiet_buffer() {
    let format = PcmFormat::new(44_100, 1, 16).expect("format should be valid");
    let buffer = PcmPlaybackBuffer::new(vec![0; 128], format).expect("buffer should be valid");

    let report = CoreAudioSink::new()
        .play_blocking(&buffer)
        .expect("quiet Core Audio playback should succeed");

    assert_eq!(report.frames_played, 128);
}
