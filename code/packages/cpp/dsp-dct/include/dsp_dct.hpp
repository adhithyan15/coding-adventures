// dsp_dct.hpp — Discrete Cosine and Sine Transforms (header-only, ISO C++17).
// ---------------------------------------------------------------------------
//
// Implements 1-D DCT-I..IV and DST-I..IV plus separable 2-D DCT/IDCT and
// DST/IDST in namespace `ca::dsp_dct`, under both un-normalised
// (`DctNorm::None`) and orthonormal (`DctNorm::Ortho`) scaling.
//
// DCT-II and DCT-III run in O(N log N) via Makhoul's reduction over an
// internal iterative radix-2 + Bluestein FFT in IEEE-754 double precision.
//
// Hardened with full VER15 CWE/CVE protections:
// - CWE-20:  Upfront enum domain checks, finite float input & output validation,
//            range-checked double -> float narrowing (no float-cast-overflow UB).
// - CWE-190: Overflow-checked size_t arithmetic for 2-D H*W and FFT sizing.
// - CWE-400: Defensive caps MAX_DCT_LEN (2^20) and MAX_SAMPLES (2^24).
// - CWE-476: Null-pointer guards on pointer-based `*_into` overloads.
// - CWE-674: Strictly iterative execution (zero recursion).
#ifndef CA_DSP_DCT_HPP
#define CA_DSP_DCT_HPP

#include <cfloat>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <exception>
#include <string>
#include <vector>

namespace ca {
namespace dsp_dct {

inline constexpr std::size_t MAX_DCT_LEN = std::size_t{1} << 20;  // 1,048,576
inline constexpr std::size_t MAX_SAMPLES = std::size_t{1} << 24;  // 16,777,216

enum class DctType { I = 1, II = 2, III = 3, IV = 4 };
enum class DstType { I = 1, II = 2, III = 3, IV = 4 };
enum class DctNorm { None = 0, Ortho = 1 };

enum class Error {
    EmptyInput,
    InvalidInput
};

class DctError : public std::exception {
public:
    explicit DctError(Error code) noexcept : code_(code) {}
    Error code() const noexcept { return code_; }
    const char* what() const noexcept override {
        return code_ == Error::EmptyInput ? "dsp_dct: empty input" : "dsp_dct: invalid input";
    }

private:
    Error code_;
};

namespace detail {

inline constexpr double PI = 3.14159265358979323846264338327950288;
inline constexpr double TWO_PI = 6.28318530717958647692528676655900577;

[[noreturn]] inline void fail(Error e) { throw DctError(e); }

inline bool checked_mul(std::size_t a, std::size_t b, std::size_t& out) noexcept {
    if (a != 0 && b > SIZE_MAX / a) return false;
    out = a * b;
    return true;
}

inline bool is_valid_dct_type(DctType t) noexcept {
    return t == DctType::I || t == DctType::II || t == DctType::III || t == DctType::IV;
}

inline bool is_valid_dst_type(DstType t) noexcept {
    return t == DstType::I || t == DstType::II || t == DstType::III || t == DstType::IV;
}

inline bool is_valid_norm(DctNorm n) noexcept {
    return n == DctNorm::None || n == DctNorm::Ortho;
}

inline float narrow_float(double v) {
    if (!std::isfinite(v) || std::abs(v) > static_cast<double>(FLT_MAX)) {
        fail(Error::InvalidInput);
    }
    float f = static_cast<float>(v);
    if (!std::isfinite(f)) {
        fail(Error::InvalidInput);
    }
    return f;
}

inline std::vector<float> narrow_vector(const std::vector<double>& src) {
    std::vector<float> out(src.size());
    for (std::size_t i = 0; i < src.size(); ++i) {
        out[i] = narrow_float(src[i]);
    }
    return out;
}

inline std::vector<double> validate_and_widen(const float* data, std::size_t n, std::size_t min_len) {
    if (n == 0) fail(Error::EmptyInput);
    if (data == nullptr) fail(Error::InvalidInput);
    if (n > MAX_DCT_LEN || n > MAX_SAMPLES) fail(Error::InvalidInput);
    if (n < min_len) fail(Error::InvalidInput);
    std::vector<double> out(n);
    for (std::size_t i = 0; i < n; ++i) {
        if (!std::isfinite(data[i])) fail(Error::InvalidInput);
        out[i] = static_cast<double>(data[i]);
    }
    return out;
}

inline void validate_2d_dims_and_nonempty(std::size_t image_len,
                                         std::uint32_t height, std::uint32_t width,
                                         std::size_t& total_out) {
    if (height == 0 || width == 0) fail(Error::InvalidInput);
    std::size_t h = static_cast<std::size_t>(height);
    std::size_t w = static_cast<std::size_t>(width);
    if (h > MAX_DCT_LEN || w > MAX_DCT_LEN) fail(Error::InvalidInput);
    std::size_t total = 0;
    if (!checked_mul(h, w, total) || total > MAX_SAMPLES) fail(Error::InvalidInput);
    if (image_len == 0) fail(Error::EmptyInput);
    total_out = total;
}

inline std::vector<double> validate_and_widen_2d(const float* image, std::size_t image_len,
                                                 std::uint32_t height, std::uint32_t width,
                                                 std::size_t min_axis_len) {
    std::size_t total = 0;
    validate_2d_dims_and_nonempty(image_len, height, width, total);
    std::size_t h = static_cast<std::size_t>(height);
    std::size_t w = static_cast<std::size_t>(width);
    if (h < min_axis_len || w < min_axis_len) fail(Error::InvalidInput);
    if (image == nullptr || image_len != total) fail(Error::InvalidInput);
    std::vector<double> out(total);
    for (std::size_t i = 0; i < total; ++i) {
        if (!std::isfinite(image[i])) fail(Error::InvalidInput);
        out[i] = static_cast<double>(image[i]);
    }
    return out;
}

inline void sincos_pi_frac(std::uint64_t num, std::uint64_t den, double& sin_out, double& cos_out) noexcept {
    std::uint64_t mod = std::uint64_t{2} * den;
    std::uint64_t rem = num % mod;
    double theta = (PI * static_cast<double>(rem)) / static_cast<double>(den);
    sin_out = std::sin(theta);
    cos_out = std::cos(theta);
}

inline bool is_pow2(std::size_t n) noexcept {
    return n >= 1 && (n & (n - 1)) == 0;
}

inline std::size_t next_pow2(std::size_t n) {
    std::size_t p = 1;
    while (p < n) {
        if (p > SIZE_MAX / 2) fail(Error::InvalidInput);
        p <<= 1;
    }
    return p;
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
        double w_step_r = std::cos(angle_step);
        double w_step_i = std::sin(angle_step);
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
                double next_wr = wr * w_step_r - wi * w_step_i;
                wi = wr * w_step_i + wi * w_step_r;
                wr = next_wr;
            }
        }
    }

    if (inverse) {
        double inv_n = 1.0 / static_cast<double>(n);
        for (double& v : buf) v *= inv_n;
    }
}

inline void fft_complex_inplace(std::vector<double>& buf, std::size_t n, bool inverse) {
    if (n == 1) return;
    if (is_pow2(n)) {
        fft_radix2_inplace(buf, n, inverse);
        return;
    }
    std::size_t two_n = 0;
    if (!checked_mul(n, 2, two_n)) fail(Error::InvalidInput);
    std::size_t m = next_pow2(two_n - 1);
    std::size_t two_m = 0;
    if (!checked_mul(m, 2, two_m)) fail(Error::InvalidInput);

    std::vector<double> a(two_m, 0.0);
    std::vector<double> b(two_m, 0.0);
    std::vector<double> chirp(two_n, 0.0);

    for (std::size_t k = 0; k < n; ++k) {
        std::uint64_t k2 = static_cast<std::uint64_t>(k) * static_cast<std::uint64_t>(k);
        double s = 0.0, c = 0.0;
        sincos_pi_frac(k2, static_cast<std::uint64_t>(n), s, c);
        chirp[2 * k] = c;
        chirp[2 * k + 1] = inverse ? s : -s;
    }

    for (std::size_t k = 0; k < n; ++k) {
        double xr = buf[2 * k];
        double xi = buf[2 * k + 1];
        double cr = chirp[2 * k];
        double ci = chirp[2 * k + 1];
        a[2 * k] = xr * cr - xi * ci;
        a[2 * k + 1] = xr * ci + xi * cr;
    }

    b[0] = chirp[0];
    b[1] = -chirp[1];
    for (std::size_t k = 1; k < n; ++k) {
        double cr = chirp[2 * k];
        double ci = -chirp[2 * k + 1];
        b[2 * k] = cr;
        b[2 * k + 1] = ci;
        b[2 * (m - k)] = cr;
        b[2 * (m - k) + 1] = ci;
    }

    fft_radix2_inplace(a, m, false);
    fft_radix2_inplace(b, m, false);
    for (std::size_t k = 0; k < m; ++k) {
        double ar = a[2 * k];
        double ai = a[2 * k + 1];
        double br = b[2 * k];
        double bi = b[2 * k + 1];
        a[2 * k] = ar * br - ai * bi;
        a[2 * k + 1] = ar * bi + ai * br;
    }
    fft_radix2_inplace(a, m, true);

    double inv_n = inverse ? (1.0 / static_cast<double>(n)) : 1.0;
    for (std::size_t k = 0; k < n; ++k) {
        double ar = a[2 * k];
        double ai = a[2 * k + 1];
        double cr = chirp[2 * k];
        double ci = chirp[2 * k + 1];
        buf[2 * k] = (ar * cr - ai * ci) * inv_n;
        buf[2 * k + 1] = (ar * ci + ai * cr) * inv_n;
    }
}

inline std::vector<double> dct_ii_core(const std::vector<double>& x, DctNorm norm) {
    std::size_t n = x.size();
    std::size_t two_n = 0;
    if (!checked_mul(n, 2, two_n)) fail(Error::InvalidInput);
    std::vector<double> interleaved(two_n, 0.0);

    std::size_t half_even = (n + 1) / 2;
    std::size_t half_odd = n / 2;
    for (std::size_t m = 0; m < half_even; ++m) {
        interleaved[2 * m] = x[2 * m];
    }
    for (std::size_t m = 0; m < half_odd; ++m) {
        interleaved[2 * (n - 1 - m)] = x[2 * m + 1];
    }

    fft_complex_inplace(interleaved, n, false);

    double s0 = (norm == DctNorm::Ortho) ? (1.0 / std::sqrt(static_cast<double>(n))) : 2.0;
    double sk = (norm == DctNorm::Ortho) ? std::sqrt(2.0 / static_cast<double>(n)) : 2.0;

    std::vector<double> out(n);
    for (std::size_t k = 0; k < n; ++k) {
        double sin_t = 0.0, cos_t = 0.0;
        sincos_pi_frac(static_cast<std::uint64_t>(k), static_cast<std::uint64_t>(2 * n), sin_t, cos_t);
        double scale = (k == 0) ? s0 : sk;
        out[k] = scale * (interleaved[2 * k] * cos_t + interleaved[2 * k + 1] * sin_t);
    }
    return out;
}

inline std::vector<double> dct_iii_unnorm_core(const std::vector<double>& x) {
    std::size_t n = x.size();
    if (n == 1) return {x[0]};
    std::size_t two_n = 0;
    if (!checked_mul(n, 2, two_n)) fail(Error::InvalidInput);
    std::vector<double> interleaved(two_n, 0.0);

    interleaved[0] = x[0];
    interleaved[1] = 0.0;
    for (std::size_t k = 1; k < n; ++k) {
        double sin_t = 0.0, cos_t = 0.0;
        sincos_pi_frac(static_cast<std::uint64_t>(k), static_cast<std::uint64_t>(2 * n), sin_t, cos_t);
        double re_k = x[k];
        double im_k = -x[n - k];
        interleaved[2 * k] = re_k * cos_t - im_k * sin_t;
        interleaved[2 * k + 1] = re_k * sin_t + im_k * cos_t;
    }

    fft_complex_inplace(interleaved, n, true);

    std::vector<double> out(n);
    double n_d = static_cast<double>(n);
    std::size_t half_even = (n + 1) / 2;
    std::size_t half_odd = n / 2;
    for (std::size_t m = 0; m < half_even; ++m) {
        out[2 * m] = interleaved[2 * m] * n_d;
    }
    for (std::size_t m = 0; m < half_odd; ++m) {
        out[2 * m + 1] = interleaved[2 * (n - 1 - m)] * n_d;
    }
    return out;
}

inline std::vector<double> dct_i_core(const std::vector<double>& x, DctNorm norm) {
    std::size_t n = x.size();
    std::size_t denom = n - 1;
    std::size_t m = 0, two_m = 0;
    if (!checked_mul(denom, 2, m) || !checked_mul(m, 2, two_m)) fail(Error::InvalidInput);
    std::vector<double> buf(two_m, 0.0);
    std::vector<double> out(n);
    if (norm == DctNorm::Ortho) {
        double inv_sqrt2 = 1.0 / std::sqrt(2.0);
        double scale = std::sqrt(2.0 / static_cast<double>(denom));
        buf[0] = x[0] * inv_sqrt2;
        buf[2 * denom] = x[denom] * inv_sqrt2;
        for (std::size_t k = 1; k < denom; ++k) {
            double half_x = 0.5 * x[k];
            buf[2 * k] = half_x;
            buf[2 * (m - k)] = half_x;
        }
        fft_complex_inplace(buf, m, false);
        for (std::size_t k = 0; k < n; ++k) {
            double wk = (k == 0 || k == n - 1) ? inv_sqrt2 : 1.0;
            out[k] = scale * wk * buf[2 * k];
        }
    } else {
        buf[0] = x[0];
        buf[2 * denom] = x[denom];
        for (std::size_t k = 1; k < denom; ++k) {
            buf[2 * k] = x[k];
            buf[2 * (m - k)] = x[k];
        }
        fft_complex_inplace(buf, m, false);
        for (std::size_t k = 0; k < n; ++k) out[k] = buf[2 * k];
    }
    return out;
}

inline std::vector<double> dct_dst_iv_core(const std::vector<double>& x, DctNorm norm, bool is_sine) {
    std::size_t n = x.size();
    std::size_t two_n = 0, four_n_dbl = 0;
    if (!checked_mul(n, 2, two_n) || !checked_mul(two_n, 2, four_n_dbl)) fail(Error::InvalidInput);
    std::vector<double> buf(four_n_dbl, 0.0);
    double scale = (norm == DctNorm::Ortho) ? std::sqrt(2.0 / static_cast<double>(n)) : 2.0;

    for (std::size_t k = 0; k < n; ++k) {
        double s = 0.0, c = 0.0;
        sincos_pi_frac(static_cast<std::uint64_t>(k), static_cast<std::uint64_t>(two_n), s, c);
        buf[2 * k] = x[k] * c;
        buf[2 * k + 1] = -x[k] * s;
    }

    fft_complex_inplace(buf, two_n, false);

    std::uint64_t four_n = std::uint64_t{4} * static_cast<std::uint64_t>(n);
    std::vector<double> out(n);
    for (std::size_t k = 0; k < n; ++k) {
        double s = 0.0, c = 0.0;
        sincos_pi_frac(std::uint64_t{2} * static_cast<std::uint64_t>(k) + 1, four_n, s, c);
        double yr = buf[2 * k], yi = buf[2 * k + 1];
        out[k] = is_sine ? (scale * (yr * s - yi * c)) : (scale * (yr * c + yi * s));
    }
    return out;
}

inline std::vector<double> dct_iv_core(const std::vector<double>& x, DctNorm norm) {
    return dct_dst_iv_core(x, norm, false);
}

inline std::vector<double> dst_i_core(const std::vector<double>& x, DctNorm norm) {
    std::size_t n = x.size();
    std::size_t n1 = n + 1;
    std::size_t m = 0, two_m = 0;
    if (!checked_mul(n1, 2, m) || !checked_mul(m, 2, two_m)) fail(Error::InvalidInput);
    std::vector<double> buf(two_m, 0.0);
    double scale = (norm == DctNorm::Ortho) ? (0.5 * std::sqrt(2.0 / static_cast<double>(n1))) : 1.0;

    for (std::size_t k = 0; k < n; ++k) {
        buf[2 * (k + 1)] = x[k];
        buf[2 * (m - (k + 1))] = -x[k];
    }
    fft_complex_inplace(buf, m, false);

    std::vector<double> out(n);
    for (std::size_t k = 0; k < n; ++k) {
        out[k] = -buf[2 * (k + 1) + 1] * scale;
    }
    return out;
}

inline std::vector<double> dst_ii_core(const std::vector<double>& x, DctNorm norm) {
    std::size_t n = x.size();
    std::vector<double> y(n);
    for (std::size_t k = 0; k < n; ++k) {
        y[k] = (k & 1u) ? -x[k] : x[k];
    }
    std::vector<double> tmp = dct_ii_core(y, norm);
    std::vector<double> out(n);
    for (std::size_t k = 0; k < n; ++k) {
        out[k] = tmp[n - 1 - k];
    }
    return out;
}

inline std::vector<double> dst_iii_core(const std::vector<double>& x, DctNorm norm) {
    std::size_t n = x.size();
    std::vector<double> y(n);
    if (norm == DctNorm::Ortho) {
        double s0 = 1.0 / std::sqrt(static_cast<double>(n));
        double sk = 1.0 / std::sqrt(2.0 * static_cast<double>(n));
        y[0] = x[n - 1] * s0;
        for (std::size_t k = 1; k < n; ++k) {
            y[k] = x[n - 1 - k] * sk;
        }
    } else {
        for (std::size_t k = 0; k < n; ++k) {
            y[k] = x[n - 1 - k];
        }
    }
    std::vector<double> out = dct_iii_unnorm_core(y);
    for (std::size_t k = 1; k < n; k += 2) {
        out[k] = -out[k];
    }
    return out;
}

inline std::vector<double> dst_iv_core(const std::vector<double>& x, DctNorm norm) {
    return dct_dst_iv_core(x, norm, true);
}

inline std::vector<double> dct_forward_double(const std::vector<double>& x, DctType type, DctNorm norm) {
    std::size_t n = x.size();
    switch (type) {
        case DctType::I:
            return dct_i_core(x, norm);
        case DctType::II:
            return dct_ii_core(x, norm);
        case DctType::III: {
            if (norm == DctNorm::Ortho) {
                std::vector<double> scaled = x;
                double s0 = 1.0 / std::sqrt(static_cast<double>(n));
                double sk = 1.0 / std::sqrt(2.0 * static_cast<double>(n));
                scaled[0] *= s0;
                for (std::size_t k = 1; k < n; ++k) scaled[k] *= sk;
                return dct_iii_unnorm_core(scaled);
            }
            return dct_iii_unnorm_core(x);
        }
        case DctType::IV:
            return dct_iv_core(x, norm);
    }
    fail(Error::InvalidInput);
}

inline std::vector<double> dct_inverse_double(const std::vector<double>& x, DctType type, DctNorm norm) {
    std::size_t n = x.size();
    if (type == DctType::III) {
        if (norm == DctNorm::Ortho) {
            return dct_forward_double(x, DctType::III, DctNorm::Ortho);
        }
        std::vector<double> out = dct_iii_unnorm_core(x);
        double inv = 1.0 / (2.0 * static_cast<double>(n));
        for (double& v : out) v *= inv;
        return out;
    }
    if (type == DctType::II) {
        if (norm == DctNorm::Ortho) {
            return dct_ii_core(x, DctNorm::Ortho);
        }
        std::vector<double> out = dct_ii_core(x, DctNorm::None);
        double inv = 1.0 / (2.0 * static_cast<double>(n));
        for (double& v : out) v *= inv;
        return out;
    }
    if (type == DctType::I) {
        if (norm == DctNorm::Ortho) return dct_i_core(x, DctNorm::Ortho);
        std::vector<double> out = dct_i_core(x, DctNorm::None);
        double inv = 1.0 / (2.0 * static_cast<double>(n - 1));
        for (double& v : out) v *= inv;
        return out;
    }
    if (type == DctType::IV) {
        if (norm == DctNorm::Ortho) return dct_iv_core(x, DctNorm::Ortho);
        std::vector<double> out = dct_iv_core(x, DctNorm::None);
        double inv = 1.0 / (2.0 * static_cast<double>(n));
        for (double& v : out) v *= inv;
        return out;
    }
    fail(Error::InvalidInput);
}

inline std::vector<double> dst_forward_double(const std::vector<double>& x, DstType type, DctNorm norm) {
    switch (type) {
        case DstType::I:   return dst_i_core(x, norm);
        case DstType::II:  return dst_ii_core(x, norm);
        case DstType::III: return dst_iii_core(x, norm);
        case DstType::IV:  return dst_iv_core(x, norm);
    }
    fail(Error::InvalidInput);
}

inline std::vector<double> dst_inverse_double(const std::vector<double>& x, DstType type, DctNorm norm) {
    std::size_t n = x.size();
    if (type == DstType::III) {
        if (norm == DctNorm::Ortho) return dst_iii_core(x, DctNorm::Ortho);
        std::vector<double> out = dst_iii_core(x, DctNorm::None);
        double inv = 1.0 / (2.0 * static_cast<double>(n));
        for (double& v : out) v *= inv;
        return out;
    }
    if (type == DstType::II) {
        if (norm == DctNorm::Ortho) return dst_ii_core(x, DctNorm::Ortho);
        std::vector<double> out = dst_ii_core(x, DctNorm::None);
        double inv = 1.0 / (2.0 * static_cast<double>(n));
        for (double& v : out) v *= inv;
        return out;
    }
    if (type == DstType::I) {
        if (norm == DctNorm::Ortho) return dst_i_core(x, DctNorm::Ortho);
        std::vector<double> out = dst_i_core(x, DctNorm::None);
        double inv = 1.0 / (2.0 * static_cast<double>(n + 1));
        for (double& v : out) v *= inv;
        return out;
    }
    if (type == DstType::IV) {
        if (norm == DctNorm::Ortho) return dst_iv_core(x, DctNorm::Ortho);
        std::vector<double> out = dst_iv_core(x, DctNorm::None);
        double inv = 1.0 / (2.0 * static_cast<double>(n));
        for (double& v : out) v *= inv;
        return out;
    }
    fail(Error::InvalidInput);
}

template <typename Fn>
inline std::vector<double> apply_2d_forward(const std::vector<double>& img, std::size_t h, std::size_t w, Fn fn) {
    std::vector<double> inter(h * w);
    std::vector<double> row(w);
    for (std::size_t r = 0; r < h; ++r) {
        std::memcpy(row.data(), img.data() + r * w, w * sizeof(double));
        std::vector<double> res = fn(row);
        std::memcpy(inter.data() + r * w, res.data(), w * sizeof(double));
    }
    std::vector<double> out(h * w);
    std::vector<double> col(h);
    for (std::size_t c = 0; c < w; ++c) {
        for (std::size_t r = 0; r < h; ++r) col[r] = inter[r * w + c];
        std::vector<double> res = fn(col);
        for (std::size_t r = 0; r < h; ++r) out[r * w + c] = res[r];
    }
    return out;
}

template <typename Fn>
inline std::vector<double> apply_2d_inverse(const std::vector<double>& img, std::size_t h, std::size_t w, Fn fn) {
    std::vector<double> inter(h * w);
    std::vector<double> col(h);
    for (std::size_t c = 0; c < w; ++c) {
        for (std::size_t r = 0; r < h; ++r) col[r] = img[r * w + c];
        std::vector<double> res = fn(col);
        for (std::size_t r = 0; r < h; ++r) inter[r * w + c] = res[r];
    }
    std::vector<double> out(h * w);
    std::vector<double> row(w);
    for (std::size_t r = 0; r < h; ++r) {
        std::memcpy(row.data(), inter.data() + r * w, w * sizeof(double));
        std::vector<double> res = fn(row);
        std::memcpy(out.data() + r * w, res.data(), w * sizeof(double));
    }
    return out;
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
// Public 1-D API                                                     //
// ------------------------------------------------------------------ //

inline std::vector<float> dct(const std::vector<float>& signal,
                              DctType type = DctType::II,
                              DctNorm norm = DctNorm::Ortho) {
    if (signal.empty()) detail::fail(Error::EmptyInput);
    if (!detail::is_valid_dct_type(type) || !detail::is_valid_norm(norm)) {
        detail::fail(Error::InvalidInput);
    }
    std::vector<double> din = detail::validate_and_widen(
        signal.data(), signal.size(), type == DctType::I ? 2 : 1);
    return detail::narrow_vector(detail::dct_forward_double(din, type, norm));
}

inline std::vector<float> idct(const std::vector<float>& coeffs,
                               DctType type = DctType::III,
                               DctNorm norm = DctNorm::Ortho) {
    if (coeffs.empty()) detail::fail(Error::EmptyInput);
    if (!detail::is_valid_dct_type(type) || !detail::is_valid_norm(norm)) {
        detail::fail(Error::InvalidInput);
    }
    std::vector<double> din = detail::validate_and_widen(
        coeffs.data(), coeffs.size(), type == DctType::I ? 2 : 1);
    return detail::narrow_vector(detail::dct_inverse_double(din, type, norm));
}

inline std::vector<float> dst(const std::vector<float>& signal,
                              DstType type = DstType::II,
                              DctNorm norm = DctNorm::Ortho) {
    if (signal.empty()) detail::fail(Error::EmptyInput);
    if (!detail::is_valid_dst_type(type) || !detail::is_valid_norm(norm)) {
        detail::fail(Error::InvalidInput);
    }
    std::vector<double> din = detail::validate_and_widen(
        signal.data(), signal.size(), type == DstType::I ? 2 : 1);
    return detail::narrow_vector(detail::dst_forward_double(din, type, norm));
}

inline std::vector<float> idst(const std::vector<float>& coeffs,
                               DstType type = DstType::III,
                               DctNorm norm = DctNorm::Ortho) {
    if (coeffs.empty()) detail::fail(Error::EmptyInput);
    if (!detail::is_valid_dst_type(type) || !detail::is_valid_norm(norm)) {
        detail::fail(Error::InvalidInput);
    }
    std::vector<double> din = detail::validate_and_widen(
        coeffs.data(), coeffs.size(), type == DstType::I ? 2 : 1);
    return detail::narrow_vector(detail::dst_inverse_double(din, type, norm));
}

// Alias-safe in-place / caller-buffer variants (`out == signal` supported).
inline void dct_into(const float* signal, std::size_t len, DctType type, DctNorm norm, float* out) {
    if (out == nullptr) detail::fail(Error::InvalidInput);
    if (len == 0) detail::fail(Error::EmptyInput);
    if (!detail::is_valid_dct_type(type) || !detail::is_valid_norm(norm)) {
        detail::fail(Error::InvalidInput);
    }
    std::vector<double> din = detail::validate_and_widen(signal, len, type == DctType::I ? 2 : 1);
    std::vector<float> tmp = detail::narrow_vector(detail::dct_forward_double(din, type, norm));
    std::memcpy(out, tmp.data(), len * sizeof(float));
}

inline void idct_into(const float* coeffs, std::size_t len, DctType type, DctNorm norm, float* out) {
    if (out == nullptr) detail::fail(Error::InvalidInput);
    if (len == 0) detail::fail(Error::EmptyInput);
    if (!detail::is_valid_dct_type(type) || !detail::is_valid_norm(norm)) {
        detail::fail(Error::InvalidInput);
    }
    std::vector<double> din = detail::validate_and_widen(coeffs, len, type == DctType::I ? 2 : 1);
    std::vector<float> tmp = detail::narrow_vector(detail::dct_inverse_double(din, type, norm));
    std::memcpy(out, tmp.data(), len * sizeof(float));
}

// ------------------------------------------------------------------ //
// Public 2-D API                                                     //
// ------------------------------------------------------------------ //

inline std::vector<float> dct_2d(const std::vector<float>& image,
                                 std::uint32_t height,
                                 std::uint32_t width,
                                 DctType type = DctType::II,
                                 DctNorm norm = DctNorm::Ortho) {
    std::size_t total = 0;
    detail::validate_2d_dims_and_nonempty(image.size(), height, width, total);
    if (!detail::is_valid_dct_type(type) || !detail::is_valid_norm(norm)) {
        detail::fail(Error::InvalidInput);
    }
    std::vector<double> din = detail::validate_and_widen_2d(
        image.data(), image.size(), height, width, type == DctType::I ? 2 : 1);
    return detail::narrow_vector(detail::apply_2d_forward(
        din, static_cast<std::size_t>(height), static_cast<std::size_t>(width),
        [type, norm](const std::vector<double>& v) {
            return detail::dct_forward_double(v, type, norm);
        }));
}

inline std::vector<float> idct_2d(const std::vector<float>& coeffs,
                                  std::uint32_t height,
                                  std::uint32_t width,
                                  DctType type = DctType::III,
                                  DctNorm norm = DctNorm::Ortho) {
    std::size_t total = 0;
    detail::validate_2d_dims_and_nonempty(coeffs.size(), height, width, total);
    if (!detail::is_valid_dct_type(type) || !detail::is_valid_norm(norm)) {
        detail::fail(Error::InvalidInput);
    }
    std::vector<double> din = detail::validate_and_widen_2d(
        coeffs.data(), coeffs.size(), height, width, type == DctType::I ? 2 : 1);
    return detail::narrow_vector(detail::apply_2d_inverse(
        din, static_cast<std::size_t>(height), static_cast<std::size_t>(width),
        [type, norm](const std::vector<double>& v) {
            return detail::dct_inverse_double(v, type, norm);
        }));
}

inline std::vector<float> dst_2d(const std::vector<float>& image,
                                 std::uint32_t height,
                                 std::uint32_t width,
                                 DstType type = DstType::II,
                                 DctNorm norm = DctNorm::Ortho) {
    std::size_t total = 0;
    detail::validate_2d_dims_and_nonempty(image.size(), height, width, total);
    if (!detail::is_valid_dst_type(type) || !detail::is_valid_norm(norm)) {
        detail::fail(Error::InvalidInput);
    }
    std::vector<double> din = detail::validate_and_widen_2d(
        image.data(), image.size(), height, width, type == DstType::I ? 2 : 1);
    return detail::narrow_vector(detail::apply_2d_forward(
        din, static_cast<std::size_t>(height), static_cast<std::size_t>(width),
        [type, norm](const std::vector<double>& v) {
            return detail::dst_forward_double(v, type, norm);
        }));
}

inline std::vector<float> idst_2d(const std::vector<float>& coeffs,
                                  std::uint32_t height,
                                  std::uint32_t width,
                                  DstType type = DstType::III,
                                  DctNorm norm = DctNorm::Ortho) {
    std::size_t total = 0;
    detail::validate_2d_dims_and_nonempty(coeffs.size(), height, width, total);
    if (!detail::is_valid_dst_type(type) || !detail::is_valid_norm(norm)) {
        detail::fail(Error::InvalidInput);
    }
    std::vector<double> din = detail::validate_and_widen_2d(
        coeffs.data(), coeffs.size(), height, width, type == DstType::I ? 2 : 1);
    return detail::narrow_vector(detail::apply_2d_inverse(
        din, static_cast<std::size_t>(height), static_cast<std::size_t>(width),
        [type, norm](const std::vector<double>& v) {
            return detail::dst_inverse_double(v, type, norm);
        }));
}

// ------------------------------------------------------------------ //
// Parsing helpers                                                    //
// ------------------------------------------------------------------ //

inline DctType parse_dct_type(const std::string& str) {
    std::string k = detail::trim_lower(str);
    if (k == "1" || k == "i" || k == "dct-i") return DctType::I;
    if (k == "2" || k == "ii" || k == "dct-ii") return DctType::II;
    if (k == "3" || k == "iii" || k == "dct-iii") return DctType::III;
    if (k == "4" || k == "iv" || k == "dct-iv") return DctType::IV;
    detail::fail(Error::InvalidInput);
}

inline DstType parse_dst_type(const std::string& str) {
    std::string k = detail::trim_lower(str);
    if (k == "1" || k == "i" || k == "dst-i") return DstType::I;
    if (k == "2" || k == "ii" || k == "dst-ii") return DstType::II;
    if (k == "3" || k == "iii" || k == "dst-iii") return DstType::III;
    if (k == "4" || k == "iv" || k == "dst-iv") return DstType::IV;
    detail::fail(Error::InvalidInput);
}

inline DctNorm parse_norm(const std::string& str) {
    std::string k = detail::trim_lower(str);
    if (k == "none" || k == "raw" || k == "unnorm" || k == "0") return DctNorm::None;
    if (k == "ortho" || k == "unitary" || k == "1") return DctNorm::Ortho;
    detail::fail(Error::InvalidInput);
}

} // namespace dsp_dct
} // namespace ca

#endif // CA_DSP_DCT_HPP
