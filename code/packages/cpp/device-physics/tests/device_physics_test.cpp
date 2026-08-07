// Tests for the C++ device-physics library, using the header-only iso_test.h
// harness (pure ISO). Reference values were captured from the real Rust crate.
#include "iso_test.h"

#include <stdexcept>

#include "device_physics.hpp"

namespace dp = ca::device_physics;
using dp::MosType;

#define REL(actual, expected) ((actual) / (expected))

template <typename F>
static bool throws(F fn) {
    try {
        fn();
    } catch (const std::exception&) {
        return true;
    }
    return false;
}

int main() {
    // ── thermal voltage & intrinsic concentration ─────────────────────────
    ISO_CHECK_EQ_DBL(dp::thermal_voltage(300.0), 0.025851999786, 1e-9);
    ISO_CHECK_EQ_DBL(dp::intrinsic_concentration(300.0), 1.0e16, 1.0);
    ISO_CHECK_EQ_DBL(REL(dp::intrinsic_concentration(400.0), 3.4618208669e18),
                     1.0, 1e-7);
    ISO_CHECK_EQ_DBL(REL(dp::intrinsic_concentration(250.0), 9.9933464020e13),
                     1.0, 1e-7);
    ISO_CHECK(throws([] { dp::intrinsic_concentration(50.0); }));

    // ── Fermi potential ───────────────────────────────────────────────────
    ISO_CHECK_EQ_DBL(dp::fermi_potential(1e23, "p", 300.0), 0.416685005326,
                     1e-9);
    ISO_CHECK_EQ_DBL(dp::fermi_potential(1e22, "n", 300.0), -0.357158575994,
                     1e-9);
    ISO_CHECK(throws([] { dp::fermi_potential(-1.0, "p", 300.0); }));
    ISO_CHECK(throws([] { dp::fermi_potential(1e23, "x", 300.0); }));

    // ── PN junction ───────────────────────────────────────────────────────
    {
        dp::PNJunction j(1e23, 1e22, 1e-8, 300.0, 1e-6, 1e-6);
        ISO_CHECK_EQ_DBL(j.built_in_voltage(), 0.773843581320, 1e-9);
        ISO_CHECK_EQ_DBL(REL(j.depletion_width(0.0), 3.3177986927e-7), 1.0,
                         1e-7);
        ISO_CHECK_EQ_DBL(REL(j.depletion_width(-5.0), 9.0626654139e-7), 1.0,
                         1e-7);
        ISO_CHECK_EQ_DBL(j.depletion_width(1.0), 0.0, 1e-30);
        ISO_CHECK_EQ_DBL(REL(j.saturation_current(), 6.5903922005e-16), 1.0,
                         1e-7);
        ISO_CHECK_EQ_DBL(REL(j.current(0.6), 7.9153045828e-6), 1.0, 1e-6);
        ISO_CHECK_EQ_DBL(j.current(0.0), 0.0, 1e-30);

        ISO_CHECK(throws([] { dp::PNJunction(-1e23, 1e22, 1e-8, 300, 1e-6, 1e-6); }));
        ISO_CHECK(throws([] { dp::PNJunction(1e23, 1e22, 0.0, 300, 1e-6, 1e-6); }));
    }

    // ── MOSFET ────────────────────────────────────────────────────────────
    {
        dp::MOSFETParams m(MosType::NMOS, 130e-9, 1e-6, 2e-9, 1e24, -0.05, 0.0,
                           300.0);
        ISO_CHECK_EQ_DBL(REL(m.c_ox(), 1.7265666235e-2), 1.0, 1e-7);
        ISO_CHECK_EQ_DBL(m.v_fb(), -0.05, 1e-12);
        ISO_CHECK_EQ_DBL(m.phi_f(), 0.476211434659, 1e-9);
        ISO_CHECK_EQ_DBL(m.gamma(), 0.333698419163, 1e-9);
        ISO_CHECK_EQ_DBL(m.threshold_voltage(0.0), 1.228086347363, 1e-8);
        ISO_CHECK_EQ_DBL(m.threshold_voltage(1.0), 1.368696754373, 1e-8);
        ISO_CHECK(throws([&] { m.threshold_voltage(-2.0); }));

        ISO_CHECK(throws([] {
            dp::MOSFETParams(MosType::NMOS, -1.0, 1e-6, 2e-9, 1e24, 0, 0, 300);
        }));
        ISO_CHECK(throws([] {
            dp::MOSFETParams(MosType::PMOS, 130e-9, 1e-6, 0.0, 1e24, 0, 0, 300);
        }));
    }

    // ── overflow inputs terminate (no d_ln infinite loop / DoS) ───────────
    {
        // na*nd overflows to +inf -> ln(+inf); huge T overflows n_i -> ln(+0).
        // Both must complete rather than hang.
        double vbi = dp::PNJunction(1e200, 1e200, 1.0, 300.0, 1e-6, 1e-6)
                         .built_in_voltage();
        ISO_CHECK(vbi == vbi);
        double vbi2 = dp::PNJunction(1e21, 1e21, 1.0, 1e250, 1e-6, 1e-6)
                          .built_in_voltage();
        (void)vbi2;
    }

    return ISO_TEST_RESULT();
}
