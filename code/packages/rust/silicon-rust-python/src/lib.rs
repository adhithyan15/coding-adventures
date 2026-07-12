//! Python C extension for the silicon simulation stack.
//!
//! Exposes `device-physics`, `mosfet-models`, and `fab-process-simulation` to
//! Python via the stable C API (PEP 384).  Uses the zero-dependency
//! `python-bridge` crate — no PyO3, no bindgen, no Python headers at build
//! time.  Compiles with a plain `cargo build` on any platform.
//!
//! ## Wire format for CrossSection (v0.1)
//!
//! A `CrossSection` is serialised as a pipe-separated list of
//! `material:thickness_nm` pairs, ordered top-to-bottom.
//! Doping profiles are elided in v0.1 (they survive process steps but are not
//! transported over the wire).
//!
//! ```text
//! ""                               # empty cross-section
//! "Si:500.0"                       # bare silicon substrate, 500 nm thick
//! "SiO2:4.8|Si:500.0"             # gate oxide on silicon
//! "Poly:50.0|SiO2:4.8|Si:500.0"  # poly gate on gate oxide on silicon
//! ```
//!
//! ## Python usage
//!
//! ```python
//! import silicon_rust_python as srp, json
//!
//! # Constants
//! print(srp.k_boltzmann())        # 1.380649e-23 J/K
//! print(srp.thermal_voltage(300.0))  # ~0.02585 V
//!
//! # Intrinsic concentration and Fermi potential
//! ni  = srp.intrinsic_concentration(300.0)          # 1e16 /m³
//! phi = srp.fermi_potential(1e23, "p", 300.0)        # ~0.45 V
//!
//! # PN junction
//! vbi = srp.pn_junction_built_in_voltage(1e23, 1e22, 300.0)
//! w   = srp.pn_junction_depletion_width(1e23, 1e22, 300.0, 0.0)
//! is_ = srp.pn_junction_saturation_current(1e23, 1e22, 1e-8, 300.0, 1e-6, 1e-6)
//! i   = srp.pn_junction_current(1e23, 1e22, 1e-8, 300.0, 1e-6, 1e-6, 0.6)
//!
//! # MOSFET threshold voltage
//! vt = srp.mosfet_threshold_voltage("NMOS", 130e-9, 1e-6, 2e-9, 1e24, -0.05, 0.0, 300.0, 0.0)
//!
//! # Level-1 MOSFET DC operating point
//! r = srp.evaluate_level1_defaults(1.8, 1.8, 0.0, 300.15)
//! print(r)  # {"id": ..., "region": "saturation", ...}
//!
//! r = srp.evaluate_level1(0.42, 220e-6, 0.05, 0.27, 0.84,
//!                          1e-6, 130e-9, 1.4,
//!                          1.8, 1.8, 0.0, 300.15)
//!
//! # Process simulation
//! cs = srp.deposit("", "Si", 500.0)           # start with Si substrate
//! cs = srp.deal_grove_oxidation(cs, 5.0)       # grow gate oxide
//! cs = srp.deposit(cs, "Poly", 50.0)           # deposit poly gate
//! print(cs)  # "Poly:50.0|SiO2:...|Si:500.0"
//!
//! cs = srp.implant(cs, "B", 30.0, 1e13)        # boron source/drain
//! cs = srp.diffuse(cs, 30.0, 1000.0)           # 30-min anneal at 1000 °C
//! rp, straggle = srp.implant_range("B", 30.0)  # 92 nm, 38 nm
//! d = srp.diffusivity_cm2_per_s("B", 1000.0)   # ~1e-14 cm²/s
//! ```

#![allow(non_snake_case)]

use std::ffi::c_int;
use std::ptr;

use device_physics as dp;
use fab_process_simulation as fps;
use mosfet_models as mm;
use python_bridge::{
    f64_to_py, parse_arg_f64, parse_arg_str, set_error, str_to_py, type_error_class,
    value_error_class, Py_DecRef, PyDict_New, PyDict_SetItem, PyMethodDef, PyModuleDef,
    PyModuleDef_Base, PyModule_Create2, PyObjectPtr, PyTuple_New, PyTuple_SetItem,
    METH_NOARGS, METH_VARARGS, PYTHON_API_VERSION,
};

// ─────────────────────────────────────────────────────────────────────────────
// CrossSection wire format helpers (pure Rust, cargo-testable)
// ─────────────────────────────────────────────────────────────────────────────

/// Return `Err` if `material` contains a wire-format delimiter (`|` or `:`).
///
/// Material names must be plain identifiers.  Allowing delimiters would let
/// a caller inject extra layers into the wire string and corrupt the
/// cross-section seen by subsequent process functions.
pub fn validate_material_name(material: &str) -> Result<(), String> {
    if material.contains('|') || material.contains(':') {
        return Err(format!(
            "material name {:?} contains a wire-format delimiter ('|' or ':'); \
             material names must not contain these characters",
            material
        ));
    }
    Ok(())
}

/// Serialise a `CrossSection` to the pipe-delimited wire format.
///
/// Doping profiles are not included in v0.1.
pub fn cs_to_wire(cs: &fps::CrossSection) -> String {
    cs.layers
        .iter()
        .map(|l| format!("{}:{}", l.material, l.thickness_nm))
        .collect::<Vec<_>>()
        .join("|")
}

/// Deserialise a `CrossSection` from the pipe-delimited wire format.
///
/// Silently skips malformed entries so that an empty string returns an empty
/// cross-section rather than an error.
pub fn cs_from_wire(s: &str) -> fps::CrossSection {
    if s.is_empty() {
        return fps::CrossSection::default();
    }
    let layers = s
        .split('|')
        .filter_map(|part| {
            let mut kv = part.splitn(2, ':');
            let material = kv.next()?.trim().to_owned();
            let thickness: f64 = kv.next()?.trim().parse().ok()?;
            Some(fps::Layer::new(material, thickness))
        })
        .collect();
    fps::CrossSection { layers }
}

// ─────────────────────────────────────────────────────────────────────────────
// Pure Rust helper for Level-1 evaluation (testable without Python)
// ─────────────────────────────────────────────────────────────────────────────

/// Evaluate the Level-1 MOSFET with explicit parameters.
///
/// Returns `(id, gm, gds, gmb, cgs, cgd, cgb, cbs, cbd, region_str)`.
/// The `region_str` is one of `"cutoff"`, `"subthreshold"`, `"triode"`,
/// `"saturation"`.
#[allow(clippy::too_many_arguments)]
pub fn eval_level1_rs(
    vt0: f64,
    kp: f64,
    lambda: f64,
    gamma: f64,
    phi: f64,
    w: f64,
    l: f64,
    n_sub: f64,
    v_gs: f64,
    v_ds: f64,
    v_bs: f64,
    t: f64,
) -> (f64, f64, f64, f64, f64, f64, f64, f64, f64, &'static str) {
    let p = mm::Level1Params {
        vt0,
        kp,
        lambda,
        gamma,
        phi,
        w,
        l,
        n_sub,
        ..Default::default()
    };
    let r = mm::evaluate_level1(&p, v_gs, v_ds, v_bs, t);
    (r.id, r.gm, r.gds, r.gmb, r.cgs, r.cgd, r.cgb, r.cbs, r.cbd, r.region.as_str())
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helper: build a Python dict from a MosResult
// ─────────────────────────────────────────────────────────────────────────────

unsafe fn mos_result_to_py_dict(r: &mm::MosResult) -> PyObjectPtr {
    // PyDict_SetItem does NOT steal references — decref key and value after.
    let dict = PyDict_New();
    // PyDict_New() returns NULL on memory exhaustion (MemoryError already set).
    if dict.is_null() {
        return ptr::null_mut();
    }

    macro_rules! set_f {
        ($k:expr, $v:expr) => {{
            let k = str_to_py($k);
            let v = f64_to_py($v);
            PyDict_SetItem(dict, k, v);
            Py_DecRef(k);
            Py_DecRef(v);
        }};
    }
    macro_rules! set_s {
        ($k:expr, $v:expr) => {{
            let k = str_to_py($k);
            let v = str_to_py($v);
            PyDict_SetItem(dict, k, v);
            Py_DecRef(k);
            Py_DecRef(v);
        }};
    }

    set_f!("id",  r.id);
    set_f!("gm",  r.gm);
    set_f!("gds", r.gds);
    set_f!("gmb", r.gmb);
    set_f!("cgs", r.cgs);
    set_f!("cgd", r.cgd);
    set_f!("cgb", r.cgb);
    set_f!("cbs", r.cbs);
    set_f!("cbd", r.cbd);
    set_s!("region", r.region.as_str());

    dict
}

// ─────────────────────────────────────────────────────────────────────────────
// Macro helpers to reduce boilerplate in argument parsing
// ─────────────────────────────────────────────────────────────────────────────

/// Parse a required f64 positional arg; on missing/wrong-type, set TypeError
/// and return null.  Use as: `let x = req_f64!(args, 0, "fn_name")?;`
/// (the `?` works because the macro expands to a block returning
/// `PyObjectPtr` — callers must use it inside an `unsafe extern "C" fn`
/// that also returns `PyObjectPtr`.)
macro_rules! req_f64 {
    ($args:expr, $idx:expr, $fn_name:expr) => {
        match parse_arg_f64($args, $idx) {
            Some(v) => v,
            None => {
                set_error(
                    type_error_class(),
                    concat!($fn_name, ": missing or non-numeric argument at position ", stringify!($idx)),
                );
                return ptr::null_mut();
            }
        }
    };
}

macro_rules! req_str {
    ($args:expr, $idx:expr, $fn_name:expr) => {
        match parse_arg_str($args, $idx) {
            Some(v) => v,
            None => {
                set_error(
                    type_error_class(),
                    concat!($fn_name, ": missing or non-string argument at position ", stringify!($idx)),
                );
                return ptr::null_mut();
            }
        }
    };
}

// ─────────────────────────────────────────────────────────────────────────────
// Python wrappers — physical constants (METH_NOARGS)
// ─────────────────────────────────────────────────────────────────────────────

unsafe extern "C" fn py_k_boltzmann(_s: PyObjectPtr, _a: PyObjectPtr) -> PyObjectPtr {
    f64_to_py(dp::K_BOLTZMANN)
}
unsafe extern "C" fn py_q_electron(_s: PyObjectPtr, _a: PyObjectPtr) -> PyObjectPtr {
    f64_to_py(dp::Q_ELECTRON)
}
unsafe extern "C" fn py_eps0(_s: PyObjectPtr, _a: PyObjectPtr) -> PyObjectPtr {
    f64_to_py(dp::EPS0)
}
unsafe extern "C" fn py_eps_si(_s: PyObjectPtr, _a: PyObjectPtr) -> PyObjectPtr {
    f64_to_py(dp::EPS_SI)
}
unsafe extern "C" fn py_eps_ox(_s: PyObjectPtr, _a: PyObjectPtr) -> PyObjectPtr {
    f64_to_py(dp::EPS_OX)
}
unsafe extern "C" fn py_n_i_300k(_s: PyObjectPtr, _a: PyObjectPtr) -> PyObjectPtr {
    f64_to_py(dp::N_I_300K)
}
unsafe extern "C" fn py_eg_si_300k(_s: PyObjectPtr, _a: PyObjectPtr) -> PyObjectPtr {
    f64_to_py(dp::EG_SI_300K)
}
unsafe extern "C" fn py_mu_n_300k(_s: PyObjectPtr, _a: PyObjectPtr) -> PyObjectPtr {
    f64_to_py(dp::MU_N_300K)
}
unsafe extern "C" fn py_mu_p_300k(_s: PyObjectPtr, _a: PyObjectPtr) -> PyObjectPtr {
    f64_to_py(dp::MU_P_300K)
}

// ─────────────────────────────────────────────────────────────────────────────
// Python wrappers — device-physics functions
// ─────────────────────────────────────────────────────────────────────────────

/// `thermal_voltage(t_kelvin: float) -> float`
///
/// V_T = kT/q [V]. At 300 K this is ≈ 0.02585 V.
unsafe extern "C" fn py_thermal_voltage(_s: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let t = req_f64!(args, 0, "thermal_voltage");
    f64_to_py(dp::thermal_voltage(t))
}

/// `intrinsic_concentration(t_kelvin: float) -> float`
///
/// Intrinsic carrier concentration n_i(T) [/m³]. Raises `ValueError` below 100 K.
unsafe extern "C" fn py_intrinsic_concentration(_s: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let t = req_f64!(args, 0, "intrinsic_concentration");
    match dp::intrinsic_concentration(t) {
        Ok(ni) => f64_to_py(ni),
        Err(msg) => {
            set_error(value_error_class(), &format!("intrinsic_concentration: {}", msg));
            ptr::null_mut()
        }
    }
}

/// `fermi_potential(n_doping: float, kind: str, t_kelvin: float) -> float`
///
/// Fermi potential φ_F [V]. `kind` must be `"p"` or `"n"`.
/// Returns `+|φ_F|` for p-type and `−|φ_F|` for n-type.
unsafe extern "C" fn py_fermi_potential(_s: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let n    = req_f64!(args, 0, "fermi_potential");
    let kind = req_str!(args, 1, "fermi_potential");
    let t    = req_f64!(args, 2, "fermi_potential");
    match dp::fermi_potential(n, &kind, t) {
        Ok(phi) => f64_to_py(phi),
        Err(msg) => {
            set_error(value_error_class(), &format!("fermi_potential: {}", msg));
            ptr::null_mut()
        }
    }
}

/// `pn_junction_built_in_voltage(na: float, nd: float, t: float) -> float`
///
/// Built-in voltage V_bi [V] for an abrupt p-n junction.
unsafe extern "C" fn py_pn_junction_built_in_voltage(_s: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let na = req_f64!(args, 0, "pn_junction_built_in_voltage");
    let nd = req_f64!(args, 1, "pn_junction_built_in_voltage");
    let t  = req_f64!(args, 2, "pn_junction_built_in_voltage");
    match dp::PNJunction::new(na, nd, 1.0, t, 1e-6, 1e-6) {
        Ok(j)    => f64_to_py(j.built_in_voltage()),
        Err(msg) => {
            set_error(value_error_class(), &format!("pn_junction_built_in_voltage: {}", msg));
            ptr::null_mut()
        }
    }
}

/// `pn_junction_depletion_width(na, nd, t, v_applied) -> float`
///
/// Total depletion-region width W [m] under applied bias `v_applied` [V].
/// Positive `v_applied` is forward bias; negative is reverse bias.
unsafe extern "C" fn py_pn_junction_depletion_width(_s: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let na         = req_f64!(args, 0, "pn_junction_depletion_width");
    let nd         = req_f64!(args, 1, "pn_junction_depletion_width");
    let t          = req_f64!(args, 2, "pn_junction_depletion_width");
    let v_applied  = req_f64!(args, 3, "pn_junction_depletion_width");
    match dp::PNJunction::new(na, nd, 1.0, t, 1e-6, 1e-6) {
        Ok(j)    => f64_to_py(j.depletion_width(v_applied)),
        Err(msg) => {
            set_error(value_error_class(), &format!("pn_junction_depletion_width: {}", msg));
            ptr::null_mut()
        }
    }
}

/// `pn_junction_saturation_current(na, nd, a, t, tau_n, tau_p) -> float`
///
/// Shockley saturation current I_S [A].
/// `a` is junction area [m²], `tau_n`/`tau_p` are minority-carrier lifetimes [s].
unsafe extern "C" fn py_pn_junction_saturation_current(_s: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let na    = req_f64!(args, 0, "pn_junction_saturation_current");
    let nd    = req_f64!(args, 1, "pn_junction_saturation_current");
    let a     = req_f64!(args, 2, "pn_junction_saturation_current");
    let t     = req_f64!(args, 3, "pn_junction_saturation_current");
    let tau_n = req_f64!(args, 4, "pn_junction_saturation_current");
    let tau_p = req_f64!(args, 5, "pn_junction_saturation_current");
    match dp::PNJunction::new(na, nd, a, t, tau_n, tau_p) {
        Ok(j)    => f64_to_py(j.saturation_current()),
        Err(msg) => {
            set_error(value_error_class(), &format!("pn_junction_saturation_current: {}", msg));
            ptr::null_mut()
        }
    }
}

/// `pn_junction_current(na, nd, a, t, tau_n, tau_p, v) -> float`
///
/// Shockley diode current I [A] at applied voltage `v` [V].
unsafe extern "C" fn py_pn_junction_current(_s: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let na    = req_f64!(args, 0, "pn_junction_current");
    let nd    = req_f64!(args, 1, "pn_junction_current");
    let a     = req_f64!(args, 2, "pn_junction_current");
    let t     = req_f64!(args, 3, "pn_junction_current");
    let tau_n = req_f64!(args, 4, "pn_junction_current");
    let tau_p = req_f64!(args, 5, "pn_junction_current");
    let v     = req_f64!(args, 6, "pn_junction_current");
    match dp::PNJunction::new(na, nd, a, t, tau_n, tau_p) {
        Ok(j)    => f64_to_py(j.current(v)),
        Err(msg) => {
            set_error(value_error_class(), &format!("pn_junction_current: {}", msg));
            ptr::null_mut()
        }
    }
}

/// `mosfet_threshold_voltage(device_type, l, w, t_ox, n_body, phi_ms, q_ox, t, v_sb) -> float`
///
/// Threshold voltage V_t [V] with body effect. `device_type` is `"NMOS"` or `"PMOS"`.
/// `v_sb` ≥ 0 is the source-to-body reverse bias [V].
unsafe extern "C" fn py_mosfet_threshold_voltage(_s: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let device_type = req_str!(args, 0, "mosfet_threshold_voltage");
    let l      = req_f64!(args, 1, "mosfet_threshold_voltage");
    let w      = req_f64!(args, 2, "mosfet_threshold_voltage");
    let t_ox   = req_f64!(args, 3, "mosfet_threshold_voltage");
    let n_body = req_f64!(args, 4, "mosfet_threshold_voltage");
    let phi_ms = req_f64!(args, 5, "mosfet_threshold_voltage");
    let q_ox   = req_f64!(args, 6, "mosfet_threshold_voltage");
    let t      = req_f64!(args, 7, "mosfet_threshold_voltage");
    let v_sb   = req_f64!(args, 8, "mosfet_threshold_voltage");
    match dp::MOSFETParams::new(&device_type, l, w, t_ox, n_body, phi_ms, q_ox, t) {
        Ok(p) => match p.threshold_voltage(v_sb) {
            Ok(vt)   => f64_to_py(vt),
            Err(msg) => {
                set_error(value_error_class(), &format!("mosfet_threshold_voltage: {}", msg));
                ptr::null_mut()
            }
        },
        Err(msg) => {
            set_error(value_error_class(), &format!("mosfet_threshold_voltage: {}", msg));
            ptr::null_mut()
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Python wrappers — mosfet-models
// ─────────────────────────────────────────────────────────────────────────────

/// `evaluate_level1(vt0, kp, lambda_, gamma, phi, w, l, n_sub, v_gs, v_ds, v_bs, t) -> dict`
///
/// Evaluate the SPICE Level-1 MOSFET at the given operating point.
/// Returns a dict with keys: `id`, `gm`, `gds`, `gmb`, `cgs`, `cgd`, `cgb`,
/// `cbs`, `cbd`, `region` (one of `"cutoff"`, `"subthreshold"`, `"triode"`,
/// `"saturation"`).
///
/// Parameters `cgso`, `cgdo`, `cgbo`, `cbs_0`, `cbd_0`, and
/// `subthreshold_enable` are held at their defaults; use `Level1Params` from
/// the Rust API for full control.
unsafe extern "C" fn py_evaluate_level1(_s: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let vt0    = req_f64!(args,  0, "evaluate_level1");
    let kp     = req_f64!(args,  1, "evaluate_level1");
    let lambda = req_f64!(args,  2, "evaluate_level1");
    let gamma  = req_f64!(args,  3, "evaluate_level1");
    let phi    = req_f64!(args,  4, "evaluate_level1");
    let w      = req_f64!(args,  5, "evaluate_level1");
    let l      = req_f64!(args,  6, "evaluate_level1");
    let n_sub  = req_f64!(args,  7, "evaluate_level1");
    let v_gs   = req_f64!(args,  8, "evaluate_level1");
    let v_ds   = req_f64!(args,  9, "evaluate_level1");
    let v_bs   = req_f64!(args, 10, "evaluate_level1");
    let t      = req_f64!(args, 11, "evaluate_level1");

    let p = mm::Level1Params {
        vt0,
        kp,
        lambda,
        gamma,
        phi,
        w,
        l,
        n_sub,
        ..Default::default()
    };
    let r = mm::evaluate_level1(&p, v_gs, v_ds, v_bs, t);
    mos_result_to_py_dict(&r)
}

/// `evaluate_level1_defaults(v_gs, v_ds, v_bs, t) -> dict`
///
/// Evaluate the Level-1 MOSFET using the default 130 nm NMOS parameter set.
/// Returns the same dict as `evaluate_level1`.
unsafe extern "C" fn py_evaluate_level1_defaults(_s: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let v_gs = req_f64!(args, 0, "evaluate_level1_defaults");
    let v_ds = req_f64!(args, 1, "evaluate_level1_defaults");
    let v_bs = req_f64!(args, 2, "evaluate_level1_defaults");
    let t    = req_f64!(args, 3, "evaluate_level1_defaults");
    let p    = mm::Level1Params::default();
    let r    = mm::evaluate_level1(&p, v_gs, v_ds, v_bs, t);
    mos_result_to_py_dict(&r)
}

// ─────────────────────────────────────────────────────────────────────────────
// Python wrappers — fab-process-simulation
// ─────────────────────────────────────────────────────────────────────────────

/// `deal_grove_oxidation(cs_str, time_min[, a_um, b_um2_per_hr]) -> str`
///
/// Grow thermal SiO₂ via the Deal-Grove model.  `a_um` and `b_um2_per_hr` are
/// optional; omit them (or pass values ≤ 0) to use the dry-O₂ 1000 °C defaults.
/// Returns the updated cross-section wire string.
unsafe extern "C" fn py_deal_grove_oxidation(_s: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let cs_str   = req_str!(args, 0, "deal_grove_oxidation");
    let time_min = req_f64!(args, 1, "deal_grove_oxidation");
    // Optional positional args: absent → None → use defaults.
    let a_um    = parse_arg_f64(args, 2).filter(|&v| v > 0.0);
    let b_um2   = parse_arg_f64(args, 3).filter(|&v| v > 0.0);
    let cs = cs_from_wire(&cs_str);
    match fps::deal_grove_oxidation(&cs, time_min, a_um, b_um2) {
        Ok(new_cs) => str_to_py(&cs_to_wire(&new_cs)),
        Err(msg)   => {
            set_error(value_error_class(), &format!("deal_grove_oxidation: {}", msg));
            ptr::null_mut()
        }
    }
}

/// `deposit(cs_str, material, thickness_nm) -> str`
///
/// Deposit a uniform film of `material` of `thickness_nm` nm on top of the
/// cross-section.  Returns the updated wire string.
unsafe extern "C" fn py_deposit(_s: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let cs_str       = req_str!(args, 0, "deposit");
    let material     = req_str!(args, 1, "deposit");
    let thickness_nm = req_f64!(args, 2, "deposit");
    if let Err(msg) = validate_material_name(&material) {
        set_error(value_error_class(), &format!("deposit: {}", msg));
        return ptr::null_mut();
    }
    let cs = cs_from_wire(&cs_str);
    match fps::deposit(&cs, &material, thickness_nm) {
        Ok(new_cs) => str_to_py(&cs_to_wire(&new_cs)),
        Err(msg)   => {
            set_error(value_error_class(), &format!("deposit: {}", msg));
            ptr::null_mut()
        }
    }
}

/// `etch(cs_str, target_material, depth_nm) -> str`
///
/// Remove `depth_nm` nm of `target_material` from the top of the cross-section.
/// The etch stops when the budget is exhausted or a different material is reached.
unsafe extern "C" fn py_etch(_s: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let cs_str          = req_str!(args, 0, "etch");
    let target_material = req_str!(args, 1, "etch");
    let depth_nm        = req_f64!(args, 2, "etch");
    let cs     = cs_from_wire(&cs_str);
    let new_cs = fps::etch(&cs, &target_material, depth_nm);
    str_to_py(&cs_to_wire(&new_cs))
}

/// `implant(cs_str, species, energy_kev, dose_per_cm2) -> str`
///
/// Add a Gaussian ion-implant profile to the topmost Si layer.
/// `species` is one of `"B"`, `"P"`, `"As"`, `"BF2"`.
/// Returns the updated cross-section wire string (doping elided in v0.1).
unsafe extern "C" fn py_implant(_s: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let cs_str       = req_str!(args, 0, "implant");
    let species      = req_str!(args, 1, "implant");
    let energy_kev   = req_f64!(args, 2, "implant");
    let dose_per_cm2 = req_f64!(args, 3, "implant");
    // Species names flow into the doping map but never into the wire format,
    // so no delimiter validation is needed for species (unlike material names).
    let cs = cs_from_wire(&cs_str);
    match fps::implant(&cs, &species, energy_kev, dose_per_cm2) {
        Ok(new_cs) => str_to_py(&cs_to_wire(&new_cs)),
        Err(msg)   => {
            set_error(value_error_class(), &format!("implant: {}", msg));
            ptr::null_mut()
        }
    }
}

/// `diffuse(cs_str, time_min[, temperature_c]) -> str`
///
/// Broaden all Gaussian doping profiles via Fick's law.
/// `temperature_c` defaults to 1000 °C (standard anneal temperature).
unsafe extern "C" fn py_diffuse(_s: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let cs_str   = req_str!(args, 0, "diffuse");
    let time_min = req_f64!(args, 1, "diffuse");
    let temp_c   = parse_arg_f64(args, 2); // None if not provided → default 1000 °C
    let cs     = cs_from_wire(&cs_str);
    let new_cs = fps::diffuse(&cs, time_min, temp_c);
    str_to_py(&cs_to_wire(&new_cs))
}

/// `implant_range(species, energy_kev) -> (float, float)`
///
/// Return `(Rp_nm, delta_Rp_nm)` from the SRIM table with linear interpolation.
/// Raises `ValueError` for unknown species.
unsafe extern "C" fn py_implant_range(_s: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let species    = req_str!(args, 0, "implant_range");
    let energy_kev = req_f64!(args, 1, "implant_range");
    match fps::implant_range(&species, energy_kev) {
        Ok((rp, straggle)) => {
            let tup = PyTuple_New(2);
            // PyTuple_New() returns NULL on memory exhaustion (MemoryError set).
            if tup.is_null() {
                return ptr::null_mut();
            }
            PyTuple_SetItem(tup, 0, f64_to_py(rp));
            PyTuple_SetItem(tup, 1, f64_to_py(straggle));
            tup
        }
        Err(msg) => {
            set_error(value_error_class(), &format!("implant_range: {}", msg));
            ptr::null_mut()
        }
    }
}

/// `diffusivity_cm2_per_s(species, temperature_c) -> float`
///
/// Arrhenius-scaled diffusivity D(T) [cm²/s] from the 1000 °C reference.
unsafe extern "C" fn py_diffusivity_cm2_per_s(_s: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let species     = req_str!(args, 0, "diffusivity_cm2_per_s");
    let temperature = req_f64!(args, 1, "diffusivity_cm2_per_s");
    f64_to_py(fps::diffusivity_cm2_per_s(&species, temperature))
}

// ─────────────────────────────────────────────────────────────────────────────
// Module methods table
// ─────────────────────────────────────────────────────────────────────────────

static mut MODULE_METHODS: [PyMethodDef; 27] = [
    // ------------ physical constants (no-arg) --------------------------------
    PyMethodDef {
        ml_name:  c"k_boltzmann".as_ptr(),
        ml_meth:  Some(py_k_boltzmann),
        ml_flags: METH_NOARGS,
        ml_doc:   c"k_boltzmann() -> float\nBoltzmann constant [J/K].".as_ptr(),
    },
    PyMethodDef {
        ml_name:  c"q_electron".as_ptr(),
        ml_meth:  Some(py_q_electron),
        ml_flags: METH_NOARGS,
        ml_doc:   c"q_electron() -> float\nElementary charge [C].".as_ptr(),
    },
    PyMethodDef {
        ml_name:  c"eps0".as_ptr(),
        ml_meth:  Some(py_eps0),
        ml_flags: METH_NOARGS,
        ml_doc:   c"eps0() -> float\nVacuum permittivity [F/m].".as_ptr(),
    },
    PyMethodDef {
        ml_name:  c"eps_si".as_ptr(),
        ml_meth:  Some(py_eps_si),
        ml_flags: METH_NOARGS,
        ml_doc:   c"eps_si() -> float\nSilicon permittivity [F/m].".as_ptr(),
    },
    PyMethodDef {
        ml_name:  c"eps_ox".as_ptr(),
        ml_meth:  Some(py_eps_ox),
        ml_flags: METH_NOARGS,
        ml_doc:   c"eps_ox() -> float\nSiO2 permittivity [F/m].".as_ptr(),
    },
    PyMethodDef {
        ml_name:  c"n_i_300k".as_ptr(),
        ml_meth:  Some(py_n_i_300k),
        ml_flags: METH_NOARGS,
        ml_doc:   c"n_i_300k() -> float\nIntrinsic concentration at 300 K [/m3].".as_ptr(),
    },
    PyMethodDef {
        ml_name:  c"eg_si_300k".as_ptr(),
        ml_meth:  Some(py_eg_si_300k),
        ml_flags: METH_NOARGS,
        ml_doc:   c"eg_si_300k() -> float\nSilicon bandgap at 300 K [eV].".as_ptr(),
    },
    PyMethodDef {
        ml_name:  c"mu_n_300k".as_ptr(),
        ml_meth:  Some(py_mu_n_300k),
        ml_flags: METH_NOARGS,
        ml_doc:   c"mu_n_300k() -> float\nElectron mobility at 300 K [m2/V/s].".as_ptr(),
    },
    PyMethodDef {
        ml_name:  c"mu_p_300k".as_ptr(),
        ml_meth:  Some(py_mu_p_300k),
        ml_flags: METH_NOARGS,
        ml_doc:   c"mu_p_300k() -> float\nHole mobility at 300 K [m2/V/s].".as_ptr(),
    },
    // ------------ device-physics functions -----------------------------------
    PyMethodDef {
        ml_name:  c"thermal_voltage".as_ptr(),
        ml_meth:  Some(py_thermal_voltage),
        ml_flags: METH_VARARGS,
        ml_doc:   c"thermal_voltage(t_kelvin: float) -> float\nV_T = kT/q [V].".as_ptr(),
    },
    PyMethodDef {
        ml_name:  c"intrinsic_concentration".as_ptr(),
        ml_meth:  Some(py_intrinsic_concentration),
        ml_flags: METH_VARARGS,
        ml_doc:   c"intrinsic_concentration(t_kelvin: float) -> float\nn_i(T) [/m3].".as_ptr(),
    },
    PyMethodDef {
        ml_name:  c"fermi_potential".as_ptr(),
        ml_meth:  Some(py_fermi_potential),
        ml_flags: METH_VARARGS,
        ml_doc:   c"fermi_potential(n_doping: float, kind: str, t_kelvin: float) -> float\nFermi potential phi_F [V].".as_ptr(),
    },
    PyMethodDef {
        ml_name:  c"pn_junction_built_in_voltage".as_ptr(),
        ml_meth:  Some(py_pn_junction_built_in_voltage),
        ml_flags: METH_VARARGS,
        ml_doc:   c"pn_junction_built_in_voltage(na, nd, t) -> float\nBuilt-in voltage V_bi [V].".as_ptr(),
    },
    PyMethodDef {
        ml_name:  c"pn_junction_depletion_width".as_ptr(),
        ml_meth:  Some(py_pn_junction_depletion_width),
        ml_flags: METH_VARARGS,
        ml_doc:   c"pn_junction_depletion_width(na, nd, t, v_applied) -> float\nDepletion width W [m].".as_ptr(),
    },
    PyMethodDef {
        ml_name:  c"pn_junction_saturation_current".as_ptr(),
        ml_meth:  Some(py_pn_junction_saturation_current),
        ml_flags: METH_VARARGS,
        ml_doc:   c"pn_junction_saturation_current(na, nd, a, t, tau_n, tau_p) -> float\nSaturation current I_S [A].".as_ptr(),
    },
    PyMethodDef {
        ml_name:  c"pn_junction_current".as_ptr(),
        ml_meth:  Some(py_pn_junction_current),
        ml_flags: METH_VARARGS,
        ml_doc:   c"pn_junction_current(na, nd, a, t, tau_n, tau_p, v) -> float\nShockley diode current I [A].".as_ptr(),
    },
    PyMethodDef {
        ml_name:  c"mosfet_threshold_voltage".as_ptr(),
        ml_meth:  Some(py_mosfet_threshold_voltage),
        ml_flags: METH_VARARGS,
        ml_doc:   c"mosfet_threshold_voltage(device_type, l, w, t_ox, n_body, phi_ms, q_ox, t, v_sb) -> float\nThreshold voltage V_t [V].".as_ptr(),
    },
    // ------------ mosfet-models functions ------------------------------------
    PyMethodDef {
        ml_name:  c"evaluate_level1".as_ptr(),
        ml_meth:  Some(py_evaluate_level1),
        ml_flags: METH_VARARGS,
        ml_doc:   c"evaluate_level1(vt0, kp, lambda_, gamma, phi, w, l, n_sub, v_gs, v_ds, v_bs, t) -> dict\nLevel-1 MOSFET DC operating point.".as_ptr(),
    },
    PyMethodDef {
        ml_name:  c"evaluate_level1_defaults".as_ptr(),
        ml_meth:  Some(py_evaluate_level1_defaults),
        ml_flags: METH_VARARGS,
        ml_doc:   c"evaluate_level1_defaults(v_gs, v_ds, v_bs, t) -> dict\nLevel-1 MOSFET with default 130 nm NMOS params.".as_ptr(),
    },
    // ------------ fab-process-simulation functions ---------------------------
    PyMethodDef {
        ml_name:  c"deal_grove_oxidation".as_ptr(),
        ml_meth:  Some(py_deal_grove_oxidation),
        ml_flags: METH_VARARGS,
        ml_doc:   c"deal_grove_oxidation(cs_str, time_min[, a_um, b_um2_per_hr]) -> str\nGrow thermal SiO2 via Deal-Grove.".as_ptr(),
    },
    PyMethodDef {
        ml_name:  c"deposit".as_ptr(),
        ml_meth:  Some(py_deposit),
        ml_flags: METH_VARARGS,
        ml_doc:   c"deposit(cs_str, material, thickness_nm) -> str\nDeposit a film layer.".as_ptr(),
    },
    PyMethodDef {
        ml_name:  c"etch".as_ptr(),
        ml_meth:  Some(py_etch),
        ml_flags: METH_VARARGS,
        ml_doc:   c"etch(cs_str, target_material, depth_nm) -> str\nEtch the top layers.".as_ptr(),
    },
    PyMethodDef {
        ml_name:  c"implant".as_ptr(),
        ml_meth:  Some(py_implant),
        ml_flags: METH_VARARGS,
        ml_doc:   c"implant(cs_str, species, energy_kev, dose_per_cm2) -> str\nIon implantation.".as_ptr(),
    },
    PyMethodDef {
        ml_name:  c"diffuse".as_ptr(),
        ml_meth:  Some(py_diffuse),
        ml_flags: METH_VARARGS,
        ml_doc:   c"diffuse(cs_str, time_min[, temperature_c]) -> str\nFick diffusion anneal.".as_ptr(),
    },
    PyMethodDef {
        ml_name:  c"implant_range".as_ptr(),
        ml_meth:  Some(py_implant_range),
        ml_flags: METH_VARARGS,
        ml_doc:   c"implant_range(species, energy_kev) -> (float, float)\nReturn (Rp_nm, delta_Rp_nm) from SRIM table.".as_ptr(),
    },
    PyMethodDef {
        ml_name:  c"diffusivity_cm2_per_s".as_ptr(),
        ml_meth:  Some(py_diffusivity_cm2_per_s),
        ml_flags: METH_VARARGS,
        ml_doc:   c"diffusivity_cm2_per_s(species, temperature_c) -> float\nArrhenius diffusivity D(T) [cm2/s].".as_ptr(),
    },
    // sentinel
    PyMethodDef { ml_name: ptr::null(), ml_meth: None, ml_flags: 0, ml_doc: ptr::null() },
];

// ─────────────────────────────────────────────────────────────────────────────
// Module definition
// ─────────────────────────────────────────────────────────────────────────────

static mut MODULE_DEF: PyModuleDef = PyModuleDef {
    m_base: PyModuleDef_Base {
        ob_base: [0u8; std::mem::size_of::<usize>() * 2],
        m_init:  None,
        m_index: 0,
        m_copy:  ptr::null_mut(),
    },
    m_name:    c"silicon_rust_python".as_ptr(),
    m_doc:     c"Rust-backed silicon simulation stack: device-physics, mosfet-models, fab-process-simulation.".as_ptr(),
    m_size:    -1,
    m_methods: &raw mut MODULE_METHODS as *mut PyMethodDef,
    m_slots:   ptr::null_mut(),
    m_traverse: ptr::null_mut(),
    m_clear:   ptr::null_mut(),
    m_free:    ptr::null_mut(),
};

// ─────────────────────────────────────────────────────────────────────────────
// Module init — called by Python on `import silicon_rust_python`
// ─────────────────────────────────────────────────────────────────────────────

/// # Safety
///
/// This is the CPython module initialization entry point, called by the
/// interpreter exactly once when the extension module is imported. It must only
/// be invoked by CPython through the standard import machinery; it reads the
/// module-global `MODULE_DEF` static and hands it to `PyModule_Create2`, so the
/// interpreter must be initialized and the ABI must match `PYTHON_API_VERSION`.
#[no_mangle]
pub unsafe extern "C" fn PyInit_silicon_rust_python() -> PyObjectPtr {
    PyModule_Create2(&raw mut MODULE_DEF, PYTHON_API_VERSION as c_int)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests — pure Rust, no Python interpreter required
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── CrossSection wire format ─────────────────────────────────────────────

    #[test]
    fn cs_wire_empty_roundtrip() {
        let cs = cs_from_wire("");
        assert!(cs.layers.is_empty());
        assert_eq!(cs_to_wire(&cs), "");
    }

    #[test]
    fn cs_wire_single_layer() {
        let cs = cs_from_wire("Si:500.0");
        assert_eq!(cs.layers.len(), 1);
        assert_eq!(cs.layers[0].material, "Si");
        assert!((cs.layers[0].thickness_nm - 500.0).abs() < 1e-9);
        let s = cs_to_wire(&cs);
        assert!(s.starts_with("Si:500"));
    }

    #[test]
    fn cs_wire_multi_layer_roundtrip() {
        let input = "Poly:50.0|SiO2:4.8|Si:500.0";
        let cs = cs_from_wire(input);
        assert_eq!(cs.layers.len(), 3);
        assert_eq!(cs.layers[0].material, "Poly");
        assert_eq!(cs.layers[1].material, "SiO2");
        assert_eq!(cs.layers[2].material, "Si");
        // Re-serialised string should preserve all three layers.
        let s = cs_to_wire(&cs);
        assert!(s.contains("Poly"), "got: {}", s);
        assert!(s.contains("SiO2"), "got: {}", s);
        assert!(s.contains("Si:500"), "got: {}", s);
    }

    #[test]
    fn cs_wire_malformed_entry_skipped() {
        // A malformed entry is silently skipped — the rest of the cross-section
        // is still parsed.
        let cs = cs_from_wire("Si:500.0|bad_entry|SiO2:4.8");
        assert_eq!(cs.layers.len(), 2, "malformed entry must be skipped");
    }

    // ── device-physics ───────────────────────────────────────────────────────

    #[test]
    fn thermal_voltage_at_300k() {
        let vt = dp::thermal_voltage(300.0);
        assert!((vt - 0.025852).abs() < 1e-5, "V_T={vt}");
    }

    #[test]
    fn pn_junction_built_in_voltage_typical() {
        let j = dp::PNJunction::new(1e23, 1e22, 1.0, 300.0, 1e-6, 1e-6).unwrap();
        let vbi = j.built_in_voltage();
        assert!(vbi > 0.6 && vbi < 1.1, "V_bi={vbi}");
    }

    #[test]
    fn mosfet_threshold_voltage_nmos() {
        let p = dp::MOSFETParams::new("NMOS", 130e-9, 1e-6, 2e-9, 1e24, -0.05, 0.0, 300.0).unwrap();
        let vt = p.threshold_voltage(0.0).unwrap();
        assert!(vt > 0.5 && vt < 1.6, "V_t={vt}");
    }

    // ── mosfet-models ────────────────────────────────────────────────────────

    #[test]
    fn eval_level1_saturation() {
        let (id, gm, gds, _gmb, _cgs, _cgd, _cgb, _cbs, _cbd, region) =
            eval_level1_rs(0.42, 220e-6, 0.05, 0.27, 0.84, 1e-6, 130e-9, 1.4,
                           1.8, 1.8, 0.0, 300.15);
        assert_eq!(region, "saturation");
        assert!(id > 0.0, "Id must be positive in saturation");
        assert!(gm > 0.0, "gm must be positive");
        assert!(gds >= 0.0, "gds must be non-negative");
    }

    #[test]
    fn eval_level1_cutoff() {
        let (id, _gm, _gds, _gmb, _cgs, _cgd, _cgb, _cbs, _cbd, region) =
            eval_level1_rs(0.42, 220e-6, 0.05, 0.27, 0.84, 1e-6, 130e-9, 1.4,
                           0.0, 1.8, 0.0, 300.15);
        // V_GS = 0 < V_t → subthreshold or cutoff
        assert!(id >= 0.0);
        assert!(region == "cutoff" || region == "subthreshold",
                "region={region}");
    }

    // ── fab-process-simulation via wire format ───────────────────────────────

    #[test]
    fn deal_grove_adds_sio2_layer() {
        let cs_in = cs_from_wire("Si:500.0");
        let cs_out = fps::deal_grove_oxidation(&cs_in, 5.0, None, None).unwrap();
        assert_eq!(cs_out.layers[0].material, "SiO2");
        assert!(cs_out.layers[0].thickness_nm > 0.0);
    }

    #[test]
    fn deposit_prepends_layer() {
        let cs_in  = cs_from_wire("Si:500.0");
        let cs_out = fps::deposit(&cs_in, "Poly", 50.0).unwrap();
        assert_eq!(cs_out.layers[0].material, "Poly");
        assert_eq!(cs_out.layers[1].material, "Si");
    }

    #[test]
    fn etch_removes_top_material() {
        let cs_in  = cs_from_wire("Poly:50.0|Si:500.0");
        let cs_out = fps::etch(&cs_in, "Poly", 50.0);
        assert_eq!(cs_out.layers.len(), 1);
        assert_eq!(cs_out.layers[0].material, "Si");
    }

    #[test]
    fn implant_range_boron_30kev() {
        let (rp, straggle) = fps::implant_range("B", 30.0).unwrap();
        assert!((rp - 92.0).abs() < 1e-6);
        assert!((straggle - 38.0).abs() < 1e-6);
    }

    #[test]
    fn diffusivity_boron_1000c() {
        let d = fps::diffusivity_cm2_per_s("B", 1000.0);
        assert!((d - 1e-14).abs() < 1e-20, "D={d}");
    }

    #[test]
    fn wire_roundtrip_after_process_steps() {
        // Build a realistic gate stack and verify the wire format round-trips.
        let cs = cs_from_wire("Si:500.0");
        let cs = fps::deal_grove_oxidation(&cs, 5.0, None, None).unwrap();
        let cs = fps::deposit(&cs, "Poly", 50.0).unwrap();
        let wire = cs_to_wire(&cs);
        let cs2  = cs_from_wire(&wire);
        assert_eq!(cs.layers.len(), cs2.layers.len());
        for (a, b) in cs.layers.iter().zip(cs2.layers.iter()) {
            assert_eq!(a.material, b.material);
            assert!((a.thickness_nm - b.thickness_nm).abs() < 1e-6,
                    "thickness mismatch: {} vs {}", a.thickness_nm, b.thickness_nm);
        }
    }
}
