// image_raw_pipeline.hpp — shared RAW colour-development pipeline, header-only
// in pure ISO C++17 (namespace ca::image_raw_pipeline). A faithful port of the
// Rust `image-raw-pipeline` crate.
// ===========================================================================
//
// Camera RAW formats all need the same four-stage colour pipeline to turn raw
// sensor values into a displayable sRGB image:
//
//     normalize (black level) -> white balance -> colour matrix -> sRGB gamma
//
// API: srgb_gamma / srgb_decode (the IEC 61966-2-1 transfer functions),
// mat3x3_mul, invert_3x3, apply_color_pipeline.
//
// DIVERGENCE FROM RUST. `invert_3x3` returns std::optional (Rust `Option`);
// `apply_color_pipeline` returns std::vector<Rgb8> (Rust `Vec`). Data types
// mirror the Rust arrays: Mat3 == std::array<std::array<double,3>,3>,
// Vec3 == std::array<double,3>.
//
// NO <cmath>. The sRGB transfer functions need a fractional pow; it is computed
// from scratch below, matching the Rust f64 powf to ~1e-12 relative. Pure ISO
// C++17: compiles under GCC, Clang and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors.
#ifndef CA_IMAGE_RAW_PIPELINE_HPP
#define CA_IMAGE_RAW_PIPELINE_HPP

#include <array>
#include <cstdint>
#include <optional>
#include <vector>

namespace ca {
namespace image_raw_pipeline {

using Vec3 = std::array<double, 3>;
using Mat3 = std::array<std::array<double, 3>, 3>;

// One raw sensor pixel (three 16-bit channels) and one developed sRGB pixel.
struct Raw16 {
    std::uint16_t r, g, b;
};
struct Rgb8 {
    std::uint8_t r, g, b;
};

inline bool operator==(const Rgb8& a, const Rgb8& b) {
    return a.r == b.r && a.g == b.g && a.b == b.b;
}

namespace detail {

inline double d_abs(double x) { return x < 0.0 ? -x : x; }

inline double pow2i(int k) {
    double result = 1.0;
    double base = k < 0 ? 0.5 : 2.0;
    int n = k < 0 ? -k : k;
    while (n > 0) {
        if (n & 1) result *= base;
        base *= base;
        n >>= 1;
    }
    return result;
}

inline double d_exp(double x) {
    if (x != x) return x;
    if (x == 0.0) return 1.0;
    if (x > 709.782712893384) return 1.7976931348623157e308;
    if (x < -745.13321910194) return 0.0;
    constexpr double INV_LN2 = 1.4426950408889634;
    constexpr double C1 = 0.693359375;
    constexpr double C2 = -2.1219444005469058277e-4;
    double kf = x * INV_LN2;
    int k = static_cast<int>(kf >= 0.0 ? kf + 0.5 : kf - 0.5);
    double r = (x - static_cast<double>(k) * C1) - static_cast<double>(k) * C2;
    double term = 1.0, sum = 1.0;
    for (int i = 1; i <= 17; i++) {
        term *= r / static_cast<double>(i);
        sum += term;
    }
    return sum * pow2i(k);
}

inline double d_ln(double x) {
    if (x != x) return x;
    if (x <= 0.0) return -1.7976931348623157e308;
    if (x > 1.7976931348623157e308) return 1.7976931348623157e308;
    int e = 0;
    double m = x;
    while (m < 1.0) { m *= 2.0; e--; }
    while (m >= 2.0) { m *= 0.5; e++; }
    double u = (m - 1.0) / (m + 1.0);
    double u2 = u * u;
    double term = u, sum = u;
    for (int n = 1; n <= 60; n++) {
        term *= u2;
        double add = term / static_cast<double>(2 * n + 1);
        sum += add;
        if (d_abs(add) < 1e-17) break;
    }
    constexpr double LN2 = 0.6931471805599453;
    return static_cast<double>(e) * LN2 + 2.0 * sum;
}

// x^y for x > 0 (the only case the sRGB transfer functions hit).
inline double d_pow_pos(double x, double y) { return d_exp(y * d_ln(x)); }

inline double clamp01(double v) {
    if (v < 0.0) return 0.0;
    if (v > 1.0) return 1.0;
    return v;
}

inline std::uint32_t sat_sub(std::uint32_t a, std::uint32_t b) {
    return a > b ? a - b : 0;
}

}  // namespace detail

// ── sRGB transfer functions ──────────────────────────────────────────────────

// sRGB EOTF: linear light -> display encoding (fixed points 0->0, 1->1;
// out-of-range inputs are not clamped).
inline double srgb_gamma(double linear) {
    if (linear <= 0.0031308) return 12.92 * linear;
    return 1.055 * detail::d_pow_pos(linear, 1.0 / 2.4) - 0.055;
}

// Inverse sRGB EOTF: display encoding -> linear light.
inline double srgb_decode(double encoded) {
    if (encoded <= 0.04045) return encoded / 12.92;
    return detail::d_pow_pos((encoded + 0.055) / 1.055, 2.4);
}

// ── 3x3 matrix ops ───────────────────────────────────────────────────────────

// out[i] = sum_j m[i][j] * v[j].
inline Vec3 mat3x3_mul(const Mat3& m, const Vec3& v) {
    return {m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
            m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
            m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2]};
}

// Analytic 3x3 inverse via Cramer's rule; std::nullopt when |det| < 1e-12.
inline std::optional<Mat3> invert_3x3(const Mat3& m) {
    double a = m[0][0], b = m[0][1], c = m[0][2];
    double d = m[1][0], e = m[1][1], f = m[1][2];
    double g = m[2][0], h = m[2][1], k = m[2][2];

    double ek_fh = e * k - f * h;
    double dk_fg = d * k - f * g;
    double dh_eg = d * h - e * g;
    double bk_ch = b * k - c * h;
    double ak_cg = a * k - c * g;
    double ah_bg = a * h - b * g;
    double bf_ce = b * f - c * e;
    double af_cd = a * f - c * d;
    double ae_bd = a * e - b * d;

    double det = a * ek_fh - b * dk_fg + c * dh_eg;
    if (detail::d_abs(det) < 1e-12) return std::nullopt;

    double inv = 1.0 / det;
    return Mat3{{{ek_fh * inv, -bk_ch * inv, bf_ce * inv},
                {-dk_fg * inv, ak_cg * inv, -af_cd * inv},
                {dh_eg * inv, -ah_bg * inv, ae_bd * inv}}};
}

// ── Full pipeline ────────────────────────────────────────────────────────────

// Develop raw pixels into sRGB (one Rgb8 per input pixel).
inline std::vector<Rgb8> apply_color_pipeline(const std::vector<Raw16>& pixels,
                                              std::uint32_t black_level,
                                              std::uint32_t white_level,
                                              const Vec3& wb,
                                              const Mat3& color_matrix) {
    double effective_white =
        static_cast<double>(detail::sat_sub(white_level, black_level));
    if (effective_white < 1.0) effective_white = 1.0;

    auto to_u8 = [](double v) -> std::uint8_t {
        double scaled = srgb_gamma(v) * 255.0;         // v in [0,1] -> >= 0
        double rounded = static_cast<double>(          // round half away from 0
            static_cast<long long>(scaled + 0.5));
        if (rounded < 0.0) rounded = 0.0;
        if (rounded > 255.0) rounded = 255.0;
        return static_cast<std::uint8_t>(rounded);
    };

    std::vector<Rgb8> out;
    out.reserve(pixels.size());
    for (const Raw16& px : pixels) {
        Vec3 norm = {
            static_cast<double>(detail::sat_sub(px.r, black_level)) / effective_white,
            static_cast<double>(detail::sat_sub(px.g, black_level)) / effective_white,
            static_cast<double>(detail::sat_sub(px.b, black_level)) / effective_white};
        Vec3 bal = {norm[0] * wb[0], norm[1] * wb[1], norm[2] * wb[2]};
        Vec3 mixed = mat3x3_mul(color_matrix, bal);
        out.push_back(Rgb8{to_u8(detail::clamp01(mixed[0])),
                           to_u8(detail::clamp01(mixed[1])),
                           to_u8(detail::clamp01(mixed[2]))});
    }
    return out;
}

}  // namespace image_raw_pipeline
}  // namespace ca

#endif  // CA_IMAGE_RAW_PIPELINE_HPP
