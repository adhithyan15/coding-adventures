// ============================================================================
// AnalogWaveform.swift
// ============================================================================

import Foundation

/// Represents any continuous-time signal (e.g., node voltage, branch current).
/// The core abstraction is that an analog waveform is a function of time: x(t).
public protocol AnalogWaveform {
    /// Evaluates the waveform at a specific point in time.
    /// - Parameter t: The time in seconds.
    /// - Returns: The amplitude (e.g., voltage or current) at time t.
    func sampleAt(_ t: Double) -> Double
}

/// A waveform that produces a constant DC amplitude regardless of time.
public struct ConstantWaveform: AnalogWaveform {
    public let amplitude: Double

    public init(amplitude: Double) {
        self.amplitude = amplitude
    }

    public func sampleAt(_ t: Double) -> Double {
        return amplitude
    }
}

/// A sinusoidal AC waveform defined by amplitude, frequency, and phase.
public struct SineWaveform: AnalogWaveform {
    public let amplitude: Double
    public let frequency: Double
    public let phase: Double

    /// Initialize a SineWaveform.
    /// - Parameters:
    ///   - amplitude: the peak deviation from zero
    ///   - frequency: the frequency in Hertz (cycles per second)
    ///   - phase: the phase shift in radians (defaults to 0.0)
    public init(amplitude: Double, frequency: Double, phase: Double = 0.0) {
        self.amplitude = amplitude
        self.frequency = frequency
        self.phase = phase
    }

    public func sampleAt(_ t: Double) -> Double {
        return amplitude * sin(2.0 * .pi * frequency * t + phase)
    }
}
