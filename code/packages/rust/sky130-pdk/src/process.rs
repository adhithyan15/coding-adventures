//! Sky130 process-level parameters.
//!
//! These are physical constants describing the 130 nm process node. They are
//! used by the SPICE engine and for educational discussions of CMOS physics.
//!
//! Numerics sourced from the Sky130 open PDK documentation:
//! https://skywater-pdk.readthedocs.io/en/main/

use serde::{Deserialize, Serialize};

/// Top-level Sky130 process parameters.
///
/// All values represent the nominal (typical) corner at 25 °C.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessMetadata {
    /// PDK name, e.g. "sky130A".
    pub name: String,
    /// Minimum feature size in nanometres (the "130" in Sky130).
    pub feature_size_nm: u32,
    /// Nominal supply voltage in volts. Sky130 uses 1.8 V for the HD cell family.
    pub vdd_nominal: f64,
    /// Gate-oxide thickness in nanometres (~4.2 nm in sky130A).
    pub gate_oxide_thickness_nm: f64,
    /// Typical NMOS threshold voltage in volts.
    pub nmos_vt_typical: f64,
    /// Typical PMOS threshold voltage in volts (negative for enhancement mode).
    pub pmos_vt_typical: f64,
    /// NMOS μ_n × C_ox product in A/V². Governs transistor drive strength.
    pub mun_cox: f64,
    /// PMOS μ_p × C_ox product in A/V². Roughly 1/3 of NMOS due to hole mobility.
    pub mup_cox: f64,
    /// Number of metal routing layers (li1 + met1-met5 = 6 total).
    pub metal_layers: u32,
    /// Standard-cell row height for the HD (high-density) library in micrometres.
    pub cell_row_height_um: f64,
}

impl Default for ProcessMetadata {
    fn default() -> Self {
        Self {
            name: "sky130A".to_string(),
            feature_size_nm: 130,
            vdd_nominal: 1.8,
            gate_oxide_thickness_nm: 4.2,
            nmos_vt_typical: 0.42,
            pmos_vt_typical: -0.51,
            // NMOS μ_n × C_ox ≈ 220 µA/V² for sky130A
            mun_cox: 220e-6,
            // PMOS μ_p × C_ox ≈ 75 µA/V² (≈ 1/3 of NMOS)
            mup_cox: 75e-6,
            metal_layers: 6,
            // sky130_fd_sc_hd cell row = 2.72 µm
            cell_row_height_um: 2.72,
        }
    }
}
