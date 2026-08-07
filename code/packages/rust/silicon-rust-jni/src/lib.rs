//! JNI native library for the silicon simulation stack.
//!
//! Exposes `device-physics`, `mosfet-models`, and `fab-process-simulation`
//! to Java (and any JVM language) through the Java Native Interface.
//!
//! The corresponding Java class is
//! `com.codingadventures.silicon.SiliconSim` in the `silicon-rust-java`
//! package.  That class loads this library with:
//!
//! ```java
//! static { System.loadLibrary("silicon_rust_jni"); }
//! ```
//!
//! ## JNI function naming convention
//!
//! For `com.codingadventures.silicon.SiliconSim.thermalVoltage`:
//!
//! ```text
//! Java_com_codingadventures_silicon_SiliconSim_thermalVoltage
//! ```
//!
//! The JVM discovers native methods by symbol name, so every exported
//! function must be `#[no_mangle]` and `extern "C"`.
//!
//! ## Wire format for CrossSection
//!
//! A `CrossSection` is serialised as a pipe-separated list of
//! `material:thickness_nm` pairs, ordered top-to-bottom:
//!
//! ```text
//! ""                               empty
//! "Si:500.0"                       bare silicon substrate
//! "SiO2:4.8|Si:500.0"             gate oxide on silicon
//! "Poly:50.0|SiO2:4.8|Si:500.0"  poly gate on gate oxide
//! ```
//!
//! `{:?}` formatting preserves the decimal point on whole-number f64 values
//! (`500.0` → `"500.0"`, not `"500"`).
//!
//! ## Error handling
//!
//! Fallible functions throw
//! `com.codingadventures.silicon.SiliconException` via
//! `jni_throw_new`.  The pending exception is propagated by the JVM once
//! the native function returns.  The native function itself returns null
//! (for object types) or 0.0 (for primitives).

// Every exported fn is a `Java_*` JNI entry point invoked only by the JVM,
// which guarantees the env pointer / handle contract; the safety obligations
// are uniform and documented in the module header above.
#![allow(clippy::missing_safety_doc)]

use std::ptr::null_mut;

use device_physics as dp;
use fab_process_simulation as fps;
use mosfet_models as mm;

use jni_bridge::{
    JNIEnv,
    jclass, jdouble, jobject, jstring, jarray, jvalue,
    jni_find_class, jni_get_method_id, jni_get_string_utf,
    jni_new_double_array, jni_new_object_a, jni_new_string_utf,
    jni_set_double_array_region, jni_throw_new,
};

// ─────────────────────────────────────────────────────────────────────────────
// Internal exception class name (slash-separated JNI path)
// ─────────────────────────────────────────────────────────────────────────────

const SILICON_EX: &str = "com/codingadventures/silicon/SiliconException";

// ─────────────────────────────────────────────────────────────────────────────
// Wire format helpers (pure Rust — fully testable without a JVM)
// ─────────────────────────────────────────────────────────────────────────────

/// Reject material/species names that contain wire-format delimiters.
///
/// Pipe (`|`) and colon (`:`) are the two reserved separators in the
/// CrossSection wire format.  If a caller can inject either character into a
/// material name, it can corrupt the wire string seen by downstream
/// process steps.
pub fn validate_name(name: &str) -> Result<(), String> {
    if name.contains('|') || name.contains(':') {
        return Err(format!(
            "name {:?} contains a wire-format delimiter ('|' or ':'); \
             names must not contain '|' or ':'",
            name
        ));
    }
    Ok(())
}

/// Serialise a `CrossSection` to the pipe-delimited wire format.
///
/// Uses `{:?}` (Debug) for `f64` so that whole-number thicknesses keep
/// their decimal point: 500.0 → "500.0", not "500".  This makes the wire
/// string round-trippable without ambiguity.
pub fn cs_to_wire(cs: &fps::CrossSection) -> String {
    cs.layers
        .iter()
        .map(|l| format!("{}:{:?}", l.material, l.thickness_nm))
        .collect::<Vec<_>>()
        .join("|")
}

/// Deserialise a `CrossSection` from the pipe-delimited wire format.
///
/// Returns `Err` if any entry is malformed or if any material name contains
/// a wire-format delimiter character (`|` or `:`).  Rejecting the entire
/// wire string on the first bad entry prevents silent data corruption.
pub fn cs_from_wire(s: &str) -> Result<fps::CrossSection, String> {
    if s.is_empty() {
        return Ok(fps::CrossSection { layers: vec![] });
    }
    let mut layers = Vec::new();
    for entry in s.split('|') {
        let mut parts = entry.splitn(2, ':');
        let material = parts
            .next()
            .ok_or_else(|| format!("bad wire entry: {:?}", entry))?
            .to_string();
        validate_name(&material).map_err(|e| format!("cs_from_wire: {e}"))?;
        let thickness_nm: f64 = parts
            .next()
            .ok_or_else(|| format!("missing thickness in {:?}", entry))?
            .parse()
            .map_err(|_| format!("bad thickness in {:?}", entry))?;
        layers.push(fps::Layer::new(&material, thickness_nm));
    }
    Ok(fps::CrossSection { layers })
}

// ─────────────────────────────────────────────────────────────────────────────
// JNI helper: create a com.codingadventures.silicon.MosResult Java object
// ─────────────────────────────────────────────────────────────────────────────

/// Allocate and fill a `com.codingadventures.silicon.MosResult` Java object.
///
/// Constructor signature used: `"(DDDDDDDDDLjava/lang/String;)V"` — nine
/// `double` args followed by the region `String`.
///
/// Returns null and leaves an exception pending if the class or constructor
/// cannot be found.
unsafe fn make_mos_result(env: *mut JNIEnv, r: &mm::MosResult) -> jobject {
    let cls = jni_find_class(env, "com/codingadventures/silicon/MosResult");
    if cls.is_null() {
        return null_mut();
    }
    let ctor = jni_get_method_id(
        env, cls, "<init>",
        "(DDDDDDDDDLjava/lang/String;)V",
    );
    if ctor.is_null() {
        return null_mut();
    }
    let region_jstr = jni_new_string_utf(env, r.region.as_str());
    if region_jstr.is_null() {
        // OOM: NewStringUTF returned null; an OutOfMemoryError is pending.
        // Return null so the JVM propagates the OOM rather than crashing on
        // a null jobject in the NewObjectA argument array.
        return null_mut();
    }
    // jvalue array: 9 doubles then the region jstring.
    let args = [
        jvalue { d: r.id  },
        jvalue { d: r.gm  },
        jvalue { d: r.gds },
        jvalue { d: r.gmb },
        jvalue { d: r.cgs },
        jvalue { d: r.cgd },
        jvalue { d: r.cgb },
        jvalue { d: r.cbs },
        jvalue { d: r.cbd },
        jvalue { l: region_jstr },
    ];
    jni_new_object_a(env, cls, ctor, args.as_ptr())
}

// ─────────────────────────────────────────────────────────────────────────────
// Physical constants — 9 infallible accessors returning jdouble
// ─────────────────────────────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_silicon_SiliconSim_kBoltzmann(
    _env: *mut JNIEnv, _class: jclass,
) -> jdouble { dp::K_BOLTZMANN }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_silicon_SiliconSim_qElectron(
    _env: *mut JNIEnv, _class: jclass,
) -> jdouble { dp::Q_ELECTRON }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_silicon_SiliconSim_eps0(
    _env: *mut JNIEnv, _class: jclass,
) -> jdouble { dp::EPS0 }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_silicon_SiliconSim_epsSi(
    _env: *mut JNIEnv, _class: jclass,
) -> jdouble { dp::EPS_SI }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_silicon_SiliconSim_epsOx(
    _env: *mut JNIEnv, _class: jclass,
) -> jdouble { dp::EPS_OX }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_silicon_SiliconSim_niAt300K(
    _env: *mut JNIEnv, _class: jclass,
) -> jdouble { dp::N_I_300K }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_silicon_SiliconSim_egSiAt300K(
    _env: *mut JNIEnv, _class: jclass,
) -> jdouble { dp::EG_SI_300K }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_silicon_SiliconSim_muN300K(
    _env: *mut JNIEnv, _class: jclass,
) -> jdouble { dp::MU_N_300K }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_silicon_SiliconSim_muP300K(
    _env: *mut JNIEnv, _class: jclass,
) -> jdouble { dp::MU_P_300K }

// ─────────────────────────────────────────────────────────────────────────────
// device-physics: thermal voltage (infallible)
// ─────────────────────────────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_silicon_SiliconSim_thermalVoltage(
    _env: *mut JNIEnv, _class: jclass, t_kelvin: jdouble,
) -> jdouble {
    dp::thermal_voltage(t_kelvin)
}

// ─────────────────────────────────────────────────────────────────────────────
// device-physics: intrinsic concentration
// ─────────────────────────────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_silicon_SiliconSim_intrinsicConcentration(
    env: *mut JNIEnv, _class: jclass, t_kelvin: jdouble,
) -> jdouble {
    match dp::intrinsic_concentration(t_kelvin) {
        Ok(v) => v,
        Err(e) => { jni_throw_new(env, SILICON_EX, &e); 0.0 }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// device-physics: Fermi potential
// ─────────────────────────────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_silicon_SiliconSim_fermiPotential(
    env: *mut JNIEnv, _class: jclass,
    n_doping: jdouble, kind: jstring, t_kelvin: jdouble,
) -> jdouble {
    let kind_str = match jni_get_string_utf(env, kind) {
        Some(s) => s,
        None => { jni_throw_new(env, SILICON_EX, "kind must not be null"); return 0.0; }
    };
    match dp::fermi_potential(n_doping, &kind_str, t_kelvin) {
        Ok(v) => v,
        Err(e) => { jni_throw_new(env, SILICON_EX, &e); 0.0 }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// device-physics: PN junction
// ─────────────────────────────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_silicon_SiliconSim_pnJunctionBuiltInVoltage(
    env: *mut JNIEnv, _class: jclass, na: jdouble, nd: jdouble, t: jdouble,
) -> jdouble {
    match dp::PNJunction::new(na, nd, 1.0, t, 1e-6, 1e-6) {
        Ok(j) => j.built_in_voltage(),
        Err(e) => { jni_throw_new(env, SILICON_EX, &e); 0.0 }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_silicon_SiliconSim_pnJunctionDepletionWidth(
    env: *mut JNIEnv, _class: jclass,
    na: jdouble, nd: jdouble, t: jdouble, v_applied: jdouble,
) -> jdouble {
    match dp::PNJunction::new(na, nd, 1.0, t, 1e-6, 1e-6) {
        Ok(j) => j.depletion_width(v_applied),
        Err(e) => { jni_throw_new(env, SILICON_EX, &e); 0.0 }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_silicon_SiliconSim_pnJunctionSaturationCurrent(
    env: *mut JNIEnv, _class: jclass,
    na: jdouble, nd: jdouble, a: jdouble, t: jdouble,
    tau_n: jdouble, tau_p: jdouble,
) -> jdouble {
    match dp::PNJunction::new(na, nd, a, t, tau_n, tau_p) {
        Ok(j) => j.saturation_current(),
        Err(e) => { jni_throw_new(env, SILICON_EX, &e); 0.0 }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_silicon_SiliconSim_pnJunctionCurrent(
    env: *mut JNIEnv, _class: jclass,
    na: jdouble, nd: jdouble, a: jdouble, t: jdouble,
    tau_n: jdouble, tau_p: jdouble, v: jdouble,
) -> jdouble {
    match dp::PNJunction::new(na, nd, a, t, tau_n, tau_p) {
        Ok(j) => {
            let is = j.saturation_current();
            let vt = dp::thermal_voltage(t);
            is * ((v / vt).exp() - 1.0)
        }
        Err(e) => { jni_throw_new(env, SILICON_EX, &e); 0.0 }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// device-physics: MOSFET threshold voltage
// ─────────────────────────────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_silicon_SiliconSim_mosfetThresholdVoltage(
    env: *mut JNIEnv, _class: jclass,
    device_type: jstring,
    l: jdouble, w: jdouble, t_ox: jdouble, n_body: jdouble,
    phi_ms: jdouble, q_ox: jdouble, t: jdouble, v_sb: jdouble,
) -> jdouble {
    let dt = match jni_get_string_utf(env, device_type) {
        Some(s) => s,
        None => { jni_throw_new(env, SILICON_EX, "deviceType must not be null"); return 0.0; }
    };
    match dp::MOSFETParams::new(&dt, l, w, t_ox, n_body, phi_ms, q_ox, t) {
        Ok(p) => match p.threshold_voltage(v_sb) {
            Ok(v) => v,
            Err(e) => { jni_throw_new(env, SILICON_EX, &e); 0.0 }
        },
        Err(e) => { jni_throw_new(env, SILICON_EX, &e); 0.0 }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// mosfet-models: Level-1 evaluation
// ─────────────────────────────────────────────────────────────────────────────

#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
// Params filled field-by-field from the many JNI scalar args for readability;
// behavior is identical to an initializer.
#[allow(clippy::field_reassign_with_default)]
pub unsafe extern "C" fn Java_com_codingadventures_silicon_SiliconSim_evaluateLevel1(
    env: *mut JNIEnv, _class: jclass,
    vt0: jdouble, kp: jdouble, lambda: jdouble, gamma: jdouble, phi: jdouble,
    w: jdouble, l: jdouble, n_sub: jdouble,
    v_gs: jdouble, v_ds: jdouble, v_bs: jdouble, t: jdouble,
) -> jobject {
    let mut p = mm::Level1Params::default();
    p.vt0    = vt0;
    p.kp     = kp;
    p.lambda = lambda;
    p.gamma  = gamma;
    p.phi    = phi;
    p.w      = w;
    p.l      = l;
    p.n_sub  = n_sub;
    let r = mm::evaluate_level1(&p, v_gs, v_ds, v_bs, t);
    make_mos_result(env, &r)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_silicon_SiliconSim_evaluateLevel1Defaults(
    env: *mut JNIEnv, _class: jclass,
    v_gs: jdouble, v_ds: jdouble, v_bs: jdouble, t: jdouble,
) -> jobject {
    let p = mm::Level1Params::default();
    let r = mm::evaluate_level1(&p, v_gs, v_ds, v_bs, t);
    make_mos_result(env, &r)
}

// ─────────────────────────────────────────────────────────────────────────────
// fab-process-simulation: process steps
// ─────────────────────────────────────────────────────────────────────────────

/// Read the cross-section wire from a possibly-null jstring.
///
/// A null `jstring` is treated as an empty cross-section (`""`), consistent
/// with all other silicon bindings.  Returns `Err` and throws a
/// `SiliconException` if the wire string contains a malformed or injected
/// entry.
unsafe fn decode_cs(env: *mut JNIEnv, cs: jstring) -> Result<fps::CrossSection, ()> {
    let s = jni_get_string_utf(env, cs).unwrap_or_default();
    cs_from_wire(&s).map_err(|e| { jni_throw_new(env, SILICON_EX, &e); })
}

/// Encode a `CrossSection` as a Java string.  Returns null on allocation
/// failure (OOM pending).
unsafe fn encode_cs(env: *mut JNIEnv, cs: &fps::CrossSection) -> jstring {
    jni_new_string_utf(env, &cs_to_wire(cs))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_silicon_SiliconSim_deposit(
    env: *mut JNIEnv, _class: jclass,
    cs: jstring, material: jstring, thickness_nm: jdouble,
) -> jstring {
    let mut xsect = match decode_cs(env, cs) {
        Ok(c) => c, Err(()) => return null_mut(),
    };
    let mat = match jni_get_string_utf(env, material) {
        Some(s) => s,
        None => { jni_throw_new(env, SILICON_EX, "material must not be null"); return null_mut(); }
    };
    if let Err(e) = validate_name(&mat) {
        jni_throw_new(env, SILICON_EX, &e);
        return null_mut();
    }
    xsect = match fps::deposit(&xsect, &mat, thickness_nm) {
        Ok(c) => c,
        Err(e) => { jni_throw_new(env, SILICON_EX, &e); return null_mut(); }
    };
    encode_cs(env, &xsect)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_silicon_SiliconSim_etch(
    env: *mut JNIEnv, _class: jclass,
    cs: jstring, target: jstring, depth_nm: jdouble,
) -> jstring {
    let xsect = match decode_cs(env, cs) {
        Ok(c) => c, Err(()) => return null_mut(),
    };
    let tgt = match jni_get_string_utf(env, target) {
        Some(s) => s,
        None => { jni_throw_new(env, SILICON_EX, "target must not be null"); return null_mut(); }
    };
    if let Err(e) = validate_name(&tgt) {
        jni_throw_new(env, SILICON_EX, &e);
        return null_mut();
    }
    let result = fps::etch(&xsect, &tgt, depth_nm);
    encode_cs(env, &result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_silicon_SiliconSim_implant(
    env: *mut JNIEnv, _class: jclass,
    cs: jstring, species: jstring, energy_kev: jdouble, dose_cm2: jdouble,
) -> jstring {
    let xsect = match decode_cs(env, cs) {
        Ok(c) => c, Err(()) => return null_mut(),
    };
    let sp = match jni_get_string_utf(env, species) {
        Some(s) => s,
        None => { jni_throw_new(env, SILICON_EX, "species must not be null"); return null_mut(); }
    };
    if let Err(e) = validate_name(&sp) {
        jni_throw_new(env, SILICON_EX, &e);
        return null_mut();
    }
    let result = match fps::implant(&xsect, &sp, energy_kev, dose_cm2) {
        Ok(c) => c,
        Err(e) => { jni_throw_new(env, SILICON_EX, &e); return null_mut(); }
    };
    encode_cs(env, &result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_silicon_SiliconSim_diffuse(
    env: *mut JNIEnv, _class: jclass, cs: jstring, time_min: jdouble,
) -> jstring {
    let xsect = match decode_cs(env, cs) {
        Ok(c) => c, Err(()) => return null_mut(),
    };
    let result = fps::diffuse(&xsect, time_min, None);
    encode_cs(env, &result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_silicon_SiliconSim_diffuseWithTemp(
    env: *mut JNIEnv, _class: jclass,
    cs: jstring, time_min: jdouble, temp_c: jdouble,
) -> jstring {
    let xsect = match decode_cs(env, cs) {
        Ok(c) => c, Err(()) => return null_mut(),
    };
    let result = fps::diffuse(&xsect, time_min, Some(temp_c));
    encode_cs(env, &result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_silicon_SiliconSim_dealGroveOxidation(
    env: *mut JNIEnv, _class: jclass, cs: jstring, time_min: jdouble,
) -> jstring {
    let xsect = match decode_cs(env, cs) {
        Ok(c) => c, Err(()) => return null_mut(),
    };
    match fps::deal_grove_oxidation(&xsect, time_min, None, None) {
        Ok(result) => encode_cs(env, &result),
        Err(e) => { jni_throw_new(env, SILICON_EX, &e); null_mut() }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_silicon_SiliconSim_dealGroveOxidationCustom(
    env: *mut JNIEnv, _class: jclass,
    cs: jstring, time_min: jdouble, a_um: jdouble, b_um2_per_hr: jdouble,
) -> jstring {
    let xsect = match decode_cs(env, cs) {
        Ok(c) => c, Err(()) => return null_mut(),
    };
    match fps::deal_grove_oxidation(&xsect, time_min, Some(a_um), Some(b_um2_per_hr)) {
        Ok(result) => encode_cs(env, &result),
        Err(e) => { jni_throw_new(env, SILICON_EX, &e); null_mut() }
    }
}

/// Returns a `double[2]` = `[rp_nm, straggle_nm]`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_silicon_SiliconSim_implantRange(
    env: *mut JNIEnv, _class: jclass, species: jstring, energy_kev: jdouble,
) -> jarray {
    let sp = match jni_get_string_utf(env, species) {
        Some(s) => s,
        None => { jni_throw_new(env, SILICON_EX, "species must not be null"); return null_mut(); }
    };
    match fps::implant_range(&sp, energy_kev) {
        Ok((rp, straggle)) => {
            let arr = jni_new_double_array(env, 2);
            if arr.is_null() { return null_mut(); }
            let vals: [jdouble; 2] = [rp, straggle];
            jni_set_double_array_region(env, arr, 0, 2, vals.as_ptr());
            arr
        }
        Err(e) => { jni_throw_new(env, SILICON_EX, &e); null_mut() }
    }
}

/// Returns diffusivity in cm²/s for the given species at `temperature_c`.
/// Infallible: returns 0.0 for unknown species (same as the underlying impl).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_silicon_SiliconSim_diffusivityCm2PerS(
    _env: *mut JNIEnv, _class: jclass, species: jstring, temperature_c: jdouble,
) -> jdouble {
    // Unknown species → 0.0 without throwing.  The underlying function
    // returns 0.0 for unrecognised species.
    let sp = jni_get_string_utf(_env, species).unwrap_or_default();
    fps::diffusivity_cm2_per_s(&sp, temperature_c)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests — pure Rust helpers (no JVM required)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // -- Wire format round-trip -----------------------------------------------

    #[test]
    fn test_cs_round_trip_empty() {
        let cs = fps::CrossSection { layers: vec![] };
        let wire = cs_to_wire(&cs);
        assert_eq!(wire, "");
        let back = cs_from_wire(&wire).unwrap();
        assert_eq!(back.layers.len(), 0);
    }

    #[test]
    fn test_cs_round_trip_single() {
        let cs = fps::CrossSection {
            layers: vec![fps::Layer::new("Si", 500.0)],
        };
        let wire = cs_to_wire(&cs);
        // {:?} format preserves decimal point
        assert_eq!(wire, "Si:500.0");
        let back = cs_from_wire(&wire).unwrap();
        assert_eq!(back.layers[0].material, "Si");
        assert!((back.layers[0].thickness_nm - 500.0).abs() < 1e-9);
    }

    #[test]
    fn test_cs_round_trip_multi() {
        let wire = "Poly:50.0|SiO2:4.8|Si:500.0";
        let cs = cs_from_wire(wire).unwrap();
        assert_eq!(cs.layers.len(), 3);
        assert_eq!(cs.layers[0].material, "Poly");
        let back = cs_to_wire(&cs);
        assert_eq!(back, wire);
    }

    #[test]
    fn test_cs_round_trip_whole_number_thickness() {
        let cs = fps::CrossSection {
            layers: vec![fps::Layer::new("Si", 500.0)],
        };
        // Without {:?}, 500.0 would format as "500" and fail to round-trip.
        let wire = cs_to_wire(&cs);
        assert!(wire.contains("500.0"), "got {:?}", wire);
        let back = cs_from_wire(&wire).unwrap();
        assert!((back.layers[0].thickness_nm - 500.0).abs() < 1e-9);
    }

    // -- validate_name rejection ----------------------------------------------

    #[test]
    fn test_validate_name_rejects_pipe() {
        assert!(validate_name("Si|SiO2").is_err());
    }

    #[test]
    fn test_validate_name_rejects_colon() {
        assert!(validate_name("Si:500").is_err());
    }

    #[test]
    fn test_validate_name_accepts_normal() {
        assert!(validate_name("Si").is_ok());
        assert!(validate_name("SiO2").is_ok());
        assert!(validate_name("Poly").is_ok());
        assert!(validate_name("B").is_ok());
        assert!(validate_name("BF2").is_ok());
    }

    // -- cs_from_wire rejection of injected entries ---------------------------

    #[test]
    fn test_cs_from_wire_rejects_injection_via_material_name() {
        // An attacker tries to inject a second layer by embedding "|" in the
        // material name.  The entire wire string must be rejected.
        let result = cs_from_wire("Evil|Si:500.0");
        // The first entry "Evil" is fine, but "|Si:500.0" splits into "Si"
        // and "500.0", which is actually valid.  The real attack is embedding
        // ":" or "|" *in* the material part that validate_name catches.
        // Test the colon-in-material case:
        let result2 = cs_from_wire("Ma:tl:500.0|Si:100.0");
        assert!(result2.is_err(), "colon in material name should fail");
        let _ = result; // "Evil" is just a 2-layer wire, not injected
    }

    #[test]
    fn test_cs_from_wire_rejects_bad_thickness() {
        assert!(cs_from_wire("Si:not_a_number").is_err());
    }

    #[test]
    fn test_cs_from_wire_rejects_missing_thickness() {
        assert!(cs_from_wire("Si").is_err());
    }

    #[test]
    fn test_cs_from_wire_rejects_colon_in_material_name() {
        // A material name containing ":" would be ambiguous with the
        // material:thickness separator.  cs_from_wire uses splitn(2, ':')
        // so "S:i:500.0" produces material "S" and thickness "i:500.0",
        // which fails to parse as f64.
        assert!(cs_from_wire("S:i:500.0").is_err());
    }
}
