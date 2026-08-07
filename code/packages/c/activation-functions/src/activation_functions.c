/*
 * activation_functions.c — implementation of the activation functions.
 * ===========================================================================
 * No <math.h>: e^x, tanh, and ln(1+x) are computed from scratch (range-reduced
 * series), so the package links without a math library. See the header for the
 * mathematics of each activation.
 */
#include "activation_functions.h"

/* ---------------------------------------------------------------------------
 *  <math.h>-free transcendental helpers
 * ------------------------------------------------------------------------- */

static double d_abs(double x) { return x < 0.0 ? -x : x; }

/* 2^k for an integer k, computed exactly by binary exponentiation (each partial
 * product is a power of two, hence exact until it under/overflows). */
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

/* e^x, via Cody-Waite range reduction: write x = k*ln2 + r with |r| <= ln2/2,
 * so e^x = 2^k * e^r and the Taylor series for e^r converges in a handful of
 * terms. The two-part ln2 constant (C1 + C2) keeps k*ln2 exact. */
static double d_exp(double x) {
    if (x != x) {
        return x; /* NaN propagates (and keeps it out of the int cast below) */
    }
    if (x == 0.0) {
        return 1.0;
    }
    /* Beyond this the result overflows a double; our callers never reach it
     * (sigmoid saturates first, softplus only ever passes x <= 0). +inf too. */
    if (x > 709.782712893384) {
        return 1.7976931348623157e308;
    }
    /* Below this e^x underflows to 0. This ALSO bounds |x| before the (int)
     * cast below: softplus can pass an arbitrarily large negative argument
     * (e.g. d_exp(-1e300)), and casting x*(1/ln2) to int would otherwise be
     * out-of-range UB. Handles -inf as well. */
    if (x < -745.13321910194) {
        return 0.0;
    }

    const double INV_LN2 = 1.4426950408889634;
    const double C1 = 0.693359375;             /* exact; C1 + C2 == ln2 */
    const double C2 = -2.1219444005469058277e-4;

    double kf = x * INV_LN2;
    int k = (int)(kf >= 0.0 ? kf + 0.5 : kf - 0.5); /* round to nearest */
    double r = (x - (double)k * C1) - (double)k * C2;

    /* Taylor series e^r = 1 + r + r^2/2! + ... (|r| <= ln2/2 ~ 0.3466). */
    double term = 1.0;
    double sum = 1.0;
    int n;
    for (n = 1; n <= 17; n++) {
        term *= r / (double)n;
        sum += term;
    }
    return sum * pow2i(k);
}

/* ln(1 + y) for y >= 0, via ln(1+y) = 2*atanh(u) with u = y/(2+y). Forming u
 * directly from y avoids the cancellation of computing ln of a near-1 value. */
static double d_ln1p(double y) {
    double u = y / (2.0 + y);
    double u2 = u * u;
    double term = u; /* current odd power u^(2n+1) */
    double sum = u;
    int n;
    for (n = 1; n <= 60; n++) {
        term *= u2;
        double add = term / (double)(2 * n + 1);
        sum += add;
        if (d_abs(add) < 1e-18) {
            break;
        }
    }
    return 2.0 * sum;
}

/* tanh(x) = (1 - e^-2|x|) / (1 + e^-2|x|), odd-extended and saturated. Using the
 * negative exponent keeps the ratio well-conditioned and avoids overflow. */
static double d_tanh(double x) {
    if (x == 0.0) {
        return 0.0;
    }
    int neg = x < 0.0;
    double ax = neg ? -x : x;
    if (ax > 20.0) {
        return neg ? -1.0 : 1.0; /* within 1 ulp of +/-1 out here */
    }
    double em2 = d_exp(-2.0 * ax);
    double t = (1.0 - em2) / (1.0 + em2);
    return neg ? -t : t;
}

/* ---------------------------------------------------------------------------
 *  Activations
 * ------------------------------------------------------------------------- */

double af_linear(double x) { return x; }
double af_linear_derivative(double x) {
    (void)x;
    return 1.0;
}

double af_sigmoid(double x) {
    /* Saturate the tails exactly as the Rust crate does. */
    if (x < -709.0) {
        return 0.0;
    }
    if (x > 709.0) {
        return 1.0;
    }
    return 1.0 / (1.0 + d_exp(-x));
}
double af_sigmoid_derivative(double x) {
    double s = af_sigmoid(x);
    return s * (1.0 - s);
}

double af_relu(double x) { return x > 0.0 ? x : 0.0; }
double af_relu_derivative(double x) { return x > 0.0 ? 1.0 : 0.0; }

double af_leaky_relu(double x) {
    return x > 0.0 ? x : AF_LEAKY_RELU_SLOPE * x;
}
double af_leaky_relu_derivative(double x) {
    return x > 0.0 ? 1.0 : AF_LEAKY_RELU_SLOPE;
}

double af_tanh(double x) { return d_tanh(x); }
double af_tanh_derivative(double x) {
    double t = d_tanh(x);
    return 1.0 - t * t;
}

double af_softplus(double x) {
    /* ln(1 + e^x) written stably: ln(1 + e^-|x|) + max(x, 0). */
    double max0 = x > 0.0 ? x : 0.0;
    return d_ln1p(d_exp(-d_abs(x))) + max0;
}
double af_softplus_derivative(double x) { return af_sigmoid(x); }
