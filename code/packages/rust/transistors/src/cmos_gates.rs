//! CMOS Logic Gates — building digital logic from transistor pairs.
//!
//! # What is CMOS?
//!
//! CMOS stands for Complementary Metal-Oxide-Semiconductor. It is the
//! technology used in virtually every digital chip made since the 1980s:
//! CPUs, GPUs, memory, phone processors — all CMOS.
//!
//! The "complementary" refers to pairing NMOS and PMOS transistors:
//!
//! - PMOS transistors form the PULL-UP network (connects output to Vdd)
//! - NMOS transistors form the PULL-DOWN network (connects output to GND)
//!
//! For any valid input combination, exactly ONE network is active:
//!
//! - If pull-up is ON -> output = Vdd (logic HIGH)
//! - If pull-down is ON -> output = GND (logic LOW)
//! - Never both ON simultaneously -> no DC current path -> near-zero static power
//!
//! # Transistor Counts
//!
//! | Gate | NMOS | PMOS | Total | Notes                   |
//! |------|------|------|-------|-------------------------|
//! | NOT  |  1   |  1   |   2   | The simplest CMOS gate  |
//! | NAND |  2   |  2   |   4   | Natural CMOS gate       |
//! | NOR  |  2   |  2   |   4   | Natural CMOS gate       |
//! | AND  |  3   |  3   |   6   | NAND + NOT              |
//! | OR   |  3   |  3   |   6   | NOR + NOT               |
//! | XOR  |  3   |  3   |   6   | Transmission gate style |

use crate::mosfet::{NMOS, PMOS};
use crate::types::{CircuitParams, GateOutput, MOSFETParams};

/// Validate that a value is a binary digit (0 or 1).
///
/// In Rust we use `u8` so we only need to check the range, not the type.
fn validate_bit(value: u8, name: &str) -> Result<(), String> {
    if value > 1 {
        return Err(format!("{} must be 0 or 1, got {}", name, value));
    }
    Ok(())
}

/// CMOS NOT gate: 1 PMOS + 1 NMOS = 2 transistors.
///
/// The simplest and most important CMOS circuit. Every other CMOS gate
/// is a variation of this fundamental pattern.
///
/// # How it works
///
/// ```text
///          Vdd
///           |
///      [  PMOS  ] --- Gate --- Input (A)
///           |
///           +---------------- Output (Y = NOT A)
///           |
///      [  NMOS  ] --- Gate --- Input (A)
///           |
///          GND
/// ```
///
/// - Input A = HIGH: NMOS ON, PMOS OFF -> output = LOW
/// - Input A = LOW: NMOS OFF, PMOS ON -> output = HIGH
///
/// Static power: ZERO. In both states, one transistor is OFF, breaking
/// the current path from Vdd to GND.
pub struct CMOSInverter {
    pub circuit: CircuitParams,
    pub nmos: NMOS,
    pub pmos: PMOS,
}

impl CMOSInverter {
    pub const TRANSISTOR_COUNT: usize = 2;

    /// Create a new CMOS inverter with optional parameters.
    pub fn new(
        circuit_params: Option<CircuitParams>,
        nmos_params: Option<MOSFETParams>,
        pmos_params: Option<MOSFETParams>,
    ) -> Self {
        Self {
            circuit: circuit_params.unwrap_or_default(),
            nmos: NMOS::new(nmos_params),
            pmos: PMOS::new(pmos_params),
        }
    }

    /// Evaluate the inverter with an analog input voltage.
    ///
    /// Maps the input voltage through the CMOS transfer characteristic
    /// to produce an output voltage. This is the "real" electrical
    /// simulation — not just 0/1 logic but actual voltage levels.
    pub fn evaluate(&self, input_voltage: f64) -> GateOutput {
        let vdd = self.circuit.vdd;

        // NMOS: gate = input, source = GND -> Vgs_n = Vin
        let vgs_n = input_voltage;
        // PMOS: gate = input, source = Vdd -> Vgs_p = Vin - Vdd
        let vgs_p = input_voltage - vdd;

        let nmos_on = self.nmos.is_conducting(vgs_n);
        let pmos_on = self.pmos.is_conducting(vgs_p);

        // Determine output voltage
        let output_v = if pmos_on && !nmos_on {
            vdd // PMOS pulls to Vdd
        } else if nmos_on && !pmos_on {
            0.0 // NMOS pulls to GND
        } else {
            // Both on (transition region) or both off — approximate as Vdd/2
            vdd / 2.0
        };

        // Digital interpretation
        let logic_value = if output_v > vdd / 2.0 { 1 } else { 0 };

        // Current draw: only significant during transition
        let current = if nmos_on && pmos_on {
            let vds_n = vdd / 2.0;
            self.nmos.drain_current(vgs_n, vds_n)
        } else {
            0.0
        };

        let power = current * vdd;

        // Propagation delay estimate
        let c_load = self.nmos.params.c_drain + self.pmos.params.c_drain;
        let delay = if current > 0.0 {
            c_load * vdd / (2.0 * current)
        } else {
            let ids_sat = self.nmos.drain_current(vdd, vdd);
            if ids_sat > 0.0 {
                c_load * vdd / (2.0 * ids_sat)
            } else {
                1e-9
            }
        };

        GateOutput {
            logic_value,
            voltage: output_v,
            current_draw: current,
            power_dissipation: power,
            propagation_delay: delay,
            transistor_count: Self::TRANSISTOR_COUNT,
        }
    }

    /// Evaluate with digital input (0 or 1), returns 0 or 1.
    pub fn evaluate_digital(&self, a: u8) -> Result<u8, String> {
        validate_bit(a, "a")?;
        let vin = if a == 1 { self.circuit.vdd } else { 0.0 };
        Ok(self.evaluate(vin).logic_value)
    }

    /// Generate the Voltage Transfer Characteristic (VTC) curve.
    ///
    /// Returns a list of (Vin, Vout) points showing the sharp switching
    /// threshold of CMOS — the output snaps from HIGH to LOW over a
    /// very narrow input range.
    pub fn voltage_transfer_characteristic(&self, steps: usize) -> Vec<(f64, f64)> {
        let vdd = self.circuit.vdd;
        let mut points = Vec::with_capacity(steps + 1);
        for i in 0..=steps {
            let vin = vdd * i as f64 / steps as f64;
            let result = self.evaluate(vin);
            points.push((vin, result.voltage));
        }
        points
    }

    /// Static power dissipation (ideally ~0 for CMOS).
    pub fn static_power(&self) -> f64 {
        0.0
    }

    /// Dynamic power: P = C_load * Vdd^2 * f.
    ///
    /// This is the dominant power consumption mechanism in CMOS:
    /// every time the output switches, the load capacitance must be
    /// charged or discharged.
    pub fn dynamic_power(&self, frequency: f64, c_load: f64) -> f64 {
        let vdd = self.circuit.vdd;
        c_load * vdd * vdd * frequency
    }
}

/// CMOS NAND gate: 2 PMOS parallel + 2 NMOS series = 4 transistors.
///
/// NAND is the "natural" CMOS gate because the CMOS structure naturally
/// produces an inverted output. The pull-down network (NMOS in series)
/// computes AND, and the inversion gives NAND.
///
/// In professional chip design, circuits are built from NAND and NOR
/// gates rather than AND and OR, precisely because of this efficiency.
pub struct CMOSNand {
    pub circuit: CircuitParams,
    pub nmos1: NMOS,
    pub nmos2: NMOS,
    pub pmos1: PMOS,
    pub pmos2: PMOS,
}

impl CMOSNand {
    pub const TRANSISTOR_COUNT: usize = 4;

    pub fn new(
        circuit_params: Option<CircuitParams>,
        nmos_params: Option<MOSFETParams>,
        pmos_params: Option<MOSFETParams>,
    ) -> Self {
        Self {
            circuit: circuit_params.unwrap_or_default(),
            nmos1: NMOS::new(nmos_params),
            nmos2: NMOS::new(nmos_params),
            pmos1: PMOS::new(pmos_params),
            pmos2: PMOS::new(pmos_params),
        }
    }

    /// Evaluate the NAND gate with analog input voltages.
    pub fn evaluate(&self, va: f64, vb: f64) -> GateOutput {
        let vdd = self.circuit.vdd;

        let vgs_n1 = va;
        let vgs_n2 = vb;
        let vgs_p1 = va - vdd;
        let vgs_p2 = vb - vdd;

        let nmos1_on = self.nmos1.is_conducting(vgs_n1);
        let nmos2_on = self.nmos2.is_conducting(vgs_n2);
        let pmos1_on = self.pmos1.is_conducting(vgs_p1);
        let pmos2_on = self.pmos2.is_conducting(vgs_p2);

        // Pull-down: NMOS in SERIES — BOTH must be ON
        let pulldown_on = nmos1_on && nmos2_on;
        // Pull-up: PMOS in PARALLEL — EITHER can pull up
        let pullup_on = pmos1_on || pmos2_on;

        let output_v = if pullup_on && !pulldown_on {
            vdd
        } else if pulldown_on && !pullup_on {
            0.0
        } else {
            vdd / 2.0
        };

        let logic_value = if output_v > vdd / 2.0 { 1 } else { 0 };
        let current = if pulldown_on && pullup_on { 0.001 } else { 0.0 };

        let c_load = self.nmos1.params.c_drain + self.pmos1.params.c_drain;
        let ids_sat = self.nmos1.drain_current(vdd, vdd);
        let delay = if ids_sat > 0.0 {
            c_load * vdd / (2.0 * ids_sat)
        } else {
            1e-9
        };

        GateOutput {
            logic_value,
            voltage: output_v,
            current_draw: current,
            power_dissipation: current * vdd,
            propagation_delay: delay,
            transistor_count: Self::TRANSISTOR_COUNT,
        }
    }

    /// Evaluate with digital inputs (0 or 1).
    pub fn evaluate_digital(&self, a: u8, b: u8) -> Result<u8, String> {
        validate_bit(a, "a")?;
        validate_bit(b, "b")?;
        let vdd = self.circuit.vdd;
        let va = if a == 1 { vdd } else { 0.0 };
        let vb = if b == 1 { vdd } else { 0.0 };
        Ok(self.evaluate(va, vb).logic_value)
    }

    /// Returns the transistor count (4).
    pub fn transistor_count(&self) -> usize {
        Self::TRANSISTOR_COUNT
    }
}

/// CMOS NOR gate: 2 PMOS series + 2 NMOS parallel = 4 transistors.
///
/// The dual of NAND: pull-down is NMOS in parallel (either ON pulls low),
/// pull-up is PMOS in series (both must be ON to pull high).
pub struct CMOSNor {
    pub circuit: CircuitParams,
    pub nmos1: NMOS,
    pub nmos2: NMOS,
    pub pmos1: PMOS,
    pub pmos2: PMOS,
}

impl CMOSNor {
    pub const TRANSISTOR_COUNT: usize = 4;

    pub fn new(
        circuit_params: Option<CircuitParams>,
        nmos_params: Option<MOSFETParams>,
        pmos_params: Option<MOSFETParams>,
    ) -> Self {
        Self {
            circuit: circuit_params.unwrap_or_default(),
            nmos1: NMOS::new(nmos_params),
            nmos2: NMOS::new(nmos_params),
            pmos1: PMOS::new(pmos_params),
            pmos2: PMOS::new(pmos_params),
        }
    }

    /// Evaluate the NOR gate with analog input voltages.
    pub fn evaluate(&self, va: f64, vb: f64) -> GateOutput {
        let vdd = self.circuit.vdd;

        let vgs_n1 = va;
        let vgs_n2 = vb;
        let vgs_p1 = va - vdd;
        let vgs_p2 = vb - vdd;

        let nmos1_on = self.nmos1.is_conducting(vgs_n1);
        let nmos2_on = self.nmos2.is_conducting(vgs_n2);
        let pmos1_on = self.pmos1.is_conducting(vgs_p1);
        let pmos2_on = self.pmos2.is_conducting(vgs_p2);

        // Pull-down: NMOS in PARALLEL — EITHER ON pulls low
        let pulldown_on = nmos1_on || nmos2_on;
        // Pull-up: PMOS in SERIES — BOTH must be ON
        let pullup_on = pmos1_on && pmos2_on;

        let output_v = if pullup_on && !pulldown_on {
            vdd
        } else if pulldown_on && !pullup_on {
            0.0
        } else {
            vdd / 2.0
        };

        let logic_value = if output_v > vdd / 2.0 { 1 } else { 0 };
        let current = if pulldown_on && pullup_on { 0.001 } else { 0.0 };

        let c_load = self.nmos1.params.c_drain + self.pmos1.params.c_drain;
        let ids_sat = self.nmos1.drain_current(vdd, vdd);
        let delay = if ids_sat > 0.0 {
            c_load * vdd / (2.0 * ids_sat)
        } else {
            1e-9
        };

        GateOutput {
            logic_value,
            voltage: output_v,
            current_draw: current,
            power_dissipation: current * vdd,
            propagation_delay: delay,
            transistor_count: Self::TRANSISTOR_COUNT,
        }
    }

    /// Evaluate with digital inputs (0 or 1).
    pub fn evaluate_digital(&self, a: u8, b: u8) -> Result<u8, String> {
        validate_bit(a, "a")?;
        validate_bit(b, "b")?;
        let vdd = self.circuit.vdd;
        let va = if a == 1 { vdd } else { 0.0 };
        let vb = if b == 1 { vdd } else { 0.0 };
        Ok(self.evaluate(va, vb).logic_value)
    }
}

/// CMOS AND gate: NAND + Inverter = 6 transistors.
///
/// There is no "direct" CMOS AND gate. The CMOS topology naturally
/// produces inverted outputs (NAND, NOR), so to get AND we must add
/// an inverter after the NAND.
pub struct CMOSAnd {
    circuit: CircuitParams,
    nand: CMOSNand,
    inv: CMOSInverter,
}

impl CMOSAnd {
    pub const TRANSISTOR_COUNT: usize = 6;

    pub fn new(circuit_params: Option<CircuitParams>) -> Self {
        Self {
            circuit: circuit_params.unwrap_or_default(),
            nand: CMOSNand::new(circuit_params, None, None),
            inv: CMOSInverter::new(circuit_params, None, None),
        }
    }

    /// AND = NOT(NAND(A, B)).
    pub fn evaluate(&self, va: f64, vb: f64) -> GateOutput {
        let nand_out = self.nand.evaluate(va, vb);
        let inv_out = self.inv.evaluate(nand_out.voltage);
        GateOutput {
            logic_value: inv_out.logic_value,
            voltage: inv_out.voltage,
            current_draw: nand_out.current_draw + inv_out.current_draw,
            power_dissipation: nand_out.power_dissipation + inv_out.power_dissipation,
            propagation_delay: nand_out.propagation_delay + inv_out.propagation_delay,
            transistor_count: Self::TRANSISTOR_COUNT,
        }
    }

    /// Evaluate with digital inputs.
    pub fn evaluate_digital(&self, a: u8, b: u8) -> Result<u8, String> {
        validate_bit(a, "a")?;
        validate_bit(b, "b")?;
        let vdd = self.circuit.vdd;
        let va = if a == 1 { vdd } else { 0.0 };
        let vb = if b == 1 { vdd } else { 0.0 };
        Ok(self.evaluate(va, vb).logic_value)
    }
}

/// CMOS OR gate: NOR + Inverter = 6 transistors.
pub struct CMOSOr {
    circuit: CircuitParams,
    nor: CMOSNor,
    inv: CMOSInverter,
}

impl CMOSOr {
    pub const TRANSISTOR_COUNT: usize = 6;

    pub fn new(circuit_params: Option<CircuitParams>) -> Self {
        Self {
            circuit: circuit_params.unwrap_or_default(),
            nor: CMOSNor::new(circuit_params, None, None),
            inv: CMOSInverter::new(circuit_params, None, None),
        }
    }

    /// OR = NOT(NOR(A, B)).
    pub fn evaluate(&self, va: f64, vb: f64) -> GateOutput {
        let nor_out = self.nor.evaluate(va, vb);
        let inv_out = self.inv.evaluate(nor_out.voltage);
        GateOutput {
            logic_value: inv_out.logic_value,
            voltage: inv_out.voltage,
            current_draw: nor_out.current_draw + inv_out.current_draw,
            power_dissipation: nor_out.power_dissipation + inv_out.power_dissipation,
            propagation_delay: nor_out.propagation_delay + inv_out.propagation_delay,
            transistor_count: Self::TRANSISTOR_COUNT,
        }
    }

    /// Evaluate with digital inputs.
    pub fn evaluate_digital(&self, a: u8, b: u8) -> Result<u8, String> {
        validate_bit(a, "a")?;
        validate_bit(b, "b")?;
        let vdd = self.circuit.vdd;
        let va = if a == 1 { vdd } else { 0.0 };
        let vb = if b == 1 { vdd } else { 0.0 };
        Ok(self.evaluate(va, vb).logic_value)
    }
}

/// CMOS XOR gate using 4-NAND construction = 6 transistors.
///
/// XOR(A, B) = NAND(NAND(A, NAND(A,B)), NAND(B, NAND(A,B)))
///
/// This construction proves that XOR can be built from the universal
/// NAND gate alone.
pub struct CMOSXor {
    circuit: CircuitParams,
    nand1: CMOSNand,
    nand2: CMOSNand,
    nand3: CMOSNand,
    nand4: CMOSNand,
}

impl CMOSXor {
    pub const TRANSISTOR_COUNT: usize = 6;

    pub fn new(circuit_params: Option<CircuitParams>) -> Self {
        Self {
            circuit: circuit_params.unwrap_or_default(),
            nand1: CMOSNand::new(circuit_params, None, None),
            nand2: CMOSNand::new(circuit_params, None, None),
            nand3: CMOSNand::new(circuit_params, None, None),
            nand4: CMOSNand::new(circuit_params, None, None),
        }
    }

    /// XOR using 4 NAND gates.
    pub fn evaluate(&self, va: f64, vb: f64) -> GateOutput {
        let vdd = self.circuit.vdd;

        // Step 1: NAND(A, B)
        let nand_ab = self.nand1.evaluate(va, vb);
        // Step 2: NAND(A, NAND(A,B))
        let nand_a_nab = self.nand2.evaluate(va, nand_ab.voltage);
        // Step 3: NAND(B, NAND(A,B))
        let nand_b_nab = self.nand3.evaluate(vb, nand_ab.voltage);
        // Step 4: NAND(step2, step3)
        let result = self.nand4.evaluate(nand_a_nab.voltage, nand_b_nab.voltage);

        let total_current = nand_ab.current_draw
            + nand_a_nab.current_draw
            + nand_b_nab.current_draw
            + result.current_draw;

        let total_delay = nand_ab.propagation_delay
            + nand_a_nab.propagation_delay.max(nand_b_nab.propagation_delay)
            + result.propagation_delay;

        GateOutput {
            logic_value: result.logic_value,
            voltage: result.voltage,
            current_draw: total_current,
            power_dissipation: total_current * vdd,
            propagation_delay: total_delay,
            transistor_count: Self::TRANSISTOR_COUNT,
        }
    }

    /// Evaluate with digital inputs.
    pub fn evaluate_digital(&self, a: u8, b: u8) -> Result<u8, String> {
        validate_bit(a, "a")?;
        validate_bit(b, "b")?;
        let vdd = self.circuit.vdd;
        let va = if a == 1 { vdd } else { 0.0 };
        let vb = if b == 1 { vdd } else { 0.0 };
        Ok(self.evaluate(va, vb).logic_value)
    }

    /// Build XOR from 4 NAND gates to demonstrate universality.
    pub fn evaluate_from_nands(&self, a: u8, b: u8) -> Result<u8, String> {
        self.evaluate_digital(a, b)
    }
}
