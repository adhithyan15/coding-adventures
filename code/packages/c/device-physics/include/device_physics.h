/*
 * device_physics.h — semiconductor device-physics primitives, in pure ISO C17.
 * A faithful port of the Rust `device-physics` crate.
 * ===========================================================================
 *
 * Closed-form textbook models (Sedra/Smith, Pierret, Streetman) in SI units:
 *
 *   thermal_voltage(T)          V_T = kT/q
 *   intrinsic_concentration(T)  n_i(T) = N_i(300) (T/300)^1.5 exp(-Eg/(2kT)(1-T/300))
 *   fermi_potential(N, kind, T) phi_F = ±V_T ln(N/n_i)
 *
 *   PN junction: built-in voltage (ln), depletion width (sqrt), Shockley
 *   saturation current, and diode current I = I_S (exp(V/V_T) - 1).
 *
 *   MOSFET: oxide capacitance, flat-band voltage, body Fermi potential,
 *   body-effect coefficient gamma (sqrt), threshold voltage with body effect.
 *
 * NO libm: the transcendentals (exp, ln, sqrt) are computed from scratch;
 * results match the Rust f64 models (captured via an oracle run) to ~1e-9
 * relative.
 *
 * DIVERGENCE FROM RUST. Rust returns `Result<_, String>`; this port returns a
 * `DpStatus` and writes results through out-parameters. The struct fields are
 * exposed directly (as in Rust); build with the checked `*_new` constructors.
 *
 * PORTABILITY. Pure ISO C17, no <math.h>. Builds clean under GCC, Clang, and
 * MSVC with -pedantic-errors / /permissive- and warnings-as-errors.
 */
#ifndef CA_DEVICE_PHYSICS_H
#define CA_DEVICE_PHYSICS_H

#ifdef __cplusplus
extern "C" {
#endif

/* ── Physical constants (SI) ─────────────────────────────────────────────── */
#define DP_K_BOLTZMANN 1.380649e-23      /* J/K  */
#define DP_Q_ELECTRON 1.602176634e-19    /* C    */
#define DP_EPS0 8.8541878128e-12         /* F/m  */
#define DP_EPS_SI (11.7 * DP_EPS0)       /* F/m  */
#define DP_EPS_OX (3.9 * DP_EPS0)        /* F/m  */
#define DP_N_I_300K 1.0e16               /* /m^3 */
#define DP_N_C 2.8e25                    /* /m^3 */
#define DP_N_V 1.04e25                   /* /m^3 */
#define DP_EG_SI_300K 1.12               /* eV   */
#define DP_MU_N_300K (1350e-4)           /* m^2/V.s */
#define DP_MU_P_300K (480e-4)            /* m^2/V.s */

typedef enum {
    DP_OK = 0,
    DP_ERR_INVALID,      /* non-positive doping / area / L / W / T_ox, bad kind */
    DP_ERR_TEMP_RANGE,   /* intrinsic_concentration below 100 K */
    DP_ERR_BODY_FORWARD  /* threshold: V_SB below -2*phi_F */
} DpStatus;

/* Thermal voltage V_T = kT/q [V]. */
double dp_thermal_voltage(double t_kelvin);

/* Intrinsic carrier concentration n_i(T) [/m^3]. DP_ERR_TEMP_RANGE for
 * T < 100 K. */
DpStatus dp_intrinsic_concentration(double t_kelvin, double *out);

/* Fermi potential [V]: +V_T ln(N/n_i) for kind "p", negated for "n".
 * DP_ERR_INVALID for N <= 0 or a kind other than "p"/"n". */
DpStatus dp_fermi_potential(double n_doping, const char *kind, double t_kelvin,
                            double *out);

/* ── PN junction ─────────────────────────────────────────────────────────── */

typedef struct {
    double na;    /* acceptor doping [/m^3] */
    double nd;    /* donor doping [/m^3]    */
    double a;     /* junction area [m^2]    */
    double t;     /* temperature [K]        */
    double tau_n; /* electron lifetime [s]  */
    double tau_p; /* hole lifetime [s]      */
} DpPNJunction;

/* Build a junction; DP_ERR_INVALID for non-positive doping or area. */
DpStatus dp_pn_new(double na, double nd, double a, double t, double tau_n,
                   double tau_p, DpPNJunction *out);

double dp_pn_built_in_voltage(const DpPNJunction *j);
double dp_pn_depletion_width(const DpPNJunction *j, double v_applied);
double dp_pn_saturation_current(const DpPNJunction *j);
double dp_pn_current(const DpPNJunction *j, double v);

/* ── MOSFET ──────────────────────────────────────────────────────────────── */

typedef enum { DP_NMOS, DP_PMOS } DpMosType;

typedef struct {
    DpMosType device_type;
    double l;      /* channel length [m]  */
    double w;      /* channel width [m]   */
    double t_ox;   /* oxide thickness [m] */
    double n_body; /* body doping [/m^3]  */
    double phi_ms; /* gate-body work-fn difference [V] */
    double q_ox;   /* oxide trapped charge [C/m^2]     */
    double t;      /* temperature [K]     */
} DpMOSFET;

/* Build MOSFET params; DP_ERR_INVALID for non-positive L/W/T_ox/N_body. */
DpStatus dp_mos_new(DpMosType device_type, double l, double w, double t_ox,
                    double n_body, double phi_ms, double q_ox, double t,
                    DpMOSFET *out);

double dp_mos_c_ox(const DpMOSFET *m);
double dp_mos_v_fb(const DpMOSFET *m);
double dp_mos_phi_f(const DpMOSFET *m);
double dp_mos_gamma(const DpMOSFET *m);
/* Threshold voltage with body effect; DP_ERR_BODY_FORWARD for V_SB < -2*phi_F. */
DpStatus dp_mos_threshold_voltage(const DpMOSFET *m, double v_sb, double *out);

#ifdef __cplusplus
}
#endif

#endif /* CA_DEVICE_PHYSICS_H */
