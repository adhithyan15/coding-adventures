// ============================================================================
// Electronics.swift
// ============================================================================

import PowerSupply
import AnalogWaveform

/// Represents an ideal resistor with constant resistance.
public struct IdealResistor {
    /// Resistance in Ohms.
    public let resistance: Double
    
    public init(resistance: Double) {
        precondition(resistance > 0, "Resistance must be strictly positive for ideal calculations")
        self.resistance = resistance
    }
    
    /// Calculates current (Amperes) given a voltage drop.
    public func current(voltage: Double) -> Double {
        return voltage / resistance
    }
    
    /// Calculates voltage drop (Volts) given a current.
    public func voltage(current: Double) -> Double {
        return current * resistance
    }
    
    /// Calculates power dissipation (Watts) given a voltage drop.
    public func power(voltage: Double) -> Double {
        return (voltage * voltage) / resistance
    }
}

/// A voltage divider consisting of two ideal resistors in series.
public struct VoltageDivider {
    public let r1: IdealResistor
    public let r2: IdealResistor
    
    public init(r1: IdealResistor, r2: IdealResistor) {
        self.r1 = r1
        self.r2 = r2
    }
    
    /// Calculates the output voltage measured across r2, given an input voltage across the series.
    public func vOut(vIn: Double) -> Double {
        let totalR = r1.resistance + r2.resistance
        return vIn * (r2.resistance / totalR)
    }
}

/// Helper methods for simple DC analysis.
public struct DCAnalysis {
    public static func current(supply: IdealDCSupply, resistor: IdealResistor) -> Double {
        return resistor.current(voltage: supply.nominalVoltage)
    }
    
    public static func powerDissipation(supply: IdealDCSupply, resistor: IdealResistor) -> Double {
        return resistor.power(voltage: supply.nominalVoltage)
    }
}

/// Helper methods for tracking simple AC responses across a resistor.
public struct SinusoidalResistorResponse {
    public let source: IdealSinusoidalSource
    public let resistor: IdealResistor
    
    public init(source: IdealSinusoidalSource, resistor: IdealResistor) {
        self.source = source
        self.resistor = resistor
    }
    
    /// Returns the active current flow at a given point in time.
    public func current(at time: Double) -> Double {
        return resistor.current(voltage: source.voltage(at: time))
    }
    
    /// Returns the instantaneous power dissipation at a given point in time.
    public func instantaneousPower(at time: Double) -> Double {
        return resistor.power(voltage: source.voltage(at: time))
    }
}
