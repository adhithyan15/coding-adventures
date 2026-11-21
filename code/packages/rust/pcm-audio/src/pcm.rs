//! PCM audio container, quantization, and clamping functions.

use crate::format::{
    PcmError, PcmFormat, MAX_DURATION_SECONDS, MAX_SAMPLES, PCM16_MAX, PCM16_MIN,
};
use oscillator::SampleBuffer;

/// Clamp an arbitrary signed 64-bit integer sample into the signed 16-bit PCM range
/// $[-32768, 32767]$ without wrap-around overflow (CWE-190).
pub fn clamp_to_pcm16(v: i64) -> i16 {
    v.clamp(PCM16_MIN as i64, PCM16_MAX as i64) as i16
}

/// Quantize a single normalized floating-point sample into a signed 16-bit PCM integer.
///
/// Returns `Ok((i16_sample, was_clipped))`.
///
/// Mathematical specification from MUS01 §18:
/// - $x \in [0.0, 1.0] \implies \text{round}(x \cdot 32767)$
/// - $x \in [-1.0, 0.0) \implies \text{round}(x \cdot 32768)$
/// - Values outside $[-1.0, 1.0]$ are clamped to the respective boundary and flag `was_clipped = true`.
/// - Non-finite values (`NaN`, `±Inf`) are rejected fail-closed with [`PcmError::NonFiniteSample`].
pub fn safe_round_sample(sample: f64) -> Result<(i16, bool), PcmError> {
    if !sample.is_finite() {
        return Err(PcmError::NonFiniteSample { value: sample });
    }

    let clipped = !(-1.0..=1.0).contains(&sample);
    let bounded = sample.clamp(-1.0, 1.0);

    let raw = if bounded >= 0.0 {
        (bounded * (PCM16_MAX as f64)).round()
    } else {
        (bounded * (-(PCM16_MIN as f64))).round()
    };

    // Clamping protects against float precision anomalies (CWE-190)
    Ok((clamp_to_pcm16(raw as i64), clipped))
}

/// Quantize a 32-bit float sample into signed 16-bit PCM.
pub fn safe_round_sample_f32(sample: f32) -> Result<(i16, bool), PcmError> {
    safe_round_sample(sample as f64)
}

/// Alias for `safe_round_sample`, matching Python `float_to_pcm16` naming convention.
pub fn float_to_pcm16(sample: f64) -> Result<(i16, bool), PcmError> {
    safe_round_sample(sample)
}

/// Convert a signed 16-bit PCM integer back to a normalized `f32` sample in $[-1.0, 1.0]$.
pub fn pcm16_to_f32(sample: i16) -> f32 {
    if sample >= 0 {
        (sample as f32) / (PCM16_MAX as f32)
    } else {
        (sample as f32) / (-(PCM16_MIN as f32))
    }
}

/// Convert a signed 16-bit PCM integer back to a normalized `f64` sample in $[-1.0, 1.0]$.
pub fn pcm16_to_f64(sample: i16) -> f64 {
    if sample >= 0 {
        (sample as f64) / (PCM16_MAX as f64)
    } else {
        (sample as f64) / (-(PCM16_MIN as f64))
    }
}

/// Container for PCM samples with timing, format, and clipping metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct PcmAudio {
    samples: Vec<i16>,
    format: PcmFormat,
    start_time_seconds: f64,
    pub(crate) clipped_sample_count: usize,
}

impl PcmAudio {
    /// Construct a `PcmAudio` buffer from pre-quantized 16-bit PCM integers.
    pub fn from_i16(
        samples: Vec<i16>,
        format: PcmFormat,
        start_time_seconds: f64,
    ) -> Result<Self, PcmError> {
        if samples.len() > MAX_SAMPLES {
            return Err(PcmError::SampleCountTooLarge {
                count: samples.len(),
                max: MAX_SAMPLES,
            });
        }
        if !start_time_seconds.is_finite() {
            return Err(PcmError::NonFiniteNumber {
                name: "start_time_seconds",
                value: start_time_seconds,
            });
        }

        let duration = (samples.len() as f64) / format.sample_rate_hz();
        if duration > MAX_DURATION_SECONDS {
            return Err(PcmError::DurationTooLong {
                duration,
                max: MAX_DURATION_SECONDS,
            });
        }

        Ok(Self {
            samples,
            format,
            start_time_seconds,
            clipped_sample_count: 0,
        })
    }

    /// Construct a `PcmAudio` buffer from a slice of `f64` samples, quantizing to 16-bit PCM.
    pub fn from_f64_slice(
        samples: &[f64],
        format: PcmFormat,
        start_time_seconds: f64,
    ) -> Result<Self, PcmError> {
        if samples.len() > MAX_SAMPLES {
            return Err(PcmError::SampleCountTooLarge {
                count: samples.len(),
                max: MAX_SAMPLES,
            });
        }
        if !start_time_seconds.is_finite() {
            return Err(PcmError::NonFiniteNumber {
                name: "start_time_seconds",
                value: start_time_seconds,
            });
        }

        let duration = (samples.len() as f64) / format.sample_rate_hz();
        if duration > MAX_DURATION_SECONDS {
            return Err(PcmError::DurationTooLong {
                duration,
                max: MAX_DURATION_SECONDS,
            });
        }

        // Checked pre-allocation
        let mut pcm_samples = Vec::with_capacity(samples.len());
        let mut clipped_count = 0;

        for &sample in samples {
            let (pcm, clipped) = safe_round_sample(sample)?;
            pcm_samples.push(pcm);
            if clipped {
                clipped_count += 1;
            }
        }

        Ok(Self {
            samples: pcm_samples,
            format,
            start_time_seconds,
            clipped_sample_count: clipped_count,
        })
    }

    /// Construct a `PcmAudio` buffer from a slice of `f32` samples.
    pub fn from_f32_slice(
        samples: &[f32],
        format: PcmFormat,
        start_time_seconds: f64,
    ) -> Result<Self, PcmError> {
        if samples.len() > MAX_SAMPLES {
            return Err(PcmError::SampleCountTooLarge {
                count: samples.len(),
                max: MAX_SAMPLES,
            });
        }
        if !start_time_seconds.is_finite() {
            return Err(PcmError::NonFiniteNumber {
                name: "start_time_seconds",
                value: start_time_seconds,
            });
        }

        let duration = (samples.len() as f64) / format.sample_rate_hz();
        if duration > MAX_DURATION_SECONDS {
            return Err(PcmError::DurationTooLong {
                duration,
                max: MAX_DURATION_SECONDS,
            });
        }

        let mut pcm_samples = Vec::with_capacity(samples.len());
        let mut clipped_count = 0;

        for &sample in samples {
            let (pcm, clipped) = safe_round_sample_f32(sample)?;
            pcm_samples.push(pcm);
            if clipped {
                clipped_count += 1;
            }
        }

        Ok(Self {
            samples: pcm_samples,
            format,
            start_time_seconds,
            clipped_sample_count: clipped_count,
        })
    }

    /// Ingest a continuous oscillator [`SampleBuffer`] using default mono 16-bit PCM format.
    pub fn from_sample_buffer(sample_buffer: &SampleBuffer) -> Result<Self, PcmError> {
        let format = PcmFormat::mono_16bit(sample_buffer.sample_rate_hz())?;
        Self::from_sample_buffer_with_format(sample_buffer, format)
    }

    /// Ingest a continuous oscillator [`SampleBuffer`] with a specific [`PcmFormat`].
    pub fn from_sample_buffer_with_format(
        sample_buffer: &SampleBuffer,
        format: PcmFormat,
    ) -> Result<Self, PcmError> {
        Self::from_f64_slice(
            sample_buffer.samples(),
            format,
            sample_buffer.start_time_seconds(),
        )
    }

    /// Return the count of stored PCM samples.
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    /// Return the spacing between adjacent samples in seconds ($1 / f_s$).
    pub fn sample_period_seconds(&self) -> f64 {
        1.0 / self.format.sample_rate_hz()
    }

    /// Return the half-open duration covered by this audio buffer in seconds.
    pub fn duration_seconds(&self) -> f64 {
        self.samples.len() as f64 / self.format.sample_rate_hz()
    }

    /// Return the timestamp for a given sample index.
    pub fn time_at(&self, index: usize) -> Result<f64, PcmError> {
        if index >= self.samples.len() {
            return Err(PcmError::IndexOutOfBounds {
                index,
                count: self.samples.len(),
            });
        }
        let timestamp = self.start_time_seconds + (index as f64) / self.format.sample_rate_hz();
        if !timestamp.is_finite() {
            return Err(PcmError::IntegerOverflow("sample timestamp"));
        }
        Ok(timestamp)
    }

    /// Immutably borrow the slice of signed 16-bit PCM integers.
    pub fn samples(&self) -> &[i16] {
        &self.samples
    }

    /// Consume the buffer and return the underlying sample vector.
    pub fn into_samples(self) -> Vec<i16> {
        self.samples
    }

    /// Immutably borrow the format metadata.
    pub fn format(&self) -> &PcmFormat {
        &self.format
    }

    /// Return the start timestamp in seconds.
    pub fn start_time_seconds(&self) -> f64 {
        self.start_time_seconds
    }

    /// Return how many samples had to be clipped to $[-1.0, 1.0]$ during quantization.
    pub fn clipped_sample_count(&self) -> usize {
        self.clipped_sample_count
    }

    /// Convert stored PCM samples to normalized `f32` numbers in $[-1.0, 1.0]$.
    pub fn to_f32_vec(&self) -> Vec<f32> {
        self.samples.iter().copied().map(pcm16_to_f32).collect()
    }

    /// Pack signed 16-bit PCM integers into little-endian bytes.
    pub fn to_little_endian_bytes(&self) -> Result<Vec<u8>, PcmError> {
        let byte_len = self
            .samples
            .len()
            .checked_mul(2)
            .ok_or(PcmError::IntegerOverflow("byte packing"))?;

        if byte_len > crate::format::MAX_WAV_BYTES {
            return Err(PcmError::OversizedWav {
                size: byte_len,
                max: crate::format::MAX_WAV_BYTES,
            });
        }

        let mut bytes = Vec::with_capacity(byte_len);
        for &sample in &self.samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        Ok(bytes)
    }

    /// Serialize this audio buffer into canonical RIFF/WAVE format bytes.
    pub fn to_wav_bytes(&self) -> Result<Vec<u8>, PcmError> {
        crate::wav::to_wav_bytes(self)
    }

    /// Parse RIFF/WAVE bytes into a `PcmAudio` buffer.
    pub fn from_wav_bytes(bytes: &[u8]) -> Result<Self, PcmError> {
        crate::wav::from_wav_bytes(bytes)
    }
}

/// Convenience helper to encode a [`SampleBuffer`] into a `PcmAudio` buffer.
pub fn encode_sample_buffer(sample_buffer: &SampleBuffer) -> Result<PcmAudio, PcmError> {
    PcmAudio::from_sample_buffer(sample_buffer)
}

/// Convenience helper for tests and demos that start with raw float samples.
pub fn samples_to_pcm_buffer(
    samples: &[f64],
    sample_rate_hz: f64,
) -> Result<PcmAudio, PcmError> {
    let format = PcmFormat::mono_16bit(sample_rate_hz)?;
    PcmAudio::from_f64_slice(samples, format, 0.0)
}
