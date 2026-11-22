// dsp_stft.hpp — Short-Time Fourier Transform, ISTFT, Spectrograms, Mel & MFCC
//                (header-only, pure ISO C++17).
// ---------------------------------------------------------------------------
//
// Self-contained pure ISO C++17 implementation (zero <cmath> / -lm) in
// namespace `ca::dsp_stft`, matching the Rust and TypeScript `dsp-stft` contracts.
// Bundles an internal O(N log N) radix-2 + Bluestein FFT and an internal
// O(N log N) Makhoul DCT-II.
//
// Hardened for VER15 & Zero-CVE:
// - Direct <cfloat>, <cstddef>, <cstdint> includes.
// - Structural/length/cap bounds checked BEFORE element finiteness scans.
// - Double-precision intermediates with |x| <= FLT_MAX guard prior to float narrowing.
// - Upfront enum domain validation, non-finite input AND output rejection.
// - Checked std::size_t math (CWE-190 / CWE-400), heap-allocated large buffers.
// - Strictly iterative execution (CWE-674), zero regex (CWE-1333).
#ifndef CA_DSP_STFT_HPP
#define CA_DSP_STFT_HPP

#include <algorithm>
#include <cfloat>
#include <cstddef>
#include <cstdint>
#include <exception>
#include <optional>
#include <string>
#include <utility>
#include <vector>

namespace ca {
namespace dsp_stft {

// Defensive resource bounds (CWE-400 / CWE-190).
inline constexpr std::size_t MAX_FFT_LEN = 1048576u;          // 2^20
inline constexpr std::size_t MAX_SAMPLES = 16777216u;         // 2^24
inline constexpr std::size_t MAX_FRAMES = 1048576u;           // 2^20
inline constexpr std::size_t MAX_MELS = 65536u;               // 2^16
inline constexpr std::uint32_t MAX_GRIFFIN_LIM_ITERS = 4096u; // bounded loop

enum class WindowType {
    Hann = 0,
    Hamming = 1,
    Blackman = 2,
    Rectangular = 3
};

enum class Error {
    EmptySignal,
    InvalidParam,
    SignalTooShort,
    InvalidSpectrogram,
    Fft
};

class StftError : public std::exception {
public:
    explicit StftError(Error code, const char* msg = "dsp_stft error") noexcept
        : code_(code), msg_(msg) {}
    Error code() const noexcept { return code_; }
    const char* what() const noexcept override { return msg_; }

private:
    Error code_;
    const char* msg_;
};

struct OlaStatus {
    bool is_nola = false;
    bool is_cola = false;
    float min_norm = 0.0f;
    float max_norm = 0.0f;
};

namespace detail {

[[noreturn]] inline void fail(Error e, const char* msg) {
    throw StftError(e, msg);
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

inline float narrow_f32(double x, Error err_kind = Error::InvalidParam) {
    if (!d_isfinite(x) || x > static_cast<double>(FLT_MAX) ||
        x < -static_cast<double>(FLT_MAX)) {
        fail(err_kind, "intermediate result overflowed finite float range");
    }
    float f = static_cast<float>(x);
    if (!f_isfinite(f)) {
        fail(err_kind, "narrowed float is non-finite");
    }
    return f;
}

inline bool is_valid_window(WindowType w) noexcept {
    return w == WindowType::Hann || w == WindowType::Hamming ||
           w == WindowType::Blackman || w == WindowType::Rectangular;
}

inline void validate_floats(const std::vector<float>& buf,
                            Error err_kind = Error::InvalidParam) {
    for (float v : buf) {
        if (!f_isfinite(v)) {
            fail(err_kind, "non-finite float in input buffer");
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
    double term = 1.0, sum = 1.0;
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

inline double d_ln(double x) noexcept {
    constexpr double SQRT_HALF = 0.70710678118654752440;
    constexpr double SQRT_TWO = 1.41421356237309504880;
    constexpr double LN2 = 0.69314718055994530942;
    if (!d_isfinite(x) || x <= 0.0) return 0.0;
    double m = x;
    int k = 0;
    while (m < SQRT_HALF) { m *= 2.0; --k; }
    while (m >= SQRT_TWO) { m *= 0.5; ++k; }
    double u = (m - 1.0) / (m + 1.0);
    double u2 = u * u;
    double term = u, sum = 0.0;
    for (int i = 0; i < 14; ++i) {
        sum += term / static_cast<double>(2 * i + 1);
        term *= u2;
    }
    return 2.0 * sum + static_cast<double>(k) * LN2;
}

inline double d_log10(double x) noexcept {
    constexpr double INV_LN10 = 0.43429448190325182765;
    return d_ln(x) * INV_LN10;
}

inline double d_pow10(double x) noexcept {
    constexpr double LN10 = 2.30258509299404568402;
    if (x > 308.2547155599167) return DBL_MAX;
    return d_exp(x * LN10);
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

inline bool is_pow2(std::size_t n) noexcept {
    return n >= 1u && (n & (n - 1u)) == 0u;
}

inline std::size_t next_pow2_checked(std::size_t n) {
    if (n <= 1u) return 1u;
    std::size_t p = 1u;
    while (p < n) {
        if (p > (SIZE_MAX >> 1)) {
            fail(Error::Fft, "power-of-two size overflow");
        }
        p <<= 1;
    }
    return p;
}

inline std::size_t check_bluestein_m(std::size_t n) {
    std::size_t two_n = 0;
    if (!checked_mul(n, 2u, two_n) || two_n == 0u) {
        fail(Error::Fft, "Bluestein convolution length overflow");
    }
    std::size_t m = next_pow2_checked(two_n - 1u);
    if (m > MAX_FFT_LEN) {
        fail(Error::Fft, "Bluestein convolution length exceeds MAX_FFT_LEN");
    }
    return m;
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

inline void fft_radix2_d(std::vector<double>& buf, std::size_t n, bool inverse) noexcept {
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

inline void fft_complex_d(std::vector<double>& buf, std::size_t n, bool inverse) {
    constexpr double PI = 3.14159265358979323846;
    if (n == 0u || n > MAX_FFT_LEN) {
        fail(Error::Fft, "invalid FFT length");
    }
    if (n == 1u) return;
    if (is_pow2(n)) {
        fft_radix2_d(buf, n, inverse);
        return;
    }

    std::size_t m = check_bluestein_m(n);
    double sign = inverse ? 1.0 : -1.0;
    std::vector<double> a(2u * m, 0.0);
    std::vector<double> b(2u * m, 0.0);
    std::uint64_t mod = 2u * static_cast<std::uint64_t>(n);

    for (std::size_t k = 0; k < n; ++k) {
        std::uint64_t sq = (static_cast<std::uint64_t>(k) * static_cast<std::uint64_t>(k)) % mod;
        double theta = sign * PI * static_cast<double>(sq) / static_cast<double>(n);
        double sw = 0.0, cw = 1.0;
        d_sincos(theta, sw, cw);
        double xr = buf[2u * k], xi = buf[2u * k + 1u];
        a[2u * k] = xr * cw - xi * sw;
        a[2u * k + 1u] = xr * sw + xi * cw;
        b[2u * k] = cw;
        b[2u * k + 1u] = -sw;
        if (k > 0u) {
            b[2u * (m - k)] = cw;
            b[2u * (m - k) + 1u] = -sw;
        }
    }

    fft_radix2_d(a, m, false);
    fft_radix2_d(b, m, false);

    for (std::size_t k = 0; k < m; ++k) {
        double ar = a[2u * k], ai = a[2u * k + 1u];
        double br = b[2u * k], bi = b[2u * k + 1u];
        a[2u * k] = ar * br - ai * bi;
        a[2u * k + 1u] = ar * bi + ai * br;
    }
    fft_radix2_d(a, m, true);

    for (std::size_t k = 0; k < n; ++k) {
        std::uint64_t sq = (static_cast<std::uint64_t>(k) * static_cast<std::uint64_t>(k)) % mod;
        double theta = sign * PI * static_cast<double>(sq) / static_cast<double>(n);
        double sw = 0.0, cw = 1.0;
        d_sincos(theta, sw, cw);
        double cr = a[2u * k], ci = a[2u * k + 1u];
        double xr = cr * cw - ci * sw;
        double xi = cr * sw + ci * cw;
        if (inverse) {
            xr /= static_cast<double>(n);
            xi /= static_cast<double>(n);
        }
        buf[2u * k] = xr;
        buf[2u * k + 1u] = xi;
    }
}

inline void dct_ii_ortho_d(const std::vector<double>& in, std::vector<double>& out) {
    constexpr double PI = 3.14159265358979323846;
    const std::size_t n = in.size();
    if (n == 0u || n > MAX_FFT_LEN) {
        fail(Error::InvalidParam, "invalid DCT length");
    }
    std::vector<double> buf(2u * n, 0.0);
    std::size_t half_even = (n + 1u) / 2u;
    std::size_t half_odd = n / 2u;
    for (std::size_t m = 0; m < half_even; ++m) {
        buf[2u * m] = in[2u * m];
    }
    for (std::size_t m = 0; m < half_odd; ++m) {
        buf[2u * (n - 1u - m)] = in[2u * m + 1u];
    }

    fft_complex_d(buf, n, false);

    double s0 = 1.0 / d_sqrt(static_cast<double>(n));
    double sk = d_sqrt(2.0 / static_cast<double>(n));
    double two_n = 2.0 * static_cast<double>(n);
    out.resize(n);
    for (std::size_t k = 0; k < n; ++k) {
        double theta = PI * static_cast<double>(k) / two_n;
        double sin_t = 0.0, cos_t = 1.0;
        d_sincos(theta, sin_t, cos_t);
        double scale = (k == 0u) ? s0 : sk;
        double val = scale * (buf[2u * k] * cos_t + buf[2u * k + 1u] * sin_t);
        if (!d_isfinite(val)) {
            fail(Error::InvalidParam, "DCT-II result non-finite");
        }
        out[k] = val;
    }
}

inline std::vector<double> build_window_d(WindowType window, std::size_t n_fft) {
    constexpr double TWO_PI = 6.28318530717958647693;
    constexpr double FOUR_PI = 12.56637061435917295385;
    if (!is_valid_window(window) || n_fft == 0u || n_fft > MAX_FFT_LEN) {
        fail(Error::InvalidParam, "invalid window or n_fft");
    }
    std::vector<double> win(n_fft, 1.0);
    if (n_fft == 1u || window == WindowType::Rectangular) {
        return win;
    }
    double m = static_cast<double>(n_fft - 1u);
    for (std::size_t n = 0; n < n_fft; ++n) {
        double nn = static_cast<double>(n);
        double s1 = 0.0, c1 = 1.0;
        d_sincos(TWO_PI * nn / m, s1, c1);
        if (window == WindowType::Hann) {
            win[n] = 0.5 * (1.0 - c1);
        } else if (window == WindowType::Hamming) {
            win[n] = 0.54 - 0.46 * c1;
        } else {
            double s2 = 0.0, c2 = 1.0;
            d_sincos(FOUR_PI * nn / m, s2, c2);
            win[n] = 0.42 - 0.5 * c1 + 0.08 * c2;
        }
    }
    return win;
}

inline std::vector<double> build_mel_filterbank_d(std::uint32_t n_mels,
                                                  std::uint32_t n_fft,
                                                  float sample_rate,
                                                  bool throw_on_err) {
    const std::size_t nm = static_cast<std::size_t>(n_mels);
    const std::size_t nf = static_cast<std::size_t>(n_fft);
    if (n_mels == 0u || n_fft == 0u || nm > MAX_MELS || nf > MAX_FFT_LEN) {
        if (throw_on_err) fail(Error::InvalidParam, "invalid n_mels or n_fft");
        return {};
    }
    if (!f_isfinite(sample_rate) || sample_rate <= 0.0f) {
        if (throw_on_err) fail(Error::InvalidParam, "sample_rate must be positive and finite");
        return {};
    }
    float fmax_f = sample_rate * 0.5f;
    float bin_per_hz_f = static_cast<float>(n_fft) / sample_rate;
    if (!f_isfinite(fmax_f) || fmax_f <= 0.0f ||
        !f_isfinite(bin_per_hz_f) || bin_per_hz_f <= 0.0f) {
        if (throw_on_err) fail(Error::InvalidParam, "sample_rate underflows/overflows");
        return {};
    }
    const std::size_t bins = nf / 2u + 1u;
    std::size_t total_len = 0;
    if (!checked_mul(nm, bins, total_len) || total_len > MAX_SAMPLES) {
        if (throw_on_err) fail(Error::InvalidParam, "mel filterbank exceeds MAX_SAMPLES");
        return {};
    }

    const double bin_per_hz = static_cast<double>(nf) / static_cast<double>(sample_rate);
    const double mel_min = 0.0;
    const double mel_max = 2595.0 * d_log10(1.0 + (static_cast<double>(sample_rate) * 0.5) / 700.0);
    if (!d_isfinite(mel_max)) {
        if (throw_on_err) fail(Error::InvalidParam, "mel_max non-finite");
        return {};
    }
    const double step = (mel_max - mel_min) / static_cast<double>(nm + 1u);
    std::vector<double> bin_pts(nm + 2u);
    for (std::size_t i = 0; i < nm + 2u; ++i) {
        double mel = mel_min + step * static_cast<double>(i);
        double hz = 700.0 * (d_pow10(mel / 2595.0) - 1.0);
        double pt = hz * bin_per_hz;
        if (!d_isfinite(hz) || !d_isfinite(pt)) {
            if (throw_on_err) fail(Error::InvalidParam, "mel filterbank anchor non-finite");
            return {};
        }
        bin_pts[i] = pt;
    }

    std::vector<double> fb(total_len, 0.0);
    for (std::size_t m = 0; m < nm; ++m) {
        double left = bin_pts[m];
        double center = bin_pts[m + 1u];
        double right = bin_pts[m + 2u];
        double row_sum = 0.0;
        for (std::size_t k = 0; k < bins; ++k) {
            double kf = static_cast<double>(k);
            double w = 0.0;
            if (kf >= left && kf <= right) {
                if (kf <= center) {
                    if (center > left) w = (kf - left) / (center - left);
                } else if (right > center) {
                    w = (right - kf) / (right - center);
                }
            }
            fb[m * bins + k] = w;
            row_sum += w;
        }
        if (row_sum > 0.0) {
            for (std::size_t k = 0; k < bins; ++k) {
                fb[m * bins + k] /= row_sum;
            }
        }
    }
    return fb;
}

} // namespace detail

// ------------------------------------------------------------------ //
// Window Parsing & Generation                                        //
// ------------------------------------------------------------------ //

inline WindowType parse_window_type(const std::string& str) {
    if (str.empty() || str.size() > 64u) {
        detail::fail(Error::InvalidParam, "window string empty or exceeds 64 chars");
    }
    auto is_sp = [](char c) { return c == ' ' || c == '\t' || c == '\r' || c == '\n'; };
    std::size_t start = 0;
    while (start < str.size() && is_sp(str[start])) ++start;
    std::size_t end = str.size();
    while (end > start && is_sp(str[end - 1u])) --end;
    if (start >= end) {
        detail::fail(Error::InvalidParam, "blank window string");
    }
    std::string tok;
    tok.reserve(end - start);
    for (std::size_t i = start; i < end; ++i) {
        char c = str[i];
        if (c >= 'A' && c <= 'Z') c = static_cast<char>(c + ('a' - 'A'));
        tok.push_back(c);
    }
    if (tok == "hann" || tok == "hanning") return WindowType::Hann;
    if (tok == "hamming") return WindowType::Hamming;
    if (tok == "blackman") return WindowType::Blackman;
    if (tok == "rectangular" || tok == "rect" || tok == "boxcar") return WindowType::Rectangular;
    detail::fail(Error::InvalidParam, "unsupported window type");
}

inline std::vector<float> build_window(WindowType window, std::uint32_t n_fft) {
    auto win_d = detail::build_window_d(window, static_cast<std::size_t>(n_fft));
    std::vector<float> out(win_d.size());
    for (std::size_t i = 0; i < win_d.size(); ++i) {
        out[i] = detail::narrow_f32(win_d[i]);
    }
    return out;
}

inline std::vector<float> hann_window(std::uint32_t length) {
    return build_window(WindowType::Hann, length);
}
inline std::vector<float> hamming_window(std::uint32_t length) {
    return build_window(WindowType::Hamming, length);
}
inline std::vector<float> blackman_window(std::uint32_t length) {
    return build_window(WindowType::Blackman, length);
}
inline std::vector<float> rectangular_window(std::uint32_t length) {
    return build_window(WindowType::Rectangular, length);
}

// ------------------------------------------------------------------ //
// STFT / ISTFT / COLA / Griffin-Lim                                  //
// ------------------------------------------------------------------ //

inline std::vector<float> stft(const std::vector<float>& signal,
                               std::uint32_t n_fft,
                               std::uint32_t hop_length,
                               WindowType window = WindowType::Hann) {
    if (signal.empty()) detail::fail(Error::EmptySignal, "empty signal");
    if (n_fft == 0u || hop_length == 0u || !detail::is_valid_window(window)) {
        detail::fail(Error::InvalidParam, "invalid n_fft, hop_length, or window");
    }
    const std::size_t n = signal.size();
    const std::size_t nf = static_cast<std::size_t>(n_fft);
    const std::size_t hop = static_cast<std::size_t>(hop_length);
    if (nf > MAX_FFT_LEN || n > MAX_SAMPLES) {
        detail::fail(Error::InvalidParam, "n_fft or signal exceeds defensive cap");
    }
    if (n < nf) {
        detail::fail(Error::SignalTooShort, "signal shorter than n_fft");
    }
    const std::size_t num_frames = 1u + (n - nf) / hop;
    if (num_frames > MAX_FRAMES) {
        detail::fail(Error::InvalidParam, "num_frames exceeds MAX_FRAMES");
    }
    const std::size_t bins = nf / 2u + 1u;
    std::size_t frame_floats = 0, out_len = 0;
    if (!detail::checked_mul(bins, 2u, frame_floats) ||
        !detail::checked_mul(num_frames, frame_floats, out_len) ||
        out_len > MAX_SAMPLES) {
        detail::fail(Error::InvalidParam, "STFT output length exceeds MAX_SAMPLES");
    }
    if (!detail::is_pow2(nf)) {
        (void)detail::check_bluestein_m(nf);
    }
    detail::validate_floats(signal, Error::InvalidParam);

    auto win_d = detail::build_window_d(window, nf);
    std::vector<double> fft_buf(2u * nf, 0.0);
    std::vector<float> out(out_len);

    for (std::size_t m = 0; m < num_frames; ++m) {
        std::size_t frame_start = m * hop;
        std::size_t out_offset = m * frame_floats;
        for (std::size_t k = 0; k < nf; ++k) {
            fft_buf[2u * k] = static_cast<double>(signal[frame_start + k]) * win_d[k];
            fft_buf[2u * k + 1u] = 0.0;
        }
        detail::fft_complex_d(fft_buf, nf, false);
        for (std::size_t k = 0; k < frame_floats; ++k) {
            out[out_offset + k] = detail::narrow_f32(fft_buf[k], Error::Fft);
        }
    }
    return out;
}

inline std::vector<float> istft(const std::vector<float>& spectrogram,
                                std::uint32_t n_fft,
                                std::uint32_t hop_length,
                                WindowType window,
                                std::uint32_t output_length) {
    if (spectrogram.empty()) detail::fail(Error::InvalidSpectrogram, "empty spectrogram");
    if (n_fft == 0u || hop_length == 0u || output_length == 0u ||
        !detail::is_valid_window(window)) {
        detail::fail(Error::InvalidParam, "invalid n_fft, hop_length, output_length, or window");
    }
    const std::size_t nf = static_cast<std::size_t>(n_fft);
    const std::size_t hop = static_cast<std::size_t>(hop_length);
    const std::size_t olen = static_cast<std::size_t>(output_length);
    if (nf > MAX_FFT_LEN || olen > MAX_SAMPLES || spectrogram.size() > MAX_SAMPLES) {
        detail::fail(Error::InvalidParam, "input exceeds defensive cap");
    }
    const std::size_t bins = nf / 2u + 1u;
    std::size_t frame_floats = 0;
    if (!detail::checked_mul(bins, 2u, frame_floats)) {
        detail::fail(Error::InvalidParam, "frame_floats overflow");
    }
    if ((spectrogram.size() % frame_floats) != 0u) {
        detail::fail(Error::InvalidSpectrogram, "spectrogram length not divisible by frame_floats");
    }
    const std::size_t num_frames = spectrogram.size() / frame_floats;
    if (num_frames > MAX_FRAMES) {
        detail::fail(Error::InvalidParam, "num_frames exceeds MAX_FRAMES");
    }
    if (!detail::is_pow2(nf)) {
        (void)detail::check_bluestein_m(nf);
    }
    detail::validate_floats(spectrogram, Error::InvalidSpectrogram);

    auto win_d = detail::build_window_d(window, nf);
    std::vector<double> fft_buf(2u * nf, 0.0);
    std::vector<double> acc(olen, 0.0);
    std::vector<double> norm(olen, 0.0);
    const std::size_t reflect_end = (nf + 1u) / 2u;

    for (std::size_t m = 0; m < num_frames; ++m) {
        std::size_t frame_start_global = 0;
        if (!detail::checked_mul(m, hop, frame_start_global)) {
            frame_start_global = olen;
        }
        std::size_t frame_start_floats = m * frame_floats;
        std::fill(fft_buf.begin(), fft_buf.end(), 0.0);
        for (std::size_t k = 0; k < frame_floats; ++k) {
            fft_buf[k] = static_cast<double>(spectrogram[frame_start_floats + k]);
        }
        for (std::size_t k = 1u; k < reflect_end; ++k) {
            std::size_t dst = nf - k;
            fft_buf[2u * dst] = fft_buf[2u * k];
            fft_buf[2u * dst + 1u] = -fft_buf[2u * k + 1u];
        }
        detail::fft_complex_d(fft_buf, nf, true);
        for (std::size_t k = 0; k < nf; ++k) {
            std::size_t idx = 0;
            if (!detail::checked_add(frame_start_global, k, idx) || idx >= olen) break;
            double w = win_d[k];
            acc[idx] += fft_buf[2u * k] * w;
            norm[idx] += w * w;
        }
    }

    std::vector<float> out(olen);
    for (std::size_t n = 0; n < olen; ++n) {
        double val = norm[n] > 1e-10 ? (acc[n] / norm[n]) : 0.0;
        out[n] = detail::narrow_f32(val, Error::InvalidSpectrogram);
    }
    return out;
}

inline OlaStatus check_ola(std::uint32_t n_fft,
                           std::uint32_t hop_length,
                           WindowType window = WindowType::Hann) {
    const std::size_t nf = static_cast<std::size_t>(n_fft);
    const std::size_t hop = static_cast<std::size_t>(hop_length);
    if (n_fft == 0u || hop_length == 0u || nf > MAX_FFT_LEN ||
        !detail::is_valid_window(window)) {
        detail::fail(Error::InvalidParam, "invalid n_fft, hop_length, or window");
    }
    std::size_t sim_len = 0;
    if (!detail::checked_mul(nf, 4u, sim_len) || sim_len > MAX_SAMPLES) {
        detail::fail(Error::InvalidParam, "n_fft too large for OLA check");
    }
    const std::size_t num_frames = 1u + (sim_len - nf) / hop;
    auto win_d = detail::build_window_d(window, nf);
    std::vector<double> norm(sim_len, 0.0);
    for (std::size_t m = 0; m < num_frames; ++m) {
        std::size_t fs = m * hop;
        for (std::size_t k = 0; k < nf && (fs + k) < sim_len; ++k) {
            norm[fs + k] += win_d[k] * win_d[k];
        }
    }
    std::size_t start_check = nf;
    std::size_t end_check = sim_len - nf;
    if (end_check <= start_check) {
        start_check = 0u;
        end_check = sim_len;
    }
    double min_v = DBL_MAX;
    double max_v = 0.0;
    for (std::size_t i = start_check; i < end_check; ++i) {
        if (norm[i] < min_v) min_v = norm[i];
        if (norm[i] > max_v) max_v = norm[i];
    }
    OlaStatus st;
    st.min_norm = detail::narrow_f32(min_v);
    st.max_norm = detail::narrow_f32(max_v);
    st.is_nola = min_v > 1e-10;
    st.is_cola = st.is_nola && ((max_v - min_v) <= 1e-3 * max_v);
    return st;
}

inline std::vector<float> griffin_lim(const std::vector<float>& mag_spec,
                                      std::uint32_t n_fft,
                                      std::uint32_t hop_length,
                                      WindowType window,
                                      std::uint32_t n_iter,
                                      std::uint32_t output_length) {
    if (mag_spec.empty()) detail::fail(Error::InvalidSpectrogram, "empty magnitude spectrogram");
    if (n_fft == 0u || hop_length == 0u || output_length == 0u ||
        n_iter == 0u || n_iter > MAX_GRIFFIN_LIM_ITERS ||
        !detail::is_valid_window(window)) {
        detail::fail(Error::InvalidParam, "invalid griffin_lim parameter");
    }
    const std::size_t nf = static_cast<std::size_t>(n_fft);
    const std::size_t olen = static_cast<std::size_t>(output_length);
    if (nf > MAX_FFT_LEN || olen > MAX_SAMPLES || mag_spec.size() > MAX_SAMPLES || olen < nf) {
        detail::fail(Error::InvalidParam, "griffin_lim dimension out of bounds");
    }
    const std::size_t bins = nf / 2u + 1u;
    if ((mag_spec.size() % bins) != 0u) {
        detail::fail(Error::InvalidSpectrogram, "mag_spec length not divisible by bins");
    }
    const std::size_t num_frames = mag_spec.size() / bins;
    if (num_frames > MAX_FRAMES) {
        detail::fail(Error::InvalidParam, "num_frames exceeds MAX_FRAMES");
    }
    if (num_frames != (1u + (olen - nf) / static_cast<std::size_t>(hop_length))) {
        detail::fail(Error::InvalidSpectrogram, "mag_spec frame count mismatches output_length");
    }
    std::size_t spec_floats = 0;
    if (!detail::checked_mul(mag_spec.size(), 2u, spec_floats) ||
        spec_floats > MAX_SAMPLES) {
        detail::fail(Error::InvalidParam, "complex spectrogram size exceeds MAX_SAMPLES");
    }
    for (float v : mag_spec) {
        if (!detail::f_isfinite(v) || v < 0.0f) {
            detail::fail(Error::InvalidSpectrogram, "mag_spec must be finite and non-negative");
        }
    }

    std::vector<float> complex_spec(spec_floats, 0.0f);
    for (std::size_t i = 0; i < mag_spec.size(); ++i) {
        complex_spec[2u * i] = mag_spec[i];
    }

    for (std::uint32_t iter = 0; iter < n_iter; ++iter) {
        auto recon = istft(complex_spec, n_fft, hop_length, window, output_length);
        auto fwd = stft(recon, n_fft, hop_length, window);
        if (fwd.size() != spec_floats) {
            detail::fail(Error::InvalidSpectrogram, "griffin_lim frame count mismatch");
        }
        for (std::size_t i = 0; i < mag_spec.size(); ++i) {
            double re = static_cast<double>(fwd[2u * i]);
            double im = static_cast<double>(fwd[2u * i + 1u]);
            double cur = detail::d_sqrt(re * re + im * im);
            double target = static_cast<double>(mag_spec[i]);
            if (cur > 1e-12) {
                complex_spec[2u * i] = detail::narrow_f32(target * (re / cur), Error::InvalidSpectrogram);
                complex_spec[2u * i + 1u] = detail::narrow_f32(target * (im / cur), Error::InvalidSpectrogram);
            } else {
                complex_spec[2u * i] = mag_spec[i];
                complex_spec[2u * i + 1u] = 0.0f;
            }
        }
    }
    return istft(complex_spec, n_fft, hop_length, window, output_length);
}

// ------------------------------------------------------------------ //
// Magnitude, Power, and Log Spectrograms                             //
// ------------------------------------------------------------------ //

inline std::vector<float> magnitude_spectrogram(const std::vector<float>& signal,
                                                std::uint32_t n_fft,
                                                std::uint32_t hop_length,
                                                WindowType window = WindowType::Hann) {
    auto spec = stft(signal, n_fft, hop_length, window);
    const std::size_t total = spec.size() / 2u;
    std::vector<float> out(total);
    for (std::size_t i = 0; i < total; ++i) {
        double re = static_cast<double>(spec[2u * i]);
        double im = static_cast<double>(spec[2u * i + 1u]);
        out[i] = detail::narrow_f32(detail::d_sqrt(re * re + im * im));
    }
    return out;
}

inline std::vector<float> power_spectrogram(const std::vector<float>& signal,
                                            std::uint32_t n_fft,
                                            std::uint32_t hop_length,
                                            WindowType window = WindowType::Hann) {
    auto spec = stft(signal, n_fft, hop_length, window);
    const std::size_t total = spec.size() / 2u;
    std::vector<float> out(total);
    for (std::size_t i = 0; i < total; ++i) {
        double re = static_cast<double>(spec[2u * i]);
        double im = static_cast<double>(spec[2u * i + 1u]);
        out[i] = detail::narrow_f32(re * re + im * im);
    }
    return out;
}

inline std::vector<float> spectrogram(const std::vector<float>& signal,
                                      std::uint32_t n_fft,
                                      std::uint32_t hop_length,
                                      WindowType window = WindowType::Hann) {
    return power_spectrogram(signal, n_fft, hop_length, window);
}

inline std::vector<float> log_spectrogram(const std::vector<float>& signal,
                                          std::uint32_t n_fft,
                                          std::uint32_t hop_length,
                                          WindowType window = WindowType::Hann,
                                          float eps = 1e-10f) {
    if (signal.empty()) detail::fail(Error::EmptySignal, "empty signal");
    if (!detail::f_isfinite(eps) || eps <= 0.0f) {
        detail::fail(Error::InvalidParam, "eps must be positive and finite");
    }
    auto pwr = power_spectrogram(signal, n_fft, hop_length, window);
    for (float& v : pwr) {
        v = detail::narrow_f32(detail::d_ln(static_cast<double>(v) + static_cast<double>(eps)));
    }
    return pwr;
}

// ------------------------------------------------------------------ //
// HTK Mel Scale, Mel Filterbank, Mel Spectrogram, and MFCC           //
// ------------------------------------------------------------------ //

inline float hz_to_mel(float f_hz) {
    if (!detail::f_isfinite(f_hz) || f_hz < 0.0f) {
        detail::fail(Error::InvalidParam, "frequency must be finite and >= 0");
    }
    return detail::narrow_f32(2595.0 * detail::d_log10(1.0 + static_cast<double>(f_hz) / 700.0));
}

inline float mel_to_hz(float mel) {
    if (!detail::f_isfinite(mel) || mel < 0.0f) {
        detail::fail(Error::InvalidParam, "mel must be finite and >= 0");
    }
    return detail::narrow_f32(700.0 * (detail::d_pow10(static_cast<double>(mel) / 2595.0) - 1.0));
}

inline std::vector<float> mel_filterbank(std::uint32_t n_mels,
                                         std::uint32_t n_fft,
                                         float sample_rate) {
    auto fb_d = detail::build_mel_filterbank_d(n_mels, n_fft, sample_rate, false);
    if (fb_d.empty()) return {};
    std::vector<float> out(fb_d.size());
    for (std::size_t i = 0; i < fb_d.size(); ++i) {
        if (!detail::d_isfinite(fb_d[i]) ||
            fb_d[i] > static_cast<double>(FLT_MAX) ||
            fb_d[i] < -static_cast<double>(FLT_MAX)) {
            return {};
        }
        out[i] = static_cast<float>(fb_d[i]);
    }
    return out;
}

inline std::vector<float> mel_spectrogram(const std::vector<float>& signal,
                                          std::uint32_t n_fft,
                                          std::uint32_t hop_length,
                                          std::uint32_t n_mels,
                                          float sample_rate,
                                          WindowType window = WindowType::Hann) {
    if (signal.empty()) detail::fail(Error::EmptySignal, "empty signal");
    if (n_fft == 0u || hop_length == 0u || n_mels == 0u || !detail::is_valid_window(window)) {
        detail::fail(Error::InvalidParam, "invalid n_fft, hop_length, n_mels, or window");
    }
    const std::size_t nf = static_cast<std::size_t>(n_fft);
    const std::size_t hop = static_cast<std::size_t>(hop_length);
    const std::size_t nm = static_cast<std::size_t>(n_mels);
    if (nf > MAX_FFT_LEN || signal.size() > MAX_SAMPLES || nm > MAX_MELS) {
        detail::fail(Error::InvalidParam, "input exceeds defensive cap");
    }
    if (!detail::f_isfinite(sample_rate) || sample_rate <= 0.0f) {
        detail::fail(Error::InvalidParam, "sample_rate must be positive and finite");
    }
    float fmax_f = sample_rate * 0.5f;
    float bin_per_hz_f = static_cast<float>(n_fft) / sample_rate;
    if (!detail::f_isfinite(fmax_f) || fmax_f <= 0.0f ||
        !detail::f_isfinite(bin_per_hz_f) || bin_per_hz_f <= 0.0f) {
        detail::fail(Error::InvalidParam, "sample_rate underflows/overflows");
    }
    if (signal.size() < nf) {
        detail::fail(Error::SignalTooShort, "signal shorter than n_fft");
    }
    const std::size_t num_frames = 1u + (signal.size() - nf) / hop;
    if (num_frames > MAX_FRAMES) {
        detail::fail(Error::InvalidParam, "num_frames exceeds MAX_FRAMES");
    }
    const std::size_t bins = nf / 2u + 1u;
    std::size_t fb_len = 0, out_len = 0;
    if (!detail::checked_mul(nm, bins, fb_len) || fb_len > MAX_SAMPLES ||
        !detail::checked_mul(num_frames, nm, out_len) || out_len > MAX_SAMPLES) {
        detail::fail(Error::InvalidParam, "mel spectrogram size exceeds MAX_SAMPLES");
    }

    auto pwr = power_spectrogram(signal, n_fft, hop_length, window);
    auto fb_d = detail::build_mel_filterbank_d(n_mels, n_fft, sample_rate, true);
    std::vector<float> out(out_len);
    for (std::size_t t = 0; t < num_frames; ++t) {
        for (std::size_t m = 0; m < nm; ++m) {
            double acc = 0.0;
            for (std::size_t k = 0; k < bins; ++k) {
                acc += fb_d[m * bins + k] * static_cast<double>(pwr[t * bins + k]);
            }
            out[t * nm + m] = detail::narrow_f32(acc);
        }
    }
    return out;
}

inline std::vector<float> mfcc(const std::vector<float>& signal,
                               std::uint32_t n_fft,
                               std::uint32_t hop_length,
                               std::uint32_t n_mels,
                               std::uint32_t n_mfcc,
                               float sample_rate,
                               WindowType window = WindowType::Hann) {
    constexpr double EPS = 1e-10;
    if (signal.empty()) detail::fail(Error::EmptySignal, "empty signal");
    if (n_mfcc == 0u || n_mfcc > n_mels) {
        detail::fail(Error::InvalidParam, "n_mfcc must be in [1, n_mels]");
    }
    auto mel = mel_spectrogram(signal, n_fft, hop_length, n_mels, sample_rate, window);
    const std::size_t nm = static_cast<std::size_t>(n_mels);
    const std::size_t nc = static_cast<std::size_t>(n_mfcc);
    const std::size_t num_frames = mel.size() / nm;
    std::size_t total_out = 0;
    if (!detail::checked_mul(num_frames, nc, total_out) || total_out > MAX_SAMPLES) {
        detail::fail(Error::InvalidParam, "MFCC output exceeds MAX_SAMPLES");
    }

    std::vector<double> row_d(nm);
    std::vector<double> dct_d;
    std::vector<float> out(total_out);
    for (std::size_t t = 0; t < num_frames; ++t) {
        for (std::size_t m = 0; m < nm; ++m) {
            double lm = detail::d_ln(static_cast<double>(mel[t * nm + m]) + EPS);
            if (!detail::d_isfinite(lm)) {
                detail::fail(Error::InvalidParam, "log mel value became non-finite");
            }
            row_d[m] = lm;
        }
        detail::dct_ii_ortho_d(row_d, dct_d);
        for (std::size_t c = 0; c < nc; ++c) {
            out[t * nc + c] = detail::narrow_f32(dct_d[c]);
        }
    }
    return out;
}

} // namespace dsp_stft
} // namespace ca

#endif // CA_DSP_STFT_HPP
