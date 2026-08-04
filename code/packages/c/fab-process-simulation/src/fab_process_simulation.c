/*
 * fab_process_simulation.c — implementation of the 1-D CMOS process simulator.
 * ===========================================================================
 * Cross-sections are deep-copied by every step (the inputs are never mutated).
 * No <math.h>: sqrt (Newton) and exp (Cody-Waite) are computed from scratch.
 */
#include "fab_process_simulation.h"

#include <stdlib.h>
#include <string.h>

#define FAB_PI 3.141592653589793

/* ---------------------------------------------------------------------------
 *  <math.h>-free sqrt / exp
 * ------------------------------------------------------------------------- */

static double d_sqrt(double x) {
    if (x <= 0.0) {
        return 0.0;
    }
    double guess = x >= 1.0 ? x : 1.0;
    int i;
    for (i = 0; i < 60; i++) {
        double next = (guess + x / guess) / 2.0;
        double diff = next - guess;
        if (diff < 0.0) {
            diff = -diff;
        }
        if (diff < 1e-15 * guess + 1e-300) {
            return next;
        }
        guess = next;
    }
    return guess;
}

static double pow2i(int k) {
    double result = 1.0;
    double base = k < 0 ? 0.5 : 2.0;
    int n = k < 0 ? -k : k;
    while (n > 0) {
        if (n & 1) {
            result *= base;
        }
        base *= base;
        n >>= 1;
    }
    return result;
}

static double d_exp(double x) {
    if (x != x) {
        return x;
    }
    if (x == 0.0) {
        return 1.0;
    }
    if (x > 709.782712893384) {
        return 1.7976931348623157e308;
    }
    if (x < -745.13321910194) {
        return 0.0;
    }
    const double INV_LN2 = 1.4426950408889634;
    const double C1 = 0.693359375;
    const double C2 = -2.1219444005469058277e-4;
    double kf = x * INV_LN2;
    int k = (int)(kf >= 0.0 ? kf + 0.5 : kf - 0.5);
    double r = (x - (double)k * C1) - (double)k * C2;
    double term = 1.0, sum = 1.0;
    int i;
    for (i = 1; i <= 17; i++) {
        term *= r / (double)i;
        sum += term;
    }
    return sum * pow2i(k);
}

static double d_floor(double x) {
    /* Truncate toward zero, then adjust down for negatives (our inputs are all
     * non-negative, so truncation suffices). */
    if (x >= 9007199254740992.0 || x <= -9007199254740992.0) {
        return x;
    }
    double t = (double)(long long)x;
    return (t > x) ? t - 1.0 : t;
}

/* ---------------------------------------------------------------------------
 *  Small helpers
 * ------------------------------------------------------------------------- */

static char *dup_cstr(const char *s) {
    size_t n = strlen(s);
    char *out = malloc(n + 1);
    if (out) {
        memcpy(out, s, n + 1);
    }
    return out;
}

static int ensure_cap(void **data, size_t *cap, size_t need, size_t elem) {
    if (need <= *cap) {
        return 0;
    }
    size_t nc = *cap ? *cap : 4;
    while (nc < need) {
        if (nc > ((size_t)-1) / 2 / elem) {
            return -1;
        }
        nc *= 2;
    }
    void *nd = realloc(*data, nc * elem);
    if (!nd) {
        return -1;
    }
    *data = nd;
    *cap = nc;
    return 0;
}

/* ---------------------------------------------------------------------------
 *  Doping / Layer / CrossSection lifecycle + deep copy
 * ------------------------------------------------------------------------- */

static void doping_free(FabDoping *d) {
    free(d->species);
    free(d->samples);
    d->species = NULL;
    d->samples = NULL;
    d->n_samples = 0;
    d->cap_samples = 0;
}

static void layer_free(FabLayer *l) {
    free(l->material);
    size_t i;
    for (i = 0; i < l->n_doping; i++) {
        doping_free(&l->doping[i]);
    }
    free(l->doping);
    l->material = NULL;
    l->doping = NULL;
    l->n_doping = 0;
    l->cap_doping = 0;
    l->thickness_nm = 0.0;
}

void fab_cross_section_init(FabCrossSection *cs) {
    cs->layers = NULL;
    cs->n_layers = 0;
    cs->cap_layers = 0;
}

void fab_cross_section_free(FabCrossSection *cs) {
    if (!cs) {
        return;
    }
    size_t i;
    for (i = 0; i < cs->n_layers; i++) {
        layer_free(&cs->layers[i]);
    }
    free(cs->layers);
    cs->layers = NULL;
    cs->n_layers = 0;
    cs->cap_layers = 0;
}

/* Append an already-owned layer value (moves its owned pointers in). 0/-1. */
static int cs_append(FabCrossSection *cs, FabLayer layer) {
    if (ensure_cap((void **)&cs->layers, &cs->cap_layers, cs->n_layers + 1,
                   sizeof(FabLayer)) != 0) {
        layer_free(&layer);
        return -1;
    }
    cs->layers[cs->n_layers++] = layer;
    return 0;
}

/* Build a bare layer value (no doping). Returns 0 or -1 (nothing allocated). */
static int make_bare_layer(const char *material, double thickness_nm,
                           FabLayer *out) {
    out->material = dup_cstr(material);
    out->thickness_nm = thickness_nm;
    out->doping = NULL;
    out->n_doping = 0;
    out->cap_doping = 0;
    return out->material ? 0 : -1;
}

int fab_cross_section_add_layer(FabCrossSection *cs, const char *material,
                                double thickness_nm) {
    FabLayer l;
    if (make_bare_layer(material, thickness_nm, &l) != 0) {
        return -1;
    }
    return cs_append(cs, l);
}

/* Deep-copy one doping profile. 0/-1. */
static int doping_copy(const FabDoping *src, FabDoping *out) {
    out->species = dup_cstr(src->species);
    out->samples = NULL;
    out->n_samples = 0;
    out->cap_samples = 0;
    if (!out->species) {
        return -1;
    }
    if (src->n_samples > 0) {
        out->samples = malloc(src->n_samples * sizeof(FabSample));
        if (!out->samples) {
            free(out->species);
            out->species = NULL;
            return -1;
        }
        memcpy(out->samples, src->samples, src->n_samples * sizeof(FabSample));
        out->n_samples = src->n_samples;
        out->cap_samples = src->n_samples;
    }
    return 0;
}

/* Deep-copy one layer. 0/-1. */
static int layer_copy(const FabLayer *src, FabLayer *out) {
    out->material = dup_cstr(src->material);
    out->thickness_nm = src->thickness_nm;
    out->doping = NULL;
    out->n_doping = 0;
    out->cap_doping = 0;
    if (!out->material) {
        return -1;
    }
    if (src->n_doping > 0) {
        out->doping = calloc(src->n_doping, sizeof(FabDoping));
        if (!out->doping) {
            free(out->material);
            out->material = NULL;
            return -1;
        }
        size_t i;
        for (i = 0; i < src->n_doping; i++) {
            if (doping_copy(&src->doping[i], &out->doping[i]) != 0) {
                size_t j;
                for (j = 0; j < i; j++) {
                    doping_free(&out->doping[j]);
                }
                free(out->doping);
                free(out->material);
                out->material = NULL;
                out->doping = NULL;
                return -1;
            }
        }
        out->n_doping = src->n_doping;
        out->cap_doping = src->n_doping;
    }
    return 0;
}

/* Append a deep copy of `src` layer. 0/-1. */
static int cs_append_copy(FabCrossSection *cs, const FabLayer *src) {
    FabLayer l;
    if (layer_copy(src, &l) != 0) {
        return -1;
    }
    return cs_append(cs, l);
}

int fab_cross_section_copy(const FabCrossSection *src, FabCrossSection *out) {
    fab_cross_section_init(out);
    size_t i;
    for (i = 0; i < src->n_layers; i++) {
        if (cs_append_copy(out, &src->layers[i]) != 0) {
            fab_cross_section_free(out);
            return -1;
        }
    }
    return 0;
}

/* ---------------------------------------------------------------------------
 *  Accessors
 * ------------------------------------------------------------------------- */

size_t fab_layer_count(const FabCrossSection *cs) { return cs->n_layers; }
const FabLayer *fab_layer_at(const FabCrossSection *cs, size_t i) {
    return &cs->layers[i];
}
const FabDoping *fab_layer_doping(const FabLayer *layer, const char *species) {
    size_t i;
    for (i = 0; i < layer->n_doping; i++) {
        if (strcmp(layer->doping[i].species, species) == 0) {
            return &layer->doping[i];
        }
    }
    return NULL;
}

static int layer_first_is_sio2(const FabCrossSection *cs) {
    return cs->n_layers > 0 && strcmp(cs->layers[0].material, "SiO2") == 0;
}

/* ---------------------------------------------------------------------------
 *  Deal-Grove oxidation
 * ------------------------------------------------------------------------- */

FabStatus fab_deal_grove_oxidation(const FabCrossSection *cs, double time_min,
                                   int has_a, double a_um, int has_b,
                                   double b_um2_per_hr, FabCrossSection *out) {
    if (time_min <= 0.0) {
        return FAB_ERR_INVALID;
    }
    double a = has_a ? a_um : FAB_DEAL_GROVE_DRY_1000C_A;
    double b = has_b ? b_um2_per_hr : FAB_DEAL_GROVE_DRY_1000C_B;

    double tau_hr = 0.0;
    if (layer_first_is_sio2(cs)) {
        double prev_um = cs->layers[0].thickness_nm / 1000.0;
        tau_hr = (prev_um * prev_um + a * prev_um) / b;
    }
    double t_hr = time_min / 60.0;
    double discriminant = a * a + 4.0 * b * (t_hr + tau_hr);
    double t_ox_um = (-a + d_sqrt(discriminant)) / 2.0;
    double t_ox_nm = t_ox_um * 1000.0;

    fab_cross_section_init(out);
    FabLayer oxide;
    if (make_bare_layer("SiO2", t_ox_nm, &oxide) != 0) {
        return FAB_ERR_NOMEM;
    }
    if (cs_append(out, oxide) != 0) {
        return FAB_ERR_NOMEM;
    }
    /* Copy the tail: skip the old oxide when replacing, else all layers. */
    size_t start = layer_first_is_sio2(cs) ? 1 : 0;
    size_t i;
    for (i = start; i < cs->n_layers; i++) {
        if (cs_append_copy(out, &cs->layers[i]) != 0) {
            fab_cross_section_free(out);
            return FAB_ERR_NOMEM;
        }
    }
    return FAB_OK;
}

/* ---------------------------------------------------------------------------
 *  Deposition
 * ------------------------------------------------------------------------- */

FabStatus fab_deposit(const FabCrossSection *cs, const char *material,
                      double thickness_nm, FabCrossSection *out) {
    if (thickness_nm <= 0.0) {
        return FAB_ERR_INVALID;
    }
    fab_cross_section_init(out);
    FabLayer top;
    if (make_bare_layer(material, thickness_nm, &top) != 0) {
        return FAB_ERR_NOMEM;
    }
    if (cs_append(out, top) != 0) {
        return FAB_ERR_NOMEM;
    }
    size_t i;
    for (i = 0; i < cs->n_layers; i++) {
        if (cs_append_copy(out, &cs->layers[i]) != 0) {
            fab_cross_section_free(out);
            return FAB_ERR_NOMEM;
        }
    }
    return FAB_OK;
}

/* ---------------------------------------------------------------------------
 *  Etching
 * ------------------------------------------------------------------------- */

FabStatus fab_etch(const FabCrossSection *cs, const char *target_material,
                   double depth_nm, FabCrossSection *out) {
    if (fab_cross_section_copy(cs, out) != 0) {
        return FAB_ERR_NOMEM;
    }
    if (depth_nm <= 0.0 || out->n_layers == 0) {
        return FAB_OK; /* unchanged */
    }
    double remaining = depth_nm;
    while (remaining > 0.0) {
        if (out->n_layers == 0) {
            break;
        }
        if (strcmp(out->layers[0].material, target_material) != 0) {
            break;
        }
        if (out->layers[0].thickness_nm > remaining) {
            out->layers[0].thickness_nm -= remaining;
            remaining = 0.0;
        } else {
            remaining -= out->layers[0].thickness_nm;
            /* Remove layers[0]: free it and shift the rest down. */
            layer_free(&out->layers[0]);
            memmove(&out->layers[0], &out->layers[1],
                    (out->n_layers - 1) * sizeof(FabLayer));
            out->n_layers--;
        }
    }
    return FAB_OK;
}

/* ---------------------------------------------------------------------------
 *  Implant-range lookup with interpolation
 * ------------------------------------------------------------------------- */

typedef struct {
    const char *species;
    unsigned energy;
    double rp, std;
} RangeEntry;

/* SRIM 2013 tabulations for a Si substrate. */
static const RangeEntry kRangeTable[] = {
    {"B", 10, 33.0, 18.0},   {"B", 30, 92.0, 38.0},   {"B", 100, 260.0, 80.0},
    {"P", 30, 39.0, 19.0},   {"P", 100, 130.0, 50.0}, {"As", 30, 22.0, 11.0},
    {"As", 100, 64.0, 28.0}, {"BF2", 30, 31.0, 19.0}, {"BF2", 60, 60.0, 30.0},
};

FabStatus fab_implant_range(const char *species, double energy_kev,
                            double *rp_nm, double *rp_std_nm) {
    /* Collect matches for this species (the table is already energy-sorted). */
    double e[8], rp[8], sd[8];
    size_t n = 0, i;
    for (i = 0; i < sizeof kRangeTable / sizeof kRangeTable[0]; i++) {
        if (strcmp(kRangeTable[i].species, species) == 0) {
            e[n] = (double)kRangeTable[i].energy;
            rp[n] = kRangeTable[i].rp;
            sd[n] = kRangeTable[i].std;
            n++;
        }
    }
    if (n == 0) {
        return FAB_ERR_UNKNOWN_SPECIES;
    }

    /* Exact match. */
    for (i = 0; i < n; i++) {
        double diff = e[i] - energy_kev;
        if (diff < 0.0) {
            diff = -diff;
        }
        if (diff < 1e-6) {
            *rp_nm = rp[i];
            *rp_std_nm = sd[i];
            return FAB_OK;
        }
    }
    /* Below the minimum: linear from origin. */
    if (energy_kev < e[0]) {
        *rp_nm = rp[0] * energy_kev / e[0];
        *rp_std_nm = sd[0] * energy_kev / e[0];
        return FAB_OK;
    }
    /* Above the maximum: scale from the highest entry. */
    if (energy_kev > e[n - 1]) {
        *rp_nm = rp[n - 1] * energy_kev / e[n - 1];
        *rp_std_nm = sd[n - 1] * energy_kev / e[n - 1];
        return FAB_OK;
    }
    /* Interpolate between bracketing entries. */
    for (i = 0; i + 1 < n; i++) {
        if (energy_kev >= e[i] && energy_kev <= e[i + 1]) {
            double f = (energy_kev - e[i]) / (e[i + 1] - e[i]);
            *rp_nm = rp[i] + f * (rp[i + 1] - rp[i]);
            *rp_std_nm = sd[i] + f * (sd[i + 1] - sd[i]);
            return FAB_OK;
        }
    }
    return FAB_ERR_UNKNOWN_SPECIES; /* unreachable given the checks above */
}

/* ---------------------------------------------------------------------------
 *  Implantation
 * ------------------------------------------------------------------------- */

/* Find or create the doping profile for `species` on a layer. */
static FabDoping *layer_doping_entry(FabLayer *layer, const char *species) {
    size_t i;
    for (i = 0; i < layer->n_doping; i++) {
        if (strcmp(layer->doping[i].species, species) == 0) {
            return &layer->doping[i];
        }
    }
    if (ensure_cap((void **)&layer->doping, &layer->cap_doping,
                   layer->n_doping + 1, sizeof(FabDoping)) != 0) {
        return NULL;
    }
    FabDoping *d = &layer->doping[layer->n_doping];
    d->species = dup_cstr(species);
    if (!d->species) {
        return NULL;
    }
    d->samples = NULL;
    d->n_samples = 0;
    d->cap_samples = 0;
    layer->n_doping++;
    return d;
}

static int doping_push(FabDoping *d, double depth, double conc) {
    if (ensure_cap((void **)&d->samples, &d->cap_samples, d->n_samples + 1,
                   sizeof(FabSample)) != 0) {
        return -1;
    }
    d->samples[d->n_samples].depth_nm = depth;
    d->samples[d->n_samples].conc_per_cm3 = conc;
    d->n_samples++;
    return 0;
}

FabStatus fab_implant(const FabCrossSection *cs, const char *species,
                      double energy_kev, double dose_per_cm2,
                      FabCrossSection *out) {
    if (dose_per_cm2 <= 0.0) {
        return FAB_ERR_INVALID;
    }
    double rp_nm, rp_std_nm;
    FabStatus st = fab_implant_range(species, energy_kev, &rp_nm, &rp_std_nm);
    if (st != FAB_OK) {
        return st;
    }
    if (fab_cross_section_copy(cs, out) != 0) {
        return FAB_ERR_NOMEM;
    }

    int si_found = 0;
    size_t li;
    for (li = 0; li < out->n_layers; li++) {
        FabLayer *layer = &out->layers[li];
        if (!si_found && strcmp(layer->material, "Si") == 0) {
            si_found = 1;
            FabDoping *profile = layer_doping_entry(layer, species);
            if (!profile) {
                fab_cross_section_free(out);
                return FAB_ERR_NOMEM;
            }
            double peak =
                dose_per_cm2 / (rp_std_nm * 1e-7 * d_sqrt(2.0 * FAB_PI));
            double cand = rp_nm + 4.0 * rp_std_nm;
            double max_depth =
                layer->thickness_nm < cand ? layer->thickness_nm : cand;
            double floor_val = d_floor(max_depth / 5.0);
            size_t n_samples = (size_t)(floor_val > 20.0 ? floor_val : 20.0);
            size_t i;
            for (i = 0; i < n_samples; i++) {
                double x_nm =
                    ((double)i + 0.5) * (max_depth / (double)n_samples);
                double dx = x_nm - rp_nm;
                double conc =
                    peak * d_exp(-(dx * dx) / (2.0 * rp_std_nm * rp_std_nm));
                if (doping_push(profile, x_nm, conc) != 0) {
                    fab_cross_section_free(out);
                    return FAB_ERR_NOMEM;
                }
            }
        }
    }
    if (!si_found) {
        fab_cross_section_free(out);
        return FAB_ERR_NO_SI;
    }
    return FAB_OK;
}

/* ---------------------------------------------------------------------------
 *  Diffusion (v0.1.0: samples preserved)
 * ------------------------------------------------------------------------- */

FabStatus fab_diffuse(const FabCrossSection *cs, double time_min, int has_temp,
                      double temperature_c, FabCrossSection *out) {
    (void)time_min;
    (void)has_temp;
    (void)temperature_c;
    /* The v0.1.0 model keeps the sampled points unchanged, so diffuse is a
     * deep copy (the broadening variance is computed but not applied). */
    if (fab_cross_section_copy(cs, out) != 0) {
        return FAB_ERR_NOMEM;
    }
    return FAB_OK;
}

/* ---------------------------------------------------------------------------
 *  Diffusivity
 * ------------------------------------------------------------------------- */

double fab_diffusivity_1000c(const char *species) {
    if (strcmp(species, "B") == 0) {
        return 1e-14;
    }
    if (strcmp(species, "P") == 0) {
        return 1.2e-14;
    }
    if (strcmp(species, "As") == 0) {
        return 4e-15;
    }
    return 1e-14; /* conservative fallback */
}

double fab_diffusivity_cm2_per_s(const char *species, double temperature_c) {
    double d0 = fab_diffusivity_1000c(species);
    double t_k = temperature_c + 273.15;
    double ratio = t_k / 1273.15;
    return d0 * ratio * ratio; /* T^2 scaling (Rust .powi(2)) */
}
