/*
 * fab_process_simulation.h — a 1-D analytical CMOS process-flow simulator, in
 * pure ISO C17. A faithful port of the Rust `fab-process-simulation` crate.
 * ===========================================================================
 *
 * Models the standard front-end fabrication steps with 1-D analytical
 * approximations calibrated against published Sky130 profiles:
 *
 *   | Step              | Model                                    |
 *   |-------------------|------------------------------------------|
 *   | Thermal oxidation | Deal-Grove (quadratic growth law, sqrt)  |
 *   | Deposition        | Uniform film addition                    |
 *   | Etching           | Layer-selective depth removal            |
 *   | Ion implantation  | Gaussian profile from an SRIM table (exp)|
 *   | Diffusion         | Fick's-law broadening (v0.1.0: no-op)    |
 *
 * A `FabCrossSection` is a top-to-bottom stack of `FabLayer`s (`layers[0]` is
 * the top). Each layer carries a doping map: species -> a list of sampled
 * (depth_nm, conc_per_cm3) points. Every step returns a NEW cross-section (the
 * inputs are never mutated), so results are deep-copied — release with
 * `fab_cross_section_free`.
 *
 * NO libm: the two transcendentals (sqrt for Deal-Grove, exp for the Gaussian
 * implant) are computed from scratch; results match the Rust f64 models to
 * well within 1e-6 relative.
 *
 * DIVERGENCE FROM RUST. Rust returns `Result<_, String>`; this port returns a
 * `FabStatus` code and writes the new cross-section through an out-parameter.
 *
 * PORTABILITY. Pure ISO C17, no <math.h>. Builds clean under GCC, Clang, and
 * MSVC with -pedantic-errors / /permissive- and warnings-as-errors.
 */
#ifndef CA_FAB_PROCESS_SIMULATION_H
#define CA_FAB_PROCESS_SIMULATION_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Deal-Grove constants for dry O2 oxidation at 1000 C. */
#define FAB_DEAL_GROVE_DRY_1000C_A 0.165   /* parabolic rate A [um]      */
#define FAB_DEAL_GROVE_DRY_1000C_B 0.0117  /* linear rate B [um^2/hr]    */

typedef enum {
    FAB_OK = 0,
    FAB_ERR_INVALID,         /* non-positive time / thickness / dose */
    FAB_ERR_UNKNOWN_SPECIES, /* implant species not in the SRIM table */
    FAB_ERR_NO_SI,           /* implant found no Si layer */
    FAB_ERR_NOMEM
} FabStatus;

/* One sampled point of a Gaussian doping profile. */
typedef struct {
    double depth_nm;
    double conc_per_cm3;
} FabSample;

/* A per-species doping profile (a growable list of samples). */
typedef struct {
    char *species;
    FabSample *samples;
    size_t n_samples, cap_samples;
} FabDoping;

/* A material layer with an optional per-species doping map. */
typedef struct {
    char *material;
    double thickness_nm;
    FabDoping *doping;
    size_t n_doping, cap_doping;
} FabLayer;

/* A vertical cross-section: a top-to-bottom stack of layers. */
typedef struct {
    FabLayer *layers;
    size_t n_layers, cap_layers;
} FabCrossSection;

/* ── Cross-section lifecycle ─────────────────────────────────────────────── */
void fab_cross_section_init(FabCrossSection *cs); /* empty */
void fab_cross_section_free(FabCrossSection *cs);
/* Append a bare (undoped) layer. Returns 0 or -1 on OOM. */
int fab_cross_section_add_layer(FabCrossSection *cs, const char *material,
                                double thickness_nm);
/* Deep-copy `src` into a fresh `out`. Returns 0 or -1 on OOM. */
int fab_cross_section_copy(const FabCrossSection *src, FabCrossSection *out);

/* ── Accessors (borrowed) ────────────────────────────────────────────────── */
size_t fab_layer_count(const FabCrossSection *cs);
const FabLayer *fab_layer_at(const FabCrossSection *cs, size_t i);
/* The doping profile for `species` on a layer, or NULL if absent. */
const FabDoping *fab_layer_doping(const FabLayer *layer, const char *species);

/* ── Process steps (each writes a NEW cross-section to *out) ──────────────── */

/* Grow thermal SiO2 via Deal-Grove. Pass has_a/has_b = 0 to use the defaults.
 * Returns FAB_ERR_INVALID for time_min <= 0. */
FabStatus fab_deal_grove_oxidation(const FabCrossSection *cs, double time_min,
                                   int has_a, double a_um, int has_b,
                                   double b_um2_per_hr, FabCrossSection *out);

/* Deposit a uniform layer on top. FAB_ERR_INVALID for thickness <= 0. */
FabStatus fab_deposit(const FabCrossSection *cs, const char *material,
                      double thickness_nm, FabCrossSection *out);

/* Etch the topmost `depth_nm` of consecutive top layers whose material equals
 * `target_material`. depth <= 0 copies the input unchanged. */
FabStatus fab_etch(const FabCrossSection *cs, const char *target_material,
                   double depth_nm, FabCrossSection *out);

/* Add a Gaussian implant profile to the topmost Si layer. FAB_ERR_INVALID for
 * dose <= 0, FAB_ERR_UNKNOWN_SPECIES, FAB_ERR_NO_SI. */
FabStatus fab_implant(const FabCrossSection *cs, const char *species,
                      double energy_kev, double dose_per_cm2,
                      FabCrossSection *out);

/* Broaden doping profiles by diffusion (v0.1.0: samples preserved). Pass
 * has_temp = 0 to default to 1000 C. */
FabStatus fab_diffuse(const FabCrossSection *cs, double time_min, int has_temp,
                      double temperature_c, FabCrossSection *out);

/* ── Helpers ─────────────────────────────────────────────────────────────── */

/* Projected range Rp and straggle dRp for (species, energy), from the SRIM
 * table with linear interpolation / extrapolation. FAB_ERR_UNKNOWN_SPECIES if
 * the species is absent. */
FabStatus fab_implant_range(const char *species, double energy_kev,
                            double *rp_nm, double *rp_std_nm);

/* Reference diffusivity at 1000 C [cm^2/s]. */
double fab_diffusivity_1000c(const char *species);
/* Arrhenius (T^2-scaled) diffusivity at `temperature_c` [cm^2/s]. */
double fab_diffusivity_cm2_per_s(const char *species, double temperature_c);

#ifdef __cplusplus
}
#endif

#endif /* CA_FAB_PROCESS_SIMULATION_H */
