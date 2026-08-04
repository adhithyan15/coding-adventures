/*
 * device_physics.c — implementation of the semiconductor device-physics models.
 * ===========================================================================
 * Plain closed-form formulas over `double` fields. No <math.h>: exp, ln, and
 * sqrt are computed from scratch. powf(x, 1.5) is written as x * sqrt(x).
 */
#include "device_physics.h"

#include <string.h>

/* ---------------------------------------------------------------------------
 *  <math.h>-free exp / ln / sqrt
 * ------------------------------------------------------------------------- */

static double d_abs(double x) { return x < 0.0 ? -x : x; }

static double d_sqrt(double x) {
    if (x <= 0.0) {
        return 0.0;
    }
    double guess = x >= 1.0 ? x : 1.0;
    int i;
    for (i = 0; i < 80; i++) {
        double next = (guess + x / guess) / 2.0;
        if (d_abs(next - guess) < 1e-15 * guess + 1e-300) {
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

/* Natural log for x > 0: reduce x = m*2^e (m in [1,2)), ln = e*ln2 + 2*atanh(u),
 * u = (m-1)/(m+1). */
static double d_ln(double x) {
    /* Guard the non-finite / non-positive cases BEFORE the reduction loops:
     * for +inf the `m >= 2` loop, and for +0 the `m < 1` loop, would otherwise
     * never terminate (a DoS reachable when e.g. na*nd overflows to +inf). A
     * physical input can never legitimately be any of these; return a large
     * sentinel (Rust's ln would yield +/-inf here) and never spin. */
    if (x != x) {
        return x; /* NaN propagates */
    }
    if (x <= 0.0) {
        return -1.7976931348623157e308; /* ln(<= 0): ~ -inf */
    }
    if (x > 1.7976931348623157e308) {
        return 1.7976931348623157e308; /* +inf: ln(+inf) ~ +inf */
    }
    /* x is now finite and positive, so both loops terminate in <= ~1074 steps. */
    int e = 0;
    double m = x;
    while (m < 1.0) {
        m *= 2.0;
        e--;
    }
    while (m >= 2.0) {
        m *= 0.5;
        e++;
    }
    double u = (m - 1.0) / (m + 1.0);
    double u2 = u * u;
    double term = u, sum = u;
    int n;
    for (n = 1; n <= 60; n++) {
        term *= u2;
        double add = term / (double)(2 * n + 1);
        sum += add;
        if (d_abs(add) < 1e-17) {
            break;
        }
    }
    const double LN2 = 0.6931471805599453;
    return (double)e * LN2 + 2.0 * sum;
}

/* ---------------------------------------------------------------------------
 *  Bulk models
 * ------------------------------------------------------------------------- */

double dp_thermal_voltage(double t_kelvin) {
    return DP_K_BOLTZMANN * t_kelvin / DP_Q_ELECTRON;
}

DpStatus dp_intrinsic_concentration(double t_kelvin, double *out) {
    if (d_abs(t_kelvin - 300.0) < 1e-9) {
        *out = DP_N_I_300K;
        return DP_OK;
    }
    if (t_kelvin < 100.0) {
        return DP_ERR_TEMP_RANGE;
    }
    double ratio = t_kelvin / 300.0;
    double factor = ratio * d_sqrt(ratio); /* ratio^1.5 */
    double vt = dp_thermal_voltage(t_kelvin);
    double bandgap_term =
        d_exp(-(DP_EG_SI_300K / (2.0 * vt)) * (1.0 - t_kelvin / 300.0));
    *out = DP_N_I_300K * factor * bandgap_term;
    return DP_OK;
}

DpStatus dp_fermi_potential(double n_doping, const char *kind, double t_kelvin,
                            double *out) {
    if (n_doping <= 0.0) {
        return DP_ERR_INVALID;
    }
    double n_i;
    DpStatus st = dp_intrinsic_concentration(t_kelvin, &n_i);
    if (st != DP_OK) {
        return st;
    }
    double magnitude = dp_thermal_voltage(t_kelvin) * d_ln(n_doping / n_i);
    if (strcmp(kind, "p") == 0) {
        *out = magnitude;
        return DP_OK;
    }
    if (strcmp(kind, "n") == 0) {
        *out = -magnitude;
        return DP_OK;
    }
    return DP_ERR_INVALID;
}

/* ---------------------------------------------------------------------------
 *  PN junction
 * ------------------------------------------------------------------------- */

DpStatus dp_pn_new(double na, double nd, double a, double t, double tau_n,
                   double tau_p, DpPNJunction *out) {
    if (na <= 0.0 || nd <= 0.0) {
        return DP_ERR_INVALID;
    }
    if (a <= 0.0) {
        return DP_ERR_INVALID;
    }
    out->na = na;
    out->nd = nd;
    out->a = a;
    out->t = t;
    out->tau_n = tau_n;
    out->tau_p = tau_p;
    return DP_OK;
}

/* n_i(T), falling back to the 300 K value on a temperature-range error (matches
 * the Rust `.unwrap_or(N_I_300K)`). */
static double ni_or_default(double t) {
    double n_i;
    if (dp_intrinsic_concentration(t, &n_i) != DP_OK) {
        return DP_N_I_300K;
    }
    return n_i;
}

double dp_pn_built_in_voltage(const DpPNJunction *j) {
    double n_i = ni_or_default(j->t);
    return dp_thermal_voltage(j->t) * d_ln((j->na * j->nd) / (n_i * n_i));
}

double dp_pn_depletion_width(const DpPNJunction *j, double v_applied) {
    double phi_bi = dp_pn_built_in_voltage(j);
    if (v_applied >= phi_bi) {
        return 0.0;
    }
    double num = 2.0 * DP_EPS_SI * (j->na + j->nd) * (phi_bi - v_applied);
    double den = DP_Q_ELECTRON * j->na * j->nd;
    return d_sqrt(num / den);
}

double dp_pn_saturation_current(const DpPNJunction *j) {
    double n_i = ni_or_default(j->t);
    double vt = dp_thermal_voltage(j->t);
    double d_n = DP_MU_N_300K * vt; /* Einstein relation D = mu*kT/q */
    double d_p = DP_MU_P_300K * vt;
    double l_n = d_sqrt(d_n * j->tau_n);
    double l_p = d_sqrt(d_p * j->tau_p);
    return DP_Q_ELECTRON * j->a * n_i * n_i *
           (d_n / (l_n * j->na) + d_p / (l_p * j->nd));
}

double dp_pn_current(const DpPNJunction *j, double v) {
    double vt = dp_thermal_voltage(j->t);
    return dp_pn_saturation_current(j) * (d_exp(v / vt) - 1.0);
}

/* ---------------------------------------------------------------------------
 *  MOSFET
 * ------------------------------------------------------------------------- */

DpStatus dp_mos_new(DpMosType device_type, double l, double w, double t_ox,
                    double n_body, double phi_ms, double q_ox, double t,
                    DpMOSFET *out) {
    if (l <= 0.0 || w <= 0.0) {
        return DP_ERR_INVALID;
    }
    if (t_ox <= 0.0) {
        return DP_ERR_INVALID;
    }
    if (n_body <= 0.0) {
        return DP_ERR_INVALID;
    }
    out->device_type = device_type;
    out->l = l;
    out->w = w;
    out->t_ox = t_ox;
    out->n_body = n_body;
    out->phi_ms = phi_ms;
    out->q_ox = q_ox;
    out->t = t;
    return DP_OK;
}

double dp_mos_c_ox(const DpMOSFET *m) { return DP_EPS_OX / m->t_ox; }

double dp_mos_v_fb(const DpMOSFET *m) {
    return m->phi_ms - m->q_ox / dp_mos_c_ox(m);
}

double dp_mos_phi_f(const DpMOSFET *m) {
    const char *kind = m->device_type == DP_NMOS ? "p" : "n";
    double v;
    if (dp_fermi_potential(m->n_body, kind, m->t, &v) != DP_OK) {
        return 0.0;
    }
    return d_abs(v);
}

double dp_mos_gamma(const DpMOSFET *m) {
    return d_sqrt(2.0 * DP_EPS_SI * DP_Q_ELECTRON * m->n_body) / dp_mos_c_ox(m);
}

DpStatus dp_mos_threshold_voltage(const DpMOSFET *m, double v_sb, double *out) {
    double phi_f = dp_mos_phi_f(m);
    double two_phi_f = 2.0 * phi_f;
    if (-two_phi_f > v_sb) {
        return DP_ERR_BODY_FORWARD;
    }
    double gamma = dp_mos_gamma(m);
    double v_t0 = dp_mos_v_fb(m) + two_phi_f + gamma * d_sqrt(two_phi_f);
    *out = v_t0 + gamma * (d_sqrt(two_phi_f + v_sb) - d_sqrt(two_phi_f));
    return DP_OK;
}
