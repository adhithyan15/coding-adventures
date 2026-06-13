//! Semiconductor device physics primitives.
//!
//! Implements physical constants for silicon and SiO2, intrinsic carrier
//! concentration, Fermi potential, PN junction analysis (built-in voltage,
//! depletion width, Shockley current), and MOSFET threshold voltage with body
//! effect.  All values use SI units (meters, Kelvin, Volts, Amperes, Farads)
//! unless otherwise noted.
//!
//! # Quick reference
//!
//! ```text
//! thermal_voltage(300) ≈ 0.02585 V        (kT/q at room temperature)
//! N_I_300K             = 1e16 /m³          (≈ 1e10 /cm³ Si at 300 K)
//! EPS_OX / T_ox        → gate oxide capacitance per unit area
//! ```
//!
//! # References
//! Sedra/Smith *Microelectronic Circuits* 7e; Pierret *Semiconductor Device
//! Fundamentals*; Streetman & Banerjee *Solid State Electronic Devices* 7e.

// ---------------------------------------------------------------------------
// Physical constants
// ---------------------------------------------------------------------------

/// Boltzmann constant [J/K].
pub const K_BOLTZMANN: f64 = 1.380_649e-23;

/// Elementary charge [C].
pub const Q_ELECTRON: f64 = 1.602_176_634e-19;

/// Vacuum permittivity ε₀ [F/m].
pub const EPS0: f64 = 8.854_187_812_8e-12;

/// Silicon permittivity εₛᵢ = 11.7 × ε₀ [F/m].
pub const EPS_SI: f64 = 11.7 * EPS0;

/// SiO₂ permittivity εₒₓ = 3.9 × ε₀ [F/m].
pub const EPS_OX: f64 = 3.9 * EPS0;

/// Silicon intrinsic carrier concentration at 300 K [/m³].
///
/// ≈ 1 × 10¹⁰ /cm³ in commonly-cited experimental values; we keep 1e16 /m³
/// (the SI equivalent) consistent with the Python device-physics package.
pub const N_I_300K: f64 = 1.0e16;

/// Effective density of states in the silicon conduction band at 300 K [/m³].
pub const N_C: f64 = 2.8e25;

/// Effective density of states in the silicon valence band at 300 K [/m³].
pub const N_V: f64 = 1.04e25;

/// Silicon bandgap at 300 K [eV].
pub const EG_SI_300K: f64 = 1.12;

/// Electron drift mobility in lightly-doped Si at low field, 300 K [m²/V·s].
pub const MU_N_300K: f64 = 1350e-4;

/// Hole drift mobility in lightly-doped Si at low field, 300 K [m²/V·s].
pub const MU_P_300K: f64 = 480e-4;

// ---------------------------------------------------------------------------
// Thermal voltage
// ---------------------------------------------------------------------------

/// Thermal voltage V_T = kT/q [V].
///
/// At 300 K this evaluates to ≈ 25.85 mV, the standard room-temperature
/// thermal voltage used throughout SPICE and semiconductor physics texts.
///
/// # Examples
/// ```
/// use device_physics::thermal_voltage;
/// let vt = thermal_voltage(300.0);
/// assert!((vt - 0.025852).abs() < 1e-5);
/// ```
pub fn thermal_voltage(t_kelvin: f64) -> f64 {
    K_BOLTZMANN * t_kelvin / Q_ELECTRON
}

// ---------------------------------------------------------------------------
// Intrinsic carrier concentration
// ---------------------------------------------------------------------------

/// Intrinsic carrier concentration n_i(T) [/m³].
///
/// Uses the standard temperature scaling
///
/// ```text
///   n_i(T) = N_I_300K × (T/300)^(3/2) × exp(−Eg/(2kT) × (1 − T/300))
/// ```
///
/// Valid above 100 K; returns an error below that because Boltzmann statistics
/// break down at very low temperatures and the result would be physically
/// meaningless (effectively 0).
///
/// # Errors
/// Returns `Err` when `T < 100.0 K`.
pub fn intrinsic_concentration(t_kelvin: f64) -> Result<f64, String> {
    if (t_kelvin - 300.0).abs() < 1e-9 {
        return Ok(N_I_300K);
    }
    if t_kelvin < 100.0 {
        return Err(format!(
            "T={t_kelvin} K below model validity (>= 100 K)"
        ));
    }
    let factor = (t_kelvin / 300.0).powf(1.5);
    let vt = thermal_voltage(t_kelvin);
    // The bandgap term models how n_i changes with T relative to 300 K.
    let bandgap_term = (-(EG_SI_300K / (2.0 * vt)) * (1.0 - t_kelvin / 300.0)).exp();
    Ok(N_I_300K * factor * bandgap_term)
}

// ---------------------------------------------------------------------------
// Fermi potential
// ---------------------------------------------------------------------------

/// Fermi potential φ_F for a doped silicon sample.
///
/// For **p-type** silicon (`kind = "p"`): φ_F = +kT/q × ln(N/nᵢ) > 0.
/// For **n-type** silicon (`kind = "n"`): φ_F = −kT/q × ln(N/nᵢ) < 0.
///
/// `N` is the majority-carrier doping concentration [/m³].
///
/// # Errors
/// Returns `Err` when `N <= 0` or `kind` is neither `"p"` nor `"n"`.
pub fn fermi_potential(n_doping: f64, kind: &str, t_kelvin: f64) -> Result<f64, String> {
    if n_doping <= 0.0 {
        return Err(format!("doping N must be > 0, got {n_doping}"));
    }
    let n_i = intrinsic_concentration(t_kelvin)?;
    let magnitude = thermal_voltage(t_kelvin) * (n_doping / n_i).ln();
    match kind {
        "p" => Ok(magnitude),
        "n" => Ok(-magnitude),
        other => Err(format!("kind must be 'p' or 'n', got {other:?}")),
    }
}

// ---------------------------------------------------------------------------
// PN junction
// ---------------------------------------------------------------------------

/// A PN junction with given doping levels and junction area.
///
/// The junction is assumed to be abrupt (step) and planar.  All calculations
/// follow the textbook depletion approximation; the saturation current uses
/// the Shockley minority-carrier diffusion model.
///
/// ```text
///   p-side: doped N_A acceptors (/m³)
///   n-side: doped N_D donors (/m³)
///   area A (m²)
/// ```
///
/// # Example
/// ```
/// use device_physics::PNJunction;
/// let j = PNJunction::new(1e23, 1e22, 1e-8, 300.0, 1e-6, 1e-6).unwrap();
/// let vbi = j.built_in_voltage();
/// assert!(vbi > 0.5 && vbi < 1.2, "built-in voltage should be ~0.7 V");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PNJunction {
    /// Acceptor doping concentration on the p-side [/m³].
    pub na: f64,
    /// Donor doping concentration on the n-side [/m³].
    pub nd: f64,
    /// Junction area [m²].
    pub a: f64,
    /// Temperature [K].
    pub t: f64,
    /// Minority-carrier electron lifetime [s].
    pub tau_n: f64,
    /// Minority-carrier hole lifetime [s].
    pub tau_p: f64,
}

impl PNJunction {
    /// Construct a PN junction.  Returns an error for non-positive dopings or area.
    pub fn new(
        na: f64,
        nd: f64,
        a: f64,
        t: f64,
        tau_n: f64,
        tau_p: f64,
    ) -> Result<Self, String> {
        if na <= 0.0 || nd <= 0.0 {
            return Err(format!("doping must be > 0, got N_A={na}, N_D={nd}"));
        }
        if a <= 0.0 {
            return Err(format!("area A must be > 0, got {a}"));
        }
        Ok(Self { na, nd, a, t, tau_n, tau_p })
    }

    /// Built-in (contact) potential V_bi [V].
    ///
    /// ```text
    ///   V_bi = kT/q × ln(N_A × N_D / nᵢ²)
    /// ```
    pub fn built_in_voltage(&self) -> f64 {
        let n_i = intrinsic_concentration(self.t).unwrap_or(N_I_300K);
        thermal_voltage(self.t) * ((self.na * self.nd) / (n_i * n_i)).ln()
    }

    /// Depletion-region total width W [m] under applied bias `v_applied` [V].
    ///
    /// Forward bias narrows the depletion region; reverse bias widens it.
    /// Positive `v_applied` means forward bias.  When `v_applied ≥ V_bi`
    /// (heavy injection), the width is clamped to 0.
    ///
    /// ```text
    ///   W = sqrt(2εSi/q × (N_A + N_D)/(N_A × N_D) × (V_bi − V_applied))
    /// ```
    pub fn depletion_width(&self, v_applied: f64) -> f64 {
        let phi_bi = self.built_in_voltage();
        if v_applied >= phi_bi {
            return 0.0;
        }
        let num = 2.0 * EPS_SI * (self.na + self.nd) * (phi_bi - v_applied);
        let den = Q_ELECTRON * self.na * self.nd;
        (num / den).sqrt()
    }

    /// Saturation current I_S [A] from minority-carrier diffusion.
    ///
    /// Uses the Einstein relation Dₙ = μₙ V_T and Dₚ = μₚ V_T to derive
    /// diffusion coefficients, then computes diffusion lengths Lₙ = √(Dₙ τₙ).
    ///
    /// ```text
    ///   I_S = q A nᵢ² (Dₙ / (Lₙ N_A) + Dₚ / (Lₚ N_D))
    /// ```
    pub fn saturation_current(&self) -> f64 {
        let n_i = intrinsic_concentration(self.t).unwrap_or(N_I_300K);
        let vt = thermal_voltage(self.t);
        let d_n = MU_N_300K * vt; // Einstein: Dₙ = μₙ kT/q
        let d_p = MU_P_300K * vt;
        let l_n = (d_n * self.tau_n).sqrt(); // diffusion length Lₙ = √(Dₙ τₙ)
        let l_p = (d_p * self.tau_p).sqrt();
        Q_ELECTRON
            * self.a
            * n_i
            * n_i
            * (d_n / (l_n * self.na) + d_p / (l_p * self.nd))
    }

    /// Diode current I [A] via the Shockley equation.
    ///
    /// ```text
    ///   I = I_S × (exp(V / V_T) − 1)
    /// ```
    pub fn current(&self, v: f64) -> f64 {
        let vt = thermal_voltage(self.t);
        self.saturation_current() * ((v / vt).exp() - 1.0)
    }
}

// ---------------------------------------------------------------------------
// MOSFET threshold voltage
// ---------------------------------------------------------------------------

/// Physical parameters of a MOSFET, sufficient to compute V_t.
///
/// 'NMOS' has a p-type body; 'PMOS' has an n-type body.
///
/// The threshold voltage includes the flat-band shift due to gate–body
/// work-function difference and trapped oxide charge, plus the body effect.
///
/// # Example
/// ```
/// use device_physics::MOSFETParams;
/// // 130 nm NMOS: t_ox = 2 nm, N_body = 1e24 /m³, phi_MS = −0.05 V
/// let p = MOSFETParams::new(
///     "NMOS",
///     130e-9, 1e-6,
///     2e-9,
///     1e24,
///     -0.05, 0.0, 300.0,
/// ).unwrap();
/// let vt = p.threshold_voltage(0.0).unwrap();
/// // High body doping (1e24 /m³) gives V_t ~ 1.2 V
/// assert!(vt > 0.5 && vt < 1.6, "threshold in range for high-doping p-well");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct MOSFETParams {
    /// Device type: `"NMOS"` or `"PMOS"`.
    pub device_type: String,
    /// Channel length L [m].
    pub l: f64,
    /// Channel width W [m].
    pub w: f64,
    /// Gate-oxide thickness T_ox [m].
    pub t_ox: f64,
    /// Body doping concentration N_body [/m³].
    pub n_body: f64,
    /// Gate–body work-function difference φ_MS [V].
    pub phi_ms: f64,
    /// Oxide trapped charge per area Q_ox [C/m²].
    pub q_ox: f64,
    /// Analysis temperature [K].
    pub t: f64,
}

impl MOSFETParams {
    /// Construct a `MOSFETParams`.  Returns an error for invalid device type,
    /// non-positive L/W/T_ox/N_body, or out-of-range temperature.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        device_type: &str,
        l: f64,
        w: f64,
        t_ox: f64,
        n_body: f64,
        phi_ms: f64,
        q_ox: f64,
        t: f64,
    ) -> Result<Self, String> {
        if device_type != "NMOS" && device_type != "PMOS" {
            return Err(format!(
                "type must be NMOS or PMOS, got {device_type:?}"
            ));
        }
        if l <= 0.0 || w <= 0.0 {
            return Err(format!("L and W must be > 0, got L={l}, W={w}"));
        }
        if t_ox <= 0.0 {
            return Err(format!("T_ox must be > 0, got {t_ox}"));
        }
        if n_body <= 0.0 {
            return Err(format!("N_body must be > 0, got {n_body}"));
        }
        Ok(Self {
            device_type: device_type.to_owned(),
            l,
            w,
            t_ox,
            n_body,
            phi_ms,
            q_ox,
            t,
        })
    }

    /// Oxide capacitance per unit area C_ox = ε_ox / T_ox [F/m²].
    pub fn c_ox(&self) -> f64 {
        EPS_OX / self.t_ox
    }

    /// Flat-band voltage V_FB = φ_MS − Q_ox/C_ox [V].
    pub fn v_fb(&self) -> f64 {
        self.phi_ms - self.q_ox / self.c_ox()
    }

    /// Magnitude of body Fermi potential φ_F [V].
    ///
    /// For NMOS (p-type body): φ_F = V_T × ln(N_body / nᵢ) > 0.
    /// For PMOS (n-type body): same magnitude.
    pub fn phi_f(&self) -> f64 {
        let kind = if self.device_type == "NMOS" { "p" } else { "n" };
        fermi_potential(self.n_body, kind, self.t)
            .map(|v| v.abs())
            .unwrap_or(0.0)
    }

    /// Body-effect coefficient γ [V^(1/2)].
    ///
    /// ```text
    ///   γ = √(2 εSi q N_body) / C_ox
    /// ```
    pub fn gamma(&self) -> f64 {
        (2.0 * EPS_SI * Q_ELECTRON * self.n_body).sqrt() / self.c_ox()
    }

    /// Threshold voltage V_t(V_SB) [V] with optional source-body reverse bias.
    ///
    /// V_SB is positive when the source is above the body (typical for NMOS
    /// with a grounded body: V_SB = 0 normally).  Increasing V_SB raises V_t
    /// (body effect).
    ///
    /// ```text
    ///   V_t0 = V_FB + 2φ_F + γ√(2φ_F)
    ///   V_t  = V_t0 + γ (√(2φ_F + V_SB) − √(2φ_F))
    /// ```
    ///
    /// # Errors
    /// Returns `Err` when `V_SB < −2φ_F` (source–body forward biased beyond
    /// the threshold point, which makes the sqrt argument negative).
    pub fn threshold_voltage(&self, v_sb: f64) -> Result<f64, String> {
        let phi_f = self.phi_f();
        let two_phi_f = 2.0 * phi_f;
        if -two_phi_f > v_sb {
            return Err(format!(
                "V_SB={v_sb} below 2*phi_F={two_phi_f}; body-source forward biased"
            ));
        }
        let gamma = self.gamma();
        let v_t0 = self.v_fb() + two_phi_f + gamma * two_phi_f.sqrt();
        Ok(v_t0 + gamma * ((two_phi_f + v_sb).sqrt() - two_phi_f.sqrt()))
    }
}
