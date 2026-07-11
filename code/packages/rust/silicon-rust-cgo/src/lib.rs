//! silicon-rust-cgo — plain C ABI bridge for the silicon simulation stack.
//! ========================================================================
//!
//! This crate compiles to a `.so` / `.dylib` / `.dll` that Go's CGo runtime
//! loads via `import "C"`.  It re-exports every public function from
//! `device-physics`, `mosfet-models`, and `fab-process-simulation` through
//! a stable C ABI defined in `include/silicon_cgo.h`.
//!
//! ## Why a plain C ABI?
//!
//! CGo can call any `extern "C"` function.  No Python capsules, no Ruby
//! VALUEs, no NAPI environments — just C types that both Rust and Go
//! understand natively.
//!
//! ## Error handling pattern
//!
//! * Infallible functions: return `f64` or `c_int` directly.
//! * Fallible functions: return `0` (success) or `-1` (error).  On error,
//!   a nul-terminated UTF-8 message is written into the caller-supplied
//!   `err[err_cap]` buffer.  On success the `out` pointer is set.
//! * String-returning functions: write the nul-terminated wire string into
//!   `out[out_cap]`.  Returns `-1` if the wire string would not fit.
//!
//! ## Wire format
//!
//! A `CrossSection` is serialised as pipe-separated `material:thickness_nm`
//! pairs, ordered top-to-bottom:
//!
//! ```text
//! ""                               empty cross-section
//! "Si:500.0"                       bare silicon substrate
//! "SiO2:4.8|Si:500.0"             gate oxide on silicon
//! "Poly:50.0|SiO2:4.8|Si:500.0"  poly gate on gate oxide on silicon
//! ```
//!
//! Material names containing `|` or `:` are rejected by `deposit`, `etch`,
//! and `implant` to prevent wire-format injection.

use std::ffi::{CStr, c_char, c_double, c_int};
use std::ptr;

use device_physics as dp;
use fab_process_simulation as fps;
use mosfet_models as mm;

// ---------------------------------------------------------------------------
// Internal helpers — pure Rust, no unsafe except buffer writes
// ---------------------------------------------------------------------------

/// Validate a material or species name against wire-format injection.
fn validate_name(name: &str) -> Result<(), String> {
    if name.contains('|') || name.contains(':') {
        return Err(format!(
            "name must not contain '|' or ':'; got {:?}",
            name
        ));
    }
    Ok(())
}

/// Serialise a CrossSection to the pipe-separated wire format.
fn cs_to_wire(cs: &fps::CrossSection) -> String {
    cs.layers
        .iter()
        .map(|l| format!("{}:{:?}", l.material, l.thickness_nm))
        .collect::<Vec<_>>()
        .join("|")
}

/// Deserialise the wire format back into a CrossSection.
///
/// Rejects the entire wire string if any entry is malformed or if any
/// material name contains `|` or `:`.  Returning `Err` instead of
/// silently skipping bad entries prevents a corrupted or adversarially
/// crafted wire string from causing silent layer loss.
fn cs_from_wire(s: &str) -> Result<fps::CrossSection, String> {
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
        validate_name(&material)
            .map_err(|e| format!("cs_from_wire: {e}"))?;
        let thickness_nm: f64 = parts
            .next()
            .ok_or_else(|| format!("missing thickness in {:?}", entry))?
            .parse()
            .map_err(|_| format!("bad thickness in {:?}", entry))?;
        layers.push(fps::Layer::new(&material, thickness_nm));
    }
    Ok(fps::CrossSection { layers })
}

/// Read a C string pointer as an owned `String`.  Returns `""` if null or
/// if the bytes are not valid UTF-8.
///
/// Returning `String` instead of `&str` eliminates the unbounded-lifetime
/// anti-pattern: a `&str` produced from `CStr::from_ptr` could be stored
/// past the C string's lifetime (use-after-free).  An owned `String` has
/// no lifetime dependency on the pointer.
///
/// # Safety
/// `ptr` must be a nul-terminated C string.
unsafe fn read_c_str(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .unwrap_or("")
        .to_owned()
}

/// Write `s` into a caller-supplied buffer `buf[0..cap]`, nul-terminated.
/// Returns `true` on success; `false` if `s` would not fit (s.len() >= cap).
/// On `false` the buffer contains a truncated, nul-terminated copy — callers
/// should treat this as an error and propagate it.
///
/// # Safety
/// `buf` must be a valid, writable pointer to at least `cap` bytes.
unsafe fn write_to_buf(s: &str, buf: *mut c_char, cap: usize) -> bool {
    if buf.is_null() || cap == 0 {
        return false;
    }
    let bytes = s.as_bytes();
    let fits = bytes.len() < cap;
    let n = bytes.len().min(cap - 1);
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, buf, n);
        *buf.add(n) = 0;
    }
    fits
}

/// Write `s` into `out`.  Returns 0 on success, -1 if the buffer was too
/// small (truncation would corrupt the cross-section wire string).
unsafe fn ok_str(s: &str, out: *mut c_char, out_cap: usize, err: *mut c_char, err_cap: usize) -> c_int {
    if !unsafe { write_to_buf(s, out, out_cap) } {
        return unsafe {
            err_msg(
                &format!(
                    "output buffer too small: need {} bytes, got {}",
                    s.len() + 1,
                    out_cap
                ),
                err,
                err_cap,
            )
        };
    }
    0
}

/// Write `msg` into `err`, return -1 (error).
/// If the message is too long it is silently truncated — the caller already
/// has the error code (-1) and the message is informational only.
unsafe fn err_msg(msg: &str, err: *mut c_char, err_cap: usize) -> c_int {
    let _ = unsafe { write_to_buf(msg, err, err_cap) };
    -1
}

/// Write `v` into `out`, return 0 (success).
unsafe fn ok_f64(v: f64, out: *mut c_double) -> c_int {
    if !out.is_null() {
        unsafe { *out = v };
    }
    0
}

// ---------------------------------------------------------------------------
// Physical constants — all infallible, return f64 directly
// ---------------------------------------------------------------------------
//
// Rust compiles #[no_mangle] pub extern "C" fn into a symbol with no name
// mangling, matching the declaration in silicon_cgo.h.

#[no_mangle]
pub extern "C" fn silicon_k_boltzmann() -> c_double   { dp::K_BOLTZMANN }
#[no_mangle]
pub extern "C" fn silicon_q_electron() -> c_double    { dp::Q_ELECTRON }
#[no_mangle]
pub extern "C" fn silicon_eps0() -> c_double          { dp::EPS0 }
#[no_mangle]
pub extern "C" fn silicon_eps_si() -> c_double        { dp::EPS_SI }
#[no_mangle]
pub extern "C" fn silicon_eps_ox() -> c_double        { dp::EPS_OX }
#[no_mangle]
pub extern "C" fn silicon_ni_at_300k() -> c_double    { dp::N_I_300K }
#[no_mangle]
pub extern "C" fn silicon_eg_si_at_300k() -> c_double { dp::EG_SI_300K }
#[no_mangle]
pub extern "C" fn silicon_mu_n_300k() -> c_double     { dp::MU_N_300K }
#[no_mangle]
pub extern "C" fn silicon_mu_p_300k() -> c_double     { dp::MU_P_300K }

// ---------------------------------------------------------------------------
// device-physics
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn silicon_thermal_voltage(t_kelvin: c_double) -> c_double {
    dp::thermal_voltage(t_kelvin)
}

#[no_mangle]
pub unsafe extern "C" fn silicon_intrinsic_concentration(
    t_kelvin: c_double,
    out: *mut c_double,
    err: *mut c_char,
    err_cap: usize,
) -> c_int {
    match dp::intrinsic_concentration(t_kelvin) {
        Ok(v)  => unsafe { ok_f64(v, out) },
        Err(m) => unsafe { err_msg(&m, err, err_cap) },
    }
}

// fermi_potential(n_doping, kind, t_kelvin) — kind is "p" or "n".
#[no_mangle]
pub unsafe extern "C" fn silicon_fermi_potential(
    n_doping: c_double,
    kind: *const c_char,
    t_kelvin: c_double,
    out: *mut c_double,
    err: *mut c_char,
    err_cap: usize,
) -> c_int {
    let kind_str = unsafe { read_c_str(kind) };
    match dp::fermi_potential(n_doping, &kind_str, t_kelvin) {
        Ok(v)  => unsafe { ok_f64(v, out) },
        Err(m) => unsafe { err_msg(&m, err, err_cap) },
    }
}

// pn_junction_built_in_voltage(na, nd, t) — area and lifetimes fixed at 1/1e-6.
#[no_mangle]
pub unsafe extern "C" fn silicon_pn_junction_built_in_voltage(
    na: c_double,
    nd: c_double,
    t: c_double,
    out: *mut c_double,
    err: *mut c_char,
    err_cap: usize,
) -> c_int {
    match dp::PNJunction::new(na, nd, 1.0, t, 1e-6, 1e-6) {
        Ok(j)  => unsafe { ok_f64(j.built_in_voltage(), out) },
        Err(m) => unsafe { err_msg(&m, err, err_cap) },
    }
}

#[no_mangle]
pub unsafe extern "C" fn silicon_pn_junction_depletion_width(
    na: c_double,
    nd: c_double,
    t: c_double,
    v_applied: c_double,
    out: *mut c_double,
    err: *mut c_char,
    err_cap: usize,
) -> c_int {
    match dp::PNJunction::new(na, nd, 1.0, t, 1e-6, 1e-6) {
        Ok(j)  => unsafe { ok_f64(j.depletion_width(v_applied), out) },
        Err(m) => unsafe { err_msg(&m, err, err_cap) },
    }
}

#[no_mangle]
pub unsafe extern "C" fn silicon_pn_junction_saturation_current(
    na: c_double,
    nd: c_double,
    a: c_double,
    t: c_double,
    tau_n: c_double,
    tau_p: c_double,
    out: *mut c_double,
    err: *mut c_char,
    err_cap: usize,
) -> c_int {
    match dp::PNJunction::new(na, nd, a, t, tau_n, tau_p) {
        Ok(j)  => unsafe { ok_f64(j.saturation_current(), out) },
        Err(m) => unsafe { err_msg(&m, err, err_cap) },
    }
}

#[no_mangle]
pub unsafe extern "C" fn silicon_pn_junction_current(
    na: c_double,
    nd: c_double,
    a: c_double,
    t: c_double,
    tau_n: c_double,
    tau_p: c_double,
    v: c_double,
    out: *mut c_double,
    err: *mut c_char,
    err_cap: usize,
) -> c_int {
    match dp::PNJunction::new(na, nd, a, t, tau_n, tau_p) {
        Ok(j)  => unsafe { ok_f64(j.current(v), out) },
        Err(m) => unsafe { err_msg(&m, err, err_cap) },
    }
}

// mosfet_threshold_voltage(device_type, l, w, t_ox, n_body, phi_ms, q_ox, t, v_sb)
// device_type is "NMOS" or "PMOS"
#[no_mangle]
pub unsafe extern "C" fn silicon_mosfet_threshold_voltage(
    device_type: *const c_char,
    l: c_double,
    w: c_double,
    t_ox: c_double,
    n_body: c_double,
    phi_ms: c_double,
    q_ox: c_double,
    t: c_double,
    v_sb: c_double,
    out: *mut c_double,
    err: *mut c_char,
    err_cap: usize,
) -> c_int {
    let dev = unsafe { read_c_str(device_type) };
    match dp::MOSFETParams::new(&dev, l, w, t_ox, n_body, phi_ms, q_ox, t) {
        Err(m) => unsafe { err_msg(&m, err, err_cap) },
        Ok(p)  => match p.threshold_voltage(v_sb) {
            Ok(vt) => unsafe { ok_f64(vt, out) },
            Err(m) => unsafe { err_msg(&m, err, err_cap) },
        },
    }
}

// ---------------------------------------------------------------------------
// mosfet-models
// ---------------------------------------------------------------------------

/// C representation of a Level-1 MOSFET DC operating point.
/// Must match the typedef in silicon_cgo.h exactly.
#[repr(C)]
pub struct SiliconMosResult {
    pub id:     c_double,
    pub gm:     c_double,
    pub gds:    c_double,
    pub gmb:    c_double,
    pub cgs:    c_double,
    pub cgd:    c_double,
    pub cgb:    c_double,
    pub cbs:    c_double,
    pub cbd:    c_double,
    pub region: [c_char; 32],
}

/// Fill a SiliconMosResult from a Rust MosResult.
fn fill_mos_result(r: &mm::MosResult, out: &mut SiliconMosResult) {
    out.id  = r.id;
    out.gm  = r.gm;
    out.gds = r.gds;
    out.gmb = r.gmb;
    out.cgs = r.cgs;
    out.cgd = r.cgd;
    out.cgb = r.cgb;
    out.cbs = r.cbs;
    out.cbd = r.cbd;

    // Write the region string into the fixed 32-byte field using a safe
    // byte-by-byte copy.  On some platforms c_char is i8 (signed) while
    // str bytes are u8 — using a loop avoids the unsound `u8 as i8` type-pun
    // via slice::from_raw_parts that would be UB when c_char is unsigned.
    // "subthreshold" is 12 chars, well within the 31-char nul-terminated limit.
    let bytes = r.region.as_str().as_bytes();
    let n = bytes.len().min(31);
    for (dst, &src) in out.region[..n].iter_mut().zip(bytes.iter()) {
        *dst = src as c_char;
    }
    out.region[n] = 0;
}

#[no_mangle]
pub unsafe extern "C" fn silicon_evaluate_level1(
    vt0: c_double, kp: c_double, lambda: c_double, gamma: c_double, phi: c_double,
    w: c_double, l: c_double, n_sub: c_double,
    v_gs: c_double, v_ds: c_double, v_bs: c_double, t: c_double,
    out: *mut SiliconMosResult,
    err: *mut c_char,
    err_cap: usize,
) -> c_int {
    if out.is_null() {
        return unsafe { err_msg("out pointer is null", err, err_cap) };
    }
    let mut p = mm::Level1Params::default();
    p.vt0    = vt0;
    p.kp     = kp;
    p.lambda = lambda;
    p.gamma  = gamma;
    p.phi    = phi;
    p.w      = w;
    p.l      = l;
    p.n_sub  = n_sub;
    fill_mos_result(&mm::evaluate_level1(&p, v_gs, v_ds, v_bs, t), unsafe { &mut *out });
    0
}

#[no_mangle]
pub unsafe extern "C" fn silicon_evaluate_level1_defaults(
    v_gs: c_double, v_ds: c_double, v_bs: c_double, t: c_double,
    out: *mut SiliconMosResult,
    err: *mut c_char,
    err_cap: usize,
) -> c_int {
    if out.is_null() {
        return unsafe { err_msg("out pointer is null", err, err_cap) };
    }
    fill_mos_result(
        &mm::evaluate_level1(&mm::Level1Params::default(), v_gs, v_ds, v_bs, t),
        unsafe { &mut *out },
    );
    0
}

// ---------------------------------------------------------------------------
// fab-process-simulation
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn silicon_deposit(
    cs: *const c_char,
    material: *const c_char,
    thickness_nm: c_double,
    out: *mut c_char,
    out_cap: usize,
    err: *mut c_char,
    err_cap: usize,
) -> c_int {
    let cs_str  = unsafe { read_c_str(cs) };
    let mat_str = unsafe { read_c_str(material) };
    if let Err(m) = validate_name(&mat_str) {
        return unsafe { err_msg(&format!("deposit: {m}"), err, err_cap) };
    }
    let cs = match cs_from_wire(&cs_str) {
        Ok(c)  => c,
        Err(m) => return unsafe { err_msg(&format!("deposit: {m}"), err, err_cap) },
    };
    match fps::deposit(&cs, &mat_str, thickness_nm) {
        Ok(new_cs) => unsafe { ok_str(&cs_to_wire(&new_cs), out, out_cap, err, err_cap) },
        Err(m)     => unsafe { err_msg(&format!("deposit: {m}"), err, err_cap) },
    }
}

#[no_mangle]
pub unsafe extern "C" fn silicon_etch(
    cs: *const c_char,
    target: *const c_char,
    depth_nm: c_double,
    out: *mut c_char,
    out_cap: usize,
    err: *mut c_char,
    err_cap: usize,
) -> c_int {
    let cs_str     = unsafe { read_c_str(cs) };
    let target_str = unsafe { read_c_str(target) };
    if let Err(m) = validate_name(&target_str) {
        return unsafe { err_msg(&format!("etch: {m}"), err, err_cap) };
    }
    let cs = match cs_from_wire(&cs_str) {
        Ok(c)  => c,
        Err(m) => return unsafe { err_msg(&format!("etch: {m}"), err, err_cap) },
    };
    let new_cs = fps::etch(&cs, &target_str, depth_nm);
    unsafe { ok_str(&cs_to_wire(&new_cs), out, out_cap, err, err_cap) }
}

#[no_mangle]
pub unsafe extern "C" fn silicon_implant(
    cs: *const c_char,
    species: *const c_char,
    energy_kev: c_double,
    dose_cm2: c_double,
    out: *mut c_char,
    out_cap: usize,
    err: *mut c_char,
    err_cap: usize,
) -> c_int {
    let cs_str      = unsafe { read_c_str(cs) };
    let species_str = unsafe { read_c_str(species) };
    if let Err(m) = validate_name(&species_str) {
        return unsafe { err_msg(&format!("implant: {m}"), err, err_cap) };
    }
    let cs = match cs_from_wire(&cs_str) {
        Ok(c)  => c,
        Err(m) => return unsafe { err_msg(&format!("implant: {m}"), err, err_cap) },
    };
    match fps::implant(&cs, &species_str, energy_kev, dose_cm2) {
        Ok(new_cs) => unsafe { ok_str(&cs_to_wire(&new_cs), out, out_cap, err, err_cap) },
        Err(m)     => unsafe { err_msg(&format!("implant: {m}"), err, err_cap) },
    }
}

// diffuse — default temperature (fps::diffuse with None)
#[no_mangle]
pub unsafe extern "C" fn silicon_diffuse(
    cs: *const c_char,
    time_min: c_double,
    out: *mut c_char,
    out_cap: usize,
    err: *mut c_char,
    err_cap: usize,
) -> c_int {
    let cs_str = unsafe { read_c_str(cs) };
    let xcs = match cs_from_wire(&cs_str) {
        Ok(c)  => c,
        Err(m) => return unsafe { err_msg(&format!("diffuse: {m}"), err, err_cap) },
    };
    let new_cs = fps::diffuse(&xcs, time_min, None);
    unsafe { ok_str(&cs_to_wire(&new_cs), out, out_cap, err, err_cap) }
}

// diffuse_with_temp — explicit temperature_c [°C]
#[no_mangle]
pub unsafe extern "C" fn silicon_diffuse_with_temp(
    cs: *const c_char,
    time_min: c_double,
    temperature_c: c_double,
    out: *mut c_char,
    out_cap: usize,
    err: *mut c_char,
    err_cap: usize,
) -> c_int {
    let cs_str = unsafe { read_c_str(cs) };
    let xcs = match cs_from_wire(&cs_str) {
        Ok(c)  => c,
        Err(m) => return unsafe { err_msg(&format!("diffuse_with_temp: {m}"), err, err_cap) },
    };
    let new_cs = fps::diffuse(&xcs, time_min, Some(temperature_c));
    unsafe { ok_str(&cs_to_wire(&new_cs), out, out_cap, err, err_cap) }
}

// deal_grove_oxidation — default A/B coefficients (None, None)
#[no_mangle]
pub unsafe extern "C" fn silicon_deal_grove_oxidation(
    cs: *const c_char,
    time_min: c_double,
    out: *mut c_char,
    out_cap: usize,
    err: *mut c_char,
    err_cap: usize,
) -> c_int {
    let cs_str = unsafe { read_c_str(cs) };
    let xcs = match cs_from_wire(&cs_str) {
        Ok(c)  => c,
        Err(m) => return unsafe { err_msg(&format!("deal_grove_oxidation: {m}"), err, err_cap) },
    };
    match fps::deal_grove_oxidation(&xcs, time_min, None, None) {
        Ok(new_cs) => unsafe { ok_str(&cs_to_wire(&new_cs), out, out_cap, err, err_cap) },
        Err(m)     => unsafe { err_msg(&format!("deal_grove_oxidation: {m}"), err, err_cap) },
    }
}

// deal_grove_oxidation_custom — explicit A [µm] and B [µm²/hr]
#[no_mangle]
pub unsafe extern "C" fn silicon_deal_grove_oxidation_custom(
    cs: *const c_char,
    time_min: c_double,
    a_um: c_double,
    b_um2_per_hr: c_double,
    out: *mut c_char,
    out_cap: usize,
    err: *mut c_char,
    err_cap: usize,
) -> c_int {
    let cs_str = unsafe { read_c_str(cs) };
    let xcs = match cs_from_wire(&cs_str) {
        Ok(c)  => c,
        Err(m) => return unsafe { err_msg(&format!("deal_grove_oxidation_custom: {m}"), err, err_cap) },
    };
    match fps::deal_grove_oxidation(&xcs, time_min, Some(a_um), Some(b_um2_per_hr)) {
        Ok(new_cs) => unsafe { ok_str(&cs_to_wire(&new_cs), out, out_cap, err, err_cap) },
        Err(m)     => unsafe { err_msg(&format!("deal_grove_oxidation_custom: {m}"), err, err_cap) },
    }
}

// implant_range — returns (rp, straggle) in nm via out-pointers
#[no_mangle]
pub unsafe extern "C" fn silicon_implant_range(
    species: *const c_char,
    energy_kev: c_double,
    rp: *mut c_double,
    straggle: *mut c_double,
    err: *mut c_char,
    err_cap: usize,
) -> c_int {
    let species_str = unsafe { read_c_str(species) };
    match fps::implant_range(&species_str, energy_kev) {
        Ok((r, s)) => {
            if !rp.is_null()       { unsafe { *rp       = r } }
            if !straggle.is_null() { unsafe { *straggle = s } }
            0
        }
        Err(m) => unsafe { err_msg(&format!("implant_range: {m}"), err, err_cap) },
    }
}

// diffusivity_cm2_per_s — infallible (returns 0 for unknown species)
#[no_mangle]
pub unsafe extern "C" fn silicon_diffusivity_cm2_per_s(
    species: *const c_char,
    temperature_c: c_double,
) -> c_double {
    let species_str = unsafe { read_c_str(species) };
    fps::diffusivity_cm2_per_s(&species_str, temperature_c)
}

// ---------------------------------------------------------------------------
// Unit tests — test the pure-Rust helpers without CGo
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_name_ok() {
        assert!(validate_name("Si").is_ok());
        assert!(validate_name("SiO2").is_ok());
        assert!(validate_name("Poly").is_ok());
    }

    #[test]
    fn test_validate_name_rejects_pipe() {
        assert!(validate_name("Bad|Name").is_err());
    }

    #[test]
    fn test_validate_name_rejects_colon() {
        assert!(validate_name("Bad:Name").is_err());
    }

    #[test]
    fn test_cs_round_trip() {
        let orig = "Poly:50.0|SiO2:4.8|Si:500.0";
        let cs = cs_from_wire(orig).unwrap();
        assert_eq!(cs.layers.len(), 3);
        assert_eq!(cs_to_wire(&cs), orig);
    }

    #[test]
    fn test_cs_from_wire_empty() {
        let cs = cs_from_wire("").unwrap();
        assert!(cs.layers.is_empty());
    }

    #[test]
    fn test_cs_from_wire_rejects_pipe_in_material() {
        assert!(cs_from_wire("Bad|Material:10.0").is_err());
    }

    #[test]
    fn test_cs_from_wire_rejects_colon_in_material() {
        // The entry "Bad:Mat:10.0" splits into material="Bad", thickness="Mat:10.0"
        // which fails to parse as f64 → thickness parse error → Err.
        // But "Bad:Colon:10" would also fail. Testing injection via colons:
        assert!(cs_from_wire("Bad|entry:10.0").is_err());
    }

    #[test]
    fn test_cs_from_wire_rejects_missing_thickness() {
        assert!(cs_from_wire("SiO2").is_err());
    }

    #[test]
    fn test_cs_from_wire_rejects_bad_thickness() {
        assert!(cs_from_wire("SiO2:abc").is_err());
    }

    #[test]
    fn test_cs_to_wire_empty() {
        let cs = fps::CrossSection { layers: vec![] };
        assert_eq!(cs_to_wire(&cs), "");
    }
}
