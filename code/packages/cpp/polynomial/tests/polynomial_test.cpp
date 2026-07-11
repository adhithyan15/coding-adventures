// Tests for the C++ polynomial library, using the iso_test.h harness. Integer
// coefficients keep results exact; pinned to the crate's identities.
#include "iso_test.h"

#include <stdexcept>
#include <vector>

#include "polynomial.hpp"

namespace poly = ca::polynomial;
using P = poly::poly;

int main() {
    // normalize / degree.
    {
        ISO_CHECK(poly::normalize(P{1.0, 0.0, 0.0}) == P{1.0});
        ISO_CHECK_EQ_UINT(poly::degree(P{3.0, 0.0, 2.0}), 2);
        ISO_CHECK(poly::normalize(P{0.0}).empty());
    }

    // add / subtract.
    {
        ISO_CHECK((poly::add(P{1.0, 2.0, 3.0}, P{4.0, 5.0}) ==
                   P{5.0, 7.0, 3.0}));
        ISO_CHECK((poly::subtract(P{5.0, 7.0, 3.0}, P{1.0, 2.0, 3.0}) ==
                   P{4.0, 5.0}));
    }

    // multiply: (1 + 2x)(3 + 4x) = 3 + 10x + 8x^2.
    {
        ISO_CHECK((poly::multiply(P{1.0, 2.0}, P{3.0, 4.0}) ==
                   P{3.0, 10.0, 8.0}));
    }

    // Long division and reconstruction.
    {
        P dividend = {5.0, 1.0, 3.0, 2.0};
        P divisor = {2.0, 1.0};
        auto qr = poly::divmod(dividend, divisor);
        ISO_CHECK((qr.first == P{3.0, -1.0, 2.0}));
        ISO_CHECK((qr.second == P{-1.0}));
        // divisor * q + r == dividend.
        ISO_CHECK((poly::add(poly::multiply(divisor, qr.first), qr.second) ==
                   dividend));
    }

    // Division by the zero polynomial throws.
    {
        bool threw = false;
        try {
            poly::divide(P{1.0, 1.0}, poly::zero());
        } catch (const std::invalid_argument &) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // Horner evaluation.
    {
        ISO_CHECK_EQ_DBL(poly::evaluate(P{1.0, 2.0, 3.0}, 2.0), 17.0, 1e-9);
        ISO_CHECK_EQ_DBL(poly::evaluate(poly::zero(), 5.0), 0.0, 1e-12);
    }

    // GCD of (x^2 - 1) and (x - 1) is a scalar multiple of (x - 1).
    {
        P a = {-1.0, 0.0, 1.0};
        P b = {-1.0, 1.0};
        P g = poly::gcd(a, b);
        ISO_CHECK_EQ_UINT(g.size(), 2);
        ISO_CHECK_EQ_DBL(poly::evaluate(g, 1.0), 0.0, 1e-9);
        ISO_CHECK(poly::modulo(a, g).empty()); // divides a exactly
    }

    return ISO_TEST_RESULT();
}
