//! Checked single-precision Direct Form II Transposed biquad sections.

use crate::{FilterError, MAX_ORDER, MAX_SAMPLES};

/// One stateful checked second-order section.
///
/// Coefficients are normalized on construction. Delays are private and change
/// only after a sample or complete block returns successfully.
#[derive(Clone, Debug)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    /// Construct from `[b0, b1, b2, a0, a1, a2]` coefficients.
    ///
    /// The five non-`a0` values are divided directly by finite, nonzero `a0`.
    /// Non-finite raw or normalized coefficients return
    /// [`FilterError::InvalidCoefficient`].
    pub fn new(coefficients: [f32; 6]) -> Result<Self, FilterError> {
        for (index, coefficient) in coefficients.iter().enumerate() {
            if !coefficient.is_finite() {
                return Err(FilterError::InvalidCoefficient(format!(
                    "coefficient[{index}] must be finite"
                )));
            }
        }

        let a0 = coefficients[3];
        if a0 == 0.0 {
            return Err(FilterError::InvalidCoefficient("a0 must be nonzero".into()));
        }

        let b0 = coefficients[0] / a0;
        let b1 = coefficients[1] / a0;
        let b2 = coefficients[2] / a0;
        let a1 = coefficients[4] / a0;
        let a2 = coefficients[5] / a0;
        for (index, normalized) in [b0, b1, b2, a1, a2].iter().enumerate() {
            if !normalized.is_finite() {
                return Err(FilterError::InvalidCoefficient(format!(
                    "normalized coefficient[{index}] must be finite"
                )));
            }
        }

        Ok(Self {
            b0,
            b1,
            b2,
            a1,
            a2,
            z1: 0.0,
            z2: 0.0,
        })
    }

    /// Process one finite sample and atomically commit its candidate delays.
    pub fn process_sample(&mut self, input: f32) -> Result<f32, FilterError> {
        if !input.is_finite() {
            return Err(FilterError::InvalidCoefficient(
                "input sample must be finite".into(),
            ));
        }

        // Keep each IEEE-754 operation separately rounded and in the
        // documented DFII-T parenthesization; do not replace with mul_add.
        let b0x = self.b0 * input;
        let output = b0x + self.z1;
        if !output.is_finite() {
            return Err(FilterError::InvalidCoefficient(
                "biquad output must be finite".into(),
            ));
        }

        let b1x = self.b1 * input;
        let a1y = self.a1 * output;
        let z1_difference = b1x - a1y;
        let next_z1 = z1_difference + self.z2;
        if !next_z1.is_finite() {
            return Err(FilterError::InvalidCoefficient(
                "candidate z1 delay must be finite".into(),
            ));
        }

        let b2x = self.b2 * input;
        let a2y = self.a2 * output;
        let next_z2 = b2x - a2y;
        if !next_z2.is_finite() {
            return Err(FilterError::InvalidCoefficient(
                "candidate z2 delay must be finite".into(),
            ));
        }

        self.z1 = next_z1;
        self.z2 = next_z2;
        Ok(output)
    }

    /// Process a nonempty block, committing state only if every sample succeeds.
    pub fn process(&mut self, input: &[f32]) -> Result<Vec<f32>, FilterError> {
        if input.is_empty() {
            return Err(FilterError::EmptySignal);
        }
        if input.len() > MAX_SAMPLES {
            return Err(FilterError::InvalidCoefficient(format!(
                "signal length {} exceeds MAX_SAMPLES {}",
                input.len(),
                MAX_SAMPLES
            )));
        }

        let mut candidate = self.clone();
        let mut output = Vec::with_capacity(input.len());
        for &sample in input {
            output.push(candidate.process_sample(sample)?);
        }
        self.z1 = candidate.z1;
        self.z2 = candidate.z2;
        Ok(output)
    }

    /// Reset both delay values to positive zero.
    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

/// Apply second-order sections to one finite signal, starting each section at
/// zero state. This operation is one-shot and does not expose streaming state.
pub fn sosfilt(sections: &[[f32; 6]], signal: &[f32]) -> Result<Vec<f32>, FilterError> {
    if signal.is_empty() {
        return Err(FilterError::EmptySignal);
    }
    if signal.len() > MAX_SAMPLES {
        return Err(FilterError::InvalidCoefficient(format!(
            "signal length {} exceeds MAX_SAMPLES {}",
            signal.len(),
            MAX_SAMPLES
        )));
    }
    if sections.len() > MAX_ORDER {
        return Err(FilterError::InvalidCoefficient(format!(
            "section count {} exceeds MAX_ORDER {}",
            sections.len(),
            MAX_ORDER
        )));
    }

    let mut filters = Vec::with_capacity(sections.len());
    for coefficients in sections {
        filters.push(Biquad::new(*coefficients)?);
    }

    if filters.is_empty() {
        let mut output = Vec::with_capacity(signal.len());
        for &sample in signal {
            if !sample.is_finite() {
                return Err(FilterError::InvalidCoefficient(
                    "input sample must be finite".into(),
                ));
            }
            output.push(sample);
        }
        return Ok(output);
    }

    let mut output = Vec::with_capacity(signal.len());
    for &sample in signal {
        let mut value = sample;
        for filter in &mut filters {
            value = filter.process_sample(value)?;
        }
        output.push(value);
    }
    Ok(output)
}
