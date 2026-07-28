//! SPICE Level-1 (Shockley) MOSFET I-V model.
//!
//! The classical square-law model with ~15 parameters, analytical Jacobian,
//! and optional subthreshold conduction.  Suitable for hand calculations and
//! the canonical CMOS smoke tests (inverter, 4-bit adder, ring oscillator).
//!
//! # Operating regions
//!
//! | Region      | Condition             | Id equation                              |
//! |-------------|----------------------|------------------------------------------|
//! | Cutoff      | V_OV ≤ 0             | 0 (or subthreshold exp if enabled)       |
//! | Triode      | 0 < V_DS < V_OV      | β(V_OV V_DS − V_DS²/2)(1 + λV_DS)      |
//! | Saturation  | V_DS ≥ V_OV          | (β/2) V_OV² (1 + λV_DS)                 |
//!
//! where β = KP × W/L and V_OV = V_GS − V_t.
//!
//! # References
//! Sedra/Smith *Microelectronic Circuits* 7e §4.7; SPICE3 user's guide.

use device_physics::thermal_voltage;

const OXIDE_PERMITTIVITY: f64 = 3.453_133e-11;

// ---------------------------------------------------------------------------
// Level-1 parameter set
// ---------------------------------------------------------------------------

/// SPICE Level-1 MOSFET parameter set.
///
/// Defaults are typical for a 130 nm-node NMOS device at room temperature.
///
/// | Parameter | Description                        | Default  |
/// |-----------|-----------------------------------|----------|
/// | VT0       | Threshold voltage at V_BS=0 (V)   | 0.42     |
/// | KP        | Transconductance μC_ox (A/V²)     | 220 µA/V²|
/// | LAMBDA    | Channel-length modulation (1/V)    | 0.05     |
/// | GAMMA     | Body-effect coefficient (√V)       | 0.27     |
/// | PHI       | Surface potential 2φ_F (V)         | 0.84     |
/// | W         | Channel width (m)                  | 1 µm     |
/// | L         | Channel length (m)                 | 130 nm   |
/// | LD        | Lateral diffusion length (m)       | 0        |
/// | TOX       | Gate oxide thickness (m)           | 100 nm   |
/// | RD        | Drain resistance (ohm)              | 0        |
/// | RS        | Source resistance (ohm)             | 0        |
/// | RSH       | Drain/source sheet resistance (ohm) | 0        |
/// | NRD       | Number of drain squares              | 1        |
/// | IS        | Drain–body saturation current (A)  | 1 fA     |
/// | N_SUB     | Subthreshold slope factor          | 1.4      |
/// | T_NOM     | Nominal temperature (K)            | 300.15   |
/// | KF        | Flicker-noise coefficient           | 0        |
/// | AF        | Flicker-noise current exponent      | 1        |
#[derive(Debug, Clone, PartialEq)]
pub struct Level1Params {
    /// Threshold voltage at zero source-body bias [V].
    pub vt0: f64,
    /// Process transconductance parameter μₙCₒₓ [A/V²].
    pub kp: f64,
    /// Channel-length modulation coefficient [1/V].
    pub lambda: f64,
    /// Body-effect coefficient γ [√V].
    pub gamma: f64,
    /// Surface potential at threshold 2φ_F [V].
    pub phi: f64,
    /// Channel width [m].
    pub w: f64,
    /// Channel length [m].
    pub l: f64,
    /// Source/drain lateral diffusion length [m].
    pub ld: f64,
    /// Gate oxide thickness [m].
    pub tox: f64,
    /// External drain resistance [ohm].
    pub rd: f64,
    /// External source resistance [ohm].
    pub rs: f64,
    /// Drain/source sheet resistance [ohm per square].
    pub rsh: f64,
    /// Number of squares in the drain diffusion.
    pub nrd: f64,
    /// Drain–body saturation current [A] (used for subthreshold floor).
    pub is: f64,
    /// Subthreshold slope factor n.  Subthreshold current ∝ exp(V_OV / (n V_T)).
    pub n_sub: f64,
    /// Nominal temperature [K].
    pub t_nom: f64,
    /// Gate–source overlap capacitance per unit width [F/m].
    pub cgso: f64,
    /// Gate–drain overlap capacitance per unit width [F/m].
    pub cgdo: f64,
    /// Gate–bulk overlap capacitance per unit length [F/m].
    pub cgbo: f64,
    /// Source–bulk zero-bias junction capacitance [F].
    pub cbs: f64,
    /// Drain–bulk zero-bias junction capacitance [F].
    pub cbd: f64,
    /// Bulk-junction potential [V].
    pub pb: f64,
    /// Bulk-junction grading coefficient.
    pub mj: f64,
    /// Forward-bias depletion-capacitance transition coefficient.
    pub fc: f64,
    /// Flicker-noise coefficient.
    pub kf: f64,
    /// Flicker-noise drain-current exponent.
    pub af: f64,
    /// Enable subthreshold current below V_t.
    pub subthreshold_enable: bool,
}

impl Default for Level1Params {
    fn default() -> Self {
        Self {
            vt0: 0.42,
            kp: 220e-6,
            lambda: 0.05,
            gamma: 0.27,
            phi: 0.84,
            w: 1e-6,
            l: 130e-9,
            ld: 0.0,
            tox: 1.0e-7,
            rd: 0.0,
            rs: 0.0,
            rsh: 0.0,
            nrd: 1.0,
            is: 1e-15,
            n_sub: 1.4,
            t_nom: 300.15,
            cgso: 0.0,
            cgdo: 0.0,
            cgbo: 0.0,
            cbs: 0.0,
            cbd: 0.0,
            pb: 0.8,
            mj: 0.5,
            fc: 0.5,
            kf: 0.0,
            af: 1.0,
            subthreshold_enable: true,
        }
    }
}

/// Return the Level-1 bulk-junction depletion capacitance.
pub fn bulk_junction_capacitance(
    zero_bias_capacitance: f64,
    junction_voltage: f64,
    junction_potential: f64,
    grading_coefficient: f64,
    forward_bias_coefficient: f64,
) -> f64 {
    if zero_bias_capacitance <= 0.0 {
        return zero_bias_capacitance;
    }
    if junction_potential <= 0.0 || grading_coefficient == 0.0 {
        return zero_bias_capacitance;
    }
    let normalized_voltage = junction_voltage / junction_potential;
    if normalized_voltage < forward_bias_coefficient {
        return zero_bias_capacitance / (1.0 - normalized_voltage).powf(grading_coefficient);
    }
    let denominator = (1.0 - forward_bias_coefficient).powf(1.0 + grading_coefficient);
    let continuation = 1.0 - forward_bias_coefficient * (1.0 + grading_coefficient)
        + grading_coefficient * normalized_voltage;
    zero_bias_capacitance * continuation / denominator
}

// ---------------------------------------------------------------------------
// Operating region
// ---------------------------------------------------------------------------

/// MOSFET operating region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Region {
    /// V_OV ≤ 0 and subthreshold disabled.
    Cutoff,
    /// V_OV ≤ 0 and subthreshold enabled.
    Subthreshold,
    /// 0 < V_DS < V_OV.
    Triode,
    /// V_DS ≥ V_OV > 0.
    Saturation,
}

impl Region {
    /// ASCII name matching the Python string values.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cutoff => "cutoff",
            Self::Subthreshold => "subthreshold",
            Self::Triode => "triode",
            Self::Saturation => "saturation",
        }
    }
}

// ---------------------------------------------------------------------------
// Evaluation result
// ---------------------------------------------------------------------------

/// One operating-point evaluation of a MOSFET.
///
/// All transconductances are in A/V and all capacitances in Farads.
#[derive(Debug, Clone, PartialEq)]
pub struct MosResult {
    /// Drain current [A] (positive for conventional NMOS direction).
    pub id: f64,
    /// Transconductance gm = ∂Id/∂Vgs [A/V].
    pub gm: f64,
    /// Output conductance gds = ∂Id/∂Vds [A/V].
    pub gds: f64,
    /// Body transconductance gmb = ∂Id/∂Vbs [A/V].
    pub gmb: f64,
    /// Total gate–source capacitance [F].
    pub cgs: f64,
    /// Total gate–drain capacitance [F].
    pub cgd: f64,
    /// Total gate–bulk capacitance [F].
    pub cgb: f64,
    /// Source–bulk capacitance [F].
    pub cbs: f64,
    /// Drain–bulk capacitance [F].
    pub cbd: f64,
    /// Operating region.
    pub region: Region,
}

// ---------------------------------------------------------------------------
// Core evaluation
// ---------------------------------------------------------------------------

/// Evaluate the Level-1 MOSFET model at the given operating point.
///
/// All voltages are NMOS-convention (positive V_GS for inversion).  For PMOS
/// devices, callers must negate the input voltages and negate Id in the result;
/// see [`Mosfet::dc`].
///
/// # Capacitance model
///
/// Overlap capacitances are per-width or per-length, scaled by W or L.
/// Intrinsic gate capacitance follows the piecewise Meyer model:
///
/// - **Cutoff / subthreshold**: C_gs = C_gd ≈ 0 (no channel inversion layer)
/// - **Triode**: C_gs = C_gd ≈ (1/2) W L KP   (channel shared 50/50)
/// - **Saturation**: C_gs = (2/3) W L KP, C_gd = 0   (pinched-off channel)
pub fn evaluate_level1(
    params: &Level1Params,
    v_gs: f64,
    v_ds: f64,
    v_bs: f64,
    t: f64,
) -> MosResult {
    let p = params;
    let effective_length = p.l - 2.0 * p.ld;
    assert!(
        p.ld.is_finite() && p.ld >= 0.0 && effective_length > 0.0,
        "MOSFET LD must be finite and non-negative with L - 2*LD > 0"
    );
    assert!(
        p.tox.is_finite() && p.tox > 0.0,
        "MOSFET TOX must be finite and positive"
    );
    assert!(
        p.rd.is_finite() && p.rd >= 0.0,
        "MOSFET RD must be finite and non-negative"
    );
    assert!(
        p.rs.is_finite() && p.rs >= 0.0,
        "MOSFET RS must be finite and non-negative"
    );
    assert!(
        p.rsh.is_finite() && p.rsh >= 0.0,
        "MOSFET RSH must be finite and non-negative"
    );
    assert!(
        p.nrd.is_finite() && p.nrd >= 0.0,
        "MOSFET NRD must be finite and non-negative"
    );
    let beta = p.kp * (p.w / effective_length);

    // Threshold with body effect.
    // The formula √(PHI − V_BS) is valid when PHI ≥ V_BS.
    // For strong forward body bias (V_BS > PHI), clamp V_t to VT0.
    let v_t = if p.phi - v_bs >= 0.0 {
        p.vt0 + p.gamma * ((p.phi - v_bs).sqrt() - p.phi.sqrt())
    } else {
        p.vt0
    };

    let v_ov = v_gs - v_t;
    let vth = thermal_voltage(t); // kT/q at operating temperature

    // Overlap capacitances scale with W or L.
    let cgs_overlap = p.cgso * p.w;
    let cgd_overlap = p.cgdo * p.w;
    let cgb_overlap = p.cgbo * effective_length;

    // Meyer gate-to-channel capacitance, partitioned by operating region below.
    let channel_capacitance = p.w * effective_length * (OXIDE_PERMITTIVITY / p.tox);
    let cbs_bulk = bulk_junction_capacitance(p.cbs, v_bs, p.pb, p.mj, p.fc);
    let cbd_bulk = bulk_junction_capacitance(p.cbd, v_bs - v_ds, p.pb, p.mj, p.fc);

    // -----------------------------------------------------------------------
    // Cutoff / subthreshold
    // -----------------------------------------------------------------------
    if v_ov <= 0.0 {
        if p.subthreshold_enable {
            // Subthreshold: Id = β n V_T² exp(V_OV/(n V_T)) (1 − exp(−V_DS/V_T))
            // This smoothly matches the strong-inversion model at V_OV ≈ 0.
            let n = p.n_sub;
            let id_sub =
                beta * n * vth * vth * (v_ov / (n * vth)).exp() * (1.0 - (-v_ds / vth).exp());
            let gm_sub = id_sub / (n * vth);
            let gds_sub = (beta * n * vth) * (v_ov / (n * vth)).exp() * (-v_ds / vth).exp();
            return MosResult {
                id: id_sub,
                gm: gm_sub,
                gds: gds_sub,
                gmb: 0.0,
                cgs: cgs_overlap + channel_capacitance,
                cgd: cgd_overlap,
                cgb: cgb_overlap,
                cbs: cbs_bulk,
                cbd: cbd_bulk,
                region: Region::Subthreshold,
            };
        }
        // Hard cutoff.
        return MosResult {
            id: 0.0,
            gm: 0.0,
            gds: 0.0,
            gmb: 0.0,
            cgs: cgs_overlap + channel_capacitance,
            cgd: cgd_overlap,
            cgb: cgb_overlap,
            cbs: cbs_bulk,
            cbd: cbd_bulk,
            region: Region::Cutoff,
        };
    }

    // Body transconductance via chain rule: gmb = −gm × dV_t/dV_BS.
    // dV_t/dV_BS = −γ / (2 √(PHI − V_BS)) when PHI > V_BS.
    let dvt_dvbs = if p.phi - v_bs > 0.0 {
        -p.gamma / (2.0 * (p.phi - v_bs).sqrt())
    } else {
        0.0
    };

    // -----------------------------------------------------------------------
    // Triode (linear) region: 0 < V_DS < V_OV
    // -----------------------------------------------------------------------
    if v_ds < v_ov {
        let id = beta * (v_ov * v_ds - v_ds * v_ds / 2.0) * (1.0 + p.lambda * v_ds);
        let gm = beta * v_ds * (1.0 + p.lambda * v_ds);
        let gds = beta * (v_ov - v_ds) * (1.0 + p.lambda * v_ds)
            + beta * (v_ov * v_ds - v_ds * v_ds / 2.0) * p.lambda;
        let gmb = -gm * dvt_dvbs;
        return MosResult {
            id,
            gm,
            gds,
            gmb,
            cgs: cgs_overlap + channel_capacitance / 2.0,
            cgd: cgd_overlap + channel_capacitance / 2.0,
            cgb: cgb_overlap,
            cbs: cbs_bulk,
            cbd: cbd_bulk,
            region: Region::Triode,
        };
    }

    // -----------------------------------------------------------------------
    // Saturation region: V_DS ≥ V_OV
    // -----------------------------------------------------------------------
    let id = (beta / 2.0) * v_ov * v_ov * (1.0 + p.lambda * v_ds);
    let gm = beta * v_ov * (1.0 + p.lambda * v_ds);
    let gds = (beta / 2.0) * v_ov * v_ov * p.lambda;
    let gmb = -gm * dvt_dvbs;
    MosResult {
        id,
        gm,
        gds,
        gmb,
        cgs: cgs_overlap + (2.0 / 3.0) * channel_capacitance,
        cgd: cgd_overlap,
        cgb: cgb_overlap,
        cbs: cbs_bulk,
        cbd: cbd_bulk,
        region: Region::Saturation,
    }
}

// ---------------------------------------------------------------------------
// MOSFET type + high-level wrapper
// ---------------------------------------------------------------------------

/// Whether a MOSFET is n-channel or p-channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MosfetType {
    /// N-channel MOSFET (p-type body; inverts with positive V_GS).
    Nmos,
    /// P-channel MOSFET (n-type body; inverts with negative V_GS).
    Pmos,
}

/// A complete MOSFET device: type (NMOS / PMOS) + Level-1 parameter set.
///
/// PMOS callers receive sign-corrected currents: internally the PMOS voltages
/// are negated before evaluation and the resulting drain current is negated
/// so the caller sees conventional PMOS convention (negative Id for drain
/// current flowing *out* of the drain).
///
/// # Example
/// ```
/// use mosfet_models::{Mosfet, MosfetType, Level1Params};
/// let m = Mosfet::new(MosfetType::Nmos, Level1Params::default());
/// let r = m.dc(1.8, 1.8, 0.0, 300.15);
/// assert_eq!(r.region.as_str(), "saturation");
/// assert!(r.id > 0.0);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Mosfet {
    /// Channel type.
    pub mos_type: MosfetType,
    /// Level-1 parameter set.
    pub params: Level1Params,
}

impl Mosfet {
    /// Create a new MOSFET with the given type and Level-1 parameters.
    pub fn new(mos_type: MosfetType, params: Level1Params) -> Self {
        Self { mos_type, params }
    }

    /// Evaluate the DC operating point.
    ///
    /// For NMOS: voltages are V_GS, V_DS, V_BS in the natural polarity.
    /// For PMOS: the caller supplies V_GS, V_DS, V_BS with PMOS sign
    ///   convention (V_GS < 0 for inversion); internally they are negated.
    ///   Id in the result is negative for conventional drain current flow.
    pub fn dc(&self, v_gs: f64, v_ds: f64, v_bs: f64, t: f64) -> MosResult {
        match self.mos_type {
            MosfetType::Nmos => evaluate_level1(&self.params, v_gs, v_ds, v_bs, t),
            MosfetType::Pmos => {
                let r = evaluate_level1(&self.params, -v_gs, -v_ds, -v_bs, t);
                MosResult {
                    id: -r.id,
                    gm: r.gm,
                    gds: r.gds,
                    gmb: r.gmb,
                    cgs: r.cgs,
                    cgd: r.cgd,
                    cgb: r.cgb,
                    cbs: r.cbs,
                    cbd: r.cbd,
                    region: r.region,
                }
            }
        }
    }
}

/// A Level-1 model wrapper (compatible with the Python `Level1Model` type).
///
/// Holds a [`Level1Params`] and delegates to [`evaluate_level1`].
#[derive(Debug, Clone, PartialEq)]
pub struct Level1Model {
    pub params: Level1Params,
}

impl Level1Model {
    pub fn new(params: Level1Params) -> Self {
        Self { params }
    }

    /// Evaluate the Level-1 model at the given operating point.
    pub fn dc(&self, v_gs: f64, v_ds: f64, v_bs: f64, t: f64) -> MosResult {
        evaluate_level1(&self.params, v_gs, v_ds, v_bs, t)
    }
}
