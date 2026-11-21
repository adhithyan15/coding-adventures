// dsp_conv.hpp — 1-D, FFT/Overlap-Add, and 2-D Convolution (header-only, pure ISO C++17).
// ---------------------------------------------------------------------------
//
// Self-contained pure ISO C++17 implementation (zero <cmath> / -lm) in
// namespace `ca::dsp_conv`, matching the Rust and TypeScript `dsp-conv` contracts.
//
// Hardened for VER15 & Zero-CVE:
// - Direct <cfloat>, <cstddef>, <cstdint> includes.
// - Structural/length/cap bounds checked BEFORE element finiteness scans.
// - Double-precision intermediates with |x| <= FLT_MAX guard prior to float narrowing.
// - Upfront enum/mode domain validation, non-finite input AND output rejection.
// - Checked std::size_t math (CWE-190 / CWE-400), heap-allocated large buffers.
// - Strictly iterative execution (CWE-674), zero regex (CWE-1333).
#ifndef CA_DSP_CONV_HPP
#define CA_DSP_CONV_HPP

#include <cfloat>
#include <cstddef>
#include <cstdint>
#include <exception>
#include <optional>
#include <string>
#include <utility>
#include <vector>

namespace ca {
namespace dsp_conv {

// Defensive resource bounds (CWE-400 / CWE-190).
inline constexpr std::size_t MAX_SAMPLES = 16777216u;    // 2^24
inline constexpr std::size_t MAX_KERNEL_LEN = 1048576u;  // 2^20
inline constexpr std::size_t MAX_FFT_LEN = 1048576u;     // 2^20

enum class BoundaryMode {
    Zero = 0,
    Replicate = 1,
    Reflect = 2,
    Wrap = 3
};

enum class ConvMode {
    Full = 0,
    Same = 1,
    Valid = 2
};

enum class Error {
    EmptySignal,
    EmptyKernel,
    ImageSizeMismatch,
    KernelTooLarge,
    InvalidInput
};

class ConvError : public std::exception {
public:
    explicit ConvError(Error code, const char* msg = "dsp_conv error") noexcept
        : code_(code), msg_(msg) {}
    Error code() const noexcept { return code_; }
    const char* what() const noexcept override { return msg_; }

private:
    Error code_;
    const char* msg_;
};

namespace detail {

[[noreturn]] inline void fail(Error e, const char* msg) {
    throw ConvError(e, msg);
}

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

inline bool f_isfinite(float x) noexcept {
    return (x == x) && ((x - x) == 0.0f) && (x <= FLT_MAX) && (x >= -FLT_MAX);
}

inline bool d_isfinite(double x) noexcept {
    return (x == x) && ((x - x) == 0.0) && (x <= DBL_MAX) && (x >= -DBL_MAX);
}

inline float narrow_f32(double x) {
    if (!d_isfinite(x) || x > static_cast<double>(FLT_MAX) ||
        x < -static_cast<double>(FLT_MAX)) {
        fail(Error::InvalidInput, "intermediate result overflowed finite float range");
    }
    float f = static_cast<float>(x);
    if (!f_isfinite(f)) {
        fail(Error::InvalidInput, "narrowed float is non-finite");
    }
    return f;
}

inline bool is_valid_boundary(BoundaryMode m) noexcept {
    return m == BoundaryMode::Zero || m == BoundaryMode::Replicate ||
           m == BoundaryMode::Reflect || m == BoundaryMode::Wrap;
}

inline bool is_valid_conv_mode(ConvMode m) noexcept {
    return m == ConvMode::Full || m == ConvMode::Same || m == ConvMode::Valid;
}

inline void validate_floats(const std::vector<float>& buf) {
    for (float v : buf) {
        if (!f_isfinite(v)) {
            fail(Error::InvalidInput, "non-finite float in input buffer");
        }
    }
}

inline double d_sqrt(double x) noexcept {
    if (!d_isfinite(x) || x <= 0.0) return 0.0;
    double m = x;
    double scale = 1.0;
    while (m < 0.25) {
        m *= 4.0;
        scale *= 0.5;
    }
    while (m >= 1.0) {
        m *= 0.25;
        scale *= 2.0;
    }
    double g = 0.5 * (1.0 + m);
    for (int iter = 0; iter < 10; ++iter) {
        g = 0.5 * (g + m / g);
    }
    return g * scale;
}

inline double pow2i_d(int k) noexcept {
    double result = 1.0;
    double base = k < 0 ? 0.5 : 2.0;
    unsigned int n = k < 0 ? static_cast<unsigned int>(-(k + 1)) + 1u
                           : static_cast<unsigned int>(k);
    while (n > 0u) {
        if (n & 1u) result *= base;
        n >>= 1;
        if (n > 0u) base *= base;
    }
    return result;
}

inline double d_exp(double x) noexcept {
    if (x != x) return x;
    if (x == 0.0) return 1.0;
    if (x > 709.782712893384) return DBL_MAX;
    if (x < -745.13321910194) return 0.0;
    constexpr double INV_LN2 = 1.4426950408889634;
    constexpr double C1 = 0.693359375;
    constexpr double C2 = -2.1219444005469058277e-4;
    double kf = x * INV_LN2;
    int k = static_cast<int>(kf >= 0.0 ? kf + 0.5 : kf - 0.5);
    double r = (x - static_cast<double>(k) * C1) - static_cast<double>(k) * C2;
    double term = 1.0;
    double sum = 1.0;
    for (int i = 1; i <= 17; ++i) {
        term *= r / static_cast<double>(i);
        sum += term;
    }
    if (k >= 1024) {
        double v = (sum * pow2i_d(1023)) * 2.0;
        return d_isfinite(v) ? v : DBL_MAX;
    }
    return sum * pow2i_d(k);
}

inline void d_sincos(double x, double& sin_out, double& cos_out) noexcept {
    constexpr double PI_HALF = 1.57079632679489661923;
    constexpr double INV_PI_HALF = 0.63661977236758134308;
    constexpr double P1 = 1.57079632673412561417;
    constexpr double P2 = 6.07710050650619224932e-11;
    constexpr double TWO_PI = 6.28318530717958647693;
    if (!d_isfinite(x)) {
        sin_out = 0.0;
        cos_out = 1.0;
        return;
    }
    if (x > 9.0e15 || x < -9.0e15) {
        sin_out = 0.0;
        cos_out = 1.0;
        return;
    }
    if (x > 1e12 || x < -1e12) {
        double k_big = static_cast<double>(static_cast<std::int64_t>(x / TWO_PI));
        x -= k_big * TWO_PI;
    }
    double qf = x * INV_PI_HALF;
    std::int64_t q = static_cast<std::int64_t>(qf >= 0.0 ? qf + 0.5 : qf - 0.5);
    double r = (x - static_cast<double>(q) * P1) - static_cast<double>(q) * P2;
    if (r > PI_HALF) {
        r -= PI_HALF;
        q += 1;
    } else if (r < -PI_HALF) {
        r += PI_HALF;
        q -= 1;
    }
    double r2 = r * r;
    double s_term = r, s_sum = r;
    double c_term = 1.0, c_sum = 1.0;
    for (int n = 1; n <= 11; ++n) {
        s_term *= -r2 / static_cast<double>((2 * n) * (2 * n + 1));
        s_sum += s_term;
        c_term *= -r2 / static_cast<double>((2 * n - 1) * (2 * n));
        c_sum += c_term;
    }
    switch (static_cast<unsigned int>(q & 3)) {
        case 0u: sin_out = s_sum; cos_out = c_sum; break;
        case 1u: sin_out = c_sum; cos_out = -s_sum; break;
        case 2u: sin_out = -s_sum; cos_out = -c_sum; break;
        default: sin_out = -c_sum; cos_out = s_sum; break;
    }
}

inline std::optional<std::size_t> extend_index_fast(std::int64_t idx, std::size_t n,
                                                    BoundaryMode mode) noexcept {
    if (idx >= 0 && static_cast<std::uint64_t>(idx) < static_cast<std::uint64_t>(n)) {
        return static_cast<std::size_t>(idx);
    }
    switch (mode) {
        case BoundaryMode::Zero:
            return std::nullopt;
        case BoundaryMode::Replicate:
            return (idx < 0) ? 0u : (n - 1u);
        case BoundaryMode::Reflect: {
            if (n == 1u) return 0u;
            std::uint64_t period = 2u * (static_cast<std::uint64_t>(n) - 1u);
            std::uint64_t m = 0;
            if (idx >= 0) {
                m = static_cast<std::uint64_t>(idx) % period;
            } else {
                std::uint64_t abs_idx = static_cast<std::uint64_t>(-(idx + 1)) + 1u;
                std::uint64_t rem = abs_idx % period;
                m = (rem == 0u) ? 0u : (period - rem);
            }
            if (m >= static_cast<std::uint64_t>(n)) m = period - m;
            return static_cast<std::size_t>(m);
        }
        case BoundaryMode::Wrap: {
            std::uint64_t m = 0;
            if (idx >= 0) {
                m = static_cast<std::uint64_t>(idx) % static_cast<std::uint64_t>(n);
            } else {
                std::uint64_t abs_idx = static_cast<std::uint64_t>(-(idx + 1)) + 1u;
                std::uint64_t rem = abs_idx % static_cast<std::uint64_t>(n);
                m = (rem == 0u) ? 0u : (static_cast<std::uint64_t>(n) - rem);
            }
            return static_cast<std::size_t>(m);
        }
        default:
            return std::nullopt;
    }
}

inline void fft_bit_reverse(std::vector<double>& buf, std::size_t n) noexcept {
    std::size_t log_n = 0;
    std::size_t tmp = n;
    while (tmp > 1u) {
        tmp >>= 1;
        ++log_n;
    }
    for (std::size_t i = 0; i < n; ++i) {
        std::size_t x = i;
        std::size_t r = 0;
        for (std::size_t b = 0; b < log_n; ++b) {
            r = (r << 1) | (x & 1u);
            x >>= 1;
        }
        if (r > i) {
            std::swap(buf[2u * i], buf[2u * r]);
            std::swap(buf[2u * i + 1u], buf[2u * r + 1u]);
        }
    }
}

inline void fft_butterflies_d(std::vector<double>& buf, std::size_t n, bool inverse) noexcept {
    constexpr double PI = 3.14159265358979323846;
    double sign = inverse ? 1.0 : -1.0;
    fft_bit_reverse(buf, n);
    std::size_t full = 2u;
    while (full <= n) {
        std::size_t half = full >> 1;
        for (std::size_t block_start = 0; block_start < n; block_start += full) {
            for (std::size_t j = 0; j < half; ++j) {
                double theta = sign * 2.0 * PI * static_cast<double>(j) / static_cast<double>(full);
                double w_im = 0.0, w_re = 1.0;
                d_sincos(theta, w_im, w_re);
                std::size_t x_idx = 2u * (block_start + j);
                std::size_t y_idx = 2u * (block_start + j + half);
                double x_re = buf[x_idx], x_im = buf[x_idx + 1u];
                double y_re = buf[y_idx], y_im = buf[y_idx + 1u];
                double t_re = w_re * y_re - w_im * y_im;
                double t_im = w_re * y_im + w_im * y_re;
                buf[x_idx] = x_re + t_re;
                buf[x_idx + 1u] = x_im + t_im;
                buf[y_idx] = x_re - t_re;
                buf[y_idx + 1u] = x_im - t_im;
            }
        }
        full <<= 1;
    }
    if (inverse) {
        double inv_n = 1.0 / static_cast<double>(n);
        for (double& v : buf) v *= inv_n;
    }
}

inline std::string normalize_token(const std::string& s) {
    if (s.empty() || s.size() > 64u) {
        fail(Error::InvalidInput, "mode string empty or exceeds 64 chars");
    }
    std::size_t start = 0;
    auto is_sp = [](char c) { return c == ' ' || c == '\t' || c == '\r' || c == '\n'; };
    while (start < s.size() && is_sp(s[start])) ++start;
    std::size_t end = s.size();
    while (end > start && is_sp(s[end - 1u])) --end;
    if (start >= end) {
        fail(Error::InvalidInput, "blank mode string");
    }
    std::string out;
    out.reserve(end - start);
    for (std::size_t i = start; i < end; ++i) {
        char c = s[i];
        if (c >= 'A' && c <= 'Z') c = static_cast<char>(c + ('a' - 'A'));
        out.push_back(c);
    }
    return out;
}

} // namespace detail

// ------------------------------------------------------------------ //
// Parsing & Index Helpers                                            //
// ------------------------------------------------------------------ //

inline BoundaryMode parse_boundary_mode(const std::string& str) {
    std::string tok = detail::normalize_token(str);
    if (tok == "zero") return BoundaryMode::Zero;
    if (tok == "replicate") return BoundaryMode::Replicate;
    if (tok == "reflect") return BoundaryMode::Reflect;
    if (tok == "wrap") return BoundaryMode::Wrap;
    detail::fail(Error::InvalidInput, "unsupported boundary mode");
}

inline ConvMode parse_conv_mode(const std::string& str) {
    std::string tok = detail::normalize_token(str);
    if (tok == "full") return ConvMode::Full;
    if (tok == "same") return ConvMode::Same;
    if (tok == "valid") return ConvMode::Valid;
    detail::fail(Error::InvalidInput, "unsupported conv mode");
}

inline std::optional<std::size_t> extend_index(std::int64_t idx, std::size_t n,
                                               BoundaryMode mode) {
    if (n == 0 || n > static_cast<std::size_t>(INT64_MAX)) {
        detail::fail(Error::InvalidInput, "n must be in [1, INT64_MAX]");
    }
    if (!detail::is_valid_boundary(mode)) {
        detail::fail(Error::InvalidInput, "invalid boundary mode");
    }
    return detail::extend_index_fast(idx, n, mode);
}

inline float sample(const std::vector<float>& signal, std::int64_t idx,
                    BoundaryMode mode) {
    if (signal.empty()) detail::fail(Error::EmptySignal, "empty signal");
    if (signal.size() > MAX_SAMPLES) detail::fail(Error::InvalidInput, "signal exceeds MAX_SAMPLES");
    if (!detail::is_valid_boundary(mode)) detail::fail(Error::InvalidInput, "invalid boundary mode");
    auto mapped = detail::extend_index_fast(idx, signal.size(), mode);
    if (!mapped.has_value()) return 0.0f;
    float v = signal[*mapped];
    if (!detail::f_isfinite(v)) detail::fail(Error::InvalidInput, "non-finite sample");
    return v;
}

inline std::size_t next_pow2(std::size_t n) {
    if (n <= 1u) return 1u;
    std::size_t p = 1u;
    while (p < n) {
        if (p > (SIZE_MAX >> 1)) {
            detail::fail(Error::InvalidInput, "next_pow2 overflow");
        }
        p <<= 1;
    }
    return p;
}

// ------------------------------------------------------------------ //
// 1-D Convolution & Cross-Correlation                                //
// ------------------------------------------------------------------ //

inline std::vector<float> conv1d(const std::vector<float>& signal,
                                 const std::vector<float>& kernel,
                                 BoundaryMode mode = BoundaryMode::Zero) {
    if (signal.empty()) detail::fail(Error::EmptySignal, "empty signal");
    if (kernel.empty()) detail::fail(Error::EmptyKernel, "empty kernel");
    if (signal.size() > MAX_SAMPLES || kernel.size() > MAX_KERNEL_LEN ||
        !detail::is_valid_boundary(mode)) {
        detail::fail(Error::InvalidInput, "invalid length or boundary mode");
    }
    detail::validate_floats(signal);
    detail::validate_floats(kernel);

    const std::size_t n = signal.size();
    const std::size_t k = kernel.size();
    const std::size_t centre = k / 2u;
    std::vector<float> out(n);

    for (std::size_t i = 0; i < n; ++i) {
        double acc = 0.0;
        for (std::size_t j = 0; j < k; ++j) {
            std::int64_t src_idx = static_cast<std::int64_t>(i) +
                                   static_cast<std::int64_t>(centre) -
                                   static_cast<std::int64_t>(j);
            auto mapped = detail::extend_index_fast(src_idx, n, mode);
            if (mapped.has_value()) {
                acc += static_cast<double>(kernel[j]) * static_cast<double>(signal[*mapped]);
            }
        }
        out[i] = detail::narrow_f32(acc);
    }
    return out;
}

inline std::vector<float> convolve(const std::vector<float>& signal,
                                   const std::vector<float>& kernel,
                                   ConvMode mode = ConvMode::Full,
                                   BoundaryMode boundary = BoundaryMode::Zero) {
    if (signal.empty()) detail::fail(Error::EmptySignal, "empty signal");
    if (kernel.empty()) detail::fail(Error::EmptyKernel, "empty kernel");
    if (signal.size() > MAX_SAMPLES || kernel.size() > MAX_KERNEL_LEN ||
        !detail::is_valid_conv_mode(mode) || !detail::is_valid_boundary(boundary)) {
        detail::fail(Error::InvalidInput, "invalid length or mode");
    }

    if (mode == ConvMode::Same) {
        return conv1d(signal, kernel, boundary);
    }

    const std::size_t n = signal.size();
    const std::size_t k = kernel.size();
    std::size_t out_len = 0;

    if (mode == ConvMode::Valid) {
        if (n < k) detail::fail(Error::KernelTooLarge, "kernel exceeds signal in valid mode");
        out_len = n - k + 1u;
    } else {
        std::size_t sum_len = 0;
        if (!detail::checked_add(n, k, sum_len) || sum_len == 0u ||
            (sum_len - 1u) > MAX_SAMPLES) {
            detail::fail(Error::InvalidInput, "full convolution output exceeds MAX_SAMPLES");
        }
        out_len = sum_len - 1u;
    }

    detail::validate_floats(signal);
    detail::validate_floats(kernel);

    std::vector<float> out(out_len);
    if (mode == ConvMode::Valid) {
        for (std::size_t i = 0; i < out_len; ++i) {
            double acc = 0.0;
            for (std::size_t j = 0; j < k; ++j) {
                acc += static_cast<double>(kernel[j]) *
                       static_cast<double>(signal[i + k - 1u - j]);
            }
            out[i] = detail::narrow_f32(acc);
        }
    } else {
        for (std::size_t i = 0; i < out_len; ++i) {
            double acc = 0.0;
            for (std::size_t j = 0; j < k; ++j) {
                std::int64_t src_idx = static_cast<std::int64_t>(i) - static_cast<std::int64_t>(j);
                auto mapped = detail::extend_index_fast(src_idx, n, boundary);
                if (mapped.has_value()) {
                    acc += static_cast<double>(kernel[j]) * static_cast<double>(signal[*mapped]);
                }
            }
            out[i] = detail::narrow_f32(acc);
        }
    }
    return out;
}

inline std::vector<float> correlate(const std::vector<float>& signal,
                                    const std::vector<float>& kernel,
                                    ConvMode mode = ConvMode::Full) {
    if (signal.empty()) detail::fail(Error::EmptySignal, "empty signal");
    if (kernel.empty()) detail::fail(Error::EmptyKernel, "empty kernel");
    if (signal.size() > MAX_SAMPLES || kernel.size() > MAX_KERNEL_LEN ||
        !detail::is_valid_conv_mode(mode)) {
        detail::fail(Error::InvalidInput, "invalid length or mode");
    }
    if (mode == ConvMode::Valid && signal.size() < kernel.size()) {
        detail::fail(Error::KernelTooLarge, "kernel exceeds signal in valid mode");
    }
    if (mode == ConvMode::Full) {
        std::size_t sum_len = 0;
        if (!detail::checked_add(signal.size(), kernel.size(), sum_len) ||
            (sum_len - 1u) > MAX_SAMPLES) {
            detail::fail(Error::InvalidInput, "full correlation output exceeds MAX_SAMPLES");
        }
    }
    detail::validate_floats(signal);
    detail::validate_floats(kernel);

    std::vector<float> rev(kernel.rbegin(), kernel.rend());
    return convolve(signal, rev, mode, BoundaryMode::Zero);
}

inline std::vector<float> convolve_fft(const std::vector<float>& signal,
                                       const std::vector<float>& kernel,
                                       ConvMode mode = ConvMode::Full) {
    if (signal.empty()) detail::fail(Error::EmptySignal, "empty signal");
    if (kernel.empty()) detail::fail(Error::EmptyKernel, "empty kernel");
    if (signal.size() > MAX_SAMPLES || kernel.size() > MAX_KERNEL_LEN ||
        !detail::is_valid_conv_mode(mode)) {
        detail::fail(Error::InvalidInput, "invalid length or mode");
    }
    const std::size_t n = signal.size();
    const std::size_t k = kernel.size();
    if (mode == ConvMode::Valid && n < k) {
        detail::fail(Error::KernelTooLarge, "kernel exceeds signal in valid mode");
    }
    std::size_t sum_len = 0;
    if (!detail::checked_add(n, k, sum_len) || sum_len == 0u ||
        (sum_len - 1u) > MAX_SAMPLES) {
        detail::fail(Error::InvalidInput, "FFT convolution output exceeds MAX_SAMPLES");
    }
    const std::size_t full_len = sum_len - 1u;
    const std::size_t fft_len = next_pow2(full_len);
    if (fft_len > MAX_FFT_LEN) {
        detail::fail(Error::InvalidInput, "required FFT length exceeds MAX_FFT_LEN");
    }
    std::size_t buf_doubles = 0;
    if (!detail::checked_mul(fft_len, 2u, buf_doubles)) {
        detail::fail(Error::InvalidInput, "FFT buffer size overflow");
    }
    detail::validate_floats(signal);
    detail::validate_floats(kernel);

    std::vector<double> sig_buf(buf_doubles, 0.0);
    std::vector<double> ker_buf(buf_doubles, 0.0);
    for (std::size_t i = 0; i < n; ++i) sig_buf[2u * i] = static_cast<double>(signal[i]);
    for (std::size_t i = 0; i < k; ++i) ker_buf[2u * i] = static_cast<double>(kernel[i]);

    detail::fft_butterflies_d(sig_buf, fft_len, false);
    detail::fft_butterflies_d(ker_buf, fft_len, false);

    for (std::size_t i = 0; i < fft_len; ++i) {
        double re_s = sig_buf[2u * i], im_s = sig_buf[2u * i + 1u];
        double re_k = ker_buf[2u * i], im_k = ker_buf[2u * i + 1u];
        double re = re_s * re_k - im_s * im_k;
        double im = re_s * im_k + im_s * re_k;
        if (!detail::d_isfinite(re) || !detail::d_isfinite(im)) {
            detail::fail(Error::InvalidInput, "FFT convolution product overflowed");
        }
        sig_buf[2u * i] = re;
        sig_buf[2u * i + 1u] = im;
    }

    std::vector<double>().swap(ker_buf);
    detail::fft_butterflies_d(sig_buf, fft_len, true);

    std::size_t out_len = full_len;
    std::size_t offset = 0u;
    if (mode == ConvMode::Same) {
        out_len = n;
        offset = k / 2u;
    } else if (mode == ConvMode::Valid) {
        out_len = n - k + 1u;
        offset = k - 1u;
    }

    std::vector<float> out(out_len);
    for (std::size_t i = 0; i < out_len; ++i) {
        out[i] = detail::narrow_f32(sig_buf[2u * (offset + i)]);
    }
    return out;
}

inline std::vector<float> fft_convolve(const std::vector<float>& signal,
                                       const std::vector<float>& kernel,
                                       ConvMode mode = ConvMode::Full) {
    return convolve_fft(signal, kernel, mode);
}

inline std::vector<float> overlap_add(const std::vector<float>& signal,
                                      const std::vector<float>& kernel,
                                      std::optional<std::size_t> block_size = std::nullopt,
                                      ConvMode mode = ConvMode::Full) {
    if (signal.empty()) detail::fail(Error::EmptySignal, "empty signal");
    if (kernel.empty()) detail::fail(Error::EmptyKernel, "empty kernel");
    if (signal.size() > MAX_SAMPLES || kernel.size() > MAX_KERNEL_LEN ||
        !detail::is_valid_conv_mode(mode)) {
        detail::fail(Error::InvalidInput, "invalid length or mode");
    }
    const std::size_t n = signal.size();
    const std::size_t k = kernel.size();

    std::size_t block_len = 0;
    if (!block_size.has_value()) {
        std::size_t np2 = next_pow2(k);
        block_len = np2 < 64u ? 64u : np2;
    } else {
        if (*block_size == 0u || *block_size > MAX_SAMPLES) {
            detail::fail(Error::InvalidInput, "block_size must be in [1, MAX_SAMPLES]");
        }
        block_len = *block_size;
    }

    if (mode == ConvMode::Valid && n < k) {
        detail::fail(Error::KernelTooLarge, "kernel exceeds signal in valid mode");
    }

    std::size_t sum_len = 0;
    if (!detail::checked_add(n, k, sum_len) || sum_len == 0u ||
        (sum_len - 1u) > MAX_SAMPLES) {
        detail::fail(Error::InvalidInput, "overlap_add output exceeds MAX_SAMPLES");
    }
    const std::size_t full_len = sum_len - 1u;

    std::size_t conv_blk_max = 0;
    if (!detail::checked_add(block_len, k, conv_blk_max) || conv_blk_max == 0u) {
        detail::fail(Error::InvalidInput, "overlap_add block length overflow");
    }
    const std::size_t fft_len = next_pow2(conv_blk_max - 1u);
    if (fft_len > MAX_FFT_LEN) {
        detail::fail(Error::InvalidInput, "required FFT length exceeds MAX_FFT_LEN");
    }
    std::size_t buf_doubles = 0;
    if (!detail::checked_mul(fft_len, 2u, buf_doubles)) {
        detail::fail(Error::InvalidInput, "FFT buffer size overflow");
    }

    detail::validate_floats(signal);
    detail::validate_floats(kernel);

    std::vector<double> ker_buf(buf_doubles, 0.0);
    std::vector<double> blk_buf(buf_doubles, 0.0);
    std::vector<double> accum(full_len, 0.0);

    for (std::size_t i = 0; i < k; ++i) ker_buf[2u * i] = static_cast<double>(kernel[i]);
    detail::fft_butterflies_d(ker_buf, fft_len, false);

    std::size_t start = 0;
    while (start < n) {
        std::size_t rem = n - start;
        std::size_t actual = rem < block_len ? rem : block_len;
        std::size_t block_conv_len = actual + k - 1u;

        std::fill(blk_buf.begin(), blk_buf.end(), 0.0);
        for (std::size_t i = 0; i < actual; ++i) {
            blk_buf[2u * i] = static_cast<double>(signal[start + i]);
        }
        detail::fft_butterflies_d(blk_buf, fft_len, false);
        for (std::size_t i = 0; i < fft_len; ++i) {
            double re_s = blk_buf[2u * i], im_s = blk_buf[2u * i + 1u];
            double re_k = ker_buf[2u * i], im_k = ker_buf[2u * i + 1u];
            double re = re_s * re_k - im_s * im_k;
            double im = re_s * im_k + im_s * re_k;
            if (!detail::d_isfinite(re) || !detail::d_isfinite(im)) {
                detail::fail(Error::InvalidInput, "overlap_add FFT product overflowed");
            }
            blk_buf[2u * i] = re;
            blk_buf[2u * i + 1u] = im;
        }
        detail::fft_butterflies_d(blk_buf, fft_len, true);

        for (std::size_t i = 0; i < block_conv_len; ++i) {
            std::size_t out_pos = start + i;
            if (out_pos < full_len) {
                accum[out_pos] += blk_buf[2u * i];
            }
        }
        start += block_len;
    }

    std::size_t out_len = full_len;
    std::size_t offset = 0u;
    if (mode == ConvMode::Same) {
        out_len = n;
        offset = k / 2u;
    } else if (mode == ConvMode::Valid) {
        out_len = n - k + 1u;
        offset = k - 1u;
    }

    std::vector<float> out(out_len);
    for (std::size_t i = 0; i < out_len; ++i) {
        out[i] = detail::narrow_f32(accum[offset + i]);
    }
    return out;
}

// ------------------------------------------------------------------ //
// 2-D and Separable 2-D Convolution                                  //
// ------------------------------------------------------------------ //

inline std::vector<float> conv2d(const std::vector<float>& image,
                                 const std::vector<float>& kernel,
                                 std::uint32_t image_height,
                                 std::uint32_t image_width,
                                 std::uint32_t kernel_height,
                                 std::uint32_t kernel_width,
                                 BoundaryMode mode = BoundaryMode::Zero) {
    if (image_height == 0u || image_width == 0u || image.empty()) {
        detail::fail(Error::ImageSizeMismatch, "image dimensions must be non-zero");
    }
    if (kernel.empty() || kernel_height == 0u || kernel_width == 0u) {
        detail::fail(Error::EmptyKernel, "kernel dimensions must be non-zero");
    }
    if (!detail::is_valid_boundary(mode)) {
        detail::fail(Error::InvalidInput, "invalid boundary mode");
    }
    const std::size_t h = static_cast<std::size_t>(image_height);
    const std::size_t w = static_cast<std::size_t>(image_width);
    const std::size_t kh = static_cast<std::size_t>(kernel_height);
    const std::size_t kw = static_cast<std::size_t>(kernel_width);

    if (image.size() > MAX_SAMPLES || kernel.size() > MAX_KERNEL_LEN) {
        detail::fail(Error::InvalidInput, "input exceeds defensive cap");
    }
    std::size_t expected_img = 0;
    if (!detail::checked_mul(h, w, expected_img)) {
        detail::fail(Error::ImageSizeMismatch, "image dimensions overflow");
    }
    if (expected_img > MAX_SAMPLES) {
        detail::fail(Error::InvalidInput, "image size exceeds MAX_SAMPLES");
    }
    if (image.size() != expected_img) {
        detail::fail(Error::ImageSizeMismatch, "image length mismatch");
    }
    std::size_t expected_ker = 0;
    if (!detail::checked_mul(kh, kw, expected_ker)) {
        detail::fail(Error::ImageSizeMismatch, "kernel dimensions overflow");
    }
    if (expected_ker > MAX_KERNEL_LEN) {
        detail::fail(Error::InvalidInput, "kernel size exceeds MAX_KERNEL_LEN");
    }
    if (kernel.size() != expected_ker) {
        detail::fail(Error::ImageSizeMismatch, "kernel length mismatch");
    }

    detail::validate_floats(image);
    detail::validate_floats(kernel);

    if (kh > h || kw > w) {
        detail::fail(Error::KernelTooLarge, "kernel exceeds image dimensions");
    }

    const std::size_t ch = kh / 2u;
    const std::size_t cw = kw / 2u;
    std::vector<float> out(expected_img);

    for (std::size_t r = 0; r < h; ++r) {
        for (std::size_t c = 0; c < w; ++c) {
            double acc = 0.0;
            for (std::size_t kr = 0; kr < kh; ++kr) {
                std::int64_t src_r = static_cast<std::int64_t>(r) +
                                     static_cast<std::int64_t>(ch) -
                                     static_cast<std::int64_t>(kr);
                auto row = detail::extend_index_fast(src_r, h, mode);
                if (!row.has_value()) continue;
                for (std::size_t kc = 0; kc < kw; ++kc) {
                    std::int64_t src_c = static_cast<std::int64_t>(c) +
                                         static_cast<std::int64_t>(cw) -
                                         static_cast<std::int64_t>(kc);
                    auto col = detail::extend_index_fast(src_c, w, mode);
                    if (!col.has_value()) continue;
                    acc += static_cast<double>(kernel[kr * kw + kc]) *
                           static_cast<double>(image[*row * w + *col]);
                }
            }
            out[r * w + c] = detail::narrow_f32(acc);
        }
    }
    return out;
}

inline std::vector<float> sep_conv2d(const std::vector<float>& image,
                                     const std::vector<float>& h_kernel,
                                     const std::vector<float>& v_kernel,
                                     std::uint32_t image_height,
                                     std::uint32_t image_width,
                                     BoundaryMode mode = BoundaryMode::Zero) {
    if (image_height == 0u || image_width == 0u || image.empty()) {
        detail::fail(Error::ImageSizeMismatch, "image dimensions must be non-zero");
    }
    if (h_kernel.empty() || v_kernel.empty()) {
        detail::fail(Error::EmptyKernel, "separable kernels must be non-empty");
    }
    if (!detail::is_valid_boundary(mode)) {
        detail::fail(Error::InvalidInput, "invalid boundary mode");
    }
    const std::size_t h = static_cast<std::size_t>(image_height);
    const std::size_t w = static_cast<std::size_t>(image_width);

    if (image.size() > MAX_SAMPLES || h_kernel.size() > MAX_KERNEL_LEN ||
        v_kernel.size() > MAX_KERNEL_LEN) {
        detail::fail(Error::InvalidInput, "input exceeds defensive cap");
    }
    std::size_t expected_img = 0;
    if (!detail::checked_mul(h, w, expected_img)) {
        detail::fail(Error::ImageSizeMismatch, "image dimensions overflow");
    }
    if (expected_img > MAX_SAMPLES) {
        detail::fail(Error::InvalidInput, "image size exceeds MAX_SAMPLES");
    }
    if (image.size() != expected_img) {
        detail::fail(Error::ImageSizeMismatch, "image length mismatch");
    }
    detail::validate_floats(image);
    detail::validate_floats(h_kernel);
    detail::validate_floats(v_kernel);

    if (h_kernel.size() > w || v_kernel.size() > h) {
        detail::fail(Error::KernelTooLarge, "separable kernel exceeds image dimension");
    }

    const std::size_t ch = h_kernel.size() / 2u;
    const std::size_t cv = v_kernel.size() / 2u;
    std::vector<double> intermediate(expected_img, 0.0);
    std::vector<float> out(expected_img);

    for (std::size_t r = 0; r < h; ++r) {
        for (std::size_t c = 0; c < w; ++c) {
            double acc = 0.0;
            for (std::size_t j = 0; j < h_kernel.size(); ++j) {
                std::int64_t src_c = static_cast<std::int64_t>(c) +
                                     static_cast<std::int64_t>(ch) -
                                     static_cast<std::int64_t>(j);
                auto col = detail::extend_index_fast(src_c, w, mode);
                if (col.has_value()) {
                    acc += static_cast<double>(h_kernel[j]) *
                           static_cast<double>(image[r * w + *col]);
                }
            }
            if (!detail::d_isfinite(acc)) {
                detail::fail(Error::InvalidInput, "sep_conv2d row pass overflowed");
            }
            intermediate[r * w + c] = acc;
        }
    }

    for (std::size_t c = 0; c < w; ++c) {
        for (std::size_t r = 0; r < h; ++r) {
            double acc = 0.0;
            for (std::size_t j = 0; j < v_kernel.size(); ++j) {
                std::int64_t src_r = static_cast<std::int64_t>(r) +
                                     static_cast<std::int64_t>(cv) -
                                     static_cast<std::int64_t>(j);
                auto row = detail::extend_index_fast(src_r, h, mode);
                if (row.has_value()) {
                    acc += static_cast<double>(v_kernel[j]) * intermediate[*row * w + c];
                }
            }
            out[r * w + c] = detail::narrow_f32(acc);
        }
    }
    return out;
}

inline std::vector<float> convolve_2d(const std::vector<float>& image,
                                      const std::vector<float>& kernel,
                                      std::uint32_t image_height,
                                      std::uint32_t image_width,
                                      std::uint32_t kernel_height,
                                      std::uint32_t kernel_width,
                                      ConvMode mode = ConvMode::Full,
                                      BoundaryMode boundary = BoundaryMode::Zero) {
    if (image_height == 0u || image_width == 0u || image.empty()) {
        detail::fail(Error::ImageSizeMismatch, "image dimensions must be non-zero");
    }
    if (kernel.empty() || kernel_height == 0u || kernel_width == 0u) {
        detail::fail(Error::EmptyKernel, "kernel dimensions must be non-zero");
    }
    if (!detail::is_valid_conv_mode(mode) || !detail::is_valid_boundary(boundary)) {
        detail::fail(Error::InvalidInput, "invalid conv or boundary mode");
    }
    if (mode == ConvMode::Same) {
        return conv2d(image, kernel, image_height, image_width,
                      kernel_height, kernel_width, boundary);
    }
    const std::size_t h = static_cast<std::size_t>(image_height);
    const std::size_t w = static_cast<std::size_t>(image_width);
    const std::size_t kh = static_cast<std::size_t>(kernel_height);
    const std::size_t kw = static_cast<std::size_t>(kernel_width);

    if (image.size() > MAX_SAMPLES || kernel.size() > MAX_KERNEL_LEN) {
        detail::fail(Error::InvalidInput, "input exceeds defensive cap");
    }
    std::size_t expected_img = 0;
    if (!detail::checked_mul(h, w, expected_img)) {
        detail::fail(Error::ImageSizeMismatch, "image dimensions overflow");
    }
    if (expected_img > MAX_SAMPLES) {
        detail::fail(Error::InvalidInput, "image size exceeds MAX_SAMPLES");
    }
    if (image.size() != expected_img) {
        detail::fail(Error::ImageSizeMismatch, "image length mismatch");
    }
    std::size_t expected_ker = 0;
    if (!detail::checked_mul(kh, kw, expected_ker)) {
        detail::fail(Error::ImageSizeMismatch, "kernel dimensions overflow");
    }
    if (expected_ker > MAX_KERNEL_LEN) {
        detail::fail(Error::InvalidInput, "kernel size exceeds MAX_KERNEL_LEN");
    }
    if (kernel.size() != expected_ker) {
        detail::fail(Error::ImageSizeMismatch, "kernel length mismatch");
    }

    std::size_t out_h = 0, out_w = 0;
    if (mode == ConvMode::Valid) {
        if (h < kh || w < kw) {
            detail::fail(Error::KernelTooLarge, "kernel exceeds image in valid mode");
        }
        out_h = h - kh + 1u;
        out_w = w - kw + 1u;
    } else {
        std::size_t sh = 0, sw = 0;
        if (!detail::checked_add(h, kh, sh) || sh == 0u ||
            !detail::checked_add(w, kw, sw) || sw == 0u) {
            detail::fail(Error::InvalidInput, "full 2D output dimension overflow");
        }
        out_h = sh - 1u;
        out_w = sw - 1u;
    }

    std::size_t out_len = 0;
    if (!detail::checked_mul(out_h, out_w, out_len) || out_len > MAX_SAMPLES) {
        detail::fail(Error::InvalidInput, "2D convolution output exceeds MAX_SAMPLES");
    }
    detail::validate_floats(image);
    detail::validate_floats(kernel);

    std::vector<float> out(out_len);
    if (mode == ConvMode::Valid) {
        for (std::size_t r = 0; r < out_h; ++r) {
            for (std::size_t c = 0; c < out_w; ++c) {
                double acc = 0.0;
                for (std::size_t kr = 0; kr < kh; ++kr) {
                    std::size_t img_r = r + kh - 1u - kr;
                    for (std::size_t kc = 0; kc < kw; ++kc) {
                        std::size_t img_c = c + kw - 1u - kc;
                        acc += static_cast<double>(kernel[kr * kw + kc]) *
                               static_cast<double>(image[img_r * w + img_c]);
                    }
                }
                out[r * out_w + c] = detail::narrow_f32(acc);
            }
        }
    } else {
        for (std::size_t r = 0; r < out_h; ++r) {
            for (std::size_t c = 0; c < out_w; ++c) {
                double acc = 0.0;
                for (std::size_t kr = 0; kr < kh; ++kr) {
                    std::int64_t src_r = static_cast<std::int64_t>(r) -
                                         static_cast<std::int64_t>(kr);
                    auto row = detail::extend_index_fast(src_r, h, boundary);
                    if (!row.has_value()) continue;
                    for (std::size_t kc = 0; kc < kw; ++kc) {
                        std::int64_t src_c = static_cast<std::int64_t>(c) -
                                             static_cast<std::int64_t>(kc);
                        auto col = detail::extend_index_fast(src_c, w, boundary);
                        if (!col.has_value()) continue;
                        acc += static_cast<double>(kernel[kr * kw + kc]) *
                               static_cast<double>(image[*row * w + *col]);
                    }
                }
                out[r * out_w + c] = detail::narrow_f32(acc);
            }
        }
    }
    return out;
}

inline std::vector<float> convolve2d(const std::vector<float>& image,
                                     const std::vector<float>& kernel,
                                     std::uint32_t image_height,
                                     std::uint32_t image_width,
                                     std::uint32_t kernel_height,
                                     std::uint32_t kernel_width,
                                     ConvMode mode = ConvMode::Full,
                                     BoundaryMode boundary = BoundaryMode::Zero) {
    return convolve_2d(image, kernel, image_height, image_width,
                       kernel_height, kernel_width, mode, boundary);
}

// ------------------------------------------------------------------ //
// Image Filter Kernel Generators                                     //
// ------------------------------------------------------------------ //

inline std::vector<float> gaussian_blur_kernel(float sigma, std::uint32_t size) {
    constexpr double TWO_PI = 6.28318530717958647693;
    const std::size_t n = static_cast<std::size_t>(size);
    if (size == 0u || (size % 2u) == 0u || n > MAX_KERNEL_LEN) {
        detail::fail(Error::InvalidInput, "size must be odd, > 0, and <= MAX_KERNEL_LEN");
    }
    if (!detail::f_isfinite(sigma) || sigma <= 0.0f) {
        detail::fail(Error::InvalidInput, "sigma must be positive and finite");
    }
    float f_two_sig = 2.0f * sigma * sigma;
    if (f_two_sig <= 0.0f || !detail::f_isfinite(f_two_sig)) {
        detail::fail(Error::InvalidInput, "sigma underflows or overflows 2*sigma^2");
    }
    const double d_sigma = static_cast<double>(sigma);
    const double two_sigma_sq = 2.0 * d_sigma * d_sigma;
    const double norm_pdf = 1.0 / (d_sigma * detail::d_sqrt(TWO_PI));
    if (!detail::d_isfinite(norm_pdf) || norm_pdf > static_cast<double>(FLT_MAX)) {
        detail::fail(Error::InvalidInput, "sigma overflows Gaussian normalization factor");
    }

    std::vector<double> tmp(n);
    double total = 0.0;
    const double centre = static_cast<double>(n - 1u) * 0.5;
    for (std::size_t i = 0; i < n; ++i) {
        double x = static_cast<double>(i) - centre;
        double v = norm_pdf * detail::d_exp(-(x * x) / two_sigma_sq);
        if (!detail::d_isfinite(v)) {
            detail::fail(Error::InvalidInput, "Gaussian kernel tap became non-finite");
        }
        tmp[i] = v;
        total += v;
    }
    if (!detail::d_isfinite(total) || total <= 0.0) {
        detail::fail(Error::InvalidInput, "Gaussian kernel sum non-positive");
    }
    std::vector<float> out(n);
    for (std::size_t i = 0; i < n; ++i) {
        out[i] = detail::narrow_f32(tmp[i] / total);
    }
    return out;
}

inline std::vector<float> box_blur_kernel(std::uint32_t size) {
    const std::size_t n = static_cast<std::size_t>(size);
    if (size == 0u || n > MAX_KERNEL_LEN) {
        detail::fail(Error::InvalidInput, "size must be in [1, MAX_KERNEL_LEN]");
    }
    float val = detail::narrow_f32(1.0 / static_cast<double>(n));
    return std::vector<float>(n, val);
}

inline std::vector<float> sobel_x_kernel() {
    return {-1.0f, 0.0f, 1.0f, -2.0f, 0.0f, 2.0f, -1.0f, 0.0f, 1.0f};
}

inline std::vector<float> sobel_y_kernel() {
    return {-1.0f, -2.0f, -1.0f, 0.0f, 0.0f, 0.0f, 1.0f, 2.0f, 1.0f};
}

inline std::vector<float> laplacian_kernel() {
    return {0.0f, 1.0f, 0.0f, 1.0f, -4.0f, 1.0f, 0.0f, 1.0f, 0.0f};
}

inline std::vector<float> sharpen_kernel(float amount = 1.0f) {
    if (!detail::f_isfinite(amount)) {
        detail::fail(Error::InvalidInput, "amount must be finite");
    }
    float center = detail::narrow_f32(1.0 + 4.0 * static_cast<double>(amount));
    float neg = detail::narrow_f32(-static_cast<double>(amount));
    return {0.0f, neg, 0.0f, neg, center, neg, 0.0f, neg, 0.0f};
}

} // namespace dsp_conv
} // namespace ca

#endif // CA_DSP_CONV_HPP
