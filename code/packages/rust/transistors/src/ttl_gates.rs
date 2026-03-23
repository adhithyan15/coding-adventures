//! TTL Logic Gates — historical BJT-based digital logic.
//!
//! # What is TTL?
//!
//! TTL stands for Transistor-Transistor Logic. It was the dominant digital
//! logic family from the mid-1960s through the 1980s, when CMOS replaced it.
//! The "7400 series" defined the standard logic gates.
//!
//! # Why TTL Lost to CMOS
//!
//! TTL's fatal flaw: STATIC POWER CONSUMPTION.
//!
//! In a TTL gate, current flows through resistors and transistors even when
//! the gate is doing nothing. A single TTL NAND gate dissipates ~1-10 mW
//! at rest. With 1 million gates, that is 10,000 watts — a space heater!
//!
//! CMOS gates consume near-zero power at rest (only transistor leakage).
//!
//! # RTL: The Predecessor to TTL
//!
//! Before TTL came RTL (Resistor-Transistor Logic), the simplest possible
//! transistor logic. It was used in the Apollo Guidance Computer that
//! landed humans on the moon in 1969.

use crate::bjt::NPN;
use crate::types::{BJTParams, GateOutput};

/// Validate binary digit.
fn validate_bit(value: u8, name: &str) -> Result<(), String> {
    if value > 1 {
        return Err(format!("{} must be 0 or 1, got {}", name, value));
    }
    Ok(())
}

/// TTL NAND gate using NPN transistors (7400-series style).
///
/// # Simplified Circuit
///
/// ```text
///         Vcc (+5V)
///          |
///          R1 (4kOhm)
///          |
///     [  Q1 (NPN)  ]     Multi-emitter input transistor
///     |-- E1 --| Input A
///     |-- E2 --| Input B
///          |
///     [  Q2 (NPN)  ]     Phase splitter
///          |
///     [  Q3 (NPN)  ]     Output transistor
///          |
///         GND
/// ```
///
/// # The Problem: Static Power
///
/// When Q3 is ON: current flows Vcc -> R1 -> Q1 -> Q2 -> Q3 -> GND.
/// This current flows CONTINUOUSLY, consuming ~1-10 mW per gate.
pub struct TTLNand {
    pub vcc: f64,
    pub params: BJTParams,
    pub r_pullup: f64,
    pub q1: NPN,
    pub q2: NPN,
    pub q3: NPN,
}

impl TTLNand {
    pub fn new(vcc: Option<f64>, bjt_params: Option<BJTParams>) -> Self {
        let params = bjt_params.unwrap_or_default();
        Self {
            vcc: vcc.unwrap_or(5.0),
            params,
            r_pullup: 4000.0,
            q1: NPN::new(Some(params)),
            q2: NPN::new(Some(params)),
            q3: NPN::new(Some(params)),
        }
    }

    /// Evaluate the TTL NAND gate with analog input voltages.
    pub fn evaluate(&self, va: f64, vb: f64) -> GateOutput {
        let vcc = self.vcc;
        let vbe_on = self.params.vbe_on;

        // TTL input thresholds
        let a_high = va > 2.0;
        let b_high = vb > 2.0;

        let (output_v, logic_value, current) = if a_high && b_high {
            // ALL inputs HIGH -> output LOW
            let output_v = self.params.vce_sat; // ~0.2V
            let current = (vcc - 2.0 * vbe_on - self.params.vce_sat) / self.r_pullup;
            (output_v, 0u8, current.max(0.0))
        } else {
            // At least one input LOW -> output HIGH
            let output_v = vcc - vbe_on; // ~4.3V
            let current = (vcc - output_v) / self.r_pullup;
            (output_v, 1u8, current.max(0.0))
        };

        let power = current * vcc;
        let delay = 10e-9; // 10 ns typical TTL

        GateOutput {
            logic_value,
            voltage: output_v,
            current_draw: current,
            power_dissipation: power,
            propagation_delay: delay,
            transistor_count: 3,
        }
    }

    /// Evaluate with digital inputs (0 or 1).
    pub fn evaluate_digital(&self, a: u8, b: u8) -> Result<u8, String> {
        validate_bit(a, "a")?;
        validate_bit(b, "b")?;
        let va = if a == 1 { self.vcc } else { 0.0 };
        let vb = if b == 1 { self.vcc } else { 0.0 };
        Ok(self.evaluate(va, vb).logic_value)
    }

    /// Static power dissipation — significantly higher than CMOS.
    ///
    /// TTL gates consume power continuously due to the resistor-based
    /// biasing. Worst case is when output is LOW (all inputs HIGH).
    pub fn static_power(&self) -> f64 {
        let current =
            (self.vcc - 2.0 * self.params.vbe_on - self.params.vce_sat) / self.r_pullup;
        current.max(0.0) * self.vcc
    }
}

/// Resistor-Transistor Logic inverter — the earliest IC logic family.
///
/// # Circuit
///
/// ```text
///         Vcc
///          |
///         Rc (collector resistor, ~1kOhm)
///          |
///     [  Q1 (NPN)  ]
///          |
///         GND
///
///     Input --- Rb (base resistor, ~10kOhm) --- Base of Q1
/// ```
///
/// # Historical Note
///
/// RTL was used in the Apollo Guidance Computer (AGC), which navigated
/// Apollo 11 to the moon in 1969. The AGC contained about 5,600 NOR
/// gates built from RTL circuits, with a clock speed of 2 MHz.
pub struct RTLInverter {
    pub vcc: f64,
    pub r_base: f64,
    pub r_collector: f64,
    pub params: BJTParams,
    pub q1: NPN,
}

impl RTLInverter {
    pub fn new(
        vcc: Option<f64>,
        r_base: Option<f64>,
        r_collector: Option<f64>,
        bjt_params: Option<BJTParams>,
    ) -> Self {
        let params = bjt_params.unwrap_or_default();
        Self {
            vcc: vcc.unwrap_or(5.0),
            r_base: r_base.unwrap_or(10_000.0),
            r_collector: r_collector.unwrap_or(1_000.0),
            params,
            q1: NPN::new(Some(params)),
        }
    }

    /// Evaluate the RTL inverter with an analog input voltage.
    pub fn evaluate(&self, v_input: f64) -> GateOutput {
        let vcc = self.vcc;
        let vbe_on = self.params.vbe_on;

        let (output_v, logic_value, current) = if v_input > vbe_on {
            // Q1 is ON
            let ib = (v_input - vbe_on) / self.r_base;
            let ic = (ib * self.params.beta)
                .min((vcc - self.params.vce_sat) / self.r_collector);
            let mut out_v = vcc - ic * self.r_collector;
            out_v = out_v.max(self.params.vce_sat);
            let lv = if out_v < vcc / 2.0 { 0 } else { 1 };
            (out_v, lv, ic + ib)
        } else {
            // Q1 is OFF — output pulled to Vcc through Rc
            (vcc, 1u8, 0.0)
        };

        let power = current * vcc;
        let delay = 50e-9; // RTL is slow: ~50 ns typical

        GateOutput {
            logic_value,
            voltage: output_v,
            current_draw: current,
            power_dissipation: power,
            propagation_delay: delay,
            transistor_count: 1,
        }
    }

    /// Evaluate with digital input (0 or 1).
    pub fn evaluate_digital(&self, a: u8) -> Result<u8, String> {
        validate_bit(a, "a")?;
        let v_input = if a == 1 { self.vcc } else { 0.0 };
        Ok(self.evaluate(v_input).logic_value)
    }
}
