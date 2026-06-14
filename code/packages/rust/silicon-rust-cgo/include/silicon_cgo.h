/*
 * silicon_cgo.h — C API for the silicon simulation stack.
 *
 * This header is shared between the Rust cdylib (silicon-rust-cgo) and the
 * Go CGo wrapper (silicon_rust_go).  It declares every symbol that the
 * cdylib exports so that both compilers agree on the types.
 *
 * Calling conventions
 * -------------------
 *   Infallible functions    — return double directly.
 *   Fallible functions      — return int (0 = success, -1 = error).
 *                             On error: nul-terminated UTF-8 message written
 *                             to err[0..err_cap-1].
 *   String-returning funcs  — write cross-section wire string into
 *                             out[0..out_cap-1], nul-terminated.
 *
 * Wire format
 * -----------
 *   A CrossSection is serialised as pipe-separated "material:thickness_nm"
 *   pairs ordered top-to-bottom:
 *       ""                               empty cross-section
 *       "Si:500.0"                       bare silicon substrate
 *       "SiO2:4.8|Si:500.0"             gate oxide on silicon
 *       "Poly:50.0|SiO2:4.8|Si:500.0"  poly on gate oxide on silicon
 *
 *   Material names containing '|' or ':' are rejected by deposit, etch,
 *   and implant (wire-format injection guard).
 */

#pragma once
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ── Physical constants (infallible) ───────────────────────────────────── */

double silicon_k_boltzmann(void);    /* 1.380649e-23  J/K              */
double silicon_q_electron(void);     /* 1.602176634e-19  C             */
double silicon_eps0(void);           /* 8.8541878e-12  F/m             */
double silicon_eps_si(void);         /* 1.0359e-10  F/m                */
double silicon_eps_ox(void);         /* 3.4531e-11  F/m                */
double silicon_ni_at_300k(void);     /* 1e16  /m³                      */
double silicon_eg_si_at_300k(void);  /* 1.12  eV                       */
double silicon_mu_n_300k(void);      /* 0.1350  m²/V·s                 */
double silicon_mu_p_300k(void);      /* 0.0480  m²/V·s                 */

/* ── device-physics ─────────────────────────────────────────────────────── */

/* Infallible — kT/q */
double silicon_thermal_voltage(double t_kelvin);

/* Fallible — return 0 on success, -1 on error */
int silicon_intrinsic_concentration(
    double t_kelvin,
    double *out, char *err, size_t err_cap);

int silicon_fermi_potential(
    double n_doping, const char *kind, double t_kelvin,
    double *out, char *err, size_t err_cap);
    /* kind: "p" or "n" */

int silicon_pn_junction_built_in_voltage(
    double na, double nd, double t,
    double *out, char *err, size_t err_cap);

int silicon_pn_junction_depletion_width(
    double na, double nd, double t, double v_applied,
    double *out, char *err, size_t err_cap);

int silicon_pn_junction_saturation_current(
    double na, double nd, double a, double t, double tau_n, double tau_p,
    double *out, char *err, size_t err_cap);

int silicon_pn_junction_current(
    double na, double nd, double a, double t, double tau_n, double tau_p,
    double v,
    double *out, char *err, size_t err_cap);

int silicon_mosfet_threshold_voltage(
    const char *device_type, double l, double w, double t_ox, double n_body,
    double phi_ms, double q_ox, double t, double v_sb,
    double *out, char *err, size_t err_cap);
    /* device_type: "NMOS" or "PMOS" */

/* ── mosfet-models ──────────────────────────────────────────────────────── */

/*
 * SiliconMosResult — Level-1 MOSFET DC operating point.
 *
 * Fields: id gm gds gmb cgs cgd cgb cbs cbd [A, S, S, S, F, F, F, F, F]
 * region: one of "cutoff" "subthreshold" "triode" "saturation" (nul-terminated)
 *
 * The region field is 32 bytes — large enough for the longest string
 * ("subthreshold") with room to spare.
 */
typedef struct {
    double id;
    double gm;
    double gds;
    double gmb;
    double cgs;
    double cgd;
    double cgb;
    double cbs;
    double cbd;
    char   region[32];
} SiliconMosResult;

int silicon_evaluate_level1(
    double vt0, double kp, double lambda, double gamma, double phi,
    double w,   double l,  double n_sub,
    double v_gs, double v_ds, double v_bs, double t,
    SiliconMosResult *out, char *err, size_t err_cap);

int silicon_evaluate_level1_defaults(
    double v_gs, double v_ds, double v_bs, double t,
    SiliconMosResult *out, char *err, size_t err_cap);
/* Uses Level1Params::default() — 130 nm NMOS device parameters. */

/* ── fab-process-simulation ─────────────────────────────────────────────── */

/*
 * All process functions write the resulting cross-section wire string into
 * out[0..out_cap-1] on success.  4096 bytes is sufficient for any realistic
 * CMOS process flow (a typical stack has < 20 layers).
 */

int silicon_deposit(
    const char *cs, const char *material, double thickness_nm,
    char *out, size_t out_cap, char *err, size_t err_cap);

int silicon_etch(
    const char *cs, const char *target, double depth_nm,
    char *out, size_t out_cap, char *err, size_t err_cap);

int silicon_implant(
    const char *cs, const char *species, double energy_kev, double dose_cm2,
    char *out, size_t out_cap, char *err, size_t err_cap);

/*
 * silicon_diffuse — anneal at the default temperature (from fps::diffuse).
 * silicon_diffuse_with_temp — anneal at an explicit temperature_c [°C].
 */
int silicon_diffuse(
    const char *cs, double time_min,
    char *out, size_t out_cap, char *err, size_t err_cap);

int silicon_diffuse_with_temp(
    const char *cs, double time_min, double temperature_c,
    char *out, size_t out_cap, char *err, size_t err_cap);

/*
 * silicon_deal_grove_oxidation — use default A/B coefficients.
 * silicon_deal_grove_oxidation_custom — supply explicit A [µm] and
 *   B [µm²/hr] Deal-Grove coefficients.
 */
int silicon_deal_grove_oxidation(
    const char *cs, double time_min,
    char *out, size_t out_cap, char *err, size_t err_cap);

int silicon_deal_grove_oxidation_custom(
    const char *cs, double time_min, double a_um, double b_um2_per_hr,
    char *out, size_t out_cap, char *err, size_t err_cap);

/* implant_range — projected range and straggle in nm */
int silicon_implant_range(
    const char *species, double energy_kev,
    double *rp, double *straggle, char *err, size_t err_cap);

/* diffusivity_cm2_per_s — infallible, returns 0 for unknown species */
double silicon_diffusivity_cm2_per_s(
    const char *species, double temperature_c);

#ifdef __cplusplus
}
#endif
