// ============================================================================
// wave — Sinusoidal wave modeling from first principles
// ============================================================================
//
// A wave is one of the most fundamental patterns in physics.  Sound, light,
// radio signals, ocean tides, AC electricity — they all follow the same
// mathematical shape: a **sinusoid**.
//
// The general equation for a sinusoidal wave is:
//
//   y(t) = A * sin(2 * pi * f * t + phi)
//
// Where:
//   - A   = amplitude  — the peak height of the wave (must be >= 0)
//   - f   = frequency  — how many complete cycles occur per second (Hz)
//   - t   = time       — the independent variable (seconds)
//   - phi = phase      — shifts the wave left or right along the time axis
//
// ============================================================================
// Why these parameters matter
// ============================================================================
//
// Imagine plucking a guitar string:
//
//   - **Amplitude** controls how loud the note is.  A bigger pluck means a
//     taller wave, which our ears perceive as louder sound.
//
//   - **Frequency** controls the pitch.  Concert A is 440 Hz — the string
//     vibrates 440 complete cycles every second.  Double the frequency to
//     880 Hz and you hear the note one octave higher.
//
//   - **Phase** controls *when* the wave starts.  Two speakers playing the
//     same note but out of phase can cancel each other out (destructive
//     interference) or reinforce each other (constructive interference).
//
// ============================================================================
// Derived quantities
// ============================================================================
//
// From frequency we can derive two useful values:
//
//   - **Period** (T) = 1 / f
//     The duration of one complete cycle.  A 440 Hz wave has a period of
//     about 2.27 milliseconds.
//
//   - **Angular frequency** (omega) = 2 * pi * f
//     Frequency expressed in radians per second instead of cycles per second.
//     This is the natural unit for the sine function, since sin() takes
//     radians.  One full cycle = 2*pi radians.
//
// ============================================================================

use trig;

// ============================================================================
// The Wave struct
// ============================================================================
//
// We store the three defining parameters of a sinusoidal wave.  Together
// they fully describe the wave's shape at any point in time.
//
// We derive Debug (for printing), Clone (for copying), and PartialEq (for
// comparison in tests).

/// A sinusoidal wave defined by amplitude, frequency, and phase.
///
/// # Example
///
/// ```
/// use wave::Wave;
///
/// // A 440 Hz sine wave (concert A) with unit amplitude and no phase shift.
/// let a440 = Wave::new(1.0, 440.0, 0.0).unwrap();
///
/// // Evaluate the wave at t = 0 seconds — sin(0) = 0.
/// assert!((a440.evaluate(0.0)).abs() < 1e-10);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Wave {
    /// Peak displacement from zero.  Must be non-negative.
    ///
    /// An amplitude of 0 produces a flat line (silence, no signal).
    /// An amplitude of 1 is often used as a "unit wave" for normalization.
    pub amplitude: f64,

    /// Number of complete cycles per second, measured in Hertz (Hz).
    ///
    /// Must be strictly positive — a wave with zero frequency would never
    /// oscillate, which contradicts the definition of a wave.
    pub frequency: f64,

    /// Phase offset in radians.
    ///
    /// A phase of 0 means the wave starts at zero and rises.
    /// A phase of PI/2 means the wave starts at its peak (cosine shape).
    /// A phase of PI means the wave starts at zero and falls (inverted).
    pub phase: f64,
}

// ============================================================================
// Implementation
// ============================================================================

impl Wave {
    // ------------------------------------------------------------------------
    // Constructor with validation
    // ------------------------------------------------------------------------
    //
    // We enforce physical constraints:
    //   - Amplitude >= 0: negative amplitude has no physical meaning.
    //     (A "negative loudness" or "negative brightness" doesn't exist.)
    //   - Frequency > 0: a wave must oscillate.  Zero frequency is a DC
    //     offset, not a wave.
    //
    // We return Result rather than panicking, because invalid parameters
    // are a normal user error, not a bug in the program.

    /// Create a new sinusoidal wave with the given parameters.
    ///
    /// # Errors
    ///
    /// Returns `Err` if:
    /// - `amplitude` is negative (must be >= 0)
    /// - `frequency` is zero or negative (must be > 0)
    ///
    /// # Examples
    ///
    /// ```
    /// use wave::Wave;
    ///
    /// let w = Wave::new(2.0, 100.0, 0.0).unwrap();
    /// assert_eq!(w.amplitude, 2.0);
    /// assert_eq!(w.frequency, 100.0);
    /// assert_eq!(w.phase, 0.0);
    /// ```
    pub fn new(amplitude: f64, frequency: f64, phase: f64) -> Result<Self, &'static str> {
        // Amplitude check: the wave's peak value cannot be negative.
        if amplitude < 0.0 {
            return Err("amplitude must be non-negative");
        }

        // Frequency check: the wave must oscillate at least once per second.
        if frequency <= 0.0 {
            return Err("frequency must be positive");
        }

        Ok(Wave {
            amplitude,
            frequency,
            phase,
        })
    }

    // ------------------------------------------------------------------------
    // Period
    // ------------------------------------------------------------------------
    //
    // The period is the inverse of frequency:
    //
    //   T = 1 / f
    //
    // If the wave completes 440 cycles per second (f = 440 Hz), each cycle
    // takes 1/440 ≈ 0.00227 seconds ≈ 2.27 milliseconds.
    //
    // Frequency and period are two ways of expressing the same idea:
    //   - Frequency: "how many cycles fit in one second?"
    //   - Period:    "how long does one cycle take?"

    /// The duration of one complete cycle, in seconds.
    ///
    /// # Formula
    ///
    /// `T = 1 / f`
    ///
    /// # Examples
    ///
    /// ```
    /// use wave::Wave;
    ///
    /// let w = Wave::new(1.0, 4.0, 0.0).unwrap();
    /// assert!((w.period() - 0.25).abs() < 1e-10);
    /// ```
    pub fn period(&self) -> f64 {
        1.0 / self.frequency
    }

    // ------------------------------------------------------------------------
    // Angular Frequency
    // ------------------------------------------------------------------------
    //
    // Angular frequency converts from "cycles per second" to "radians per
    // second."  Since one full cycle = 2*pi radians:
    //
    //   omega = 2 * pi * f
    //
    // This is the coefficient of `t` inside the sine function.  When we
    // write sin(omega * t + phi), at t = T (one period), omega * T = 2*pi,
    // so sin wraps around exactly once.  That's the whole point.

    /// The angular frequency in radians per second.
    ///
    /// # Formula
    ///
    /// `omega = 2 * pi * f`
    ///
    /// # Examples
    ///
    /// ```
    /// use wave::Wave;
    /// use trig::PI;
    ///
    /// let w = Wave::new(1.0, 1.0, 0.0).unwrap();
    /// assert!((w.angular_frequency() - 2.0 * PI).abs() < 1e-10);
    /// ```
    pub fn angular_frequency(&self) -> f64 {
        2.0 * trig::PI * self.frequency
    }

    // ------------------------------------------------------------------------
    // Evaluate
    // ------------------------------------------------------------------------
    //
    // This is the core function: given a time `t`, compute the wave's value.
    //
    //   y(t) = A * sin(2 * pi * f * t + phi)
    //
    // Let's trace through a concrete example to build intuition:
    //
    //   Wave: A=3, f=2 Hz, phi=0
    //   At t=0:     y = 3 * sin(0)        = 0
    //   At t=0.125: y = 3 * sin(pi/2)     = 3     (quarter cycle, peak)
    //   At t=0.25:  y = 3 * sin(pi)       = 0     (half cycle, back to zero)
    //   At t=0.375: y = 3 * sin(3*pi/2)   = -3    (three-quarter cycle, trough)
    //   At t=0.5:   y = 3 * sin(2*pi)     = 0     (full cycle, back to start)
    //
    // The wave completes 2 full cycles every second (f=2), with peaks at ±3.

    /// Evaluate the wave at time `t` (in seconds).
    ///
    /// # Formula
    ///
    /// `y(t) = amplitude * sin(2 * pi * frequency * t + phase)`
    ///
    /// # Examples
    ///
    /// ```
    /// use wave::Wave;
    /// use trig::PI;
    ///
    /// let w = Wave::new(1.0, 1.0, 0.0).unwrap();
    ///
    /// // At t=0, sin(0) = 0
    /// assert!(w.evaluate(0.0).abs() < 1e-10);
    ///
    /// // At t=0.25 (quarter period), sin(pi/2) = 1
    /// assert!((w.evaluate(0.25) - 1.0).abs() < 1e-10);
    /// ```
    pub fn evaluate(&self, t: f64) -> f64 {
        // Compute the argument to sine:
        //   theta = 2 * pi * f * t + phase
        //
        // This maps time into radians.  At t=0, theta = phase.
        // After one full period (t = 1/f), theta = 2*pi + phase,
        // which is equivalent to phase (since sin is 2*pi-periodic).
        let theta = 2.0 * trig::PI * self.frequency * t + self.phase;

        // Scale the sine value by amplitude.
        self.amplitude * trig::sin(theta)
    }
}
