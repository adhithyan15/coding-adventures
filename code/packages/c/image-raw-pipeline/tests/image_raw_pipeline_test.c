/*
 * Tests for the C image-raw-pipeline library, using the header-only iso_test.h
 * harness (pure ISO). Expected values mirror the Rust crate's own unit tests;
 * the sRGB reference numbers were computed independently (IEC 61966-2-1).
 */
#include "iso_test.h"

#include <stdlib.h> /* free */

#include "image_raw_pipeline.h"

static double dabs(double x) { return x < 0 ? -x : x; }

/* Identity and R<->B swap 3x3 matrices, used throughout. */
static const double ID[3][3] = {{1, 0, 0}, {0, 1, 0}, {0, 0, 1}};
static const double SWAP[3][3] = {{0, 0, 1}, {0, 1, 0}, {1, 0, 0}};
static const double NEUTRAL_WB[3] = {1.0, 1.0, 1.0};

int main(void) {
    /* ── sRGB gamma ───────────────────────────────────────────────────── */
    ISO_CHECK(dabs(irp_srgb_gamma(0.0)) < 1e-15);
    ISO_CHECK(dabs(irp_srgb_gamma(1.0) - 1.0) < 1e-10);
    /* linear segment is exact: V = 12.92*L for L <= 0.0031308 */
    ISO_CHECK(dabs(irp_srgb_gamma(0.001) - 12.92 * 0.001) < 1e-15);
    ISO_CHECK(dabs(irp_srgb_gamma(0.0031308) - 12.92 * 0.0031308) < 1e-12);
    /* power segment: reference values from the IEC standard */
    ISO_CHECK_EQ_DBL(irp_srgb_gamma(0.5), 0.735356983052, 1e-9);
    ISO_CHECK_EQ_DBL(irp_srgb_gamma(0.004), 0.050708713977, 1e-9);
    ISO_CHECK_EQ_DBL(irp_srgb_gamma(0.18), 0.461356129500, 1e-9);
    ISO_CHECK(irp_srgb_gamma(-0.01) < 0.0); /* negative not clamped */
    /* monotone increasing over [0,1] */
    {
        double prev = irp_srgb_gamma(0.0);
        for (int i = 1; i <= 100; i++) {
            double v = irp_srgb_gamma((double)i / 100.0);
            ISO_CHECK(v > prev);
            prev = v;
        }
    }

    /* ── sRGB decode ──────────────────────────────────────────────────── */
    ISO_CHECK(dabs(irp_srgb_decode(0.0)) < 1e-15);
    ISO_CHECK(dabs(irp_srgb_decode(1.0) - 1.0) < 1e-10);
    ISO_CHECK(dabs(irp_srgb_decode(0.02) - 0.02 / 12.92) < 1e-15);
    ISO_CHECK(dabs(irp_srgb_decode(0.04045) - 0.04045 / 12.92) < 1e-12);
    ISO_CHECK_EQ_DBL(irp_srgb_decode(0.5), 0.214041140482, 1e-9);
    ISO_CHECK_EQ_DBL(irp_srgb_decode(0.05), 0.003935939504, 1e-9);

    /* ── round trips (decode∘gamma == id and gamma∘decode == id) ──────── */
    for (int i = 0; i <= 50; i++) {
        double x = (double)i / 50.0;
        ISO_CHECK(dabs(irp_srgb_decode(irp_srgb_gamma(x)) - x) < 1e-10);
        ISO_CHECK(dabs(irp_srgb_gamma(irp_srgb_decode(x)) - x) < 1e-10);
    }

    /* ── mat3x3_mul ───────────────────────────────────────────────────── */
    {
        double out[3];
        double v[3] = {3.0, 5.0, 7.0};
        irp_mat3x3_mul(ID, v, out);
        ISO_CHECK(out[0] == 3.0 && out[1] == 5.0 && out[2] == 7.0);

        double z[3][3] = {{0, 0, 0}, {0, 0, 0}, {0, 0, 0}};
        double v2[3] = {1, 2, 3};
        irp_mat3x3_mul(z, v2, out);
        ISO_CHECK(out[0] == 0 && out[1] == 0 && out[2] == 0);

        double vs[3] = {1, 2, 3};
        irp_mat3x3_mul(SWAP, vs, out); /* R<->B swap */
        ISO_CHECK(out[0] == 3.0 && out[1] == 2.0 && out[2] == 1.0);

        double known[3][3] = {{1, 2, 3}, {4, 5, 6}, {7, 8, 9}};
        double e0[3] = {1, 0, 0};
        irp_mat3x3_mul(known, e0, out); /* first column */
        ISO_CHECK(dabs(out[0] - 1) < 1e-12 && dabs(out[1] - 4) < 1e-12 &&
                  dabs(out[2] - 7) < 1e-12);

        double scale[3][3] = {{2, 0, 0}, {0, 3, 0}, {0, 0, 4}};
        double ones[3] = {1, 1, 1};
        irp_mat3x3_mul(scale, ones, out);
        ISO_CHECK(dabs(out[0] - 2) < 1e-12 && dabs(out[1] - 3) < 1e-12 &&
                  dabs(out[2] - 4) < 1e-12);
    }

    /* ── invert_3x3 ───────────────────────────────────────────────────── */
    {
        double inv[3][3];
        ISO_CHECK(irp_invert_3x3(ID, inv) == 1);
        for (int r = 0; r < 3; r++)
            for (int c = 0; c < 3; c++)
                ISO_CHECK(dabs(inv[r][c] - (r == c ? 1.0 : 0.0)) < 1e-12);

        double diag[3][3] = {{2, 0, 0}, {0, 3, 0}, {0, 0, 4}};
        ISO_CHECK(irp_invert_3x3(diag, inv) == 1);
        ISO_CHECK(dabs(inv[0][0] - 0.5) < 1e-12);
        ISO_CHECK(dabs(inv[1][1] - 1.0 / 3.0) < 1e-12);
        ISO_CHECK(dabs(inv[2][2] - 0.25) < 1e-12);

        /* the R<->B swap is its own inverse */
        ISO_CHECK(irp_invert_3x3(SWAP, inv) == 1);
        for (int r = 0; r < 3; r++)
            for (int c = 0; c < 3; c++)
                ISO_CHECK(dabs(inv[r][c] - SWAP[r][c]) < 1e-12);

        /* singular matrices return 0 */
        double zero[3][3] = {{0, 0, 0}, {0, 0, 0}, {0, 0, 0}};
        ISO_CHECK(irp_invert_3x3(zero, inv) == 0);
        double rankdef[3][3] = {{1, 2, 3}, {1, 2, 3}, {1, 2, 3}};
        ISO_CHECK(irp_invert_3x3(rankdef, inv) == 0);

        /* M * inv(M) == I for a typical camera colour matrix */
        double cam[3][3] = {{1.392, -0.418, 0.026},
                            {-0.254, 1.614, -0.360},
                            {0.068, -0.584, 1.516}};
        ISO_CHECK(irp_invert_3x3(cam, inv) == 1);
        for (int i = 0; i < 3; i++) {
            double e[3] = {0, 0, 0};
            e[i] = 1.0;
            double t1[3], t2[3];
            irp_mat3x3_mul(inv, e, t1);
            irp_mat3x3_mul(cam, t1, t2);
            for (int j = 0; j < 3; j++)
                ISO_CHECK(dabs(t2[j] - (i == j ? 1.0 : 0.0)) < 1e-8);
        }
    }

    /* ── full pipeline ────────────────────────────────────────────────── */
    {
        IrpRgb8 *out = NULL;

        /* empty input -> empty (NULL) output */
        ISO_CHECK(irp_apply_color_pipeline(NULL, 0, 0, 65535, NEUTRAL_WB, ID,
                                           &out) == IRP_OK);
        ISO_CHECK(out == NULL);

        /* pure white -> (255,255,255) */
        IrpRaw16 white[1] = {{65535, 65535, 65535}};
        ISO_CHECK(irp_apply_color_pipeline(white, 1, 0, 65535, NEUTRAL_WB, ID,
                                           &out) == IRP_OK);
        ISO_CHECK(out[0].r == 255 && out[0].g == 255 && out[0].b == 255);
        free(out);

        /* pure black -> (0,0,0) */
        IrpRaw16 black[1] = {{0, 0, 0}};
        ISO_CHECK(irp_apply_color_pipeline(black, 1, 0, 65535, NEUTRAL_WB, ID,
                                           &out) == IRP_OK);
        ISO_CHECK(out[0].r == 0 && out[0].g == 0 && out[0].b == 0);
        free(out);

        /* black-level subtraction: input == black level -> 0 */
        IrpRaw16 mid[1] = {{32768, 32768, 32768}};
        ISO_CHECK(irp_apply_color_pipeline(mid, 1, 32768, 65535, NEUTRAL_WB, ID,
                                           &out) == IRP_OK);
        ISO_CHECK(out[0].r == 0 && out[0].g == 0 && out[0].b == 0);
        free(out);

        /* below black level clamps to 0 (12-bit sensor) */
        IrpRaw16 dark[1] = {{100, 100, 100}};
        ISO_CHECK(irp_apply_color_pipeline(dark, 1, 512, 4095, NEUTRAL_WB, ID,
                                           &out) == IRP_OK);
        ISO_CHECK(out[0].r == 0 && out[0].g == 0 && out[0].b == 0);
        free(out);

        /* 12-bit full scale -> 255 */
        IrpRaw16 full12[1] = {{4095, 4095, 4095}};
        ISO_CHECK(irp_apply_color_pipeline(full12, 1, 0, 4095, NEUTRAL_WB, ID,
                                           &out) == IRP_OK);
        ISO_CHECK(out[0].r == 255 && out[0].g == 255 && out[0].b == 255);
        free(out);

        /* white balance: 2x on red saturates it, green stays mid */
        double wb_red[3] = {2.0, 1.0, 1.0};
        ISO_CHECK(irp_apply_color_pipeline(mid, 1, 0, 65535, wb_red, ID,
                                           &out) == IRP_OK);
        ISO_CHECK(out[0].r == 255);
        ISO_CHECK(out[0].g < 200);
        free(out);

        /* neutral WB mid-grey: all channels equal */
        ISO_CHECK(irp_apply_color_pipeline(mid, 1, 0, 65535, NEUTRAL_WB, ID,
                                           &out) == IRP_OK);
        ISO_CHECK(out[0].r == out[0].g && out[0].g == out[0].b);
        free(out);

        /* colour matrix swaps R and B: pure red -> pure blue */
        IrpRaw16 red[1] = {{65535, 0, 0}};
        ISO_CHECK(irp_apply_color_pipeline(red, 1, 0, 65535, NEUTRAL_WB, SWAP,
                                           &out) == IRP_OK);
        ISO_CHECK(out[0].r == 0 && out[0].g == 0 && out[0].b == 255);
        free(out);

        /* identity matrix preserves channels */
        ISO_CHECK(irp_apply_color_pipeline(red, 1, 0, 65535, NEUTRAL_WB, ID,
                                           &out) == IRP_OK);
        ISO_CHECK(out[0].r == 255 && out[0].g == 0 && out[0].b == 0);
        free(out);

        /* multiple pixels: primaries map to primaries */
        IrpRaw16 prim[3] = {{65535, 0, 0}, {0, 65535, 0}, {0, 0, 65535}};
        ISO_CHECK(irp_apply_color_pipeline(prim, 3, 0, 65535, NEUTRAL_WB, ID,
                                           &out) == IRP_OK);
        ISO_CHECK(out[0].r == 255 && out[0].g == 0 && out[0].b == 0);
        ISO_CHECK(out[1].r == 0 && out[1].g == 255 && out[1].b == 0);
        ISO_CHECK(out[2].r == 0 && out[2].g == 0 && out[2].b == 255);
        free(out);

        /* overexposure clamps to 255, never wraps */
        IrpRaw16 bright[1] = {{50000, 50000, 50000}};
        double wb3[3] = {3.0, 3.0, 3.0};
        ISO_CHECK(irp_apply_color_pipeline(bright, 1, 0, 65535, wb3, ID,
                                           &out) == IRP_OK);
        ISO_CHECK(out[0].r == 255 && out[0].g == 255 && out[0].b == 255);
        free(out);

        /* larger image: correct length, no crash */
        {
            enum { N = 1000 };
            IrpRaw16 big[N];
            for (int i = 0; i < N; i++) {
                uint16_t v = (uint16_t)((i * 65) % 65535);
                big[i].r = v;
                big[i].g = v;
                big[i].b = v;
            }
            ISO_CHECK(irp_apply_color_pipeline(big, N, 0, 65535, NEUTRAL_WB, ID,
                                               &out) == IRP_OK);
            ISO_CHECK(out != NULL);
            free(out);
        }
    }

    return ISO_TEST_RESULT();
}
