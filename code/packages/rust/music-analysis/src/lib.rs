//! # coding_adventures_music_analysis (MUS08)
//!
//! Music Understanding, Analysis & Sample-to-Score Transcription in pure, safe Rust.
//!
//! Where `MUS00` through `MUS07` define forward synthesis, `MUS08` provides the inverse layer:
//! - **Audio Feature Extraction**: Spectral centroid, flux, rolloff, zero-crossing rate (ZCR), RMS energy.
//! - **Pitch Detection**: Iterative YIN algorithm and Harmonic Product Spectrum (HPS) with parabolic interpolation.
//! - **Onset Detection**: STFT spectral flux and Haar wavelet detail energy fusion with adaptive thresholding.
//! - **Harmonic Profile Extraction**: Single-note overtone and ADSR envelope extraction into `musical_instruments::InstrumentProfile`.
//! - **Score Transcription**: Sample-to-score transcription into `music_machine_core::PortableScore`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod dsp_utils;
pub mod features;
pub mod frequency;
pub mod harmonic;
pub mod onset;
pub mod pitch;
pub mod transcription;
pub mod types;

// Re-exports
pub use dsp_utils::{
    build_hann_window, checked_frame_bounds, compute_spectral_flux_step,
    parabolic_interpolation, rfft_magnitudes, validate_audio_inputs, validate_framing,
    validate_pcm_source_bounds,
};
pub use features::{
    rms_energy, rms_energy_framed, spectral_centroid, spectral_flux, spectral_flux_sequence,
    spectral_rolloff, zero_crossing_rate, zero_crossing_rate_framed,
};
pub use frequency::{
    frequency_to_note, frequency_to_note_a440, frequency_to_note_opt, PITCH_NAMES,
};
pub use harmonic::{extract_harmonic_profile, extract_harmonic_profile_simple};
pub use onset::{detect_onsets, detect_onsets_samples, detect_onsets_simple};
pub use pitch::{hps_pitch, hps_pitch_simple, yin_pitch, yin_pitch_simple, yin_pitch_track};
pub use transcription::{
    transcribe_pcm_buffer, transcribe_pcm_to_portable_score, transcribe_samples,
    TranscribePcmSource, TranscriptionOptions,
};
pub use types::{
    validate_finite_float, validate_positive_float, validate_samples, validate_spectrum,
    AnalysisError, NotePitch, DEFAULT_A4_HZ, MAX_BPM, MAX_FFT_LEN, MAX_FRAMES, MAX_ONSETS,
    MAX_PARTIALS, MAX_SAMPLE_RATE_HZ, MAX_SAMPLES, MIN_BPM, MIN_SAMPLE_RATE_HZ,
};
