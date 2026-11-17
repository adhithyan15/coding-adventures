//! Continuous instrument voice signal generator (MUS05 §10).

use std::f64::consts::PI;

use crate::envelope::{
    validate_duration, validate_non_negative, validate_positive, validate_unit, AdsrEnvelope,
    MAX_NOTE_DURATION_SECONDS,
};
use crate::error::InstrumentError;
use crate::partial::HarmonicPartial;
use crate::profile::{InstrumentProfile, MAX_PARTIALS};
use crate::splitmix64::SplitMix64;
use crate::variation::VariationProfile;

/// Maximum allowable sample rate in Hertz (CWE-400 guard).
pub const MAX_SAMPLE_RATE_HZ: f64 = 192_000.0;

/// Default audio sampling rate in Hertz.
pub const DEFAULT_SAMPLE_RATE_HZ: f64 = 44_100.0;

const TWO_PI: f64 = 2.0 * PI;

/// Continuous signal representing one musical instrument playing one note.
#[derive(Debug, Clone, PartialEq)]
pub struct InstrumentSignal {
    /// Fundamental note frequency in Hertz.
    pub fundamental_hz: f64,
    /// Timbre instrument profile.
    pub instrument: InstrumentProfile,
    /// Note duration in seconds (before release tail).
    pub note_duration_seconds: f64,
    /// Event velocity/amplitude multiplier in [0.0, 1.0].
    pub amplitude: f64,
    /// Sampling rate in Hertz for Nyquist ceiling evaluation.
    pub sample_rate_hz: f64,
    /// Active variation profile.
    pub variation_profile: VariationProfile,
    /// Effective fundamental frequency after pitch jitter.
    pub effective_fundamental_hz: f64,
    /// Effective amplitude after velocity/amplitude jitter.
    pub effective_amplitude: f64,
    /// Effective envelope after attack jitter.
    pub effective_envelope: AdsrEnvelope,
    /// Effective harmonic partials after harmonic jitter.
    pub effective_harmonic_profile: Vec<HarmonicPartial>,
    normalizer: f64,
}

impl InstrumentSignal {
    /// Construct a new validated instrument signal.
    pub fn new(
        fundamental_hz: f64,
        instrument: InstrumentProfile,
        note_duration_seconds: f64,
        amplitude: f64,
        sample_rate_hz: f64,
        variation_profile: Option<VariationProfile>,
    ) -> Result<Self, InstrumentError> {
        let fund_hz = validate_positive("fundamental_hz", fundamental_hz)?;
        let dur = validate_duration("note_duration_seconds", note_duration_seconds)?;
        let amp = validate_unit("amplitude", amplitude)?;
        let sr = validate_sample_rate("sample_rate_hz", sample_rate_hz)?;

        if instrument.harmonic_profile.len() > MAX_PARTIALS {
            return Err(InstrumentError::BudgetExceeded {
                reason: format!(
                    "instrument.harmonic_profile partial count {} exceeds MAX_PARTIALS={MAX_PARTIALS}",
                    instrument.harmonic_profile.len()
                ),
            });
        }

        let var = variation_profile.unwrap_or_else(|| instrument.variation_profile.clone());

        let (eff_fund, eff_amp, eff_env, eff_partials) = if let Some(seed) = var.seed {
            let mut rng = SplitMix64::new(seed);

            // 1. Pitch jitter in cents: f' = f * 2^(cents / 1200)
            let pitch_draw = rng.next_float_signed();
            let delta_cents = pitch_draw * var.pitch_cents_jitter;
            let ratio = 2.0_f64.powf(delta_cents / 1200.0);
            let eff_fund = fund_hz * ratio;
            if !eff_fund.is_finite() || eff_fund <= 0.0 {
                return Err(InstrumentError::InvalidParameter {
                    name: "effective_fundamental_hz",
                    reason: format!("jittered fundamental must be positive finite, got {eff_fund}"),
                });
            }

            // 2. Amplitude jitter
            let amp_draw = rng.next_float_signed();
            let amp_factor = 1.0 + amp_draw * var.amplitude_jitter;
            let eff_amp = (amp * amp_factor).clamp(0.0, 1.0);

            // 3. Attack timing jitter (ms to seconds)
            let atk_draw = rng.next_float_signed();
            let atk_delta = atk_draw * (var.attack_jitter_ms / 1000.0);
            let base_env = &instrument.envelope_profile;
            let eff_atk =
                (base_env.attack_seconds + atk_delta).clamp(0.0, MAX_NOTE_DURATION_SECONDS);
            let eff_env = AdsrEnvelope::new(
                eff_atk,
                base_env.decay_seconds,
                base_env.sustain_level,
                base_env.release_seconds,
            )?;

            // 4. Harmonic overtone jitter
            let eff_partials = if var.harmonic_jitter > 0.0 {
                let mut varied = Vec::with_capacity(instrument.harmonic_profile.len());
                for p in &instrument.harmonic_profile {
                    let h_draw = rng.next_float_signed();
                    let p_factor = 1.0 + h_draw * var.harmonic_jitter;
                    let p_amp = (p.amplitude * p_factor).max(0.0);
                    varied.push(HarmonicPartial::with_phase(
                        p.frequency_multiplier,
                        p_amp,
                        p.phase_cycles,
                    )?);
                }
                varied
            } else {
                instrument.harmonic_profile.clone()
            };

            (eff_fund, eff_amp, eff_env, eff_partials)
        } else {
            (
                fund_hz,
                amp,
                instrument.envelope_profile,
                instrument.harmonic_profile.clone(),
            )
        };

        let total: f64 = eff_partials.iter().map(|p| p.amplitude).sum();
        if !total.is_finite() {
            return Err(InstrumentError::NonFiniteNumber {
                name: "amplitude_normalizer",
                value: total,
            });
        }
        let normalizer = if total > 0.0 { total } else { 1.0 };

        Ok(Self {
            fundamental_hz: fund_hz,
            instrument,
            note_duration_seconds: dur,
            amplitude: amp,
            sample_rate_hz: sr,
            variation_profile: var,
            effective_fundamental_hz: eff_fund,
            effective_amplitude: eff_amp,
            effective_envelope: eff_env,
            effective_harmonic_profile: eff_partials,
            normalizer,
        })
    }

    /// Compute the instantaneous signal amplitude at note-local `time_seconds`.
    pub fn value_at(&self, time_seconds: f64) -> Result<f64, InstrumentError> {
        let time = validate_non_negative("time_seconds", time_seconds)?;

        if self.instrument.synthesis_kind == "silence" {
            return Ok(0.0);
        }

        let nyquist_hz = self.sample_rate_hz / 2.0;
        let mut raw = 0.0;
        let eff_fund = self.effective_fundamental_hz;

        for partial in &self.effective_harmonic_profile {
            let partial_hz = eff_fund * partial.frequency_multiplier;
            if partial_hz >= nyquist_hz {
                continue;
            }
            // Reduce to [0, 1) cycles first: trig::sin is NaN beyond 2^29 radians
            // (PHY00a 3.4.1, 7).  The subtraction is exact.
            let phase = partial_hz * time + partial.phase_cycles;
            raw += partial.amplitude * trig::sin(TWO_PI * (phase - phase.floor()));
        }

        let envelope = self
            .effective_envelope
            .value_at(time, self.note_duration_seconds)?;

        let sample = self.effective_amplitude * self.instrument.gain * envelope * raw / self.normalizer;
        if !sample.is_finite() {
            return Err(InstrumentError::NonFiniteNumber {
                name: "sample",
                value: sample,
            });
        }
        Ok(sample)
    }
}

impl oscillator::ContinuousSignal for InstrumentSignal {
    fn value_at(&self, time_seconds: f64) -> Result<f64, oscillator::OscillatorError> {
        self.value_at(time_seconds).map_err(|e| match e {
            InstrumentError::NonFiniteNumber { name, value } => {
                oscillator::OscillatorError::NonFiniteNumber { name, value }
            }
            InstrumentError::NegativeNumber { name, value } => {
                oscillator::OscillatorError::NegativeNumber { name, value }
            }
            other => oscillator::OscillatorError::InvalidParameter {
                name: "time_seconds",
                reason: other.to_string(),
            },
        })
    }
}

pub(crate) fn validate_sample_rate(
    name: &'static str,
    value: f64,
) -> Result<f64, InstrumentError> {
    let positive = validate_positive(name, value)?;
    if positive > MAX_SAMPLE_RATE_HZ {
        return Err(InstrumentError::BudgetExceeded {
            reason: format!(
                "{name} {positive} exceeds MAX_SAMPLE_RATE_HZ={MAX_SAMPLE_RATE_HZ}"
            ),
        });
    }
    Ok(positive)
}
