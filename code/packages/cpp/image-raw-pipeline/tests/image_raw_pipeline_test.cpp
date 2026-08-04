// Tests for the C++ image-raw-pipeline library, using the header-only
// iso_test.h harness (pure ISO). Expected values mirror the Rust crate's own
// unit tests; the sRGB reference numbers are from IEC 61966-2-1.
#include "iso_test.h"

#include "image_raw_pipeline.hpp"

namespace irp = ca::image_raw_pipeline;
using irp::Mat3;
using irp::Raw16;
using irp::Rgb8;
using irp::Vec3;

static double dabs(double x) { return x < 0 ? -x : x; }

static const Mat3 ID = {{{1, 0, 0}, {0, 1, 0}, {0, 0, 1}}};
static const Mat3 SWAP = {{{0, 0, 1}, {0, 1, 0}, {1, 0, 0}}};
static const Vec3 NEUTRAL = {1.0, 1.0, 1.0};

int main() {
    // ── sRGB gamma ───────────────────────────────────────────────────────
    ISO_CHECK(dabs(irp::srgb_gamma(0.0)) < 1e-15);
    ISO_CHECK(dabs(irp::srgb_gamma(1.0) - 1.0) < 1e-10);
    ISO_CHECK(dabs(irp::srgb_gamma(0.001) - 12.92 * 0.001) < 1e-15);
    ISO_CHECK(dabs(irp::srgb_gamma(0.0031308) - 12.92 * 0.0031308) < 1e-12);
    ISO_CHECK_EQ_DBL(irp::srgb_gamma(0.5), 0.735356983052, 1e-9);
    ISO_CHECK_EQ_DBL(irp::srgb_gamma(0.004), 0.050708713977, 1e-9);
    ISO_CHECK_EQ_DBL(irp::srgb_gamma(0.18), 0.461356129500, 1e-9);
    ISO_CHECK(irp::srgb_gamma(-0.01) < 0.0);
    {
        double prev = irp::srgb_gamma(0.0);
        for (int i = 1; i <= 100; i++) {
            double v = irp::srgb_gamma(static_cast<double>(i) / 100.0);
            ISO_CHECK(v > prev);
            prev = v;
        }
    }

    // ── sRGB decode ──────────────────────────────────────────────────────
    ISO_CHECK(dabs(irp::srgb_decode(0.0)) < 1e-15);
    ISO_CHECK(dabs(irp::srgb_decode(1.0) - 1.0) < 1e-10);
    ISO_CHECK(dabs(irp::srgb_decode(0.02) - 0.02 / 12.92) < 1e-15);
    ISO_CHECK(dabs(irp::srgb_decode(0.04045) - 0.04045 / 12.92) < 1e-12);
    ISO_CHECK_EQ_DBL(irp::srgb_decode(0.5), 0.214041140482, 1e-9);
    ISO_CHECK_EQ_DBL(irp::srgb_decode(0.05), 0.003935939504, 1e-9);

    // ── round trips ──────────────────────────────────────────────────────
    for (int i = 0; i <= 50; i++) {
        double x = static_cast<double>(i) / 50.0;
        ISO_CHECK(dabs(irp::srgb_decode(irp::srgb_gamma(x)) - x) < 1e-10);
        ISO_CHECK(dabs(irp::srgb_gamma(irp::srgb_decode(x)) - x) < 1e-10);
    }

    // ── mat3x3_mul ───────────────────────────────────────────────────────
    {
        Vec3 out = irp::mat3x3_mul(ID, {3.0, 5.0, 7.0});
        ISO_CHECK(out[0] == 3.0 && out[1] == 5.0 && out[2] == 7.0);
        Mat3 z = {{{0, 0, 0}, {0, 0, 0}, {0, 0, 0}}};
        out = irp::mat3x3_mul(z, {1, 2, 3});
        ISO_CHECK(out[0] == 0 && out[1] == 0 && out[2] == 0);
        out = irp::mat3x3_mul(SWAP, {1, 2, 3});
        ISO_CHECK(out[0] == 3.0 && out[1] == 2.0 && out[2] == 1.0);
        Mat3 known = {{{1, 2, 3}, {4, 5, 6}, {7, 8, 9}}};
        out = irp::mat3x3_mul(known, {1, 0, 0});
        ISO_CHECK(dabs(out[0] - 1) < 1e-12 && dabs(out[1] - 4) < 1e-12 &&
                  dabs(out[2] - 7) < 1e-12);
        Mat3 scale = {{{2, 0, 0}, {0, 3, 0}, {0, 0, 4}}};
        out = irp::mat3x3_mul(scale, {1, 1, 1});
        ISO_CHECK(dabs(out[0] - 2) < 1e-12 && dabs(out[1] - 3) < 1e-12 &&
                  dabs(out[2] - 4) < 1e-12);
    }

    // ── invert_3x3 ───────────────────────────────────────────────────────
    {
        auto inv = irp::invert_3x3(ID);
        ISO_CHECK(inv.has_value());
        for (int r = 0; r < 3; r++)
            for (int c = 0; c < 3; c++)
                ISO_CHECK(dabs((*inv)[r][c] - (r == c ? 1.0 : 0.0)) < 1e-12);

        Mat3 diag = {{{2, 0, 0}, {0, 3, 0}, {0, 0, 4}}};
        inv = irp::invert_3x3(diag);
        ISO_CHECK(inv.has_value());
        ISO_CHECK(dabs((*inv)[0][0] - 0.5) < 1e-12);
        ISO_CHECK(dabs((*inv)[1][1] - 1.0 / 3.0) < 1e-12);
        ISO_CHECK(dabs((*inv)[2][2] - 0.25) < 1e-12);

        inv = irp::invert_3x3(SWAP);  // self-inverse
        ISO_CHECK(inv.has_value());
        for (int r = 0; r < 3; r++)
            for (int c = 0; c < 3; c++)
                ISO_CHECK(dabs((*inv)[r][c] - SWAP[r][c]) < 1e-12);

        Mat3 zero = {{{0, 0, 0}, {0, 0, 0}, {0, 0, 0}}};
        ISO_CHECK(!irp::invert_3x3(zero).has_value());
        Mat3 rankdef = {{{1, 2, 3}, {1, 2, 3}, {1, 2, 3}}};
        ISO_CHECK(!irp::invert_3x3(rankdef).has_value());

        Mat3 cam = {{{1.392, -0.418, 0.026},
                     {-0.254, 1.614, -0.360},
                     {0.068, -0.584, 1.516}}};
        inv = irp::invert_3x3(cam);
        ISO_CHECK(inv.has_value());
        for (int i = 0; i < 3; i++) {
            Vec3 e = {0, 0, 0};
            e[static_cast<std::size_t>(i)] = 1.0;
            Vec3 result = irp::mat3x3_mul(cam, irp::mat3x3_mul(*inv, e));
            for (int j = 0; j < 3; j++)
                ISO_CHECK(dabs(result[static_cast<std::size_t>(j)] -
                               (i == j ? 1.0 : 0.0)) < 1e-8);
        }
    }

    // ── full pipeline ────────────────────────────────────────────────────
    {
        // empty input -> empty output
        ISO_CHECK(irp::apply_color_pipeline({}, 0, 65535, NEUTRAL, ID).empty());

        // pure white -> (255,255,255)
        auto out = irp::apply_color_pipeline({{65535, 65535, 65535}}, 0, 65535,
                                             NEUTRAL, ID);
        ISO_CHECK(out[0] == (Rgb8{255, 255, 255}));

        // pure black -> (0,0,0)
        out = irp::apply_color_pipeline({{0, 0, 0}}, 0, 65535, NEUTRAL, ID);
        ISO_CHECK(out[0] == (Rgb8{0, 0, 0}));

        // black-level subtraction
        out = irp::apply_color_pipeline({{32768, 32768, 32768}}, 32768, 65535,
                                        NEUTRAL, ID);
        ISO_CHECK(out[0] == (Rgb8{0, 0, 0}));

        // below black level clamps to 0
        out = irp::apply_color_pipeline({{100, 100, 100}}, 512, 4095, NEUTRAL,
                                        ID);
        ISO_CHECK(out[0] == (Rgb8{0, 0, 0}));

        // 12-bit full scale -> 255
        out = irp::apply_color_pipeline({{4095, 4095, 4095}}, 0, 4095, NEUTRAL,
                                        ID);
        ISO_CHECK(out[0] == (Rgb8{255, 255, 255}));

        // white balance 2x red saturates it
        out = irp::apply_color_pipeline({{32768, 32768, 32768}}, 0, 65535,
                                        {2.0, 1.0, 1.0}, ID);
        ISO_CHECK(out[0].r == 255);
        ISO_CHECK(out[0].g < 200);

        // neutral WB mid-grey: channels equal
        out = irp::apply_color_pipeline({{32768, 32768, 32768}}, 0, 65535,
                                        NEUTRAL, ID);
        ISO_CHECK(out[0].r == out[0].g && out[0].g == out[0].b);

        // colour matrix swaps R and B
        out = irp::apply_color_pipeline({{65535, 0, 0}}, 0, 65535, NEUTRAL,
                                        SWAP);
        ISO_CHECK(out[0] == (Rgb8{0, 0, 255}));

        // identity preserves channels
        out = irp::apply_color_pipeline({{65535, 0, 0}}, 0, 65535, NEUTRAL, ID);
        ISO_CHECK(out[0] == (Rgb8{255, 0, 0}));

        // multiple pixels: primaries map to primaries
        out = irp::apply_color_pipeline(
            {{65535, 0, 0}, {0, 65535, 0}, {0, 0, 65535}}, 0, 65535, NEUTRAL,
            ID);
        ISO_CHECK(out[0] == (Rgb8{255, 0, 0}));
        ISO_CHECK(out[1] == (Rgb8{0, 255, 0}));
        ISO_CHECK(out[2] == (Rgb8{0, 0, 255}));

        // overexposure clamps to 255
        out = irp::apply_color_pipeline({{50000, 50000, 50000}}, 0, 65535,
                                        {3.0, 3.0, 3.0}, ID);
        ISO_CHECK(out[0] == (Rgb8{255, 255, 255}));

        // larger image: correct length
        std::vector<Raw16> big;
        for (int i = 0; i < 1000; i++) {
            std::uint16_t v = static_cast<std::uint16_t>((i * 65) % 65535);
            big.push_back(Raw16{v, v, v});
        }
        out = irp::apply_color_pipeline(big, 0, 65535, NEUTRAL, ID);
        ISO_CHECK(out.size() == 1000);
    }

    return ISO_TEST_RESULT();
}
