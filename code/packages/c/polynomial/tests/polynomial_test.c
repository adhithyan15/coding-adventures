/* Tests for the C polynomial library, using the iso_test.h harness. Uses
 * integer coefficients so results are exact f64 values (comparable directly),
 * pinned to the identities from the crate docs. */
#include "iso_test.h"

#include "polynomial.h"

/* Compare a result buffer of `n` doubles against expected integer values. */
static int poly_eq(const double *got, size_t n, const double *want,
                   size_t wantn) {
    size_t i;
    if (n != wantn) {
        return 0;
    }
    for (i = 0; i < n; i++) {
        if (got[i] != want[i]) {
            return 0;
        }
    }
    return 1;
}

int main(void) {
    double out[32], out2[32];

    /* normalize strips trailing zeros; degree follows. */
    {
        double p[3] = {1.0, 0.0, 0.0};
        size_t n = poly_normalize(p, 3, out);
        double want[1] = {1.0};
        ISO_CHECK(poly_eq(out, n, want, 1));
        ISO_CHECK_EQ_UINT(poly_degree(p, 3), 0);
    }
    {
        double p[3] = {3.0, 0.0, 2.0};
        ISO_CHECK_EQ_UINT(poly_degree(p, 3), 2);
        double z[1] = {0.0};
        ISO_CHECK_EQ_UINT(poly_normalize(z, 1, out), 0); /* zero poly -> empty */
    }

    /* Addition: (1 + 2x + 3x^2) + (4 + 5x) = 5 + 7x + 3x^2. */
    {
        double a[3] = {1.0, 2.0, 3.0};
        double b[2] = {4.0, 5.0};
        double want[3] = {5.0, 7.0, 3.0};
        size_t n = poly_add(a, 3, b, 2, out);
        ISO_CHECK(poly_eq(out, n, want, 3));
    }

    /* Subtraction with cancellation: (5+7x+3x^2) - (1+2x+3x^2) = 4 + 5x. */
    {
        double a[3] = {5.0, 7.0, 3.0};
        double b[3] = {1.0, 2.0, 3.0};
        double want[2] = {4.0, 5.0};
        size_t n = poly_subtract(a, 3, b, 3, out);
        ISO_CHECK(poly_eq(out, n, want, 2));
    }

    /* Multiplication: (1 + 2x)(3 + 4x) = 3 + 10x + 8x^2. */
    {
        double a[2] = {1.0, 2.0};
        double b[2] = {3.0, 4.0};
        double want[3] = {3.0, 10.0, 8.0};
        size_t n = poly_multiply(a, 2, b, 2, out);
        ISO_CHECK(poly_eq(out, n, want, 3));
    }

    /* Long division: (5 + x + 3x^2 + 2x^3) / (2 + x) = q [3,-1,2] r [-1]. */
    {
        double dividend[4] = {5.0, 1.0, 3.0, 2.0};
        double divisor[2] = {2.0, 1.0};
        double wq[3] = {3.0, -1.0, 2.0};
        double wr[1] = {-1.0};
        size_t ql = 0, rl = 0;
        ISO_CHECK(poly_divmod(dividend, 4, divisor, 2, out, &ql, out2, &rl));
        ISO_CHECK(poly_eq(out, ql, wq, 3));
        ISO_CHECK(poly_eq(out2, rl, wr, 1));
        /* Reconstruct: divisor*q + r == dividend. */
        {
            double prod[8], sum[8];
            size_t pn = poly_multiply(divisor, 2, out, ql, prod);
            size_t sn = poly_add(prod, pn, out2, rl, sum);
            ISO_CHECK(poly_eq(sum, sn, dividend, 4));
        }
    }

    /* Division by zero polynomial fails. */
    {
        double dividend[2] = {1.0, 1.0};
        double zero[1] = {0.0};
        size_t ql = 0;
        ISO_CHECK(!poly_divide(dividend, 2, zero, 1, out, &ql));
    }

    /* Horner evaluation: (1 + 2x + 3x^2) at x = 2 is 1 + 4 + 12 = 17. */
    {
        double p[3] = {1.0, 2.0, 3.0};
        ISO_CHECK_EQ_DBL(poly_evaluate(p, 3, 2.0), 17.0, 1e-9);
        double z[1] = {0.0};
        ISO_CHECK_EQ_DBL(poly_evaluate(z, 1, 5.0), 0.0, 1e-12);
    }

    /* GCD: (x^2 - 1) and (x - 1) share (x - 1) up to a scalar. */
    {
        double a[3] = {-1.0, 0.0, 1.0}; /* x^2 - 1 = (x-1)(x+1) */
        double b[2] = {-1.0, 1.0};      /* x - 1 */
        size_t n = poly_gcd(a, 3, b, 2, out);
        /* gcd is degree 1 (a scalar multiple of x - 1). */
        ISO_CHECK_EQ_UINT(n, 2);
        /* Its root is x = 1, so evaluating at 1 gives 0. */
        ISO_CHECK_EQ_DBL(poly_evaluate(out, n, 1.0), 0.0, 1e-9);
        /* And it divides both a and b with zero remainder. */
        {
            double rem[8];
            size_t rl = 0;
            ISO_CHECK(poly_modulo(a, 3, out, n, rem, &rl));
            ISO_CHECK_EQ_UINT(rl, 0);
        }
    }

    return ISO_TEST_RESULT();
}
