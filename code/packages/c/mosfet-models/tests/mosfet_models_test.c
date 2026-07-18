/*
 * mosfet_models_test.c — tests for the SPICE Level-1 MOSFET model.
 * ===========================================================================
 *
 * Mirrors the Rust integration tests: parameter defaults, region detection
 * (cutoff / subthreshold / triode / saturation), the sign of gm/gds/gmb, the
 * body effect, PMOS sign conventions, the capacitance model, and a saturation
 * Id cross-check. Pure arithmetic (no allocation), verified under ASan+UBSan.
 * The thermal voltage and sqrt/exp come from the composed pure-ISO
 * device-physics and float-math packages.
 */
#include "mosfet_models/mosfet_models.h"
#include "iso_test.h"

static MosLevel1Params defaults(void) {
    MosLevel1Params p;
    mos_level1_params_default(&p);
    return p;
}

/* Rust: test_default_params. */
static void test_default_params(void) {
    MosLevel1Params p = defaults();
    ISO_CHECK_EQ_DBL(p.vt0, 0.42, 1e-12);
    ISO_CHECK_EQ_DBL(p.kp, 220e-6, 1e-9);
    ISO_CHECK_EQ_DBL(p.lambda, 0.05, 1e-12);
    ISO_CHECK_EQ_DBL(p.gamma, 0.27, 1e-12);
    ISO_CHECK_EQ_DBL(p.phi, 0.84, 1e-12);
    ISO_CHECK_EQ_DBL(p.w, 1e-6, 1e-15);
    ISO_CHECK_EQ_DBL(p.l, 130e-9, 1e-15);
    ISO_CHECK(p.subthreshold_enable != 0);
}

/* Rust: test_saturation_region. */
static void test_saturation_region(void) {
    MosLevel1Params p = defaults();
    MosResult r = mos_evaluate_level1(&p, 1.8, 1.8, 0.0, 300.15);
    ISO_CHECK_EQ_INT(r.region, MOS_REGION_SATURATION);
    ISO_CHECK(r.id > 0.0);
}

/* Rust: test_triode_region. */
static void test_triode_region(void) {
    MosLevel1Params p = defaults();
    MosResult r = mos_evaluate_level1(&p, 1.8, 0.1, 0.0, 300.15);
    ISO_CHECK_EQ_INT(r.region, MOS_REGION_TRIODE);
    ISO_CHECK(r.id > 0.0);
}

/* Rust: test_cutoff_region. */
static void test_cutoff_region(void) {
    MosLevel1Params p = defaults();
    MosResult r;
    p.subthreshold_enable = 0;
    r = mos_evaluate_level1(&p, 0.0, 1.0, 0.0, 300.15);
    ISO_CHECK_EQ_INT(r.region, MOS_REGION_CUTOFF);
    ISO_CHECK(r.id == 0.0);
    ISO_CHECK(r.gm == 0.0);
}

/* Rust: test_subthreshold_region. */
static void test_subthreshold_region(void) {
    MosLevel1Params p = defaults();
    MosResult r = mos_evaluate_level1(&p, 0.2, 1.0, 0.0, 300.15);
    ISO_CHECK_EQ_INT(r.region, MOS_REGION_SUBTHRESHOLD);
    ISO_CHECK(r.id > 0.0 && r.id < 1e-5);
}

/* Rust: test_gm_positive_in_saturation / test_gds_positive_in_saturation. */
static void test_gm_gds_positive(void) {
    MosLevel1Params p = defaults();
    MosResult r = mos_evaluate_level1(&p, 1.8, 1.8, 0.0, 300.15);
    ISO_CHECK(r.gm > 0.0);
    ISO_CHECK(r.gds > 0.0); /* channel-length modulation */
}

/* Rust: test_gds_without_clm. */
static void test_gds_without_clm(void) {
    MosLevel1Params p = defaults();
    MosResult r;
    double ag;
    p.lambda = 0.0;
    r = mos_evaluate_level1(&p, 1.8, 1.8, 0.0, 300.15);
    ag = r.gds < 0.0 ? -r.gds : r.gds;
    ISO_CHECK(ag < 1e-12); /* gds ~ 0 when lambda = 0 */
}

/* Rust: test_body_effect_raises_vt. */
static void test_body_effect(void) {
    MosLevel1Params p = defaults();
    MosResult r0 = mos_evaluate_level1(&p, 1.2, 1.2, 0.0, 300.15);
    MosResult r1 = mos_evaluate_level1(&p, 1.2, 1.2, -1.0, 300.15);
    ISO_CHECK(r1.id < r0.id); /* reverse body bias raises V_t, lowers Id */
}

/* Rust: test_gmb_nonzero_with_body_bias. */
static void test_gmb_nonzero(void) {
    MosLevel1Params p = defaults();
    MosResult r = mos_evaluate_level1(&p, 1.8, 1.8, -0.5, 300.15);
    ISO_CHECK(r.gmb > 0.0);
}

/* Rust: test_pmos_dc_negative_id. */
static void test_pmos_negative_id(void) {
    MosLevel1Params p = defaults();
    MosResult r = mosfet_dc(MOS_PMOS, &p, -1.8, -1.8, 0.0, 300.15);
    ISO_CHECK(r.id < 0.0);
    ISO_CHECK_EQ_INT(r.region, MOS_REGION_SATURATION);
}

/* Rust: test_nmos_pmos_magnitude_match. */
static void test_nmos_pmos_match(void) {
    MosLevel1Params p = defaults();
    MosResult rn = mosfet_dc(MOS_NMOS, &p, 1.8, 1.8, 0.0, 300.15);
    MosResult rp = mosfet_dc(MOS_PMOS, &p, -1.8, -1.8, 0.0, 300.15);
    double diff = (rn.id + rp.id) < 0.0 ? -(rn.id + rp.id) : (rn.id + rp.id);
    ISO_CHECK(diff < 1e-12);
}

/* Rust: test_level1_model_dc (Level1Model.dc == NMOS evaluate). */
static void test_level1_model_dc(void) {
    MosLevel1Params p = defaults();
    MosResult r = mosfet_dc(MOS_NMOS, &p, 1.8, 1.8, 0.0, 300.15);
    ISO_CHECK_EQ_INT(r.region, MOS_REGION_SATURATION);
    ISO_CHECK(r.id > 0.0);
}

/* Rust: test_region_as_str. */
static void test_region_as_str(void) {
    ISO_CHECK_STR_EQ(mos_region_str(MOS_REGION_CUTOFF), "cutoff");
    ISO_CHECK_STR_EQ(mos_region_str(MOS_REGION_SUBTHRESHOLD), "subthreshold");
    ISO_CHECK_STR_EQ(mos_region_str(MOS_REGION_TRIODE), "triode");
    ISO_CHECK_STR_EQ(mos_region_str(MOS_REGION_SATURATION), "saturation");
}

/* Rust: test_capacitances_nonnegative. */
static void test_capacitances_nonnegative(void) {
    MosLevel1Params p = defaults();
    double pts[4][2] = {{0.0, 0.0}, {0.2, 0.5}, {1.2, 0.1}, {1.8, 1.8}};
    int i;
    for (i = 0; i < 4; i++) {
        MosResult r = mos_evaluate_level1(&p, pts[i][0], pts[i][1], 0.0, 300.15);
        ISO_CHECK(r.cgs >= 0.0);
        ISO_CHECK(r.cgd >= 0.0);
        ISO_CHECK(r.cgb >= 0.0);
    }
}

/* Rust: test_overlap_caps_scale_with_w. */
static void test_overlap_caps_scale(void) {
    MosLevel1Params p = defaults();
    MosResult r;
    p.cgso = 5e-10;
    p.w = 2e-6;
    r = mos_evaluate_level1(&p, 1.8, 1.8, 0.0, 300.15);
    ISO_CHECK(r.cgs > 5e-10 * 2e-6); /* includes the overlap cap */
}

/* Rust: test_saturation_id_formula (lambda = 0 → Id = (beta/2) V_OV^2). */
static void test_saturation_id_formula(void) {
    MosLevel1Params p = defaults();
    double v_gs = 1.8, v_ov, beta, expected, rel_err;
    MosResult r;
    p.lambda = 0.0;
    v_ov = v_gs - p.vt0; /* V_BS = 0 → V_t = VT0 */
    beta = p.kp * (p.w / p.l);
    expected = (beta / 2.0) * v_ov * v_ov;
    r = mos_evaluate_level1(&p, v_gs, v_gs, 0.0, 300.15);
    rel_err = (r.id - expected) < 0.0 ? -(r.id - expected) : (r.id - expected);
    rel_err /= expected;
    ISO_CHECK(rel_err < 1e-9);
}

int main(void) {
    test_default_params();
    test_saturation_region();
    test_triode_region();
    test_cutoff_region();
    test_subthreshold_region();
    test_gm_gds_positive();
    test_gds_without_clm();
    test_body_effect();
    test_gmb_nonzero();
    test_pmos_negative_id();
    test_nmos_pmos_match();
    test_level1_model_dc();
    test_region_as_str();
    test_capacitances_nonnegative();
    test_overlap_caps_scale();
    test_saturation_id_formula();
    return ISO_TEST_RESULT();
}
