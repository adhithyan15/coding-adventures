/*
 * Tests for the C device-physics library, using the header-only iso_test.h
 * harness (pure ISO). Reference values were captured from the real Rust crate
 * (an oracle run), so the closed-form models match exactly.
 */
#include "iso_test.h"

#include "device_physics.h"

/* Relative closeness for physical quantities (our from-scratch exp/ln/sqrt are
 * accurate to ~1e-12, so 1e-8 relative is comfortable). */
#define REL(actual, expected) ((actual) / (expected))

int main(void) {
    /* ── thermal voltage & intrinsic concentration ───────────────────────── */
    ISO_CHECK_EQ_DBL(dp_thermal_voltage(300.0), 0.025851999786, 1e-9);
    {
        double ni;
        ISO_CHECK(dp_intrinsic_concentration(300.0, &ni) == DP_OK);
        ISO_CHECK_EQ_DBL(ni, 1.0e16, 1.0); /* exact 300 K value */
        ISO_CHECK(dp_intrinsic_concentration(400.0, &ni) == DP_OK);
        ISO_CHECK_EQ_DBL(REL(ni, 3.4618208669e18), 1.0, 1e-7);
        ISO_CHECK(dp_intrinsic_concentration(250.0, &ni) == DP_OK);
        ISO_CHECK_EQ_DBL(REL(ni, 9.9933464020e13), 1.0, 1e-7);
        /* Below 100 K is out of model validity. */
        ISO_CHECK(dp_intrinsic_concentration(50.0, &ni) == DP_ERR_TEMP_RANGE);
    }

    /* ── Fermi potential ─────────────────────────────────────────────────── */
    {
        double phi;
        ISO_CHECK(dp_fermi_potential(1e23, "p", 300.0, &phi) == DP_OK);
        ISO_CHECK_EQ_DBL(phi, 0.416685005326, 1e-9);
        ISO_CHECK(dp_fermi_potential(1e22, "n", 300.0, &phi) == DP_OK);
        ISO_CHECK_EQ_DBL(phi, -0.357158575994, 1e-9);
        ISO_CHECK(dp_fermi_potential(-1.0, "p", 300.0, &phi) == DP_ERR_INVALID);
        ISO_CHECK(dp_fermi_potential(1e23, "x", 300.0, &phi) == DP_ERR_INVALID);
    }

    /* ── PN junction ─────────────────────────────────────────────────────── */
    {
        DpPNJunction j;
        ISO_CHECK(dp_pn_new(1e23, 1e22, 1e-8, 300.0, 1e-6, 1e-6, &j) == DP_OK);
        ISO_CHECK_EQ_DBL(dp_pn_built_in_voltage(&j), 0.773843581320, 1e-9);
        ISO_CHECK_EQ_DBL(REL(dp_pn_depletion_width(&j, 0.0), 3.3177986927e-7),
                         1.0, 1e-7);
        ISO_CHECK_EQ_DBL(REL(dp_pn_depletion_width(&j, -5.0), 9.0626654139e-7),
                         1.0, 1e-7);
        /* Forward bias >= V_bi clamps the width to 0. */
        ISO_CHECK_EQ_DBL(dp_pn_depletion_width(&j, 1.0), 0.0, 1e-30);
        ISO_CHECK_EQ_DBL(REL(dp_pn_saturation_current(&j), 6.5903922005e-16),
                         1.0, 1e-7);
        ISO_CHECK_EQ_DBL(REL(dp_pn_current(&j, 0.6), 7.9153045828e-6), 1.0,
                         1e-6);
        /* Zero bias -> zero current (exp(0)-1 == 0). */
        ISO_CHECK_EQ_DBL(dp_pn_current(&j, 0.0), 0.0, 1e-30);

        DpPNJunction bad;
        ISO_CHECK(dp_pn_new(-1e23, 1e22, 1e-8, 300.0, 1e-6, 1e-6, &bad) ==
                  DP_ERR_INVALID);
        ISO_CHECK(dp_pn_new(1e23, 1e22, 0.0, 300.0, 1e-6, 1e-6, &bad) ==
                  DP_ERR_INVALID);
    }

    /* ── MOSFET ──────────────────────────────────────────────────────────── */
    {
        DpMOSFET m;
        ISO_CHECK(dp_mos_new(DP_NMOS, 130e-9, 1e-6, 2e-9, 1e24, -0.05, 0.0,
                             300.0, &m) == DP_OK);
        ISO_CHECK_EQ_DBL(REL(dp_mos_c_ox(&m), 1.7265666235e-2), 1.0, 1e-7);
        ISO_CHECK_EQ_DBL(dp_mos_v_fb(&m), -0.05, 1e-12);
        ISO_CHECK_EQ_DBL(dp_mos_phi_f(&m), 0.476211434659, 1e-9);
        ISO_CHECK_EQ_DBL(dp_mos_gamma(&m), 0.333698419163, 1e-9);

        double vt;
        ISO_CHECK(dp_mos_threshold_voltage(&m, 0.0, &vt) == DP_OK);
        ISO_CHECK_EQ_DBL(vt, 1.228086347363, 1e-8);
        ISO_CHECK(dp_mos_threshold_voltage(&m, 1.0, &vt) == DP_OK);
        ISO_CHECK_EQ_DBL(vt, 1.368696754373, 1e-8); /* body effect raises V_t */

        /* V_SB below -2*phi_F forward-biases the body-source junction. */
        ISO_CHECK(dp_mos_threshold_voltage(&m, -2.0, &vt) == DP_ERR_BODY_FORWARD);

        DpMOSFET bad;
        ISO_CHECK(dp_mos_new(DP_NMOS, -1.0, 1e-6, 2e-9, 1e24, 0.0, 0.0, 300.0,
                             &bad) == DP_ERR_INVALID);
        ISO_CHECK(dp_mos_new(DP_PMOS, 130e-9, 1e-6, 0.0, 1e24, 0.0, 0.0, 300.0,
                             &bad) == DP_ERR_INVALID);
    }

    /* ── overflow inputs terminate (no d_ln infinite loop / DoS) ─────────── */
    {
        /* na*nd overflows to +inf -> (na*nd)/n_i^2 == +inf -> ln(+inf). Must
         * return (a finite/inf sentinel), not hang. */
        DpPNJunction j;
        ISO_CHECK(dp_pn_new(1e200, 1e200, 1.0, 300.0, 1e-6, 1e-6, &j) == DP_OK);
        double vbi = dp_pn_built_in_voltage(&j); /* completes, no hang */
        ISO_CHECK(vbi == vbi);                   /* a value (not stuck) */
        /* huge T overflows n_i -> ratio underflows to +0 -> ln(+0). */
        DpPNJunction j2;
        ISO_CHECK(dp_pn_new(1e21, 1e21, 1.0, 1e250, 1e-6, 1e-6, &j2) == DP_OK);
        double vbi2 = dp_pn_built_in_voltage(&j2); /* completes */
        (void)vbi2;
    }

    return ISO_TEST_RESULT();
}
