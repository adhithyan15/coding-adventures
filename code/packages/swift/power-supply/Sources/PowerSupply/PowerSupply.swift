// ============================================================================
// PowerSupply.swift
// ============================================================================

import AnalogWaveform

/// An interface for electrical power sources.
public protocol PowerSupply {
    /// Returns the source voltage at a particular instant in time.
    func voltage(at time: Double) -> Double
}

/// An ideal DC power supply providing constant voltage.
public struct IdealDCSupply: PowerSupply {
    public let nominalVoltage: Double
    private let waveform: ConstantWaveform

    public init(voltage: Double) {
        self.nominalVoltage = voltage
        self.waveform = ConstantWaveform(amplitude: voltage)
    }

    public func voltage(at time: Double) -> Double {
        return waveform.sampleAt(time)
    }
}

/// An ideal AC power supply providing sinusoidal voltage.
public struct IdealSinusoidalSource: PowerSupply {
    private let waveform: SineWaveform

    public init(peakVoltage: Double, frequency: Double, phase: Double = 0.0) {
        self.waveform = SineWaveform(amplitude: peakVoltage, frequency: frequency, phase: phase)
    }

    public func voltage(at time: Double) -> Double {
        return waveform.sampleAt(time)
    }
}
