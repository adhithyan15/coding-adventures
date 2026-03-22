//! MOSFET Transistors — the building blocks of modern digital circuits.
//!
//! # What is a MOSFET?
//!
//! MOSFET stands for Metal-Oxide-Semiconductor Field-Effect Transistor. It is
//! the most common type of transistor in the world — every CPU, GPU, and phone
//! chip is built from billions of MOSFETs.
//!
//! A MOSFET has three terminals:
//!
//! - **Gate (G)**: The control terminal. Voltage here controls the switch.
//! - **Drain (D)**: Current flows IN here (for NMOS) or OUT here (for PMOS).
//! - **Source (S)**: Current flows OUT here (for NMOS) or IN here (for PMOS).
//!
//! The key insight: a MOSFET is VOLTAGE-controlled. Applying a voltage to the
//! gate creates an electric field that either allows or blocks current flow
//! between drain and source. No current flows into the gate itself (it's
//! insulated by a thin oxide layer), which means:
//!
//! - Near-zero input power consumption
//! - Very high input impedance (good for amplifiers)
//! - Can be packed extremely densely on a chip
//!
//! # NMOS vs PMOS
//!
//! The two MOSFET types are complementary — they turn on under opposite conditions:
//!
//! - NMOS: Gate HIGH -> ON (conducts drain to source)
//! - PMOS: Gate LOW  -> ON (conducts source to drain)
//!
//! This complementary behavior is the foundation of CMOS (Complementary MOS)
//! logic. By pairing NMOS and PMOS transistors, we can build gates that consume
//! near-zero power in steady state — only burning energy during transitions.

use crate::types::{MOSFETParams, MOSFETRegion};

/// N-channel MOSFET transistor.
///
/// An NMOS transistor conducts current from drain to source when the gate
/// voltage exceeds the threshold voltage (Vgs > Vth). Think of it as a
/// normally-OPEN switch that CLOSES when you apply voltage to the gate.
///
/// # Water analogy
///
/// Imagine a water pipe with an electrically-controlled valve:
///
/// ```text
/// Water pressure (Vdd) --> [VALVE] --> Water out (Vss/ground)
///                            ^
///                        Gate voltage
/// ```
///
/// - Gate voltage HIGH: valve opens, water flows (current flows D->S)
/// - Gate voltage LOW: valve closed, water blocked (no current)
/// - Gate voltage MEDIUM: valve partially open (analog amplifier mode)
///
/// # In a digital circuit
///
/// When used as a digital switch, NMOS connects the output to GROUND:
///
/// ```text
///     Output --|
///              | NMOS (gate = input signal)
///              |
///             GND
/// ```
///
/// - Input HIGH -> NMOS ON -> output pulled to GND (LOW)
/// - Input LOW  -> NMOS OFF -> output disconnected from GND
pub struct NMOS {
    pub params: MOSFETParams,
}

impl NMOS {
    /// Create a new NMOS transistor with the given parameters.
    ///
    /// If `None` is passed, default 180nm CMOS process parameters are used.
    pub fn new(params: Option<MOSFETParams>) -> Self {
        Self {
            params: params.unwrap_or_default(),
        }
    }

    /// Determine the operating region given terminal voltages.
    ///
    /// The operating region determines which equations govern current flow.
    /// For NMOS:
    ///
    /// - Cutoff: Vgs < Vth (gate voltage below threshold)
    /// - Linear: Vgs >= Vth AND Vds < Vgs - Vth
    /// - Saturation: Vgs >= Vth AND Vds >= Vgs - Vth
    pub fn region(&self, vgs: f64, vds: f64) -> MOSFETRegion {
        let vth = self.params.vth;

        if vgs < vth {
            return MOSFETRegion::Cutoff;
        }

        let vov = vgs - vth; // Overdrive voltage
        if vds < vov {
            MOSFETRegion::Linear
        } else {
            MOSFETRegion::Saturation
        }
    }

    /// Calculate drain-to-source current (Ids) in amperes.
    ///
    /// Uses the simplified MOSFET current equations (Shockley model):
    ///
    /// - **Cutoff** (Vgs < Vth): Ids = 0. No channel exists.
    ///
    /// - **Linear** (Vgs >= Vth, Vds < Vgs - Vth):
    ///   Ids = k * ((Vgs - Vth) * Vds - 0.5 * Vds^2).
    ///   The transistor acts like a voltage-controlled resistor.
    ///
    /// - **Saturation** (Vgs >= Vth, Vds >= Vgs - Vth):
    ///   Ids = 0.5 * k * (Vgs - Vth)^2.
    ///   The channel is "pinched off" at the drain end.
    ///   Current depends only on Vgs, not Vds.
    pub fn drain_current(&self, vgs: f64, vds: f64) -> f64 {
        let region = self.region(vgs, vds);
        let k = self.params.k;
        let vth = self.params.vth;

        match region {
            MOSFETRegion::Cutoff => 0.0,
            MOSFETRegion::Linear => {
                let vov = vgs - vth;
                k * (vov * vds - 0.5 * vds * vds)
            }
            MOSFETRegion::Saturation => {
                let vov = vgs - vth;
                0.5 * k * vov * vov
            }
        }
    }

    /// Digital abstraction: is this transistor ON?
    ///
    /// Returns `true` when the gate voltage exceeds the threshold voltage.
    /// This is the simplified view used in digital circuit analysis —
    /// the transistor is either fully ON or fully OFF.
    pub fn is_conducting(&self, vgs: f64) -> bool {
        vgs >= self.params.vth
    }

    /// Output voltage when used as a pull-down switch.
    ///
    /// In a CMOS circuit, NMOS transistors form the pull-down network
    /// (connecting output to ground):
    ///
    /// - ON: output ~ 0V (pulled to ground through low-resistance channel)
    /// - OFF: output ~ Vdd (pulled up by load resistor)
    pub fn output_voltage(&self, vgs: f64, vdd: f64) -> f64 {
        if self.is_conducting(vgs) {
            0.0
        } else {
            vdd
        }
    }

    /// Calculate small-signal transconductance gm.
    ///
    /// Transconductance is the key parameter for amplifier design.
    /// It tells you how much the output current changes per unit
    /// change in input voltage: gm = dIds / dVgs.
    ///
    /// In saturation: gm = k * (Vgs - Vth).
    ///
    /// Returns 0.0 in cutoff.
    pub fn transconductance(&self, vgs: f64, vds: f64) -> f64 {
        let region = self.region(vgs, vds);
        if region == MOSFETRegion::Cutoff {
            return 0.0;
        }

        let vov = vgs - self.params.vth;
        self.params.k * vov
    }
}

/// P-channel MOSFET transistor.
///
/// A PMOS transistor is the complement of NMOS. It conducts current from
/// source to drain when the gate voltage is LOW (below the source voltage
/// by more than |Vth|). Think of it as a normally-CLOSED switch that OPENS
/// when you apply voltage.
///
/// # Why PMOS matters
///
/// PMOS transistors form the pull-UP network in CMOS gates. When we need
/// to connect the output to Vdd (logic HIGH), PMOS transistors do the job.
///
/// # Voltage conventions
///
/// PMOS uses the same equations as NMOS, but with reversed voltage
/// polarities. For PMOS, Vgs and Vds are typically negative (because
/// the source is connected to Vdd, the highest voltage in the circuit).
///
/// In this implementation, we handle the sign conventions internally
/// using absolute values.
pub struct PMOS {
    pub params: MOSFETParams,
}

impl PMOS {
    /// Create a new PMOS transistor with the given parameters.
    pub fn new(params: Option<MOSFETParams>) -> Self {
        Self {
            params: params.unwrap_or_default(),
        }
    }

    /// Determine operating region for PMOS.
    ///
    /// Uses the magnitudes of Vgs and Vds (which are typically negative
    /// in a circuit).
    pub fn region(&self, vgs: f64, vds: f64) -> MOSFETRegion {
        let vth = self.params.vth;
        let abs_vgs = vgs.abs();
        let abs_vds = vds.abs();

        if abs_vgs < vth {
            return MOSFETRegion::Cutoff;
        }

        let vov = abs_vgs - vth;
        if abs_vds < vov {
            MOSFETRegion::Linear
        } else {
            MOSFETRegion::Saturation
        }
    }

    /// Calculate source-to-drain current for PMOS.
    ///
    /// Same equations as NMOS but using absolute values of voltages.
    /// Current magnitude is returned (always >= 0).
    pub fn drain_current(&self, vgs: f64, vds: f64) -> f64 {
        let region = self.region(vgs, vds);
        let k = self.params.k;
        let vth = self.params.vth;

        match region {
            MOSFETRegion::Cutoff => 0.0,
            MOSFETRegion::Linear => {
                let abs_vgs = vgs.abs();
                let abs_vds = vds.abs();
                let vov = abs_vgs - vth;
                k * (vov * abs_vds - 0.5 * abs_vds * abs_vds)
            }
            MOSFETRegion::Saturation => {
                let abs_vgs = vgs.abs();
                let vov = abs_vgs - vth;
                0.5 * k * vov * vov
            }
        }
    }

    /// Digital abstraction: is this PMOS transistor ON?
    ///
    /// PMOS turns ON when Vgs is sufficiently negative (gate pulled
    /// below the source). Returns `true` when |Vgs| >= Vth.
    pub fn is_conducting(&self, vgs: f64) -> bool {
        vgs.abs() >= self.params.vth
    }

    /// Output voltage when used as a pull-up switch.
    ///
    /// PMOS forms the pull-up network in CMOS:
    /// - ON: output ~ Vdd
    /// - OFF: output ~ 0V
    pub fn output_voltage(&self, vgs: f64, vdd: f64) -> f64 {
        if self.is_conducting(vgs) {
            vdd
        } else {
            0.0
        }
    }

    /// Calculate small-signal transconductance gm for PMOS.
    ///
    /// Same formula as NMOS but using absolute values.
    pub fn transconductance(&self, vgs: f64, vds: f64) -> f64 {
        let region = self.region(vgs, vds);
        if region == MOSFETRegion::Cutoff {
            return 0.0;
        }

        let vov = vgs.abs() - self.params.vth;
        self.params.k * vov
    }
}
