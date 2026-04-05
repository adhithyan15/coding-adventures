// ============================================================================
// Wave.swift — Simple Harmonic Wave Model
// ============================================================================
//
// A sinusoidal wave is the fundamental building block of all signal
// processing and wireless communication.  Every signal — from AM radio
// to 5G — is ultimately a combination of sine waves.
//
// A single sinusoidal wave is fully described by three numbers:
//
//   y(t) = A · sin(2π·f·t + φ)
//
// where:
//   A  = amplitude  (peak displacement from zero)
//   f  = frequency  (cycles per second, in Hertz)
//   φ  = phase      (starting offset, in radians)
//
// From these three, we derive:
//   T  = 1/f        (period — time for one full cycle)
//   ω  = 2π·f       (angular frequency — radians per second)
//
// Layer: PHY01 (physics layer 1 — depends on trig for sin/PI)
// Spec:  code/specs/PHY01-wave.md
// ============================================================================

import Foundation

/// An immutable sinusoidal wave: y(t) = A · sin(2π·f·t + φ)
public struct Wave {

    // ========================================================================
    // MARK: - Properties
    // ========================================================================

    /// Peak amplitude (must be >= 0).
    public let amplitude: Double

    /// Frequency in Hertz (must be > 0).
    public let frequency: Double

    /// Phase offset in radians.
    public let phase: Double

    // ========================================================================
    // MARK: - Initializer
    // ========================================================================

    /// Create a new wave.
    ///
    /// - Parameters:
    ///   - amplitude: Peak displacement (must be >= 0).
    ///   - frequency: Cycles per second in Hz (must be > 0).
    ///   - phase: Starting offset in radians (default 0).
    public init(amplitude: Double, frequency: Double, phase: Double = 0.0) {
        precondition(amplitude >= 0, "Amplitude must be non-negative (got \(amplitude))")
        precondition(frequency > 0, "Frequency must be positive (got \(frequency))")
        self.amplitude = amplitude
        self.frequency = frequency
        self.phase = phase
    }

    // ========================================================================
    // MARK: - Derived Properties
    // ========================================================================

    /// Period: time for one complete cycle (seconds).
    ///
    /// T = 1/f
    ///
    /// A 440 Hz wave (the note A) has a period of about 2.27 milliseconds.
    public var period: Double {
        return 1.0 / frequency
    }

    /// Angular frequency in radians per second.
    ///
    /// ω = 2π·f
    ///
    /// While frequency counts cycles per second, angular frequency counts
    /// radians per second.  One full cycle = 2π radians.
    public var angularFrequency: Double {
        return 2.0 * Double.pi * frequency
    }

    // ========================================================================
    // MARK: - Evaluation
    // ========================================================================
    //
    // The evaluate method is the heart of the package.  Given a moment in
    // time t (in seconds), it returns the wave's displacement.
    //
    //   y(t) = A · sin(2π·f·t + φ)
    //
    // This single formula generates sine waves, cosine waves (phase = π/2),
    // inverted waves (phase = π), and everything in between.
    // ========================================================================

    /// Evaluate the wave at time t (seconds).
    ///
    /// - Parameter t: time in seconds
    /// - Returns: displacement at time t
    public func evaluate(at t: Double) -> Double {
        return amplitude * sin(2.0 * Double.pi * frequency * t + phase)
    }
}
