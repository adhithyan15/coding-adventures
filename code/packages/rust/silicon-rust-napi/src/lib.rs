//! Node.js N-API addon for the silicon simulation stack.
//!
//! Exposes `device-physics`, `mosfet-models`, and `fab-process-simulation` to
//! JavaScript and TypeScript via the stable N-API (ABI version 8).  Uses the
//! zero-dependency `node-bridge` crate — no napi-rs, no bindgen, no Node.js
//! headers at build time.
//!
//! ## Why `#[cfg(not(test))]` on all N-API code
//!
//! On Windows, MSVC's linker (`lld-link`) resolves ALL undefined symbols at
//! link time, even dead code.  The N-API symbols (`napi_create_function`,
//! etc.) live in `node.exe`, not in a static library we can easily provide
//! during `cargo test`.  Gating every N-API import and function with
//! `#[cfg(not(test))]` excludes them entirely from the test binary, so
//! `cargo test` links cleanly on all platforms without Node.js.
//!
//! When building the cdylib (the actual `.node` addon), `#[cfg(test)]` is
//! not set, so all N-API code is compiled in normally.
//!
//! ## Wire format for CrossSection (v0.1)
//!
//! A `CrossSection` is serialised as a pipe-separated list of
//! `material:thickness_nm` pairs, ordered top-to-bottom:
//!
//! ```text
//! ""                               # empty
//! "Si:500.0"                       # bare silicon substrate
//! "SiO2:4.8|Si:500.0"             # gate oxide on silicon
//! "Poly:50.0|SiO2:4.8|Si:500.0"  # poly gate on gate oxide
//! ```
//!
//! ## JavaScript usage
//!
//! ```javascript
//! const srp = require('./silicon_rust_napi.node');
//!
//! // Physical constants
//! console.log(srp.kBoltzmann());             // 1.380649e-23 J/K
//! console.log(srp.thermalVoltage(300));      // 0.025852 V
//!
//! // Process simulation
//! let cs = srp.deposit("", "Si", 500.0);
//! cs = srp.dealGroveOxidation(cs, 5.0);
//! cs = srp.deposit(cs, "Poly", 50.0);
//!
//! // Level-1 MOSFET at a 130 nm node
//! const r = srp.evaluateLevel1Defaults(1.8, 1.8, 0.0, 300.15);
//! console.log(r.region);  // "saturation"
//! ```

use device_physics as dp;
use fab_process_simulation as fps;
use mosfet_models as mm;

// N-API imports — excluded from test builds so the test binary does not pull
// in undefined Node.js symbols and fail to link on Windows (MSVC/lld-link).
#[cfg(not(test))]
use node_bridge::{
    create_function, f64_from_js, f64_to_js, get_cb_info, object_new, set_named_property,
    str_from_js, str_to_js, throw_error, undefined,
    napi_callback_info, napi_env, napi_value,
};

// ─────────────────────────────────────────────────────────────────────────────
// CrossSection wire format helpers (pure Rust, always compiled, cargo-testable)
// ─────────────────────────────────────────────────────────────────────────────

/// Return `Err` if `material` contains a wire-format delimiter (`|` or `:`).
///
/// Material names must be plain ASCII identifiers.  Delimiters would let a
/// caller inject extra layers into the wire string and corrupt the
/// cross-section seen by downstream process steps.
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
/// Doping profiles are elided in v0.1 — they survive process steps but are
/// not transported across the FFI boundary.
pub fn cs_to_wire(cs: &fps::CrossSection) -> String {
    cs.layers
        .iter()
        .map(|l| format!("{}:{}", l.material, l.thickness_nm))
        .collect::<Vec<_>>()
        .join("|")
}

/// Deserialise a `CrossSection` from the pipe-delimited wire format.
///
/// Silently skips malformed entries so an empty string returns an empty
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
// Pure-Rust helper for Level-1 evaluation (testable without Node.js)
// ─────────────────────────────────────────────────────────────────────────────

/// Evaluate the Level-1 MOSFET model with explicit parameters.
///
/// Returns `(id, gm, gds, gmb, cgs, cgd, cgb, cbs, cbd, region_str)`.
/// `region_str` is one of `"cutoff"`, `"subthreshold"`, `"triode"`,
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
// N-API implementation — excluded from test builds
//
// Everything from here to the end of the `napi_impl` section is compiled only
// when building the library or cdylib (not when running `cargo test`).
// ─────────────────────────────────────────────────────────────────────────────

/// Parse a required `f64` positional argument.
///
/// Throws a JS TypeError and causes the calling function to early-return
/// `undefined` if the argument is missing or not a number.
#[cfg(not(test))]
macro_rules! req_f64 {
    ($env:expr, $args:expr, $idx:expr, $fn_name:expr) => {
        match $args.get($idx).and_then(|&v| f64_from_js($env, v)) {
            Some(v) => v,
            None => {
                throw_error(
                    $env,
                    concat!(
                        $fn_name,
                        ": missing or non-number argument at position ",
                        stringify!($idx)
                    ),
                );
                return undefined($env);
            }
        }
    };
}

/// Parse a required `String` positional argument.
///
/// Throws a JS TypeError and causes the calling function to early-return
/// `undefined` if the argument is missing or not a string.
#[cfg(not(test))]
macro_rules! req_str {
    ($env:expr, $args:expr, $idx:expr, $fn_name:expr) => {
        match $args.get($idx).and_then(|&v| str_from_js($env, v)) {
            Some(v) => v,
            None => {
                throw_error(
                    $env,
                    concat!(
                        $fn_name,
                        ": missing or non-string argument at position ",
                        stringify!($idx)
                    ),
                );
                return undefined($env);
            }
        }
    };
}

/// Convert a `MosResult` struct to a plain JS object:
/// `{ id, gm, gds, gmb, cgs, cgd, cgb, cbs, cbd, region }`.
#[cfg(not(test))]
unsafe fn mos_result_to_js_object(env: napi_env, r: &mm::MosResult) -> napi_value {
    let obj = object_new(env);
    set_named_property(env, obj, "id",     f64_to_js(env, r.id));
    set_named_property(env, obj, "gm",     f64_to_js(env, r.gm));
    set_named_property(env, obj, "gds",    f64_to_js(env, r.gds));
    set_named_property(env, obj, "gmb",    f64_to_js(env, r.gmb));
    set_named_property(env, obj, "cgs",    f64_to_js(env, r.cgs));
    set_named_property(env, obj, "cgd",    f64_to_js(env, r.cgd));
    set_named_property(env, obj, "cgb",    f64_to_js(env, r.cgb));
    set_named_property(env, obj, "cbs",    f64_to_js(env, r.cbs));
    set_named_property(env, obj, "cbd",    f64_to_js(env, r.cbd));
    set_named_property(env, obj, "region", str_to_js(env, r.region.as_str()));
    obj
}

// ── Physical constants (no arguments) ────────────────────────────────────────

#[cfg(not(test))]
unsafe extern "C" fn napi_k_boltzmann(env: napi_env, _info: napi_callback_info) -> napi_value {
    f64_to_js(env, dp::K_BOLTZMANN)
}

#[cfg(not(test))]
unsafe extern "C" fn napi_q_electron(env: napi_env, _info: napi_callback_info) -> napi_value {
    f64_to_js(env, dp::Q_ELECTRON)
}

#[cfg(not(test))]
unsafe extern "C" fn napi_eps0(env: napi_env, _info: napi_callback_info) -> napi_value {
    f64_to_js(env, dp::EPS0)
}

#[cfg(not(test))]
unsafe extern "C" fn napi_eps_si(env: napi_env, _info: napi_callback_info) -> napi_value {
    f64_to_js(env, dp::EPS_SI)
}

#[cfg(not(test))]
unsafe extern "C" fn napi_eps_ox(env: napi_env, _info: napi_callback_info) -> napi_value {
    f64_to_js(env, dp::EPS_OX)
}

#[cfg(not(test))]
unsafe extern "C" fn napi_ni_at_300k(env: napi_env, _info: napi_callback_info) -> napi_value {
    f64_to_js(env, dp::N_I_300K)
}

#[cfg(not(test))]
unsafe extern "C" fn napi_eg_si_300k(env: napi_env, _info: napi_callback_info) -> napi_value {
    f64_to_js(env, dp::EG_SI_300K)
}

#[cfg(not(test))]
unsafe extern "C" fn napi_mu_n_300k(env: napi_env, _info: napi_callback_info) -> napi_value {
    f64_to_js(env, dp::MU_N_300K)
}

#[cfg(not(test))]
unsafe extern "C" fn napi_mu_p_300k(env: napi_env, _info: napi_callback_info) -> napi_value {
    f64_to_js(env, dp::MU_P_300K)
}

// ── device-physics functions ──────────────────────────────────────────────────

/// `thermalVoltage(tKelvin: number): number` — V_T = kT/q [V].
#[cfg(not(test))]
unsafe extern "C" fn napi_thermal_voltage(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 1);
    let t = req_f64!(env, args, 0, "thermalVoltage");
    f64_to_js(env, dp::thermal_voltage(t))
}

/// `intrinsicConcentration(tKelvin: number): number` — n_i(T) [/m³].
#[cfg(not(test))]
unsafe extern "C" fn napi_intrinsic_concentration(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 1);
    let t = req_f64!(env, args, 0, "intrinsicConcentration");
    match dp::intrinsic_concentration(t) {
        Ok(ni)   => f64_to_js(env, ni),
        Err(msg) => {
            throw_error(env, &format!("intrinsicConcentration: {}", msg));
            undefined(env)
        }
    }
}

/// `fermiPotential(nDoping, kind, tKelvin): number` — φ_F [V].
#[cfg(not(test))]
unsafe extern "C" fn napi_fermi_potential(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 3);
    let n    = req_f64!(env, args, 0, "fermiPotential");
    let kind = req_str!(env, args, 1, "fermiPotential");
    let t    = req_f64!(env, args, 2, "fermiPotential");
    match dp::fermi_potential(n, &kind, t) {
        Ok(phi)  => f64_to_js(env, phi),
        Err(msg) => {
            throw_error(env, &format!("fermiPotential: {}", msg));
            undefined(env)
        }
    }
}

/// `pnJunctionBuiltInVoltage(na, nd, t): number` — V_bi [V].
#[cfg(not(test))]
unsafe extern "C" fn napi_pn_junction_built_in_voltage(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 3);
    let na = req_f64!(env, args, 0, "pnJunctionBuiltInVoltage");
    let nd = req_f64!(env, args, 1, "pnJunctionBuiltInVoltage");
    let t  = req_f64!(env, args, 2, "pnJunctionBuiltInVoltage");
    match dp::PNJunction::new(na, nd, 1.0, t, 1e-6, 1e-6) {
        Ok(j)    => f64_to_js(env, j.built_in_voltage()),
        Err(msg) => {
            throw_error(env, &format!("pnJunctionBuiltInVoltage: {}", msg));
            undefined(env)
        }
    }
}

/// `pnJunctionDepletionWidth(na, nd, t, vApplied): number` — W [m].
#[cfg(not(test))]
unsafe extern "C" fn napi_pn_junction_depletion_width(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 4);
    let na        = req_f64!(env, args, 0, "pnJunctionDepletionWidth");
    let nd        = req_f64!(env, args, 1, "pnJunctionDepletionWidth");
    let t         = req_f64!(env, args, 2, "pnJunctionDepletionWidth");
    let v_applied = req_f64!(env, args, 3, "pnJunctionDepletionWidth");
    match dp::PNJunction::new(na, nd, 1.0, t, 1e-6, 1e-6) {
        Ok(j)    => f64_to_js(env, j.depletion_width(v_applied)),
        Err(msg) => {
            throw_error(env, &format!("pnJunctionDepletionWidth: {}", msg));
            undefined(env)
        }
    }
}

/// `pnJunctionSaturationCurrent(na, nd, a, t, tauN, tauP): number` — I_S [A].
#[cfg(not(test))]
unsafe extern "C" fn napi_pn_junction_saturation_current(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 6);
    let na    = req_f64!(env, args, 0, "pnJunctionSaturationCurrent");
    let nd    = req_f64!(env, args, 1, "pnJunctionSaturationCurrent");
    let a     = req_f64!(env, args, 2, "pnJunctionSaturationCurrent");
    let t     = req_f64!(env, args, 3, "pnJunctionSaturationCurrent");
    let tau_n = req_f64!(env, args, 4, "pnJunctionSaturationCurrent");
    let tau_p = req_f64!(env, args, 5, "pnJunctionSaturationCurrent");
    match dp::PNJunction::new(na, nd, a, t, tau_n, tau_p) {
        Ok(j)    => f64_to_js(env, j.saturation_current()),
        Err(msg) => {
            throw_error(env, &format!("pnJunctionSaturationCurrent: {}", msg));
            undefined(env)
        }
    }
}

/// `pnJunctionCurrent(na, nd, a, t, tauN, tauP, v): number` — I [A].
#[cfg(not(test))]
unsafe extern "C" fn napi_pn_junction_current(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 7);
    let na    = req_f64!(env, args, 0, "pnJunctionCurrent");
    let nd    = req_f64!(env, args, 1, "pnJunctionCurrent");
    let a     = req_f64!(env, args, 2, "pnJunctionCurrent");
    let t     = req_f64!(env, args, 3, "pnJunctionCurrent");
    let tau_n = req_f64!(env, args, 4, "pnJunctionCurrent");
    let tau_p = req_f64!(env, args, 5, "pnJunctionCurrent");
    let v     = req_f64!(env, args, 6, "pnJunctionCurrent");
    match dp::PNJunction::new(na, nd, a, t, tau_n, tau_p) {
        Ok(j)    => f64_to_js(env, j.current(v)),
        Err(msg) => {
            throw_error(env, &format!("pnJunctionCurrent: {}", msg));
            undefined(env)
        }
    }
}

/// `mosfetThresholdVoltage(deviceType, l, w, tOx, nBody, phiMs, qOx, t, vSb): number`
#[cfg(not(test))]
unsafe extern "C" fn napi_mosfet_threshold_voltage(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 9);
    let device_type = req_str!(env, args, 0, "mosfetThresholdVoltage");
    let l      = req_f64!(env, args, 1, "mosfetThresholdVoltage");
    let w      = req_f64!(env, args, 2, "mosfetThresholdVoltage");
    let t_ox   = req_f64!(env, args, 3, "mosfetThresholdVoltage");
    let n_body = req_f64!(env, args, 4, "mosfetThresholdVoltage");
    let phi_ms = req_f64!(env, args, 5, "mosfetThresholdVoltage");
    let q_ox   = req_f64!(env, args, 6, "mosfetThresholdVoltage");
    let t      = req_f64!(env, args, 7, "mosfetThresholdVoltage");
    let v_sb   = req_f64!(env, args, 8, "mosfetThresholdVoltage");
    match dp::MOSFETParams::new(&device_type, l, w, t_ox, n_body, phi_ms, q_ox, t) {
        Ok(p) => match p.threshold_voltage(v_sb) {
            Ok(vt)   => f64_to_js(env, vt),
            Err(msg) => {
                throw_error(env, &format!("mosfetThresholdVoltage: {}", msg));
                undefined(env)
            }
        },
        Err(msg) => {
            throw_error(env, &format!("mosfetThresholdVoltage: {}", msg));
            undefined(env)
        }
    }
}

// ── mosfet-models functions ───────────────────────────────────────────────────

/// `evaluateLevel1(vt0, kp, lambda, gamma, phi, w, l, nSub,
///                 vGs, vDs, vBs, t): MosResult`
#[cfg(not(test))]
unsafe extern "C" fn napi_evaluate_level1(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 12);
    let vt0    = req_f64!(env, args,  0, "evaluateLevel1");
    let kp     = req_f64!(env, args,  1, "evaluateLevel1");
    let lambda = req_f64!(env, args,  2, "evaluateLevel1");
    let gamma  = req_f64!(env, args,  3, "evaluateLevel1");
    let phi    = req_f64!(env, args,  4, "evaluateLevel1");
    let w      = req_f64!(env, args,  5, "evaluateLevel1");
    let l      = req_f64!(env, args,  6, "evaluateLevel1");
    let n_sub  = req_f64!(env, args,  7, "evaluateLevel1");
    let v_gs   = req_f64!(env, args,  8, "evaluateLevel1");
    let v_ds   = req_f64!(env, args,  9, "evaluateLevel1");
    let v_bs   = req_f64!(env, args, 10, "evaluateLevel1");
    let t      = req_f64!(env, args, 11, "evaluateLevel1");

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
    mos_result_to_js_object(env, &r)
}

/// `evaluateLevel1Defaults(vGs, vDs, vBs, t): MosResult`
#[cfg(not(test))]
unsafe extern "C" fn napi_evaluate_level1_defaults(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 4);
    let v_gs = req_f64!(env, args, 0, "evaluateLevel1Defaults");
    let v_ds = req_f64!(env, args, 1, "evaluateLevel1Defaults");
    let v_bs = req_f64!(env, args, 2, "evaluateLevel1Defaults");
    let t    = req_f64!(env, args, 3, "evaluateLevel1Defaults");
    let p    = mm::Level1Params::default();
    let r    = mm::evaluate_level1(&p, v_gs, v_ds, v_bs, t);
    mos_result_to_js_object(env, &r)
}

// ── fab-process-simulation functions ─────────────────────────────────────────

/// `dealGroveOxidation(csStr, timeMin[, aUm, bUm2PerHr]): string`
#[cfg(not(test))]
unsafe extern "C" fn napi_deal_grove_oxidation(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 4);
    let cs_str   = req_str!(env, args, 0, "dealGroveOxidation");
    let time_min = req_f64!(env, args, 1, "dealGroveOxidation");
    let a_um  = args.get(2).and_then(|&v| f64_from_js(env, v)).filter(|&x| x > 0.0);
    let b_um2 = args.get(3).and_then(|&v| f64_from_js(env, v)).filter(|&x| x > 0.0);
    let cs = cs_from_wire(&cs_str);
    match fps::deal_grove_oxidation(&cs, time_min, a_um, b_um2) {
        Ok(new_cs) => str_to_js(env, &cs_to_wire(&new_cs)),
        Err(msg)   => {
            throw_error(env, &format!("dealGroveOxidation: {}", msg));
            undefined(env)
        }
    }
}

/// `deposit(csStr, material, thicknessNm): string`
///
/// Rejects material names containing `|` or `:` to prevent layer injection.
#[cfg(not(test))]
unsafe extern "C" fn napi_deposit(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 3);
    let cs_str       = req_str!(env, args, 0, "deposit");
    let material     = req_str!(env, args, 1, "deposit");
    let thickness_nm = req_f64!(env, args, 2, "deposit");
    if let Err(msg) = validate_material_name(&material) {
        throw_error(env, &format!("deposit: {}", msg));
        return undefined(env);
    }
    let cs = cs_from_wire(&cs_str);
    match fps::deposit(&cs, &material, thickness_nm) {
        Ok(new_cs) => str_to_js(env, &cs_to_wire(&new_cs)),
        Err(msg)   => {
            throw_error(env, &format!("deposit: {}", msg));
            undefined(env)
        }
    }
}

/// `etch(csStr, targetMaterial, depthNm): string`
#[cfg(not(test))]
unsafe extern "C" fn napi_etch(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 3);
    let cs_str          = req_str!(env, args, 0, "etch");
    let target_material = req_str!(env, args, 1, "etch");
    let depth_nm        = req_f64!(env, args, 2, "etch");
    let cs     = cs_from_wire(&cs_str);
    let new_cs = fps::etch(&cs, &target_material, depth_nm);
    str_to_js(env, &cs_to_wire(&new_cs))
}

/// `implant(csStr, species, energyKev, doseCm2): string`
#[cfg(not(test))]
unsafe extern "C" fn napi_implant(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 4);
    let cs_str       = req_str!(env, args, 0, "implant");
    let species      = req_str!(env, args, 1, "implant");
    let energy_kev   = req_f64!(env, args, 2, "implant");
    let dose_per_cm2 = req_f64!(env, args, 3, "implant");
    let cs = cs_from_wire(&cs_str);
    match fps::implant(&cs, &species, energy_kev, dose_per_cm2) {
        Ok(new_cs) => str_to_js(env, &cs_to_wire(&new_cs)),
        Err(msg)   => {
            throw_error(env, &format!("implant: {}", msg));
            undefined(env)
        }
    }
}

/// `diffuse(csStr, timeMin[, temperatureC]): string`
#[cfg(not(test))]
unsafe extern "C" fn napi_diffuse(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 3);
    let cs_str   = req_str!(env, args, 0, "diffuse");
    let time_min = req_f64!(env, args, 1, "diffuse");
    let temp_c   = args.get(2).and_then(|&v| f64_from_js(env, v));
    let cs     = cs_from_wire(&cs_str);
    let new_cs = fps::diffuse(&cs, time_min, temp_c);
    str_to_js(env, &cs_to_wire(&new_cs))
}

/// `implantRange(species, energyKev): { rp: number, straggle: number }`
#[cfg(not(test))]
unsafe extern "C" fn napi_implant_range(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 2);
    let species    = req_str!(env, args, 0, "implantRange");
    let energy_kev = req_f64!(env, args, 1, "implantRange");
    match fps::implant_range(&species, energy_kev) {
        Ok((rp, straggle)) => {
            let obj = object_new(env);
            set_named_property(env, obj, "rp",       f64_to_js(env, rp));
            set_named_property(env, obj, "straggle", f64_to_js(env, straggle));
            obj
        }
        Err(msg) => {
            throw_error(env, &format!("implantRange: {}", msg));
            undefined(env)
        }
    }
}

/// `diffusivityCm2PerS(species, temperatureC): number`
#[cfg(not(test))]
unsafe extern "C" fn napi_diffusivity_cm2_per_s(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 2);
    let species     = req_str!(env, args, 0, "diffusivityCm2PerS");
    let temperature = req_f64!(env, args, 1, "diffusivityCm2PerS");
    f64_to_js(env, fps::diffusivity_cm2_per_s(&species, temperature))
}

// ─────────────────────────────────────────────────────────────────────────────
// Module registration entry point
//
// Node.js calls this symbol the first time `require()` loads the addon.  We
// receive an empty `exports` object and attach all 26 functions before
// returning it.
// ─────────────────────────────────────────────────────────────────────────────

/// # Safety
///
/// This is the N-API module entry point invoked by Node.js itself. `env` must
/// be the valid `napi_env` and `exports` the valid `napi_value` object that the
/// Node runtime passes when it loads the addon; callers other than the Node
/// loader must uphold the same contract.
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn napi_register_module_v1(
    env: napi_env,
    exports: napi_value,
) -> napi_value {
    // ── Physical constants ────────────────────────────────────────────────────
    set_named_property(env, exports, "kBoltzmann",
        create_function(env, "kBoltzmann", Some(napi_k_boltzmann)));
    set_named_property(env, exports, "qElectron",
        create_function(env, "qElectron", Some(napi_q_electron)));
    set_named_property(env, exports, "eps0",
        create_function(env, "eps0", Some(napi_eps0)));
    set_named_property(env, exports, "epsSi",
        create_function(env, "epsSi", Some(napi_eps_si)));
    set_named_property(env, exports, "epsOx",
        create_function(env, "epsOx", Some(napi_eps_ox)));
    set_named_property(env, exports, "niAt300k",
        create_function(env, "niAt300k", Some(napi_ni_at_300k)));
    set_named_property(env, exports, "egSiAt300k",
        create_function(env, "egSiAt300k", Some(napi_eg_si_300k)));
    set_named_property(env, exports, "muN300k",
        create_function(env, "muN300k", Some(napi_mu_n_300k)));
    set_named_property(env, exports, "muP300k",
        create_function(env, "muP300k", Some(napi_mu_p_300k)));

    // ── device-physics functions ──────────────────────────────────────────────
    set_named_property(env, exports, "thermalVoltage",
        create_function(env, "thermalVoltage", Some(napi_thermal_voltage)));
    set_named_property(env, exports, "intrinsicConcentration",
        create_function(env, "intrinsicConcentration", Some(napi_intrinsic_concentration)));
    set_named_property(env, exports, "fermiPotential",
        create_function(env, "fermiPotential", Some(napi_fermi_potential)));
    set_named_property(env, exports, "pnJunctionBuiltInVoltage",
        create_function(env, "pnJunctionBuiltInVoltage", Some(napi_pn_junction_built_in_voltage)));
    set_named_property(env, exports, "pnJunctionDepletionWidth",
        create_function(env, "pnJunctionDepletionWidth", Some(napi_pn_junction_depletion_width)));
    set_named_property(env, exports, "pnJunctionSaturationCurrent",
        create_function(env, "pnJunctionSaturationCurrent", Some(napi_pn_junction_saturation_current)));
    set_named_property(env, exports, "pnJunctionCurrent",
        create_function(env, "pnJunctionCurrent", Some(napi_pn_junction_current)));
    set_named_property(env, exports, "mosfetThresholdVoltage",
        create_function(env, "mosfetThresholdVoltage", Some(napi_mosfet_threshold_voltage)));

    // ── mosfet-models functions ───────────────────────────────────────────────
    set_named_property(env, exports, "evaluateLevel1",
        create_function(env, "evaluateLevel1", Some(napi_evaluate_level1)));
    set_named_property(env, exports, "evaluateLevel1Defaults",
        create_function(env, "evaluateLevel1Defaults", Some(napi_evaluate_level1_defaults)));

    // ── fab-process-simulation functions ─────────────────────────────────────
    set_named_property(env, exports, "dealGroveOxidation",
        create_function(env, "dealGroveOxidation", Some(napi_deal_grove_oxidation)));
    set_named_property(env, exports, "deposit",
        create_function(env, "deposit", Some(napi_deposit)));
    set_named_property(env, exports, "etch",
        create_function(env, "etch", Some(napi_etch)));
    set_named_property(env, exports, "implant",
        create_function(env, "implant", Some(napi_implant)));
    set_named_property(env, exports, "diffuse",
        create_function(env, "diffuse", Some(napi_diffuse)));
    set_named_property(env, exports, "implantRange",
        create_function(env, "implantRange", Some(napi_implant_range)));
    set_named_property(env, exports, "diffusivityCm2PerS",
        create_function(env, "diffusivityCm2PerS", Some(napi_diffusivity_cm2_per_s)));

    exports
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests — pure Rust, no Node.js required
//
// N-API code is excluded from test builds via `#[cfg(not(test))]` on all
// N-API imports and functions.  `cargo test` compiles and links cleanly on all
// platforms (Linux, macOS, Windows) without needing Node.js or node.lib.
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
        assert!(s.starts_with("Si:500"), "got: {s}");
    }

    #[test]
    fn cs_wire_multi_layer_roundtrip() {
        let input = "Poly:50.0|SiO2:4.8|Si:500.0";
        let cs = cs_from_wire(input);
        assert_eq!(cs.layers.len(), 3);
        assert_eq!(cs.layers[0].material, "Poly");
        assert_eq!(cs.layers[1].material, "SiO2");
        assert_eq!(cs.layers[2].material, "Si");
        let s = cs_to_wire(&cs);
        assert!(s.contains("Poly"),   "missing Poly in: {s}");
        assert!(s.contains("SiO2"),   "missing SiO2 in: {s}");
        assert!(s.contains("Si:500"), "missing Si:500 in: {s}");
    }

    #[test]
    fn cs_wire_malformed_entry_skipped() {
        let cs = cs_from_wire("Si:500.0|bad_entry|SiO2:4.8");
        assert_eq!(cs.layers.len(), 2, "malformed entry must be skipped");
    }

    #[test]
    fn validate_material_rejects_delimiters() {
        assert!(validate_material_name("Si").is_ok());
        assert!(validate_material_name("SiO2").is_ok());
        assert!(validate_material_name("Poly|extra").is_err());
        assert!(validate_material_name("bad:name").is_err());
    }

    // ── device-physics ───────────────────────────────────────────────────────

    #[test]
    fn thermal_voltage_at_300k() {
        let vt = dp::thermal_voltage(300.0);
        assert!((vt - 0.025852).abs() < 1e-5, "V_T={vt}");
    }

    #[test]
    fn intrinsic_concentration_valid() {
        let ni = dp::intrinsic_concentration(300.0).unwrap();
        assert!(ni > 1e15 && ni < 1e17, "n_i={ni}");
    }

    #[test]
    fn intrinsic_concentration_below_100k_fails() {
        assert!(dp::intrinsic_concentration(50.0).is_err());
    }

    #[test]
    fn pn_junction_built_in_voltage_typical() {
        let j = dp::PNJunction::new(1e23, 1e22, 1.0, 300.0, 1e-6, 1e-6).unwrap();
        let vbi = j.built_in_voltage();
        assert!(vbi > 0.6 && vbi < 1.1, "V_bi={vbi}");
    }

    #[test]
    fn mosfet_threshold_voltage_nmos() {
        let p = dp::MOSFETParams::new("NMOS", 130e-9, 1e-6, 2e-9, 1e24, -0.05, 0.0, 300.0)
            .unwrap();
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
        assert!(id  > 0.0,  "Id must be positive");
        assert!(gm  > 0.0,  "gm must be positive");
        assert!(gds >= 0.0, "gds must be non-negative");
    }

    #[test]
    fn eval_level1_cutoff() {
        let (id, _gm, _gds, _gmb, _cgs, _cgd, _cgb, _cbs, _cbd, region) =
            eval_level1_rs(0.42, 220e-6, 0.05, 0.27, 0.84, 1e-6, 130e-9, 1.4,
                           0.0, 1.8, 0.0, 300.15);
        assert!(id >= 0.0);
        assert!(
            region == "cutoff" || region == "subthreshold",
            "region={region}"
        );
    }

    // ── fab-process-simulation ───────────────────────────────────────────────

    #[test]
    fn deal_grove_adds_sio2_layer() {
        let cs_in  = cs_from_wire("Si:500.0");
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
    fn etch_removes_top_layer() {
        let cs_in  = cs_from_wire("Poly:50.0|Si:500.0");
        let cs_out = fps::etch(&cs_in, "Poly", 50.0);
        assert_eq!(cs_out.layers.len(), 1);
        assert_eq!(cs_out.layers[0].material, "Si");
    }

    #[test]
    fn implant_range_boron_30kev() {
        let (rp, straggle) = fps::implant_range("B", 30.0).unwrap();
        assert!((rp       - 92.0).abs() < 1e-6, "rp={rp}");
        assert!((straggle - 38.0).abs() < 1e-6, "straggle={straggle}");
    }

    #[test]
    fn diffusivity_boron_1000c() {
        let d = fps::diffusivity_cm2_per_s("B", 1000.0);
        assert!((d - 1e-14).abs() < 1e-20, "D={d}");
    }

    #[test]
    fn wire_roundtrip_after_gate_stack() {
        let cs   = cs_from_wire("Si:500.0");
        let cs   = fps::deal_grove_oxidation(&cs, 5.0, None, None).unwrap();
        let cs   = fps::deposit(&cs, "Poly", 50.0).unwrap();
        let wire = cs_to_wire(&cs);
        let cs2  = cs_from_wire(&wire);
        assert_eq!(cs.layers.len(), cs2.layers.len());
        for (a, b) in cs.layers.iter().zip(cs2.layers.iter()) {
            assert_eq!(a.material, b.material);
            assert!(
                (a.thickness_nm - b.thickness_nm).abs() < 1e-6,
                "thickness mismatch: {} vs {}",
                a.thickness_nm,
                b.thickness_nm
            );
        }
    }
}
