//! BJT Transistors — the original solid-state amplifier.
//!
//! # What is a BJT?
//!
//! BJT stands for Bipolar Junction Transistor. Invented in 1947 at Bell Labs
//! by John Bardeen, Walter Brattain, and William Shockley, the BJT replaced
//! vacuum tubes and launched the electronics revolution.
//!
//! A BJT has three terminals:
//!
//! - **Base (B)**: The control terminal. Current here controls the switch.
//! - **Collector (C)**: Current flows IN here (for NPN) or OUT here (for PNP).
//! - **Emitter (E)**: Current flows OUT here (for NPN) or IN here (for PNP).
//!
//! The key difference from MOSFETs: a BJT is CURRENT-controlled. You must
//! supply a continuous current to the base to keep it on. This means:
//!
//! - Base current = wasted power (even in steady state)
//! - Lower input impedance than MOSFETs
//! - But historically faster switching (before CMOS caught up)
//!
//! # The Current Gain (beta)
//!
//! The magic of the BJT is current amplification: Ic = beta * Ib.
//! A tiny base current (microamps) controls a much larger collector current
//! (milliamps). Beta is typically 50-300 for small-signal transistors.
//!
//! # Why CMOS Replaced BJT for Digital Logic
//!
//! In TTL: static power ~1-10 mW per gate. A chip with 1 million gates
//! would consume 1-10 kW just sitting idle!
//!
//! In CMOS: static power ~nanowatts per gate. A chip with 1 billion gates
//! consumes milliwatts in idle.

use crate::types::{BJTParams, BJTRegion};

/// NPN bipolar junction transistor.
///
/// An NPN transistor turns ON when current flows into the base terminal
/// (Vbe > ~0.7V). A small base current controls a much larger collector
/// current through the current gain relationship: Ic = beta * Ib.
///
/// # Operating regions
///
/// - **Cutoff**: Vbe < 0.7V -> no base current -> no collector current.
/// - **Active**: Vbe >= 0.7V, Vce > 0.2V -> Ic = beta * Ib. AMPLIFIER region.
/// - **Saturation**: Vbe >= 0.7V, Vce <= 0.2V -> transistor fully ON as switch.
pub struct NPN {
    pub params: BJTParams,
}

impl NPN {
    /// Create a new NPN transistor with the given parameters.
    pub fn new(params: Option<BJTParams>) -> Self {
        Self {
            params: params.unwrap_or_default(),
        }
    }

    /// Determine the operating region from terminal voltages.
    ///
    /// - Cutoff: Vbe < Vbe_on
    /// - Saturation: Vbe >= Vbe_on AND Vce <= Vce_sat
    /// - Active: Vbe >= Vbe_on AND Vce > Vce_sat
    pub fn region(&self, vbe: f64, vce: f64) -> BJTRegion {
        if vbe < self.params.vbe_on {
            return BJTRegion::Cutoff;
        }

        if vce <= self.params.vce_sat {
            BJTRegion::Saturation
        } else {
            BJTRegion::Active
        }
    }

    /// Calculate collector current (Ic) in amperes.
    ///
    /// Uses the simplified Ebers-Moll model:
    ///
    /// - **Cutoff**: Ic = 0
    /// - **Active**: Ic = Is * (exp(Vbe / Vt) - 1), where Vt ~ 26mV at room temp
    /// - **Saturation**: Same formula (transistor at edge of saturation)
    ///
    /// The exponential relationship is why BJTs are such good amplifiers —
    /// a small change in Vbe causes a large change in Ic.
    pub fn collector_current(&self, vbe: f64, vce: f64) -> f64 {
        let region = self.region(vbe, vce);

        if region == BJTRegion::Cutoff {
            return 0.0;
        }

        // Thermal voltage: Vt = kT/q ~ 26mV at room temperature
        let vt = 0.026;
        let exponent = (vbe / vt).min(40.0); // Clamp to prevent overflow
        self.params.is_ * (exponent.exp() - 1.0)
    }

    /// Calculate base current (Ib) in amperes.
    ///
    /// Ib = Ic / beta in the active region.
    ///
    /// This is the "wasted" current that makes BJTs less efficient than
    /// MOSFETs for digital logic.
    pub fn base_current(&self, vbe: f64, vce: f64) -> f64 {
        let ic = self.collector_current(vbe, vce);
        if ic == 0.0 {
            return 0.0;
        }
        ic / self.params.beta
    }

    /// Digital abstraction: is this transistor ON?
    ///
    /// Returns `true` when Vbe >= Vbe_on (typically 0.7V).
    pub fn is_conducting(&self, vbe: f64) -> bool {
        vbe >= self.params.vbe_on
    }

    /// Calculate small-signal transconductance gm.
    ///
    /// For a BJT in the active region: gm = Ic / Vt.
    ///
    /// BJTs typically have higher gm than MOSFETs for the same current,
    /// which is why they're still preferred for some analog applications.
    pub fn transconductance(&self, vbe: f64, vce: f64) -> f64 {
        let ic = self.collector_current(vbe, vce);
        if ic == 0.0 {
            return 0.0;
        }
        let vt = 0.026;
        ic / vt
    }
}

/// PNP bipolar junction transistor.
///
/// The complement of NPN. A PNP transistor turns ON when the base is
/// pulled LOW relative to the emitter (Veb > 0.7V, equivalently
/// Vbe < -0.7V in our convention).
///
/// # Voltage conventions
///
/// For PNP, the "natural" voltages are reversed from NPN:
/// - Vbe is typically NEGATIVE (base below emitter)
/// - Vce is typically NEGATIVE (collector below emitter)
///
/// We use absolute values internally, same as PMOS.
pub struct PNP {
    pub params: BJTParams,
}

impl PNP {
    /// Create a new PNP transistor with the given parameters.
    pub fn new(params: Option<BJTParams>) -> Self {
        Self {
            params: params.unwrap_or_default(),
        }
    }

    /// Determine operating region for PNP.
    ///
    /// Uses absolute values of Vbe and Vce since PNP operates with
    /// reversed polarities.
    pub fn region(&self, vbe: f64, vce: f64) -> BJTRegion {
        let abs_vbe = vbe.abs();
        let abs_vce = vce.abs();

        if abs_vbe < self.params.vbe_on {
            return BJTRegion::Cutoff;
        }

        if abs_vce <= self.params.vce_sat {
            BJTRegion::Saturation
        } else {
            BJTRegion::Active
        }
    }

    /// Calculate collector current magnitude for PNP.
    ///
    /// Same equations as NPN but using absolute values.
    /// Returns current magnitude (always >= 0).
    pub fn collector_current(&self, vbe: f64, vce: f64) -> f64 {
        let region = self.region(vbe, vce);

        if region == BJTRegion::Cutoff {
            return 0.0;
        }

        let abs_vbe = vbe.abs();
        let vt = 0.026;
        let exponent = (abs_vbe / vt).min(40.0);
        self.params.is_ * (exponent.exp() - 1.0)
    }

    /// Calculate base current magnitude for PNP.
    pub fn base_current(&self, vbe: f64, vce: f64) -> f64 {
        let ic = self.collector_current(vbe, vce);
        if ic == 0.0 {
            return 0.0;
        }
        ic / self.params.beta
    }

    /// Digital abstraction: is this PNP transistor ON?
    ///
    /// PNP turns ON when |Vbe| >= Vbe_on (base pulled below emitter).
    pub fn is_conducting(&self, vbe: f64) -> bool {
        vbe.abs() >= self.params.vbe_on
    }

    /// Calculate small-signal transconductance gm for PNP.
    pub fn transconductance(&self, vbe: f64, vce: f64) -> f64 {
        let ic = self.collector_current(vbe, vce);
        if ic == 0.0 {
            return 0.0;
        }
        let vt = 0.026;
        ic / vt
    }
}
