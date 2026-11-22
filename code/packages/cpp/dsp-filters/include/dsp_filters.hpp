// dsp_filters.hpp — FIR, IIR, Biquad, SOS, and Butterworth Filters (header-only, ISO C++17).
// ---------------------------------------------------------------------------
//
// Implements in namespace `ca::dsp_filters`:
// - FIR filtering (`fir`, `fir_fft`) and windowed-sinc FIR design
//   (`design_low_pass`, `design_high_pass`, `design_band_pass`, `design_band_stop`).
// - Direct-Form-II Transposed IIR filtering (`iir`, `filtfilt`) + `*_into`.
// - Failure-atomic `Biquad` with Audio EQ Cookbook constructors.
// - Second-Order Sections (`sosfilt`, `sosfiltfilt`) + `*_into`.
// - Bilinear-transform Butterworth filter design (`lowpass`, `highpass`,
//   `bandpass`, `bandstop`) as SOS cascades (orders 1..64) and `(b, a)` pairs.
//
// Hardened with full VER15 CWE/CVE protections:
// - CWE-20:  Upfront enum domain checks, finite float input & output checks,
//            a0 != 0 checks, range-checked double -> float narrowing.
// - CWE-190: Overflow-checked size_t arithmetic for convolution, padding, FFT.
// - CWE-400: Defensive caps MAX_SAMPLES (2^24), MAX_TAPS (2^20), MAX_ORDER (1024).
// - CWE-476: Null-pointer guards on `*_into` pointer-based overloads.
// - CWE-674: Strictly iterative execution (zero recursion).
#ifndef CA_DSP_FILTERS_HPP
#define CA_DSP_FILTERS_HPP

#include <algorithm>
#include <array>
#include <cfloat>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <exception>
#include <string>
#include <utility>
#include <vector>

namespace ca {
namespace dsp_filters {

inline constexpr std::size_t MAX_SAMPLES = std::size_t{1} << 24;        // 16,777,216
inline constexpr std::size_t MAX_TAPS = std::size_t{1} << 20;           // 1,048,576
inline constexpr std::size_t MAX_ORDER = 1024;
inline constexpr std::uint32_t MAX_BUTTER_ORDER = 64;

enum class Error {
    EmptySignal,
    EmptyKernel,
    InvalidCoefficient
};

class FilterError : public std::exception {
public:
    explicit FilterError(Error code) noexcept : code_(code) {}
    Error code() const noexcept { return code_; }
    const char* what() const noexcept override {
        switch (code_) {
            case Error::EmptySignal: return "dsp_filters: empty signal";
            case Error::EmptyKernel: return "dsp_filters: empty kernel";
            case Error::InvalidCoefficient: return "dsp_filters: invalid coefficient";
        }
        return "dsp_filters: error";
    }

private:
    Error code_;
};

enum class WindowType { Rectangular = 0, Hamming = 1, Hann = 2, Blackman = 3 };
enum class ButterworthType { Lowpass = 0, Highpass = 1, Bandpass = 2, Bandstop = 3 };

using SosSection = std::array<float, 6>; // [b0, b1, b2, a0, a1, a2]

namespace detail {

inline constexpr double PI = 3.14159265358979323846264338327950288;
inline constexpr double TWO_PI = 6.28318530717958647692528676655900577;

[[noreturn]] inline void fail(Error e) { throw FilterError(e); }

inline bool checked_add(std::size_t a, std::size_t b, std::size_t& out) noexcept {
    if (SIZE_MAX - a < b) return false;
    out = a + b;
    return true;
}

inline bool checked_mul(std::size_t a, std::size_t b, std::size_t& out) noexcept {
    if (a != 0 && b > SIZE_MAX / a) return false;
    out = a * b;
    return true;
}

inline bool is_valid_window(WindowType w) noexcept {
    return w == WindowType::Rectangular || w == WindowType::Hamming ||
           w == WindowType::Hann || w == WindowType::Blackman;
}

inline bool is_valid_butter_type(ButterworthType t) noexcept {
    return t == ButterworthType::Lowpass || t == ButterworthType::Highpass ||
           t == ButterworthType::Bandpass || t == ButterworthType::Bandstop;
}

inline float narrow_float(double v) {
    if (!std::isfinite(v) || std::abs(v) > static_cast<double>(FLT_MAX)) {
        fail(Error::InvalidCoefficient);
    }
    float f = static_cast<float>(v);
    if (!std::isfinite(f)) {
        fail(Error::InvalidCoefficient);
    }
    return f;
}

inline float checked_mul_f(float a, float b) {
    return narrow_float(static_cast<double>(a) * static_cast<double>(b));
}

inline float checked_add_f(float a, float b) {
    return narrow_float(static_cast<double>(a) + static_cast<double>(b));
}

inline float checked_sub_f(float a, float b) {
    return narrow_float(static_cast<double>(a) - static_cast<double>(b));
}

// Separately rounded binary32 DFII-T recurrence (no FMA/reassociation, DSP03).
inline float step_df2t_f32(float b0, float b1, float b2, float a1, float a2,
                           float input, float& cz1, float& cz2) {
    float b0x = checked_mul_f(b0, input);
    float y = checked_add_f(b0x, cz1);
    float b1x = checked_mul_f(b1, input);
    float a1y = checked_mul_f(a1, y);
    float z1_diff = checked_sub_f(b1x, a1y);
    float nz1 = checked_add_f(z1_diff, cz2);
    float b2x = checked_mul_f(b2, input);
    float a2y = checked_mul_f(a2, y);
    float nz2 = checked_sub_f(b2x, a2y);
    cz1 = nz1;
    cz2 = nz2;
    return y;
}

inline std::vector<float> narrow_vector(const std::vector<double>& src) {
    std::vector<float> out(src.size());
    for (std::size_t i = 0; i < src.size(); ++i) {
        out[i] = narrow_float(src[i]);
    }
    return out;
}

inline std::vector<double> validate_and_widen(const float* arr, std::size_t n,
                                              std::size_t max_len, Error empty_err) {
    if (n == 0) fail(empty_err);
    if (arr == nullptr || n > max_len) fail(Error::InvalidCoefficient);
    std::vector<double> out(n);
    for (std::size_t i = 0; i < n; ++i) {
        if (!std::isfinite(arr[i])) fail(Error::InvalidCoefficient);
        out[i] = static_cast<double>(arr[i]);
    }
    return out;
}

inline void fft_radix2_inplace(std::vector<double>& buf, std::size_t n, bool inverse) noexcept {
    std::size_t j = 0;
    for (std::size_t i = 1; i < n; ++i) {
        std::size_t bit = n >> 1;
        while (j & bit) {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if (i < j) {
            std::swap(buf[2 * i], buf[2 * j]);
            std::swap(buf[2 * i + 1], buf[2 * j + 1]);
        }
    }

    double sign = inverse ? 1.0 : -1.0;
    for (std::size_t len = 2; len <= n; len <<= 1) {
        std::size_t half = len >> 1;
        double angle_step = sign * TWO_PI / static_cast<double>(len);
        double wr_step = std::cos(angle_step);
        double wi_step = std::sin(angle_step);
        for (std::size_t i = 0; i < n; i += len) {
            double wr = 1.0;
            double wi = 0.0;
            for (std::size_t k = 0; k < half; ++k) {
                std::size_t idx1 = 2 * (i + k);
                std::size_t idx2 = 2 * (i + k + half);
                double ur = buf[idx1];
                double ui = buf[idx1 + 1];
                double vr = buf[idx2] * wr - buf[idx2 + 1] * wi;
                double vi = buf[idx2] * wi + buf[idx2 + 1] * wr;
                buf[idx1] = ur + vr;
                buf[idx1 + 1] = ui + vi;
                buf[idx2] = ur - vr;
                buf[idx2 + 1] = ui - vi;
                double next_wr = wr * wr_step - wi * wi_step;
                wi = wr * wi_step + wi * wr_step;
                wr = next_wr;
            }
        }
    }

    if (inverse) {
        double inv_n = 1.0 / static_cast<double>(n);
        for (double& v : buf) v *= inv_n;
    }
}

inline double eval_window(WindowType window, std::size_t k, double m) noexcept {
    if (m == 0.0) return 1.0;
    double n_f = static_cast<double>(k);
    switch (window) {
        case WindowType::Rectangular: return 1.0;
        case WindowType::Hamming:     return 0.54 - 0.46 * std::cos(TWO_PI * n_f / m);
        case WindowType::Hann:        return 0.5 * (1.0 - std::cos(TWO_PI * n_f / m));
        case WindowType::Blackman:
            return 0.42 - 0.5 * std::cos(TWO_PI * n_f / m) + 0.08 * std::cos(2.0 * TWO_PI * n_f / m);
    }
    return 1.0;
}

inline std::vector<double> design_low_pass_double(double fc, std::size_t n, WindowType window) {
    double m = static_cast<double>(n - 1);
    double centre = m * 0.5;
    std::vector<double> h(n);
    double sum = 0.0;
    for (std::size_t k = 0; k < n; ++k) {
        double x = static_cast<double>(k) - centre;
        double val = (x == 0.0) ? (2.0 * fc) : (std::sin(TWO_PI * fc * x) / (PI * x));
        h[k] = val * eval_window(window, k, m);
        sum += h[k];
    }
    if (std::abs(sum) > 0.0) {
        double inv = 1.0 / sum;
        for (double& v : h) v *= inv;
    }
    return h;
}

inline std::vector<double> design_band_pass_double(double f1, double f2, std::size_t n, WindowType window) {
    double m = static_cast<double>(n - 1);
    double centre = m * 0.5;
    std::vector<double> h(n);
    for (std::size_t k = 0; k < n; ++k) {
        double x = static_cast<double>(k) - centre;
        double val = (x == 0.0)
                     ? (2.0 * (f2 - f1))
                     : ((std::sin(TWO_PI * f2 * x) - std::sin(TWO_PI * f1 * x)) / (PI * x));
        h[k] = val * eval_window(window, k, m);
    }
    double f0 = 0.5 * (f1 + f2);
    double gain = 0.0;
    for (std::size_t k = 0; k < n; ++k) {
        double x = static_cast<double>(k) - centre;
        gain += h[k] * std::cos(TWO_PI * f0 * x);
    }
    double gain_abs = std::abs(gain);
    if (gain_abs > 0.0) {
        double inv = 1.0 / gain_abs;
        for (double& v : h) v *= inv;
    }
    return h;
}

inline std::vector<double> iir_run_double(const std::vector<double>& sig,
                                          const std::vector<double>& b,
                                          const std::vector<double>& a) {
    if (a[0] == 0.0) fail(Error::InvalidCoefficient);
    std::size_t order = std::max(b.size(), a.size()) - 1;
    if (order > MAX_ORDER) fail(Error::InvalidCoefficient);

    double inv_a0 = 1.0 / a[0];
    std::vector<double> b_scaled(order + 1, 0.0);
    std::vector<double> a_scaled(order + 1, 0.0);
    for (std::size_t k = 0; k < b.size(); ++k) {
        b_scaled[k] = b[k] * inv_a0;
        if (!std::isfinite(b_scaled[k])) fail(Error::InvalidCoefficient);
    }
    for (std::size_t k = 0; k < a.size(); ++k) {
        a_scaled[k] = a[k] * inv_a0;
        if (!std::isfinite(a_scaled[k])) fail(Error::InvalidCoefficient);
    }

    std::vector<double> state(order, 0.0);
    std::vector<double> out(sig.size());
    for (std::size_t i = 0; i < sig.size(); ++i) {
        double x = sig[i];
        double y = b_scaled[0] * x + (order > 0 ? state[0] : 0.0);
        if (!std::isfinite(y)) fail(Error::InvalidCoefficient);
        out[i] = y;
        if (order > 0) {
            for (std::size_t k = 0; k + 1 < order; ++k) {
                state[k] = b_scaled[k + 1] * x - a_scaled[k + 1] * y + state[k + 1];
            }
            state[order - 1] = b_scaled[order] * x - a_scaled[order] * y;
        }
    }
    return out;
}

inline SosSection make_sos_from_doubles(double b0, double b1, double b2,
                                        double a0, double a1, double a2) {
    if (a0 == 0.0 || !std::isfinite(a0)) fail(Error::InvalidCoefficient);
    double inv_a0 = 1.0 / a0;
    return SosSection{
        narrow_float(b0 * inv_a0),
        narrow_float(b1 * inv_a0),
        narrow_float(b2 * inv_a0),
        1.0f,
        narrow_float(a1 * inv_a0),
        narrow_float(a2 * inv_a0)
    };
}

inline void cplx_sqrt(double re, double im, double& sr, double& si) noexcept {
    double r = std::hypot(re, im);
    if (r == 0.0) {
        sr = 0.0;
        si = 0.0;
        return;
    }
    if (re >= 0.0) {
        double u = std::sqrt(0.5 * (r + re));
        sr = u;
        si = (u > 0.0) ? (0.5 * im / u) : 0.0;
    } else {
        double v = std::sqrt(0.5 * (r - re));
        if (im < 0.0) v = -v;
        si = v;
        sr = (v != 0.0) ? (0.5 * im / v) : 0.0;
    }
}

inline std::pair<SosSection, SosSection> emit_bp_bs_pair(double pr, double pi,
                                                         double b_band, double w0_sq,
                                                         bool is_bs) {
    double hr = 0.5 * pr * b_band;
    double hi = 0.5 * pi * b_band;
    double dr = (hr * hr - hi * hi) - w0_sq;
    double di = 2.0 * hr * hi;
    double sr = 0.0, si = 0.0;
    cplx_sqrt(dr, di, sr, si);
    double u1r = hr + sr, u1i = hi + si;
    double u2r = hr - sr, u2i = hi - si;
    double c1_a = -2.0 * u1r, c0_a = u1r * u1r + u1i * u1i;
    double c1_b = -2.0 * u2r, c0_b = u2r * u2r + u2i * u2i;
    if (!is_bs) {
        return {
            make_sos_from_doubles(b_band, 0.0, -b_band, 1.0 + c1_a + c0_a, 2.0 * (c0_a - 1.0), 1.0 - c1_a + c0_a),
            make_sos_from_doubles(b_band, 0.0, -b_band, 1.0 + c1_b + c0_b, 2.0 * (c0_b - 1.0), 1.0 - c1_b + c0_b)
        };
    }
    double nb0 = 1.0 + w0_sq, nb1 = 2.0 * (w0_sq - 1.0), nb2 = 1.0 + w0_sq;
    return {
        make_sos_from_doubles(nb0, nb1, nb2, 1.0 + c1_a + c0_a, 2.0 * (c0_a - 1.0), 1.0 - c1_a + c0_a),
        make_sos_from_doubles(nb0, nb1, nb2, 1.0 + c1_b + c0_b, 2.0 * (c0_b - 1.0), 1.0 - c1_b + c0_b)
    };
}

inline std::string trim_lower(const std::string& s) {
    std::size_t start = 0;
    while (start < s.size() && (s[start] == ' ' || s[start] == '\t' || s[start] == '\n' || s[start] == '\r')) {
        ++start;
    }
    std::size_t end = s.size();
    while (end > start && (s[end - 1] == ' ' || s[end - 1] == '\t' || s[end - 1] == '\n' || s[end - 1] == '\r')) {
        --end;
    }
    std::string out;
    out.reserve(end - start);
    for (std::size_t i = start; i < end; ++i) {
        char c = s[i];
        if (c >= 'A' && c <= 'Z') c = static_cast<char>(c - 'A' + 'a');
        out.push_back(c);
    }
    return out;
}

} // namespace detail

// ------------------------------------------------------------------ //
// FIR Filtering & Windowed-Sinc Design                               //
// ------------------------------------------------------------------ //

inline std::vector<float> fir(const std::vector<float>& signal, const std::vector<float>& kernel) {
    auto sig = detail::validate_and_widen(signal.data(), signal.size(), MAX_SAMPLES, Error::EmptySignal);
    auto ker = detail::validate_and_widen(kernel.data(), kernel.size(), MAX_TAPS, Error::EmptyKernel);
    std::size_t sum_len = 0;
    if (!detail::checked_add(sig.size(), ker.size(), sum_len) || sum_len == 0) {
        detail::fail(Error::InvalidCoefficient);
    }
    std::size_t total = sum_len - 1;
    if (total > MAX_SAMPLES) detail::fail(Error::InvalidCoefficient);

    std::vector<double> work(total, 0.0);
    for (std::size_t i = 0; i < total; ++i) {
        std::size_t j_lo = (i + 1 > sig.size()) ? (i + 1 - sig.size()) : 0;
        std::size_t j_hi = (i + 1 < ker.size()) ? (i + 1) : ker.size();
        double acc = 0.0;
        for (std::size_t j = j_lo; j < j_hi; ++j) {
            acc += ker[j] * sig[i - j];
        }
        work[i] = acc;
    }
    return detail::narrow_vector(work);
}

inline std::vector<float> fir_fft(const std::vector<float>& signal, const std::vector<float>& kernel) {
    auto sig = detail::validate_and_widen(signal.data(), signal.size(), MAX_SAMPLES, Error::EmptySignal);
    auto ker = detail::validate_and_widen(kernel.data(), kernel.size(), MAX_TAPS, Error::EmptyKernel);
    std::size_t sum_len = 0;
    if (!detail::checked_add(sig.size(), ker.size(), sum_len) || sum_len == 0) {
        detail::fail(Error::InvalidCoefficient);
    }
    std::size_t total = sum_len - 1;
    if (total > MAX_SAMPLES) detail::fail(Error::InvalidCoefficient);

    std::size_t fft_len = 2;
    while (fft_len < total) {
        if (fft_len > SIZE_MAX / 2) detail::fail(Error::InvalidCoefficient);
        fft_len <<= 1;
    }
    std::size_t two_fft = 0;
    if (fft_len > MAX_SAMPLES || !detail::checked_mul(fft_len, 2, two_fft)) {
        detail::fail(Error::InvalidCoefficient);
    }

    std::vector<double> a(two_fft, 0.0);
    std::vector<double> b(two_fft, 0.0);
    for (std::size_t i = 0; i < sig.size(); ++i) a[2 * i] = sig[i];
    for (std::size_t i = 0; i < ker.size(); ++i) b[2 * i] = ker[i];

    detail::fft_radix2_inplace(a, fft_len, false);
    detail::fft_radix2_inplace(b, fft_len, false);
    for (std::size_t i = 0; i < fft_len; ++i) {
        double ar = a[2 * i], ai = a[2 * i + 1];
        double br = b[2 * i], bi = b[2 * i + 1];
        a[2 * i] = ar * br - ai * bi;
        a[2 * i + 1] = ar * bi + ai * br;
    }
    detail::fft_radix2_inplace(a, fft_len, true);

    std::vector<float> out(total);
    for (std::size_t i = 0; i < total; ++i) {
        out[i] = detail::narrow_float(a[2 * i]);
    }
    return out;
}

inline std::vector<float> design_low_pass(float cutoff_norm, std::uint32_t num_taps,
                                          WindowType window = WindowType::Hamming) {
    if (!detail::is_valid_window(window)) detail::fail(Error::InvalidCoefficient);
    if (num_taps < 1u || (num_taps % 2u) == 0u || static_cast<std::size_t>(num_taps) > MAX_TAPS) {
        detail::fail(Error::InvalidCoefficient);
    }
    if (!std::isfinite(cutoff_norm) || cutoff_norm < 0.0f || cutoff_norm > 0.5f) {
        detail::fail(Error::InvalidCoefficient);
    }
    return detail::narrow_vector(detail::design_low_pass_double(
        static_cast<double>(cutoff_norm), static_cast<std::size_t>(num_taps), window));
}

inline std::vector<float> design_high_pass(float cutoff_norm, std::uint32_t num_taps,
                                           WindowType window = WindowType::Hamming) {
    if (!detail::is_valid_window(window)) detail::fail(Error::InvalidCoefficient);
    if (num_taps < 1u || (num_taps % 2u) == 0u || static_cast<std::size_t>(num_taps) > MAX_TAPS) {
        detail::fail(Error::InvalidCoefficient);
    }
    if (!std::isfinite(cutoff_norm) || cutoff_norm < 0.0f || cutoff_norm > 0.5f) {
        detail::fail(Error::InvalidCoefficient);
    }
    std::vector<double> h = detail::design_low_pass_double(
        static_cast<double>(cutoff_norm), static_cast<std::size_t>(num_taps), window);
    for (double& v : h) v = -v;
    h[(h.size() - 1) / 2] += 1.0;
    return detail::narrow_vector(h);
}

inline std::vector<float> design_band_pass(float low_norm, float high_norm, std::uint32_t num_taps,
                                           WindowType window = WindowType::Hamming) {
    if (!detail::is_valid_window(window)) detail::fail(Error::InvalidCoefficient);
    if (num_taps < 1u || (num_taps % 2u) == 0u || static_cast<std::size_t>(num_taps) > MAX_TAPS) {
        detail::fail(Error::InvalidCoefficient);
    }
    if (!std::isfinite(low_norm) || !std::isfinite(high_norm)) detail::fail(Error::InvalidCoefficient);
    if (!(low_norm >= 0.0f && low_norm < high_norm && high_norm <= 0.5f)) {
        detail::fail(Error::InvalidCoefficient);
    }
    return detail::narrow_vector(detail::design_band_pass_double(
        static_cast<double>(low_norm), static_cast<double>(high_norm),
        static_cast<std::size_t>(num_taps), window));
}

inline std::vector<float> design_band_stop(float low_norm, float high_norm, std::uint32_t num_taps,
                                           WindowType window = WindowType::Hamming) {
    if (!detail::is_valid_window(window)) detail::fail(Error::InvalidCoefficient);
    if (num_taps < 1u || (num_taps % 2u) == 0u || static_cast<std::size_t>(num_taps) > MAX_TAPS) {
        detail::fail(Error::InvalidCoefficient);
    }
    if (!std::isfinite(low_norm) || !std::isfinite(high_norm)) detail::fail(Error::InvalidCoefficient);
    if (!(low_norm >= 0.0f && low_norm < high_norm && high_norm <= 0.5f)) {
        detail::fail(Error::InvalidCoefficient);
    }
    std::vector<double> h = detail::design_band_pass_double(
        static_cast<double>(low_norm), static_cast<double>(high_norm),
        static_cast<std::size_t>(num_taps), window);
    for (double& v : h) v = -v;
    h[(h.size() - 1) / 2] += 1.0;
    return detail::narrow_vector(h);
}

// ------------------------------------------------------------------ //
// Direct & Zero-Phase IIR Filtering                                  //
// ------------------------------------------------------------------ //

inline std::vector<float> iir(const std::vector<float>& signal,
                              const std::vector<float>& b,
                              const std::vector<float>& a) {
    auto sig = detail::validate_and_widen(signal.data(), signal.size(), MAX_SAMPLES, Error::EmptySignal);
    auto bd = detail::validate_and_widen(b.data(), b.size(), MAX_TAPS, Error::EmptyKernel);
    auto ad = detail::validate_and_widen(a.data(), a.size(), MAX_TAPS, Error::EmptyKernel);
    return detail::narrow_vector(detail::iir_run_double(sig, bd, ad));
}

inline void iir_into(const float* signal, std::size_t sig_len,
                     const float* b, std::size_t b_len,
                     const float* a, std::size_t a_len,
                     float* out) {
    if (out == nullptr) detail::fail(Error::InvalidCoefficient);
    auto sig = detail::validate_and_widen(signal, sig_len, MAX_SAMPLES, Error::EmptySignal);
    auto bd = detail::validate_and_widen(b, b_len, MAX_TAPS, Error::EmptyKernel);
    auto ad = detail::validate_and_widen(a, a_len, MAX_TAPS, Error::EmptyKernel);
    std::vector<float> tmp = detail::narrow_vector(detail::iir_run_double(sig, bd, ad));
    std::memcpy(out, tmp.data(), sig_len * sizeof(float));
}

namespace detail {

inline std::vector<float> filtfilt_core(const std::vector<double>& sig,
                                        const std::vector<double>& bd,
                                        const std::vector<double>& ad) {
    std::size_t n = sig.size();
    std::size_t max_coeff_len = std::max(bd.size(), ad.size());
    std::size_t pad_len = 3 * max_coeff_len;
    bool use_padding = n > pad_len;
    std::size_t work_len = n;
    if (use_padding) {
        std::size_t two_pad = 0;
        if (!checked_mul(pad_len, 2, two_pad) ||
            !checked_add(n, two_pad, work_len) ||
            work_len > MAX_SAMPLES) {
            fail(Error::InvalidCoefficient);
        }
    }

    std::vector<double> padded(work_len);
    if (use_padding) {
        double x0 = sig[0];
        double x_end = sig[n - 1];
        for (std::size_t i = 0; i < pad_len; ++i) padded[i] = 2.0 * x0 - sig[pad_len - i];
        for (std::size_t i = 0; i < n; ++i) padded[pad_len + i] = sig[i];
        for (std::size_t i = 0; i < pad_len; ++i) padded[pad_len + n + i] = 2.0 * x_end - sig[n - 2 - i];
    } else {
        padded = sig;
    }

    std::vector<double> fwd = iir_run_double(padded, bd, ad);
    std::reverse(fwd.begin(), fwd.end());
    std::vector<double> bwd = iir_run_double(fwd, bd, ad);

    std::vector<float> out(n);
    for (std::size_t i = 0; i < n; ++i) {
        std::size_t idx = use_padding ? (work_len - 1 - (pad_len + i)) : (n - 1 - i);
        out[i] = narrow_float(bwd[idx]);
    }
    return out;
}

} // namespace detail

inline std::vector<float> filtfilt(const std::vector<float>& signal,
                                   const std::vector<float>& b,
                                   const std::vector<float>& a) {
    auto sig = detail::validate_and_widen(signal.data(), signal.size(), MAX_SAMPLES, Error::EmptySignal);
    auto bd = detail::validate_and_widen(b.data(), b.size(), MAX_TAPS, Error::EmptyKernel);
    auto ad = detail::validate_and_widen(a.data(), a.size(), MAX_TAPS, Error::EmptyKernel);
    return detail::filtfilt_core(sig, bd, ad);
}

inline void filtfilt_into(const float* signal, std::size_t sig_len,
                          const float* b, std::size_t b_len,
                          const float* a, std::size_t a_len,
                          float* out) {
    if (out == nullptr) detail::fail(Error::InvalidCoefficient);
    auto sig = detail::validate_and_widen(signal, sig_len, MAX_SAMPLES, Error::EmptySignal);
    auto bd = detail::validate_and_widen(b, b_len, MAX_TAPS, Error::EmptyKernel);
    auto ad = detail::validate_and_widen(a, a_len, MAX_TAPS, Error::EmptyKernel);
    std::vector<float> tmp = detail::filtfilt_core(sig, bd, ad);
    std::memcpy(out, tmp.data(), sig_len * sizeof(float));
}

// ------------------------------------------------------------------ //
// Biquad (Failure-Atomic Checked DFII-T)                             //
// ------------------------------------------------------------------ //

class Biquad {
public:
    Biquad(float b0, float b1, float b2, float a0, float a1, float a2) {
        for (float v : {b0, b1, b2, a0, a1, a2}) {
            if (!std::isfinite(v)) detail::fail(Error::InvalidCoefficient);
        }
        if (a0 == 0.0f) detail::fail(Error::InvalidCoefficient);
        double inv_a0 = 1.0 / static_cast<double>(a0);
        b0_ = detail::narrow_float(static_cast<double>(b0) * inv_a0);
        b1_ = detail::narrow_float(static_cast<double>(b1) * inv_a0);
        b2_ = detail::narrow_float(static_cast<double>(b2) * inv_a0);
        a1_ = detail::narrow_float(static_cast<double>(a1) * inv_a0);
        a2_ = detail::narrow_float(static_cast<double>(a2) * inv_a0);
    }

    explicit Biquad(const SosSection& sec)
        : Biquad(sec[0], sec[1], sec[2], sec[3], sec[4], sec[5]) {}

    float b0() const noexcept { return b0_; }
    float b1() const noexcept { return b1_; }
    float b2() const noexcept { return b2_; }
    float a1() const noexcept { return a1_; }
    float a2() const noexcept { return a2_; }
    float z1() const noexcept { return z1_; }
    float z2() const noexcept { return z2_; }

    void reset() noexcept {
        z1_ = 0.0f;
        z2_ = 0.0f;
    }

    float process_sample(float input) {
        if (!std::isfinite(input)) detail::fail(Error::InvalidCoefficient);
        float nz1 = z1_;
        float nz2 = z2_;
        float y = advance(input, nz1, nz2);
        z1_ = nz1;
        z2_ = nz2;
        return y;
    }

    std::vector<float> process(const std::vector<float>& signal) {
        if (signal.empty()) detail::fail(Error::EmptySignal);
        if (signal.size() > MAX_SAMPLES) detail::fail(Error::InvalidCoefficient);
        float nz1 = z1_;
        float nz2 = z2_;
        std::vector<float> out(signal.size());
        for (std::size_t i = 0; i < signal.size(); ++i) {
            if (!std::isfinite(signal[i])) detail::fail(Error::InvalidCoefficient);
            out[i] = advance(signal[i], nz1, nz2);
        }
        z1_ = nz1;
        z2_ = nz2;
        return out;
    }

    void process_into(const float* signal, std::size_t sig_len, float* out) {
        if (out == nullptr) detail::fail(Error::InvalidCoefficient);
        if (sig_len == 0) detail::fail(Error::EmptySignal);
        if (signal == nullptr || sig_len > MAX_SAMPLES) detail::fail(Error::InvalidCoefficient);
        float nz1 = z1_;
        float nz2 = z2_;
        std::vector<float> tmp(sig_len);
        for (std::size_t i = 0; i < sig_len; ++i) {
            if (!std::isfinite(signal[i])) detail::fail(Error::InvalidCoefficient);
            tmp[i] = advance(signal[i], nz1, nz2);
        }
        std::memcpy(out, tmp.data(), sig_len * sizeof(float));
        z1_ = nz1;
        z2_ = nz2;
    }

    static Biquad lowpass(float cutoff_norm, float q = 0.7071067811865475f) {
        validate_cookbook(cutoff_norm, q);
        double w = detail::TWO_PI * static_cast<double>(cutoff_norm);
        double s = std::sin(w), c = std::cos(w), alpha = s / (2.0 * static_cast<double>(q));
        return from_doubles(0.5 * (1.0 - c), 1.0 - c, 0.5 * (1.0 - c), 1.0 + alpha, -2.0 * c, 1.0 - alpha);
    }

    static Biquad highpass(float cutoff_norm, float q = 0.7071067811865475f) {
        validate_cookbook(cutoff_norm, q);
        double w = detail::TWO_PI * static_cast<double>(cutoff_norm);
        double s = std::sin(w), c = std::cos(w), alpha = s / (2.0 * static_cast<double>(q));
        return from_doubles(0.5 * (1.0 + c), -(1.0 + c), 0.5 * (1.0 + c), 1.0 + alpha, -2.0 * c, 1.0 - alpha);
    }

    static Biquad bandpass(float center_norm, float q = 1.0f) {
        validate_cookbook(center_norm, q);
        double w = detail::TWO_PI * static_cast<double>(center_norm);
        double s = std::sin(w), c = std::cos(w), alpha = s / (2.0 * static_cast<double>(q));
        return from_doubles(0.5 * s, 0.0, -0.5 * s, 1.0 + alpha, -2.0 * c, 1.0 - alpha);
    }

    static Biquad notch(float center_norm, float q = 1.0f) {
        validate_cookbook(center_norm, q);
        double w = detail::TWO_PI * static_cast<double>(center_norm);
        double s = std::sin(w), c = std::cos(w), alpha = s / (2.0 * static_cast<double>(q));
        return from_doubles(1.0, -2.0 * c, 1.0, 1.0 + alpha, -2.0 * c, 1.0 - alpha);
    }

    static Biquad allpass(float center_norm, float q = 1.0f) {
        validate_cookbook(center_norm, q);
        double w = detail::TWO_PI * static_cast<double>(center_norm);
        double s = std::sin(w), c = std::cos(w), alpha = s / (2.0 * static_cast<double>(q));
        return from_doubles(1.0 - alpha, -2.0 * c, 1.0 + alpha, 1.0 + alpha, -2.0 * c, 1.0 - alpha);
    }

    static Biquad peak(float center_norm, float gain_db, float q = 1.0f) {
        validate_cookbook(center_norm, q);
        if (!std::isfinite(gain_db)) detail::fail(Error::InvalidCoefficient);
        double a_gain = std::pow(10.0, static_cast<double>(gain_db) / 40.0);
        if (!std::isfinite(a_gain) || a_gain <= 0.0) detail::fail(Error::InvalidCoefficient);
        double w = detail::TWO_PI * static_cast<double>(center_norm);
        double s = std::sin(w), c = std::cos(w), alpha = s / (2.0 * static_cast<double>(q));
        return from_doubles(1.0 + alpha * a_gain, -2.0 * c, 1.0 - alpha * a_gain,
                            1.0 + alpha / a_gain, -2.0 * c, 1.0 - alpha / a_gain);
    }

private:
    float b0_ = 0.0f;
    float b1_ = 0.0f;
    float b2_ = 0.0f;
    float a1_ = 0.0f;
    float a2_ = 0.0f;
    float z1_ = 0.0f;
    float z2_ = 0.0f;

    static void validate_cookbook(float freq_norm, float q) {
        if (!std::isfinite(freq_norm) || freq_norm <= 0.0f || freq_norm >= 0.5f) {
            detail::fail(Error::InvalidCoefficient);
        }
        if (!std::isfinite(q) || q <= 0.0f) {
            detail::fail(Error::InvalidCoefficient);
        }
    }

    static Biquad from_doubles(double b0, double b1, double b2, double a0, double a1, double a2) {
        if (a0 == 0.0 || !std::isfinite(a0)) detail::fail(Error::InvalidCoefficient);
        double inv = 1.0 / a0;
        return Biquad(detail::narrow_float(b0 * inv), detail::narrow_float(b1 * inv),
                      detail::narrow_float(b2 * inv), 1.0f,
                      detail::narrow_float(a1 * inv), detail::narrow_float(a2 * inv));
    }

    float advance(float input, float& cz1, float& cz2) const {
        return detail::step_df2t_f32(b0_, b1_, b2_, a1_, a2_, input, cz1, cz2);
    }
};

// ------------------------------------------------------------------ //
// Second-Order Sections (SOS) Direct & Zero-Phase Filtering          //
// ------------------------------------------------------------------ //

namespace detail {

inline std::vector<double> sos_run_doubles(const std::vector<Biquad>& filters, const std::vector<double>& in) {
    if (filters.empty()) return in;
    std::size_t m = filters.size();
    std::vector<float> z1(m, 0.0f);
    std::vector<float> z2(m, 0.0f);
    std::vector<double> out(in.size());
    for (std::size_t i = 0; i < in.size(); ++i) {
        float v = narrow_float(in[i]);
        for (std::size_t s = 0; s < m; ++s) {
            const Biquad& bq = filters[s];
            v = step_df2t_f32(bq.b0(), bq.b1(), bq.b2(), bq.a1(), bq.a2(), v, z1[s], z2[s]);
        }
        out[i] = static_cast<double>(v);
    }
    return out;
}

} // namespace detail

inline std::vector<float> sosfilt(const std::vector<SosSection>& sections,
                                  const std::vector<float>& signal) {
    auto sig = detail::validate_and_widen(signal.data(), signal.size(), MAX_SAMPLES, Error::EmptySignal);
    if (sections.size() > MAX_ORDER) detail::fail(Error::InvalidCoefficient);
    std::vector<Biquad> filters;
    filters.reserve(sections.size());
    for (const auto& s : sections) filters.emplace_back(s);
    return detail::narrow_vector(detail::sos_run_doubles(filters, sig));
}

inline void sosfilt_into(const SosSection* sections, std::size_t num_sections,
                         const float* signal, std::size_t sig_len, float* out) {
    if (out == nullptr) detail::fail(Error::InvalidCoefficient);
    auto sig = detail::validate_and_widen(signal, sig_len, MAX_SAMPLES, Error::EmptySignal);
    if (num_sections > MAX_ORDER) detail::fail(Error::InvalidCoefficient);
    if (num_sections > 0 && sections == nullptr) detail::fail(Error::InvalidCoefficient);
    std::vector<Biquad> filters;
    filters.reserve(num_sections);
    for (std::size_t i = 0; i < num_sections; ++i) filters.emplace_back(sections[i]);
    std::vector<float> tmp = detail::narrow_vector(detail::sos_run_doubles(filters, sig));
    std::memcpy(out, tmp.data(), sig_len * sizeof(float));
}

namespace detail {

inline std::vector<float> sosfiltfilt_core(const std::vector<Biquad>& filters,
                                           const std::vector<double>& sig) {
    if (filters.empty()) return narrow_vector(sig);

    std::size_t n = sig.size();
    std::size_t pad_len = 3 * (2 * filters.size() + 1);
    bool use_padding = n > pad_len;
    std::size_t work_len = n;
    if (use_padding) {
        std::size_t two_pad = 0;
        if (!checked_mul(pad_len, 2, two_pad) ||
            !checked_add(n, two_pad, work_len) ||
            work_len > MAX_SAMPLES) {
            fail(Error::InvalidCoefficient);
        }
    }

    std::vector<double> padded(work_len);
    if (use_padding) {
        double x0 = sig[0], x_end = sig[n - 1];
        for (std::size_t i = 0; i < pad_len; ++i) padded[i] = 2.0 * x0 - sig[pad_len - i];
        for (std::size_t i = 0; i < n; ++i) padded[pad_len + i] = sig[i];
        for (std::size_t i = 0; i < pad_len; ++i) padded[pad_len + n + i] = 2.0 * x_end - sig[n - 2 - i];
    } else {
        padded = sig;
    }

    std::vector<double> fwd = sos_run_doubles(filters, padded);
    std::reverse(fwd.begin(), fwd.end());
    std::vector<double> bwd = sos_run_doubles(filters, fwd);

    std::vector<float> out(n);
    for (std::size_t i = 0; i < n; ++i) {
        std::size_t idx = use_padding ? (work_len - 1 - (pad_len + i)) : (n - 1 - i);
        out[i] = narrow_float(bwd[idx]);
    }
    return out;
}

} // namespace detail

inline std::vector<float> sosfiltfilt(const std::vector<SosSection>& sections,
                                      const std::vector<float>& signal) {
    auto sig = detail::validate_and_widen(signal.data(), signal.size(), MAX_SAMPLES, Error::EmptySignal);
    if (sections.size() > MAX_ORDER) detail::fail(Error::InvalidCoefficient);
    std::vector<Biquad> filters;
    filters.reserve(sections.size());
    for (const auto& s : sections) filters.emplace_back(s);
    return detail::sosfiltfilt_core(filters, sig);
}

inline void sosfiltfilt_into(const SosSection* sections, std::size_t num_sections,
                             const float* signal, std::size_t sig_len, float* out) {
    if (out == nullptr) detail::fail(Error::InvalidCoefficient);
    auto sig = detail::validate_and_widen(signal, sig_len, MAX_SAMPLES, Error::EmptySignal);
    if (num_sections > MAX_ORDER) detail::fail(Error::InvalidCoefficient);
    if (num_sections > 0 && sections == nullptr) detail::fail(Error::InvalidCoefficient);
    std::vector<Biquad> filters;
    filters.reserve(num_sections);
    for (std::size_t i = 0; i < num_sections; ++i) filters.emplace_back(sections[i]);
    std::vector<float> tmp = detail::sosfiltfilt_core(filters, sig);
    std::memcpy(out, tmp.data(), sig_len * sizeof(float));
}

// ------------------------------------------------------------------ //
// Butterworth Filter Design (LP / HP / BP / BS)                      //
// ------------------------------------------------------------------ //

inline std::vector<SosSection> butterworth_sos(ButterworthType type, std::uint32_t order,
                                               float low_cutoff_norm, float high_cutoff_norm = 0.0f) {
    if (!detail::is_valid_butter_type(type)) detail::fail(Error::InvalidCoefficient);
    if (order == 0u || order > MAX_BUTTER_ORDER) detail::fail(Error::InvalidCoefficient);

    if (type == ButterworthType::Lowpass || type == ButterworthType::Highpass) {
        if (!std::isfinite(low_cutoff_norm) || low_cutoff_norm <= 0.0f || low_cutoff_norm >= 0.5f) {
            detail::fail(Error::InvalidCoefficient);
        }
    } else {
        if (!std::isfinite(low_cutoff_norm) || !std::isfinite(high_cutoff_norm)) {
            detail::fail(Error::InvalidCoefficient);
        }
        if (!(low_cutoff_norm > 0.0f && low_cutoff_norm < high_cutoff_norm && high_cutoff_norm < 0.5f)) {
            detail::fail(Error::InvalidCoefficient);
        }
    }

    std::vector<SosSection> sec;
    std::size_t pairs = static_cast<std::size_t>(order / 2u);

    if (type == ButterworthType::Lowpass || type == ButterworthType::Highpass) {
        double k_warp = std::tan(detail::PI * static_cast<double>(low_cutoff_norm));
        double k2 = k_warp * k_warp;
        bool is_hp = (type == ButterworthType::Highpass);
        for (std::size_t k = 0; k < pairs; ++k) {
            double psi = detail::PI * static_cast<double>(2u * k + 1u) / static_cast<double>(2u * order);
            double sigma = std::sin(psi);
            double a0 = 1.0 + 2.0 * sigma * k_warp + k2;
            double a1 = 2.0 * (k2 - 1.0);
            double a2 = 1.0 - 2.0 * sigma * k_warp + k2;
            if (!is_hp) {
                sec.push_back(detail::make_sos_from_doubles(k2, 2.0 * k2, k2, a0, a1, a2));
            } else {
                sec.push_back(detail::make_sos_from_doubles(1.0, -2.0, 1.0, a0, a1, a2));
            }
        }
        if ((order & 1u) != 0u) {
            double a0 = 1.0 + k_warp;
            double a1 = k_warp - 1.0;
            if (!is_hp) {
                sec.push_back(detail::make_sos_from_doubles(k_warp, k_warp, 0.0, a0, a1, 0.0));
            } else {
                sec.push_back(detail::make_sos_from_doubles(1.0, -1.0, 0.0, a0, a1, 0.0));
            }
        }
    } else {
        double w1 = std::tan(detail::PI * static_cast<double>(low_cutoff_norm));
        double w2 = std::tan(detail::PI * static_cast<double>(high_cutoff_norm));
        double b_band = w2 - w1;
        double w0_sq = w1 * w2;
        bool is_bs = (type == ButterworthType::Bandstop);
        for (std::size_t k = 0; k < pairs; ++k) {
            double psi = detail::PI * static_cast<double>(2u * k + 1u) / static_cast<double>(2u * order);
            double pr = -std::sin(psi);
            double pi = is_bs ? -std::cos(psi) : std::cos(psi);
            auto pair = detail::emit_bp_bs_pair(pr, pi, b_band, w0_sq, is_bs);
            sec.push_back(pair.first);
            sec.push_back(pair.second);
        }
        if ((order & 1u) != 0u) {
            double a0 = 1.0 + b_band + w0_sq;
            double a1 = 2.0 * (w0_sq - 1.0);
            double a2 = 1.0 - b_band + w0_sq;
            if (!is_bs) {
                sec.push_back(detail::make_sos_from_doubles(b_band, 0.0, -b_band, a0, a1, a2));
            } else {
                sec.push_back(detail::make_sos_from_doubles(1.0 + w0_sq, 2.0 * (w0_sq - 1.0), 1.0 + w0_sq, a0, a1, a2));
            }
        }
    }
    return sec;
}

namespace detail {

inline std::pair<std::vector<float>, std::vector<float>> sos_to_ba(const std::vector<SosSection>& sec,
                                                                   std::size_t poly_len) {
    std::vector<double> b_acc = {1.0};
    std::vector<double> a_acc = {1.0};
    for (const auto& s : sec) {
        std::vector<double> nb(b_acc.size() + 2, 0.0);
        std::vector<double> na(a_acc.size() + 2, 0.0);
        double sb[3] = {static_cast<double>(s[0]), static_cast<double>(s[1]), static_cast<double>(s[2])};
        double sa[3] = {static_cast<double>(s[3]), static_cast<double>(s[4]), static_cast<double>(s[5])};
        for (std::size_t i = 0; i < b_acc.size(); ++i) {
            for (std::size_t j = 0; j < 3; ++j) {
                nb[i + j] += b_acc[i] * sb[j];
                na[i + j] += a_acc[i] * sa[j];
            }
        }
        b_acc = std::move(nb);
        a_acc = std::move(na);
    }
    b_acc.resize(poly_len);
    a_acc.resize(poly_len);
    return {narrow_vector(b_acc), narrow_vector(a_acc)};
}

} // namespace detail

inline std::pair<std::vector<float>, std::vector<float>> butterworth_lowpass(std::uint32_t order, float cutoff_norm) {
    if (order != 1u && order != 2u) detail::fail(Error::InvalidCoefficient);
    return detail::sos_to_ba(butterworth_sos(ButterworthType::Lowpass, order, cutoff_norm), order + 1u);
}

inline std::pair<std::vector<float>, std::vector<float>> butterworth_highpass(std::uint32_t order, float cutoff_norm) {
    if (order != 1u && order != 2u) detail::fail(Error::InvalidCoefficient);
    return detail::sos_to_ba(butterworth_sos(ButterworthType::Highpass, order, cutoff_norm), order + 1u);
}

inline std::pair<std::vector<float>, std::vector<float>> butterworth_bandpass(std::uint32_t order,
                                                                              float low_norm, float high_norm) {
    if (order != 1u && order != 2u) detail::fail(Error::InvalidCoefficient);
    return detail::sos_to_ba(butterworth_sos(ButterworthType::Bandpass, order, low_norm, high_norm), 2u * order + 1u);
}

inline std::pair<std::vector<float>, std::vector<float>> butterworth_bandstop(std::uint32_t order,
                                                                              float low_norm, float high_norm) {
    if (order != 1u && order != 2u) detail::fail(Error::InvalidCoefficient);
    return detail::sos_to_ba(butterworth_sos(ButterworthType::Bandstop, order, low_norm, high_norm), 2u * order + 1u);
}

// ------------------------------------------------------------------ //
// Parsing Helpers                                                    //
// ------------------------------------------------------------------ //

inline WindowType parse_window(const std::string& str) {
    std::string k = detail::trim_lower(str);
    if (k == "rectangular" || k == "rect" || k == "boxcar") return WindowType::Rectangular;
    if (k == "hamming") return WindowType::Hamming;
    if (k == "hann" || k == "hanning") return WindowType::Hann;
    if (k == "blackman") return WindowType::Blackman;
    detail::fail(Error::InvalidCoefficient);
}

inline ButterworthType parse_butterworth_type(const std::string& str) {
    std::string k = detail::trim_lower(str);
    if (k == "lowpass" || k == "low" || k == "lp") return ButterworthType::Lowpass;
    if (k == "highpass" || k == "high" || k == "hp") return ButterworthType::Highpass;
    if (k == "bandpass" || k == "band" || k == "bp") return ButterworthType::Bandpass;
    if (k == "bandstop" || k == "notch" || k == "bs") return ButterworthType::Bandstop;
    detail::fail(Error::InvalidCoefficient);
}

} // namespace dsp_filters
} // namespace ca

#endif // CA_DSP_FILTERS_HPP
