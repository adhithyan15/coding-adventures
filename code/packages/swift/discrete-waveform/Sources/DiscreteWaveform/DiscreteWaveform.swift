// ============================================================================
// DiscreteWaveform.swift
// ============================================================================

import AnalogWaveform

/// A discrete waveform is a time series of samples taken from an analog waveform.
public struct DiscreteWaveform {
    /// The discrete samples over time.
    public let samples: [Double]
    
    /// The sampling rate in Hz (samples per second).
    public let sampleRate: Double

    /// Direct initialization from an array of samples.
    public init(samples: [Double], sampleRate: Double) {
        precondition(sampleRate > 0, "sampleRate must be strictly positive")
        self.samples = samples
        self.sampleRate = sampleRate
    }

    /// Generates a discrete waveform by sampling an analog waveform over a specific duration.
    public init(from analog: AnalogWaveform, sampleRate: Double, duration: Double) {
        precondition(sampleRate > 0, "sampleRate must be strictly positive")
        precondition(duration >= 0, "duration must be non-negative")
        
        self.sampleRate = sampleRate
        // Compute discrete sample points
        let numSamples = Int(duration * sampleRate)
        var generatedSamples = [Double]()
        generatedSamples.reserveCapacity(numSamples)
        
        let period = 1.0 / sampleRate
        for i in 0..<numSamples {
            let t = Double(i) * period
            generatedSamples.append(analog.sampleAt(t))
        }
        self.samples = generatedSamples
    }

    /// The interval in seconds between samples.
    public var samplePeriod: Double {
        return 1.0 / sampleRate
    }

    /// The total duration of the sampled waveform in seconds.
    public var duration: Double {
        return Double(samples.count) / sampleRate
    }

    /// Implements zero-order-hold reconstruction.
    /// Values are held constant until the next sample is reached.
    public func zeroOrderHold(at t: Double) -> Double {
        if samples.isEmpty { return 0.0 }
        if t < 0 { return samples.first! }
        
        let index = Int(t * sampleRate)
        if index >= samples.count {
            return samples.last!
        }
        return samples[index]
    }
}
