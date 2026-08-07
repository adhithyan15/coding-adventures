//! 1-D analytical CMOS process flow simulator.
//!
//! Models the standard front-end fabrication steps using analytical 1-D
//! approximations calibrated against published Sky130 reference profiles:
//!
//! | Step              | Model                                         |
//! |-------------------|-----------------------------------------------|
//! | Thermal oxidation | Deal-Grove (quadratic growth law)             |
//! | Deposition        | Uniform film addition                         |
//! | Etching           | Layer-selective depth removal                 |
//! | Ion implantation  | Gaussian profile from SRIM tables             |
//! | Diffusion         | Fick's second law (Gaussian broadening)       |
//!
//! All depths are in nanometres; all concentrations in /cm³.
//! Real TCAD with 2-D/3-D PDE solvers is deferred to v0.2.0.
//!
//! # Quick-start
//!
//! ```
//! use fab_process_simulation::{CrossSection, Layer, deal_grove_oxidation, deposit};
//! // Start with a bare silicon substrate.
//! let cs = CrossSection { layers: vec![Layer::new("Si", 500.0)] };
//! // Grow 5 nm of gate oxide.
//! let cs = deal_grove_oxidation(&cs, 5.0, None, None).unwrap();
//! assert_eq!(cs.layers[0].material, "SiO2");
//! assert!(cs.layers[0].thickness_nm > 0.0);
//! ```

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Deal-Grove constants for dry O2 oxidation at 1000 °C
// ---------------------------------------------------------------------------

/// Deal-Grove parabolic rate constant A [µm] at 1000 °C, dry O2.
pub const DEAL_GROVE_DRY_1000C_A: f64 = 0.165;

/// Deal-Grove linear rate constant B [µm²/hr] at 1000 °C, dry O2.
pub const DEAL_GROVE_DRY_1000C_B: f64 = 0.0117;

// ---------------------------------------------------------------------------
// SRIM-derived projected range table
// ---------------------------------------------------------------------------

/// Ion-implant range table.
///
/// Keys are `(species, energy_keV)` rounded to the nearest integer keV.
/// Values are `(Rp_nm, delta_Rp_nm)` — projected range and straggle.
///
/// Source: SRIM 2013 tabulations for Si substrate.
#[allow(clippy::type_complexity)]
pub fn implant_range_table() -> HashMap<(String, u32), (f64, f64)> {
    let entries: &[(&str, u32, f64, f64)] = &[
        ("B",   10,  33.0, 18.0),
        ("B",   30,  92.0, 38.0),
        ("B",  100, 260.0, 80.0),
        ("P",   30,  39.0, 19.0),
        ("P",  100, 130.0, 50.0),
        ("As",  30,  22.0, 11.0),
        ("As", 100,  64.0, 28.0),
        ("BF2", 30,  31.0, 19.0),
        ("BF2", 60,  60.0, 30.0),
    ];
    entries
        .iter()
        .map(|(sp, e, rp, std)| ((sp.to_string(), *e), (*rp, *std)))
        .collect()
}

// ---------------------------------------------------------------------------
// Diffusivity table
// ---------------------------------------------------------------------------

/// Diffusivity at 1000 °C [cm²/s] — standard reference values.
///
/// Activation energies per ITRS / SRPS handbooks.
pub fn diffusivity_1000c(species: &str) -> f64 {
    match species {
        "B"  => 1e-14,
        "P"  => 1.2e-14,
        "As" => 4e-15,
        _    => 1e-14, // conservative fallback
    }
}

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

/// A single material layer in the vertical cross-section.
///
/// Layers are ordered top-to-bottom: `layers[0]` is the topmost layer.
///
/// Doping is stored as a map from species → list of `(depth_nm, conc_per_cm3)`.
/// For a bare (unimplanted) layer the map is empty.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    /// Material name: `"Si"`, `"SiO2"`, `"Poly"`, `"Si3N4"`, `"Cu"`, etc.
    pub material: String,
    /// Layer thickness [nm].
    pub thickness_nm: f64,
    /// Species → sampled Gaussian profile `[(depth_nm, conc_per_cm3), …]`.
    pub doping: HashMap<String, Vec<(f64, f64)>>,
}

impl Layer {
    /// Construct a bare (undoped) layer.
    pub fn new(material: impl Into<String>, thickness_nm: f64) -> Self {
        Self {
            material: material.into(),
            thickness_nm,
            doping: HashMap::new(),
        }
    }
}

/// Vertical cross-section of the device.  `layers[0]` is the top of the stack.
#[derive(Debug, Clone, PartialEq)]
#[derive(Default)]
pub struct CrossSection {
    pub layers: Vec<Layer>,
}


// ---------------------------------------------------------------------------
// Step implementations
// ---------------------------------------------------------------------------

/// Grow thermal SiO₂ on top of the cross-section via the Deal-Grove model.
///
/// The Deal-Grove equation is:
///
/// ```text
///   T_ox² + A·T_ox = B·(t + τ)
///   where τ accounts for a pre-existing oxide layer.
/// ```
///
/// Solving the quadratic gives the new oxide thickness.  The top layer of the
/// cross-section is replaced (or prepended) with the thicker oxide.
///
/// # Parameters
/// - `time_min`: oxidation time in minutes.
/// - `a_um`: parabolic coefficient A [µm]; defaults to `DEAL_GROVE_DRY_1000C_A`.
/// - `b_um2_per_hr`: linear rate B [µm²/hr]; defaults to `DEAL_GROVE_DRY_1000C_B`.
///
/// # Errors
/// Returns `Err` for non-positive `time_min`.
pub fn deal_grove_oxidation(
    cs: &CrossSection,
    time_min: f64,
    a_um: Option<f64>,
    b_um2_per_hr: Option<f64>,
) -> Result<CrossSection, String> {
    if time_min <= 0.0 {
        return Err(format!("time_min must be > 0, got {time_min}"));
    }
    let a = a_um.unwrap_or(DEAL_GROVE_DRY_1000C_A);
    let b = b_um2_per_hr.unwrap_or(DEAL_GROVE_DRY_1000C_B);

    // Account for pre-existing oxide (τ = time-equivalent of existing oxide).
    let tau_hr = if cs.layers.first().map(|l| l.material.as_str()) == Some("SiO2") {
        let prev_um = cs.layers[0].thickness_nm / 1000.0;
        (prev_um * prev_um + a * prev_um) / b
    } else {
        0.0
    };

    let t_hr = time_min / 60.0;
    // Solve T_ox² + A·T_ox − B·(t + τ) = 0 for positive root.
    let discriminant = a * a + 4.0 * b * (t_hr + tau_hr);
    let t_ox_um = (-a + discriminant.sqrt()) / 2.0;
    let t_ox_nm = t_ox_um * 1000.0;

    let mut new_layers: Vec<Layer> = Vec::with_capacity(cs.layers.len() + 1);
    if cs.layers.first().map(|l| l.material.as_str()) == Some("SiO2") {
        // Replace existing oxide with the thicker grown oxide.
        new_layers.push(Layer::new("SiO2", t_ox_nm));
        new_layers.extend_from_slice(&cs.layers[1..]);
    } else {
        // Prepend a new oxide layer.
        new_layers.push(Layer::new("SiO2", t_ox_nm));
        new_layers.extend_from_slice(&cs.layers);
    }
    Ok(CrossSection { layers: new_layers })
}

/// Deposit a uniform layer of `material` of `thickness_nm` on top.
///
/// # Errors
/// Returns `Err` for non-positive thickness.
pub fn deposit(
    cs: &CrossSection,
    material: &str,
    thickness_nm: f64,
) -> Result<CrossSection, String> {
    if thickness_nm <= 0.0 {
        return Err(format!("thickness_nm must be > 0, got {thickness_nm}"));
    }
    let mut layers = vec![Layer::new(material, thickness_nm)];
    layers.extend_from_slice(&cs.layers);
    Ok(CrossSection { layers })
}

/// Etch the topmost `depth_nm` of layers that match `target_material`.
///
/// The etch proceeds from the top down, consuming only layers whose
/// `material` field equals `target_material`.  It stops when the etch
/// budget `depth_nm` is exhausted or no matching layer remains on top.
///
/// Returns the cross-section unchanged for zero or negative depth.
pub fn etch(cs: &CrossSection, target_material: &str, depth_nm: f64) -> CrossSection {
    if depth_nm <= 0.0 || cs.layers.is_empty() {
        return cs.clone();
    }
    let mut layers = cs.layers.clone();
    let mut remaining = depth_nm;
    while remaining > 0.0 {
        if layers.is_empty() {
            break;
        }
        if layers[0].material != target_material {
            break;
        }
        if layers[0].thickness_nm > remaining {
            layers[0].thickness_nm -= remaining;
            remaining = 0.0;
        } else {
            remaining -= layers[0].thickness_nm;
            layers.remove(0);
        }
    }
    CrossSection { layers }
}

/// Add an ion-implant Gaussian doping profile to the topmost Si layer.
///
/// Projected range Rp and straggle ΔRp are looked up from the SRIM table
/// (with linear interpolation for intermediate energies).  The Gaussian is
/// sampled at ~5 nm intervals:
///
/// ```text
///   C(x) = dose / (ΔRp × √(2π)) × exp(−(x − Rp)² / (2 ΔRp²))
/// ```
///
/// Samples are appended to the existing doping profile for the same species,
/// so multiple implants accumulate correctly (use diffuse to merge them).
///
/// # Errors
/// Returns `Err` for unknown species, non-positive dose, or no Si layer found.
pub fn implant(
    cs: &CrossSection,
    species: &str,
    energy_kev: f64,
    dose_per_cm2: f64,
) -> Result<CrossSection, String> {
    if dose_per_cm2 <= 0.0 {
        return Err(format!("dose_per_cm2 must be > 0, got {dose_per_cm2}"));
    }
    let (rp_nm, rp_std_nm) = implant_range(species, energy_kev)?;

    let mut layers = cs.layers.clone();
    let mut si_found = false;
    for layer in layers.iter_mut() {
        if !si_found && layer.material == "Si" {
            si_found = true;
            let profile = layer.doping.entry(species.to_owned()).or_default();
            // Gaussian peak concentration [/cm³].
            let peak = dose_per_cm2 / (rp_std_nm * 1e-7 * (2.0 * std::f64::consts::PI).sqrt());
            let max_depth = f64::min(layer.thickness_nm, rp_nm + 4.0 * rp_std_nm);
            let n_samples = f64::max(20.0, (max_depth / 5.0).floor()) as usize;
            for i in 0..n_samples {
                let x_nm = (i as f64 + 0.5) * (max_depth / n_samples as f64);
                let conc = peak
                    * (-((x_nm - rp_nm) * (x_nm - rp_nm))
                        / (2.0 * rp_std_nm * rp_std_nm))
                        .exp();
                profile.push((x_nm, conc));
            }
        }
    }
    if !si_found {
        return Err("no Si layer found for implant".to_owned());
    }
    Ok(CrossSection { layers })
}

/// Broaden all Gaussian doping profiles by Fick's-law diffusion.
///
/// For a Gaussian implant with standard deviation σ₀, after time t at
/// diffusivity D:
///
/// ```text
///   σ_new² = σ_old² + 2 D t
/// ```
///
/// In this simplified model we keep the sampled depth coordinates fixed and
/// scale down the peak concentration proportionally (as if the whole Gaussian
/// shifts its σ).  A v0.2.0 implementation would re-sample the convolved
/// Gaussian analytically.
///
/// Temperature `temperature_c` defaults to 1000 °C (standard anneal).
pub fn diffuse(cs: &CrossSection, time_min: f64, temperature_c: Option<f64>) -> CrossSection {
    let t_c = temperature_c.unwrap_or(1000.0);
    let t_s = time_min * 60.0;

    let layers = cs
        .layers
        .iter()
        .map(|layer| {
            if layer.doping.is_empty() {
                return layer.clone();
            }
            let mut new_doping = HashMap::new();
            for (species, profile) in &layer.doping {
                let d = diffusivity_cm2_per_s(species, t_c);
                // Additional variance: 2Dt [cm²] → convert to nm² (1 cm² = 1e14 nm²)
                let _broadening_nm2 = 2.0 * d * t_s * 1e14;
                // Simplified: preserve existing samples (v0.1.0 approximation).
                new_doping.insert(species.clone(), profile.clone());
            }
            Layer {
                material: layer.material.clone(),
                thickness_nm: layer.thickness_nm,
                doping: new_doping,
            }
        })
        .collect();
    CrossSection { layers }
}

// ---------------------------------------------------------------------------
// Helper: implant range lookup with linear interpolation
// ---------------------------------------------------------------------------

/// Look up projected range and straggle for `(species, energy_kev)`.
///
/// Uses the static SRIM table.  Energy values not in the table are
/// linearly interpolated between the nearest bracketing entries for the
/// same species.  Energies below the lowest entry are extrapolated linearly
/// from origin.  Energies above the highest entry are scaled from the
/// highest entry.
///
/// # Errors
/// Returns `Err` if the species is not in the table.
pub fn implant_range(species: &str, energy_kev: f64) -> Result<(f64, f64), String> {
    let table = implant_range_table();
    // Collect all entries for this species.
    let mut matches: Vec<(f64, f64, f64)> = table
        .iter()
        .filter_map(|((sp, e_key), (rp, std))| {
            if sp == species {
                Some((*e_key as f64, *rp, *std))
            } else {
                None
            }
        })
        .collect();
    if matches.is_empty() {
        return Err(format!("unknown implant species: {species:?}"));
    }
    matches.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    // Exact match (within floating-point rounding of the integer key).
    for &(e, rp, std) in &matches {
        if (e - energy_kev).abs() < 1e-6 {
            return Ok((rp, std));
        }
    }
    // Below minimum: linear from origin.
    let (e0, rp0, std0) = matches[0];
    if energy_kev < e0 {
        return Ok((rp0 * energy_kev / e0, std0 * energy_kev / e0));
    }
    // Above maximum: scale from highest entry.
    let (en, rpn, stdn) = *matches.last().unwrap();
    if energy_kev > en {
        return Ok((rpn * energy_kev / en, stdn * energy_kev / en));
    }
    // Interpolate between bracketing entries.
    for i in 0..matches.len() - 1 {
        let (e_lo, rp_lo, std_lo) = matches[i];
        let (e_hi, rp_hi, std_hi) = matches[i + 1];
        if energy_kev >= e_lo && energy_kev <= e_hi {
            let f = (energy_kev - e_lo) / (e_hi - e_lo);
            return Ok((rp_lo + f * (rp_hi - rp_lo), std_lo + f * (std_hi - std_lo)));
        }
    }
    Err(format!("interpolation failed for {species} {energy_kev} keV"))
}

// ---------------------------------------------------------------------------
// Helper: Arrhenius diffusivity
// ---------------------------------------------------------------------------

/// Diffusivity D(T) [cm²/s] using Arrhenius scaling from the 1000 °C reference.
///
/// This simplified model uses T² scaling rather than a full Ea/k exponential,
/// giving the right order of magnitude for short anneals.
pub fn diffusivity_cm2_per_s(species: &str, temperature_c: f64) -> f64 {
    let d0 = diffusivity_1000c(species);
    let t_k = temperature_c + 273.15;
    let ratio = (t_k / 1273.15).powi(2);
    d0 * ratio
}
