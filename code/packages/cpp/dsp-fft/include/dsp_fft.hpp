// dsp_fft.hpp — Iterative 1-D FFT / IFFT / RFFT / IRFFT (header-only, ISO C++17).
// ==============================================================================
//
// A faithful C++17 port of the `dsp-fft` crate (DSP01) in `namespace ca::dsp_fft`,
// built on `ca::dsp_complex::ComplexTensor`.
//
// Algorithms:
// - Iterative decimation-in-time radix-2 Cooley-Tukey for power-of-two N = 2^k.
// - Bluestein chirp-Z convolution for arbitrary N >= 1.
// - Half-spectrum RFFT / IRFFT (`floor(N/2) + 1` bins) for real signals.
// - Backward normalization: forward scale 1, inverse scale 1/N.
//
// Hardened against VER15 CWEs:
// - CWE-20:  Rejects NaN, +Inf, -Inf, zero length, odd interleaved buffers,
//            mismatched `irfft` bin counts, and non-pow2 sizes on `*_scalar`.
// - CWE-190: Overflow-checked `std::size_t` arithmetic.
// - CWE-400: Pre-flight `MAX_FFT_LEN` (2^20) & `MAX_SAMPLES` (2^24) bounds.
// - CWE-476: Pointer-length overloads reject `nullptr`.
// - CWE-674: Strictly iterative, zero recursion.
#ifndef CA_DSP_FFT_HPP
#define CA_DSP_FFT_HPP

#include "dsp_complex.hpp"

#include <cmath>
#include <cstddef>
#include <cstdint>
#include <exception>
#include <limits>
#include <utility>
#include <vector>

namespace ca {
namespace dsp_fft {

inline constexpr std::size_t MAX_FFT_LEN = (std::size_t{1} << 20);
inline constexpr std::size_t MAX_SAMPLES = (std::size_t{1} << 24);

enum class Error {
    InvalidInput,
    NotPowerOfTwo
};

enum class Direction {
    Forward,
    Inverse
};

class FftError : public std::exception {
public:
    explicit FftError(Error code) noexcept : code_(code) {}
    Error code() const noexcept { return code_; }
    const char* what() const noexcept override {
        switch (code_) {
        case Error::InvalidInput:
            return "dsp_fft: invalid input";
        case Error::NotPowerOfTwo:
            return "dsp_fft: length is not a power of two";
        }
        return "dsp_fft: unknown error";
    }

private:
    Error code_;
};

inline bool is_power_of_two(std::size_t n) noexcept {
    return (n != 0u) && ((n & (n - 1u)) == 0u);
}

namespace detail {

inline constexpr double PI_D = 3.14159265358979323846;
inline constexpr double TWO_PI_D = 6.28318530717958647692;

[[noreturn]] inline void fail(Error e) { throw FftError(e); }

inline bool is_finite(float x) noexcept {
    return (x == x) && ((x - x) == 0.0f);
}

inline void require_all_finite(const float* buf, std::size_t count) {
    if (buf == nullptr && count > 0u) {
        fail(Error::InvalidInput);
    }
    for (std::size_t i = 0u; i < count; ++i) {
        if (!is_finite(buf[i])) {
            fail(Error::InvalidInput);
        }
    }
}

inline void require_all_finite(const std::vector<float>& buf) {
    require_all_finite(buf.data(), buf.size());
}

inline void require_valid_dir(Direction dir) {
    if (dir != Direction::Forward && dir != Direction::Inverse) {
        fail(Error::InvalidInput);
    }
}

inline ca::dsp_complex::ComplexTensor wrap_interleaved(std::vector<float>&& buf) {
    require_all_finite(buf);
    try {
        return ca::dsp_complex::ComplexTensor::from_interleaved(std::move(buf));
    } catch (const ca::dsp_complex::ComplexError&) {
        fail(Error::InvalidInput);
    }
}

inline bool checked_mul(std::size_t a, std::size_t b, std::size_t& out) noexcept {
    if (a != 0 && b > std::numeric_limits<std::size_t>::max() / a) {
        return false;
    }
    out = a * b;
    return true;
}

inline std::size_t validate_interleaved_bounds(const float* buf, std::size_t float_len,
                                               bool require_pow2) {
    if (buf == nullptr && float_len > 0u) {
        fail(Error::InvalidInput);
    }
    if ((float_len % 2u) != 0u) {
        fail(Error::InvalidInput);
    }
    if (float_len > MAX_SAMPLES) {
        fail(Error::InvalidInput);
    }
    std::size_t n = float_len / 2u;
    if (n == 0u || n > MAX_FFT_LEN) {
        fail(Error::InvalidInput);
    }
    if (require_pow2 && !is_power_of_two(n)) {
        fail(Error::NotPowerOfTwo);
    }
    return n;
}

inline std::size_t validate_interleaved(const float* buf, std::size_t float_len,
                                        bool require_pow2) {
    std::size_t n = validate_interleaved_bounds(buf, float_len, require_pow2);
    require_all_finite(buf, float_len);
    return n;
}

struct BluesteinBounds {
    std::size_t m;
    std::size_t two_m;
};

inline BluesteinBounds require_bluestein_bounds(std::size_t n) {
    if (n == 0u || n > MAX_FFT_LEN) {
        fail(Error::InvalidInput);
    }
    if (n == 1u) {
        return {1u, 2u};
    }
    std::size_t two_n = 0u;
    if (!checked_mul(n, 2u, two_n) || two_n < 1u) {
        fail(Error::InvalidInput);
    }
    std::size_t conv_len = two_n - 1u;
    std::size_t m = 1u;
    while (m < conv_len) {
        if (m > MAX_FFT_LEN / 2u) {
            fail(Error::InvalidInput);
        }
        m <<= 1u;
    }
    std::size_t two_m = 0u;
    if (m > MAX_FFT_LEN || !checked_mul(m, 2u, two_m) || two_m > MAX_SAMPLES) {
        fail(Error::InvalidInput);
    }
    return {m, two_m};
}

inline std::size_t bit_reverse(std::size_t x, unsigned int bits) noexcept {
    std::size_t r = 0u;
    for (unsigned int b = 0u; b < bits; ++b) {
        r = (r << 1u) | (x & 1u);
        x >>= 1u;
    }
    return r;
}

inline void bit_reversal_permute(std::vector<float>& buf, std::size_t n) noexcept {
    unsigned int log_n = 0u;
    std::size_t tmp = n;
    while (tmp > 1u) {
        tmp >>= 1u;
        log_n += 1u;
    }
    for (std::size_t i = 0u; i < n; ++i) {
        std::size_t j = bit_reverse(i, log_n);
        if (j > i) {
            std::swap(buf[2u * i], buf[2u * j]);
            std::swap(buf[2u * i + 1u], buf[2u * j + 1u]);
        }
    }
}

inline void butterflies(std::vector<float>& buf, std::size_t n, Direction dir) noexcept {
    double sign = (dir == Direction::Forward) ? -1.0 : 1.0;
    std::size_t full = 2u;
    while (full <= n) {
        std::size_t half = full / 2u;
        for (std::size_t block_start = 0u; block_start < n; block_start += full) {
            for (std::size_t j = 0u; j < half; ++j) {
                double theta = sign * TWO_PI_D * static_cast<double>(j) / static_cast<double>(full);
                float w_re = static_cast<float>(std::cos(theta));
                float w_im = static_cast<float>(std::sin(theta));

                std::size_t x_idx = 2u * (block_start + j);
                std::size_t y_idx = 2u * (block_start + j + half);
                float x_re = buf[x_idx];
                float x_im = buf[x_idx + 1u];
                float y_re = buf[y_idx];
                float y_im = buf[y_idx + 1u];

                float t_re = w_re * y_re - w_im * y_im;
                float t_im = w_re * y_im + w_im * y_re;

                buf[x_idx] = x_re + t_re;
                buf[x_idx + 1u] = x_im + t_im;
                buf[y_idx] = x_re - t_re;
                buf[y_idx + 1u] = x_im - t_im;
            }
        }
        full *= 2u;
    }
}

inline std::vector<float> build_chirp(std::size_t n, double sign) {
    std::size_t two_n_sz = 0u;
    if (!checked_mul(n, 2u, two_n_sz)) {
        fail(Error::InvalidInput);
    }
    std::vector<float> out(two_n_sz);
    std::uint64_t two_n = static_cast<std::uint64_t>(n) * 2ULL;
    for (std::size_t k = 0u; k < n; ++k) {
        std::uint64_t k_sq = static_cast<std::uint64_t>(k) * static_cast<std::uint64_t>(k);
        std::uint64_t residue = k_sq % two_n;
        double theta = sign * PI_D * static_cast<double>(residue) / static_cast<double>(n);
        out[2u * k] = static_cast<float>(std::cos(theta));
        out[2u * k + 1u] = static_cast<float>(std::sin(theta));
    }
    return out;
}

} // namespace detail

inline std::vector<float> fft_scalar(const float* signal, std::size_t float_len) {
    std::size_t n = detail::validate_interleaved(signal, float_len, true);
    std::vector<float> buf(signal, signal + float_len);
    detail::bit_reversal_permute(buf, n);
    detail::butterflies(buf, n, Direction::Forward);
    detail::require_all_finite(buf);
    return buf;
}

inline std::vector<float> fft_scalar(const std::vector<float>& signal) {
    return fft_scalar(signal.empty() ? nullptr : signal.data(), signal.size());
}

inline std::vector<float> ifft_scalar(const float* spectrum, std::size_t float_len) {
    std::size_t n = detail::validate_interleaved(spectrum, float_len, true);
    std::vector<float> buf(spectrum, spectrum + float_len);
    detail::bit_reversal_permute(buf, n);
    detail::butterflies(buf, n, Direction::Inverse);
    float inv_n = 1.0f / static_cast<float>(n);
    for (float& x : buf) {
        x *= inv_n;
    }
    detail::require_all_finite(buf);
    return buf;
}

inline std::vector<float> ifft_scalar(const std::vector<float>& spectrum) {
    return ifft_scalar(spectrum.empty() ? nullptr : spectrum.data(), spectrum.size());
}

inline std::vector<float> bluestein_scalar(const float* signal, std::size_t float_len,
                                           Direction dir) {
    detail::require_valid_dir(dir);
    std::size_t n = detail::validate_interleaved_bounds(signal, float_len, false);
    auto bounds = detail::require_bluestein_bounds(n);
    detail::require_all_finite(signal, float_len);
    if (n == 1u) {
        return {signal[0], signal[1]};
    }

    std::size_t two_n = float_len;
    std::size_t m = bounds.m;
    std::size_t two_m = bounds.two_m;

    double sign = (dir == Direction::Forward) ? -1.0 : 1.0;
    std::vector<float> chirp = detail::build_chirp(n, sign);

    std::vector<float> a(two_m, 0.0f);
    for (std::size_t k = 0u; k < n; ++k) {
        float xr = signal[2u * k];
        float xi = signal[2u * k + 1u];
        float cr = chirp[2u * k];
        float ci = chirp[2u * k + 1u];
        a[2u * k] = xr * cr - xi * ci;
        a[2u * k + 1u] = xr * ci + xi * cr;
    }

    std::vector<float> b(two_m, 0.0f);
    for (std::size_t k = 0u; k < n; ++k) {
        b[2u * k] = chirp[2u * k];
        b[2u * k + 1u] = -chirp[2u * k + 1u];
    }
    for (std::size_t k = 1u; k < n; ++k) {
        std::size_t idx = m - k;
        b[2u * idx] = chirp[2u * k];
        b[2u * idx + 1u] = -chirp[2u * k + 1u];
    }

    std::vector<float> a_spec = fft_scalar(a);
    std::vector<float> b_spec = fft_scalar(b);
    std::vector<float> c_spec(two_m, 0.0f);
    for (std::size_t k = 0u; k < m; ++k) {
        float ar = a_spec[2u * k];
        float ai = a_spec[2u * k + 1u];
        float br = b_spec[2u * k];
        float bi = b_spec[2u * k + 1u];
        c_spec[2u * k] = ar * br - ai * bi;
        c_spec[2u * k + 1u] = ar * bi + ai * br;
    }
    std::vector<float> conv = ifft_scalar(c_spec);

    float inv_n = (dir == Direction::Inverse) ? (1.0f / static_cast<float>(n)) : 1.0f;
    std::vector<float> out(two_n);
    for (std::size_t k = 0u; k < n; ++k) {
        float cr = conv[2u * k];
        float ci = conv[2u * k + 1u];
        float wr = chirp[2u * k];
        float wi = chirp[2u * k + 1u];
        float re = wr * cr - wi * ci;
        float im = wr * ci + wi * cr;
        out[2u * k] = re * inv_n;
        out[2u * k + 1u] = im * inv_n;
    }
    detail::require_all_finite(out);
    return out;
}

inline std::vector<float> bluestein_scalar(const std::vector<float>& signal, Direction dir) {
    return bluestein_scalar(signal.empty() ? nullptr : signal.data(), signal.size(), dir);
}

namespace detail {

inline std::vector<float> run_complex_auto(const float* interleaved, std::size_t n,
                                           Direction dir) {
    std::size_t float_len = 0u;
    if (!checked_mul(n, 2u, float_len)) {
        fail(Error::InvalidInput);
    }
    if (is_power_of_two(n)) {
        return (dir == Direction::Forward)
                   ? fft_scalar(interleaved, float_len)
                   : ifft_scalar(interleaved, float_len);
    }
    return bluestein_scalar(interleaved, float_len, dir);
}

} // namespace detail

inline std::vector<float> rfft_scalar(const float* signal, std::size_t signal_len) {
    if (signal == nullptr || signal_len == 0u || signal_len > MAX_FFT_LEN ||
        signal_len > MAX_SAMPLES) {
        detail::fail(Error::InvalidInput);
    }
    std::size_t two_n = 0u;
    if (!detail::checked_mul(signal_len, 2u, two_n) || two_n > MAX_SAMPLES) {
        detail::fail(Error::InvalidInput);
    }
    if (!is_power_of_two(signal_len)) {
        (void)detail::require_bluestein_bounds(signal_len);
    }
    std::size_t half_bins = signal_len / 2u + 1u;
    std::size_t out_floats = 0u;
    if (!detail::checked_mul(half_bins, 2u, out_floats)) {
        detail::fail(Error::InvalidInput);
    }
    detail::require_all_finite(signal, signal_len);
    std::vector<float> interleaved(two_n, 0.0f);
    for (std::size_t i = 0u; i < signal_len; ++i) {
        interleaved[2u * i] = signal[i];
        interleaved[2u * i + 1u] = 0.0f;
    }
    std::vector<float> full = detail::run_complex_auto(interleaved.data(), signal_len,
                                                       Direction::Forward);
    return std::vector<float>(full.begin(),
                              full.begin() + static_cast<std::ptrdiff_t>(out_floats));
}

inline std::vector<float> rfft_scalar(const std::vector<float>& signal) {
    return rfft_scalar(signal.empty() ? nullptr : signal.data(), signal.size());
}

inline std::vector<float> irfft_scalar(const float* half_spectrum, std::size_t float_len,
                                       std::uint32_t output_length) {
    std::size_t n = static_cast<std::size_t>(output_length);
    if (half_spectrum == nullptr || n == 0u || n > MAX_FFT_LEN ||
        float_len > MAX_SAMPLES || (float_len % 2u) != 0u) {
        detail::fail(Error::InvalidInput);
    }
    std::size_t half_bins = float_len / 2u;
    std::size_t expected_half_bins = n / 2u + 1u;
    if (half_bins != expected_half_bins) {
        detail::fail(Error::InvalidInput);
    }
    std::size_t two_n = 0u;
    if (!detail::checked_mul(n, 2u, two_n) || two_n > MAX_SAMPLES) {
        detail::fail(Error::InvalidInput);
    }
    if (!is_power_of_two(n)) {
        (void)detail::require_bluestein_bounds(n);
    }
    detail::require_all_finite(half_spectrum, float_len);

    std::vector<float> full(two_n, 0.0f);
    for (std::size_t i = 0u; i < float_len; ++i) {
        full[i] = half_spectrum[i];
    }
    std::size_t reflect_end = (n + 1u) / 2u;
    for (std::size_t k = 1u; k < reflect_end; ++k) {
        std::size_t dst = n - k;
        full[2u * dst] = full[2u * k];
        full[2u * dst + 1u] = -full[2u * k + 1u];
    }
    std::vector<float> inverted = detail::run_complex_auto(full.data(), n, Direction::Inverse);
    std::vector<float> out(n);
    for (std::size_t k = 0u; k < n; ++k) {
        out[k] = inverted[2u * k];
    }
    detail::require_all_finite(out);
    return out;
}

inline std::vector<float> irfft_scalar(const std::vector<float>& half_spectrum,
                                       std::uint32_t output_length) {
    return irfft_scalar(half_spectrum.empty() ? nullptr : half_spectrum.data(),
                        half_spectrum.size(), output_length);
}

inline ca::dsp_complex::ComplexTensor fft(const float* signal, std::size_t len,
                                          bool is_complex = false) {
    if (signal == nullptr || len == 0u) {
        detail::fail(Error::InvalidInput);
    }
    if (is_complex) {
        std::size_t n = detail::validate_interleaved_bounds(signal, len, false);
        if (!is_power_of_two(n)) {
            (void)detail::require_bluestein_bounds(n);
        }
        detail::require_all_finite(signal, len);
        std::vector<float> out = detail::run_complex_auto(signal, n, Direction::Forward);
        return detail::wrap_interleaved(std::move(out));
    }
    if (len > MAX_FFT_LEN || len > MAX_SAMPLES) {
        detail::fail(Error::InvalidInput);
    }
    std::size_t two_n = 0u;
    if (!detail::checked_mul(len, 2u, two_n) || two_n > MAX_SAMPLES) {
        detail::fail(Error::InvalidInput);
    }
    if (!is_power_of_two(len)) {
        (void)detail::require_bluestein_bounds(len);
    }
    detail::require_all_finite(signal, len);
    std::vector<float> interleaved(two_n, 0.0f);
    for (std::size_t i = 0u; i < len; ++i) {
        interleaved[2u * i] = signal[i];
        interleaved[2u * i + 1u] = 0.0f;
    }
    std::vector<float> out = detail::run_complex_auto(interleaved.data(), len,
                                                      Direction::Forward);
    return detail::wrap_interleaved(std::move(out));
}

inline ca::dsp_complex::ComplexTensor fft(const std::vector<float>& signal,
                                          bool is_complex = false) {
    return fft(signal.empty() ? nullptr : signal.data(), signal.size(), is_complex);
}

inline ca::dsp_complex::ComplexTensor fft(const ca::dsp_complex::ComplexTensor& signal) {
    const auto& data = signal.as_interleaved();
    return fft(data.empty() ? nullptr : data.data(), data.size(), true);
}

inline ca::dsp_complex::ComplexTensor ifft(const ca::dsp_complex::ComplexTensor& spectrum) {
    std::size_t n = spectrum.len();
    const auto& data = spectrum.as_interleaved();
    std::size_t two_n = 0u;
    if (n == 0u || n > MAX_FFT_LEN || !detail::checked_mul(n, 2u, two_n) ||
        two_n > MAX_SAMPLES || data.size() != two_n) {
        detail::fail(Error::InvalidInput);
    }
    if (!is_power_of_two(n)) {
        (void)detail::require_bluestein_bounds(n);
    }
    detail::require_all_finite(data);
    std::vector<float> out = detail::run_complex_auto(data.data(), n, Direction::Inverse);
    return detail::wrap_interleaved(std::move(out));
}

inline ca::dsp_complex::ComplexTensor rfft(const float* signal, std::size_t len) {
    return detail::wrap_interleaved(rfft_scalar(signal, len));
}

inline ca::dsp_complex::ComplexTensor rfft(const std::vector<float>& signal) {
    return rfft(signal.empty() ? nullptr : signal.data(), signal.size());
}

inline std::vector<float> irfft(const ca::dsp_complex::ComplexTensor& half_spectrum,
                                std::uint32_t output_length) {
    return irfft_scalar(half_spectrum.as_interleaved(), output_length);
}

} // namespace dsp_fft
} // namespace ca

#endif // CA_DSP_FFT_HPP
