//! silicon-rust-ruby-native — Ruby native extension for the silicon sim stack
//! =========================================================================
//!
//! This crate compiles to a `.so` / `.bundle` / `.dll` that Ruby loads with
//! `require "silicon_rust_ruby_native"`.  After loading, Ruby code can call
//! any of 26 functions on the `SiliconRustRuby` module:
//!
//! ```ruby
//! require "coding_adventures/silicon_rust_ruby"
//!
//! # Physical constants
//! SiliconRustRuby.k_boltzmann      # => 1.380649e-23 (J/K)
//! SiliconRustRuby.thermal_voltage(300) # => 0.025852 (V)
//!
//! # PN junction
//! vbi = SiliconRustRuby.pn_junction_built_in_voltage(1e23, 1e22, 300)
//!
//! # MOSFET Level-1 operating point (default 130 nm NMOS params)
//! r = SiliconRustRuby.evaluate_level1_defaults(1.8, 1.8, 0.0, 300.15)
//! r[:region]  # => "saturation"
//! r[:id]      # => drain current [A]
//!
//! # Process simulation (cross-section wire format: "material:nm|material:nm")
//! cs = SiliconRustRuby.deposit("", "Si", 500.0)
//! cs = SiliconRustRuby.deal_grove_oxidation(cs, 5.0)
//! cs = SiliconRustRuby.deposit(cs, "Poly", 50.0)
//! # cs => "Poly:50.0|SiO2:...|Si:500.0"
//! ```
//!
//! ## Architecture
//!
//! ```text
//! silicon_rust_ruby (Ruby gem)
//!   ↓ require
//! silicon_rust_ruby_native.{so,bundle,dll}   ← THIS CRATE
//!   ↓ Rust function calls
//! device-physics   mosfet-models   fab-process-simulation
//! ```
//!
//! ## Safety strategy
//!
//! Ruby's C API is fundamentally unsafe: it uses `VALUE` (an opaque usize),
//! longjmps on exceptions, and requires holding the GVL for object allocation.
//! We minimise exposure by:
//!
//! * Doing all real work in pure Rust before touching Ruby (compute first,
//!   convert the result to Ruby values last).
//! * Letting `ruby-bridge` own every `unsafe` call to `libruby`.
//! * Catching errors as `Result<_, String>` in Rust and converting them to
//!   Ruby `RuntimeError` exceptions via `ruby_bridge::raise_runtime_error`.
//!
//! ## Wire format
//!
//! A `CrossSection` crosses the FFI boundary as a pipe-separated string of
//! `material:thickness_nm` pairs ordered top-to-bottom:
//!
//! ```text
//! ""                              # empty cross-section
//! "Si:500.0"                      # bare silicon substrate
//! "SiO2:4.8|Si:500.0"            # gate oxide on silicon
//! "Poly:50.0|SiO2:4.8|Si:500.0" # poly gate on gate oxide on silicon
//! ```
//!
//! The same format is used by `silicon-rust-python` and `silicon-rust-napi`.
//! Material names are validated by `validate_material_name` to reject `|` and
//! `:` which would corrupt the wire format.

use std::os::raw::{c_char, c_int, c_void};

use ruby_bridge::VALUE;

use device_physics as dp;
use fab_process_simulation as fps;
use mosfet_models as mm;

// ---------------------------------------------------------------------------
// Wire format helpers (pure Rust)
// ---------------------------------------------------------------------------

/// Reject material names containing `|` or `:` — wire format injection guard.
pub fn validate_material_name(material: &str) -> Result<(), String> {
    if material.contains('|') || material.contains(':') {
        return Err(format!(
            "material name must not contain '|' or ':'; got {:?}",
            material
        ));
    }
    Ok(())
}

/// Serialise a `CrossSection` to the pipe-separated wire format.
pub fn cs_to_wire(cs: &fps::CrossSection) -> String {
    cs.layers
        .iter()
        .map(|l| format!("{}:{:?}", l.material, l.thickness_nm))
        .collect::<Vec<_>>()
        .join("|")
}

/// Deserialise the wire format back into a `CrossSection`.
///
/// Silently skips any entry that does not parse as `material:f64`.
pub fn cs_from_wire(s: &str) -> fps::CrossSection {
    if s.is_empty() {
        return fps::CrossSection { layers: vec![] };
    }
    let layers = s
        .split('|')
        .filter_map(|entry| {
            let mut parts = entry.splitn(2, ':');
            let material = parts.next()?.to_string();
            let thickness_nm: f64 = parts.next()?.parse().ok()?;
            Some(fps::Layer::new(&material, thickness_nm))
        })
        .collect();
    fps::CrossSection { layers }
}

// ---------------------------------------------------------------------------
// Macros: extract Ruby argument as String or f64.
//
// All functions use argc=-1 convention:
//   extern "C" fn(argc: c_int, argv: *const VALUE, self: VALUE) -> VALUE
//
// argc=-1 lets every function share one signature, simplifying registration.
// argv[i] is the i-th Ruby argument.
// ---------------------------------------------------------------------------

macro_rules! req_str {
    ($argv:expr, $idx:expr, $fn_name:literal) => {{
        let raw = unsafe { *$argv.add($idx) };
        match ruby_bridge::str_from_rb(raw) {
            Some(s) => s,
            None => ruby_bridge::raise_runtime_error(concat!(
                $fn_name,
                ": argument must be a String"
            )),
        }
    }};
}

macro_rules! req_f64 {
    ($argv:expr, $idx:expr) => {{
        let raw = unsafe { *$argv.add($idx) };
        ruby_bridge::f64_from_rb(raw)
    }};
}

// ---------------------------------------------------------------------------
// Ruby Symbol helper.
//
// `:name` in Ruby is an interned symbol VALUE.  The simplest portable way
// to obtain one without binding `rb_id2sym` (which may not be exported in
// all Ruby versions) is to evaluate the symbol literal once via
// `rb_eval_string`.
//
// We use 'static byte string literals so the pointer is always valid.
// ---------------------------------------------------------------------------

macro_rules! rb_sym {
    ($name:literal) => {
        unsafe {
            ruby_bridge::rb_eval_string(
                concat!(":", $name, "\0").as_bytes().as_ptr() as *const c_char,
            )
        }
    };
}

// ---------------------------------------------------------------------------
// Helper: convert a MosResult to a Ruby Hash with symbol keys.
//
// Returns a Ruby Hash with these keys (all symbols):
//   :id, :gm, :gds, :gmb, :cgs, :cgd, :cgb, :cbs, :cbd, :region
// ---------------------------------------------------------------------------

fn mos_result_to_ruby_hash(r: &mm::MosResult) -> VALUE {
    let h = ruby_bridge::hash_new();
    ruby_bridge::hash_aset(h, rb_sym!("id"),     ruby_bridge::f64_to_rb(r.id));
    ruby_bridge::hash_aset(h, rb_sym!("gm"),     ruby_bridge::f64_to_rb(r.gm));
    ruby_bridge::hash_aset(h, rb_sym!("gds"),    ruby_bridge::f64_to_rb(r.gds));
    ruby_bridge::hash_aset(h, rb_sym!("gmb"),    ruby_bridge::f64_to_rb(r.gmb));
    ruby_bridge::hash_aset(h, rb_sym!("cgs"),    ruby_bridge::f64_to_rb(r.cgs));
    ruby_bridge::hash_aset(h, rb_sym!("cgd"),    ruby_bridge::f64_to_rb(r.cgd));
    ruby_bridge::hash_aset(h, rb_sym!("cgb"),    ruby_bridge::f64_to_rb(r.cgb));
    ruby_bridge::hash_aset(h, rb_sym!("cbs"),    ruby_bridge::f64_to_rb(r.cbs));
    ruby_bridge::hash_aset(h, rb_sym!("cbd"),    ruby_bridge::f64_to_rb(r.cbd));
    ruby_bridge::hash_aset(
        h,
        rb_sym!("region"),
        ruby_bridge::str_to_rb(r.region.as_str()),
    );
    h
}

// ---------------------------------------------------------------------------
// Physical constants (9) — no arguments, argc=0
// ---------------------------------------------------------------------------
//
// Ruby calling convention for argc=0 singleton methods:
//   extern "C" fn(_self: VALUE) -> VALUE

extern "C" fn rb_k_boltzmann(_self: VALUE) -> VALUE   { ruby_bridge::f64_to_rb(dp::K_BOLTZMANN) }
extern "C" fn rb_q_electron(_self: VALUE) -> VALUE    { ruby_bridge::f64_to_rb(dp::Q_ELECTRON) }
extern "C" fn rb_eps0(_self: VALUE) -> VALUE          { ruby_bridge::f64_to_rb(dp::EPS0) }
extern "C" fn rb_eps_si(_self: VALUE) -> VALUE        { ruby_bridge::f64_to_rb(dp::EPS_SI) }
extern "C" fn rb_eps_ox(_self: VALUE) -> VALUE        { ruby_bridge::f64_to_rb(dp::EPS_OX) }
extern "C" fn rb_ni_at_300k(_self: VALUE) -> VALUE    { ruby_bridge::f64_to_rb(dp::N_I_300K) }
extern "C" fn rb_eg_si_at_300k(_self: VALUE) -> VALUE { ruby_bridge::f64_to_rb(dp::EG_SI_300K) }
extern "C" fn rb_mu_n_300k(_self: VALUE) -> VALUE     { ruby_bridge::f64_to_rb(dp::MU_N_300K) }
extern "C" fn rb_mu_p_300k(_self: VALUE) -> VALUE     { ruby_bridge::f64_to_rb(dp::MU_P_300K) }

// ---------------------------------------------------------------------------
// device-physics — variadic convention (argc=-1)
// ---------------------------------------------------------------------------
//
// Ruby calling convention for argc=-1:
//   extern "C" fn(argc: c_int, argv: *const VALUE, self: VALUE) -> VALUE

extern "C" fn rb_thermal_voltage(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 {
        ruby_bridge::raise_runtime_error("thermal_voltage(t_kelvin): expected 1 argument");
    }
    ruby_bridge::f64_to_rb(dp::thermal_voltage(req_f64!(argv, 0)))
}

extern "C" fn rb_intrinsic_concentration(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 {
        ruby_bridge::raise_runtime_error("intrinsic_concentration(t_kelvin): expected 1 argument");
    }
    match dp::intrinsic_concentration(req_f64!(argv, 0)) {
        Ok(v)    => ruby_bridge::f64_to_rb(v),
        Err(msg) => ruby_bridge::raise_runtime_error(&format!("intrinsic_concentration: {msg}")),
    }
}

// fermi_potential(n_doping, kind, t_kelvin) — kind is "p" or "n".
extern "C" fn rb_fermi_potential(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 3 {
        ruby_bridge::raise_runtime_error(
            "fermi_potential(n_doping, kind, t_kelvin): expected 3 arguments",
        );
    }
    let n    = req_f64!(argv, 0);
    let kind = req_str!(argv, 1, "fermi_potential");
    let t    = req_f64!(argv, 2);
    match dp::fermi_potential(n, &kind, t) {
        Ok(v)    => ruby_bridge::f64_to_rb(v),
        Err(msg) => ruby_bridge::raise_runtime_error(&format!("fermi_potential: {msg}")),
    }
}

// pn_junction_built_in_voltage(na, nd, t) — area and lifetimes fixed at 1/1e-6.
// (matching the NAPI binding: pn_built_in_voltage depends only on na, nd, t)
extern "C" fn rb_pn_junction_built_in_voltage(
    argc: c_int,
    argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    if argc != 3 {
        ruby_bridge::raise_runtime_error(
            "pn_junction_built_in_voltage(na, nd, t): expected 3 arguments",
        );
    }
    let na = req_f64!(argv, 0);
    let nd = req_f64!(argv, 1);
    let t  = req_f64!(argv, 2);
    match dp::PNJunction::new(na, nd, 1.0, t, 1e-6, 1e-6) {
        Ok(j)    => ruby_bridge::f64_to_rb(j.built_in_voltage()),
        Err(msg) => ruby_bridge::raise_runtime_error(&format!("pn_junction_built_in_voltage: {msg}")),
    }
}

// pn_junction_depletion_width(na, nd, t, v_applied)
extern "C" fn rb_pn_junction_depletion_width(
    argc: c_int,
    argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    if argc != 4 {
        ruby_bridge::raise_runtime_error(
            "pn_junction_depletion_width(na, nd, t, v_applied): expected 4 arguments",
        );
    }
    let na        = req_f64!(argv, 0);
    let nd        = req_f64!(argv, 1);
    let t         = req_f64!(argv, 2);
    let v_applied = req_f64!(argv, 3);
    match dp::PNJunction::new(na, nd, 1.0, t, 1e-6, 1e-6) {
        Ok(j)    => ruby_bridge::f64_to_rb(j.depletion_width(v_applied)),
        Err(msg) => ruby_bridge::raise_runtime_error(&format!("pn_junction_depletion_width: {msg}")),
    }
}

// pn_junction_saturation_current(na, nd, a, t, tau_n, tau_p)
extern "C" fn rb_pn_junction_saturation_current(
    argc: c_int,
    argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    if argc != 6 {
        ruby_bridge::raise_runtime_error(
            "pn_junction_saturation_current(na, nd, a, t, tau_n, tau_p): expected 6 arguments",
        );
    }
    let na    = req_f64!(argv, 0);
    let nd    = req_f64!(argv, 1);
    let a     = req_f64!(argv, 2);
    let t     = req_f64!(argv, 3);
    let tau_n = req_f64!(argv, 4);
    let tau_p = req_f64!(argv, 5);
    match dp::PNJunction::new(na, nd, a, t, tau_n, tau_p) {
        Ok(j)    => ruby_bridge::f64_to_rb(j.saturation_current()),
        Err(msg) => ruby_bridge::raise_runtime_error(&format!("pn_junction_saturation_current: {msg}")),
    }
}

// pn_junction_current(na, nd, a, t, tau_n, tau_p, v)
extern "C" fn rb_pn_junction_current(
    argc: c_int,
    argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    if argc != 7 {
        ruby_bridge::raise_runtime_error(
            "pn_junction_current(na, nd, a, t, tau_n, tau_p, v): expected 7 arguments",
        );
    }
    let na    = req_f64!(argv, 0);
    let nd    = req_f64!(argv, 1);
    let a     = req_f64!(argv, 2);
    let t     = req_f64!(argv, 3);
    let tau_n = req_f64!(argv, 4);
    let tau_p = req_f64!(argv, 5);
    let v     = req_f64!(argv, 6);
    match dp::PNJunction::new(na, nd, a, t, tau_n, tau_p) {
        Ok(j)    => ruby_bridge::f64_to_rb(j.current(v)),
        Err(msg) => ruby_bridge::raise_runtime_error(&format!("pn_junction_current: {msg}")),
    }
}

// mosfet_threshold_voltage(device_type, l, w, t_ox, n_body, phi_ms, q_ox, t, v_sb)
// device_type is "NMOS" or "PMOS"
extern "C" fn rb_mosfet_threshold_voltage(
    argc: c_int,
    argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    if argc != 9 {
        ruby_bridge::raise_runtime_error(
            "mosfet_threshold_voltage(device_type, l, w, t_ox, n_body, phi_ms, q_ox, t, v_sb): expected 9 arguments",
        );
    }
    let device_type = req_str!(argv, 0, "mosfet_threshold_voltage");
    let l      = req_f64!(argv, 1);
    let w      = req_f64!(argv, 2);
    let t_ox   = req_f64!(argv, 3);
    let n_body = req_f64!(argv, 4);
    let phi_ms = req_f64!(argv, 5);
    let q_ox   = req_f64!(argv, 6);
    let t      = req_f64!(argv, 7);
    let v_sb   = req_f64!(argv, 8);
    match dp::MOSFETParams::new(&device_type, l, w, t_ox, n_body, phi_ms, q_ox, t) {
        Ok(p) => match p.threshold_voltage(v_sb) {
            Ok(vt)   => ruby_bridge::f64_to_rb(vt),
            Err(msg) => ruby_bridge::raise_runtime_error(&format!("mosfet_threshold_voltage: {msg}")),
        },
        Err(msg) => ruby_bridge::raise_runtime_error(&format!("mosfet_threshold_voltage: {msg}")),
    }
}

// ---------------------------------------------------------------------------
// mosfet-models
// ---------------------------------------------------------------------------

// evaluate_level1(vt0, kp, lambda, gamma, phi, w, l, n_sub, v_gs, v_ds, v_bs, t)
//
// Uses Level1Params::default() for the 8 non-bias fields not exposed here
// (is, t_nom, cgso, cgdo, cgbo, cbs, cbd, subthreshold_enable), then
// overrides the 8 physically-meaningful parameters that the caller supplies.
extern "C" fn rb_evaluate_level1(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 12 {
        ruby_bridge::raise_runtime_error(
            "evaluate_level1(vt0,kp,lambda,gamma,phi,w,l,n_sub,v_gs,v_ds,v_bs,t): expected 12 arguments",
        );
    }
    let vt0    = req_f64!(argv,  0);
    let kp     = req_f64!(argv,  1);
    let lambda = req_f64!(argv,  2);
    let gamma  = req_f64!(argv,  3);
    let phi    = req_f64!(argv,  4);
    let w      = req_f64!(argv,  5);
    let l      = req_f64!(argv,  6);
    let n_sub  = req_f64!(argv,  7);
    let v_gs   = req_f64!(argv,  8);
    let v_ds   = req_f64!(argv,  9);
    let v_bs   = req_f64!(argv, 10);
    let t      = req_f64!(argv, 11);

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

    mos_result_to_ruby_hash(&mm::evaluate_level1(&p, v_gs, v_ds, v_bs, t))
}

// evaluate_level1_defaults(v_gs, v_ds, v_bs, t)
// Uses all Level1Params::default() fields (130 nm NMOS device).
extern "C" fn rb_evaluate_level1_defaults(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 4 {
        ruby_bridge::raise_runtime_error(
            "evaluate_level1_defaults(v_gs, v_ds, v_bs, t): expected 4 arguments",
        );
    }
    let v_gs = req_f64!(argv, 0);
    let v_ds = req_f64!(argv, 1);
    let v_bs = req_f64!(argv, 2);
    let t    = req_f64!(argv, 3);
    mos_result_to_ruby_hash(&mm::evaluate_level1(&mm::Level1Params::default(), v_gs, v_ds, v_bs, t))
}

// ---------------------------------------------------------------------------
// fab-process-simulation
// ---------------------------------------------------------------------------

// deal_grove_oxidation(cs_str, time_min [, a_um, b_um2_per_hr])
// argc ∈ {2, 4}
extern "C" fn rb_deal_grove_oxidation(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 && argc != 4 {
        ruby_bridge::raise_runtime_error(
            "deal_grove_oxidation(cs_str, time_min [, a_um, b_um2_per_hr]): expected 2 or 4 arguments",
        );
    }
    let cs_str   = req_str!(argv, 0, "deal_grove_oxidation");
    let time_min = req_f64!(argv, 1);
    let a_um     = if argc == 4 { Some(req_f64!(argv, 2)) } else { None };
    let b        = if argc == 4 { Some(req_f64!(argv, 3)) } else { None };
    let cs = cs_from_wire(&cs_str);
    match fps::deal_grove_oxidation(&cs, time_min, a_um, b) {
        Ok(new_cs) => ruby_bridge::str_to_rb(&cs_to_wire(&new_cs)),
        Err(msg)   => ruby_bridge::raise_runtime_error(&format!("deal_grove_oxidation: {msg}")),
    }
}

// deposit(cs_str, material, thickness_nm)
extern "C" fn rb_deposit(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 3 {
        ruby_bridge::raise_runtime_error("deposit(cs_str, material, thickness_nm): expected 3 arguments");
    }
    let cs_str       = req_str!(argv, 0, "deposit");
    let material     = req_str!(argv, 1, "deposit");
    let thickness_nm = req_f64!(argv, 2);
    if let Err(msg) = validate_material_name(&material) {
        ruby_bridge::raise_runtime_error(&format!("deposit: {msg}"));
    }
    let cs = cs_from_wire(&cs_str);
    match fps::deposit(&cs, &material, thickness_nm) {
        Ok(new_cs) => ruby_bridge::str_to_rb(&cs_to_wire(&new_cs)),
        Err(msg)   => ruby_bridge::raise_runtime_error(&format!("deposit: {msg}")),
    }
}

// etch(cs_str, target_material, depth_nm)
extern "C" fn rb_etch(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 3 {
        ruby_bridge::raise_runtime_error("etch(cs_str, target_material, depth_nm): expected 3 arguments");
    }
    let cs_str          = req_str!(argv, 0, "etch");
    let target_material = req_str!(argv, 1, "etch");
    let depth_nm        = req_f64!(argv, 2);
    if let Err(msg) = validate_material_name(&target_material) {
        ruby_bridge::raise_runtime_error(&format!("etch: {msg}"));
    }
    ruby_bridge::str_to_rb(&cs_to_wire(&fps::etch(&cs_from_wire(&cs_str), &target_material, depth_nm)))
}

// implant(cs_str, species, energy_kev, dose_cm2)
extern "C" fn rb_implant(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 4 {
        ruby_bridge::raise_runtime_error("implant(cs_str, species, energy_kev, dose_cm2): expected 4 arguments");
    }
    let cs_str     = req_str!(argv, 0, "implant");
    let species    = req_str!(argv, 1, "implant");
    let energy_kev = req_f64!(argv, 2);
    let dose_cm2   = req_f64!(argv, 3);
    if let Err(msg) = validate_material_name(&species) {
        ruby_bridge::raise_runtime_error(&format!("implant: {msg}"));
    }
    match fps::implant(&cs_from_wire(&cs_str), &species, energy_kev, dose_cm2) {
        Ok(new_cs) => ruby_bridge::str_to_rb(&cs_to_wire(&new_cs)),
        Err(msg)   => ruby_bridge::raise_runtime_error(&format!("implant: {msg}")),
    }
}

// diffuse(cs_str, time_min [, temperature_c])
// argc ∈ {2, 3}
extern "C" fn rb_diffuse(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 && argc != 3 {
        ruby_bridge::raise_runtime_error(
            "diffuse(cs_str, time_min [, temperature_c]): expected 2 or 3 arguments",
        );
    }
    let cs_str   = req_str!(argv, 0, "diffuse");
    let time_min = req_f64!(argv, 1);
    let temp_c   = if argc == 3 { Some(req_f64!(argv, 2)) } else { None };
    ruby_bridge::str_to_rb(&cs_to_wire(&fps::diffuse(&cs_from_wire(&cs_str), time_min, temp_c)))
}

// implant_range(species, energy_kev) → Hash { rp: Float, straggle: Float }
extern "C" fn rb_implant_range(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 {
        ruby_bridge::raise_runtime_error("implant_range(species, energy_kev): expected 2 arguments");
    }
    let species    = req_str!(argv, 0, "implant_range");
    let energy_kev = req_f64!(argv, 1);
    match fps::implant_range(&species, energy_kev) {
        Ok((rp, straggle)) => {
            let h = ruby_bridge::hash_new();
            ruby_bridge::hash_aset(h, rb_sym!("rp"),       ruby_bridge::f64_to_rb(rp));
            ruby_bridge::hash_aset(h, rb_sym!("straggle"), ruby_bridge::f64_to_rb(straggle));
            h
        }
        Err(msg) => ruby_bridge::raise_runtime_error(&format!("implant_range: {msg}")),
    }
}

// diffusivity_cm2_per_s(species, temperature_c) → Float
extern "C" fn rb_diffusivity_cm2_per_s(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 {
        ruby_bridge::raise_runtime_error("diffusivity_cm2_per_s(species, temperature_c): expected 2 arguments");
    }
    let species       = req_str!(argv, 0, "diffusivity_cm2_per_s");
    let temperature_c = req_f64!(argv, 1);
    if let Err(msg) = validate_material_name(&species) {
        ruby_bridge::raise_runtime_error(&format!("diffusivity_cm2_per_s: {msg}"));
    }
    ruby_bridge::f64_to_rb(fps::diffusivity_cm2_per_s(&species, temperature_c))
}

// ---------------------------------------------------------------------------
// Init_silicon_rust_ruby_native — Ruby's dlopen() entry point
// ---------------------------------------------------------------------------
//
// Ruby looks for `Init_<basename>` in the loaded .so.  Our gem loads
// `silicon_rust_ruby_native.{so,bundle,dll}`, so this function must be named
// exactly `Init_silicon_rust_ruby_native`.
//
// We define all 26 functions as module functions on `SiliconRustRuby`.
// A module function is callable both as `SiliconRustRuby.thermal_voltage(300)`
// and as a private instance method when the module is `include`d.
//
// argc = 0:  extern "C" fn(_self: VALUE) -> VALUE
// argc = -1: extern "C" fn(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE

#[no_mangle]
pub extern "C" fn Init_silicon_rust_ruby_native() {
    let m = ruby_bridge::define_module("SiliconRustRuby");

    // ── Physical constants (argc=0) ──────────────────────────────────────────
    ruby_bridge::define_module_function_raw(m, "k_boltzmann",   rb_k_boltzmann   as *const c_void, 0);
    ruby_bridge::define_module_function_raw(m, "q_electron",    rb_q_electron    as *const c_void, 0);
    ruby_bridge::define_module_function_raw(m, "eps0",          rb_eps0          as *const c_void, 0);
    ruby_bridge::define_module_function_raw(m, "eps_si",        rb_eps_si        as *const c_void, 0);
    ruby_bridge::define_module_function_raw(m, "eps_ox",        rb_eps_ox        as *const c_void, 0);
    ruby_bridge::define_module_function_raw(m, "ni_at_300k",    rb_ni_at_300k    as *const c_void, 0);
    ruby_bridge::define_module_function_raw(m, "eg_si_at_300k", rb_eg_si_at_300k as *const c_void, 0);
    ruby_bridge::define_module_function_raw(m, "mu_n_300k",     rb_mu_n_300k     as *const c_void, 0);
    ruby_bridge::define_module_function_raw(m, "mu_p_300k",     rb_mu_p_300k     as *const c_void, 0);

    // ── device-physics (argc=-1) ─────────────────────────────────────────────
    ruby_bridge::define_module_function_raw(m, "thermal_voltage",                rb_thermal_voltage               as *const c_void, -1);
    ruby_bridge::define_module_function_raw(m, "intrinsic_concentration",        rb_intrinsic_concentration       as *const c_void, -1);
    ruby_bridge::define_module_function_raw(m, "fermi_potential",                rb_fermi_potential               as *const c_void, -1);
    ruby_bridge::define_module_function_raw(m, "pn_junction_built_in_voltage",   rb_pn_junction_built_in_voltage  as *const c_void, -1);
    ruby_bridge::define_module_function_raw(m, "pn_junction_depletion_width",    rb_pn_junction_depletion_width   as *const c_void, -1);
    ruby_bridge::define_module_function_raw(m, "pn_junction_saturation_current", rb_pn_junction_saturation_current as *const c_void, -1);
    ruby_bridge::define_module_function_raw(m, "pn_junction_current",            rb_pn_junction_current           as *const c_void, -1);
    ruby_bridge::define_module_function_raw(m, "mosfet_threshold_voltage",       rb_mosfet_threshold_voltage      as *const c_void, -1);

    // ── mosfet-models (argc=-1) ──────────────────────────────────────────────
    ruby_bridge::define_module_function_raw(m, "evaluate_level1",          rb_evaluate_level1          as *const c_void, -1);
    ruby_bridge::define_module_function_raw(m, "evaluate_level1_defaults", rb_evaluate_level1_defaults as *const c_void, -1);

    // ── fab-process-simulation (argc=-1) ────────────────────────────────────
    ruby_bridge::define_module_function_raw(m, "deal_grove_oxidation",  rb_deal_grove_oxidation  as *const c_void, -1);
    ruby_bridge::define_module_function_raw(m, "deposit",               rb_deposit               as *const c_void, -1);
    ruby_bridge::define_module_function_raw(m, "etch",                  rb_etch                  as *const c_void, -1);
    ruby_bridge::define_module_function_raw(m, "implant",               rb_implant               as *const c_void, -1);
    ruby_bridge::define_module_function_raw(m, "diffuse",               rb_diffuse               as *const c_void, -1);
    ruby_bridge::define_module_function_raw(m, "implant_range",         rb_implant_range         as *const c_void, -1);
    ruby_bridge::define_module_function_raw(m, "diffusivity_cm2_per_s", rb_diffusivity_cm2_per_s as *const c_void, -1);
}
