// dsp_complex.hpp — Interleaved Complex Tensor & Scalar Operations (pure ISO C++17).
// ==============================================================================
//
// A faithful C++17 port of the `dsp-complex` crate (DSP00 / DSP01) in
// `namespace ca::dsp_complex`. Stores `N` complex `float` elements as an
// interleaved `std::vector<float>` of length `2 * N`:
//
//     [re_0, im_0, re_1, im_1, ..., re_{N-1}, im_{N-1}]
//
// Hardened against VER15 CWEs:
// - CWE-20:  Finite-number validation (rejects NaN, +Inf, -Inf), odd interleaved
//            length rejection, length-mismatch rejection, division-by-zero guard.
// - CWE-120/125/787: Bounds-checked element access (`at`, `set`).
// - CWE-190: Overflow-checked `std::size_t` arithmetic (`checked_add`, `checked_mul`).
// - CWE-400: Pre-flight `MAX_SAMPLES` (2^24) cap before allocation.
// - CWE-476: Pointer-length overloads validate `nullptr` when count > 0.
// - CWE-674: Strictly iterative, zero recursion.
#ifndef CA_DSP_COMPLEX_HPP
#define CA_DSP_COMPLEX_HPP

#include <array>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <exception>
#include <limits>
#include <utility>
#include <vector>

namespace ca {
namespace dsp_complex {

inline constexpr std::size_t MAX_SAMPLES = (std::size_t{1} << 24);

enum class Error {
    InvalidInput,
    LengthMismatch,
    OddInterleavedLength,
    OutOfRange,
    DivisionByZero
};

class ComplexError : public std::exception {
public:
    explicit ComplexError(Error code) noexcept : code_(code) {}
    Error code() const noexcept { return code_; }
    const char* what() const noexcept override {
        switch (code_) {
        case Error::InvalidInput:
            return "dsp_complex: invalid input";
        case Error::LengthMismatch:
            return "dsp_complex: length mismatch";
        case Error::OddInterleavedLength:
            return "dsp_complex: odd interleaved length";
        case Error::OutOfRange:
            return "dsp_complex: index out of range";
        case Error::DivisionByZero:
            return "dsp_complex: division by zero";
        }
        return "dsp_complex: unknown error";
    }

private:
    Error code_;
};

namespace detail {

[[noreturn]] inline void fail(Error e) { throw ComplexError(e); }

inline bool is_finite(float x) noexcept {
    return (x == x) && ((x - x) == 0.0f);
}

inline bool is_finite_double(double x) noexcept {
    return (x == x) && ((x - x) == 0.0);
}

inline bool checked_add(std::size_t a, std::size_t b, std::size_t& out) noexcept {
    if (std::numeric_limits<std::size_t>::max() - a < b) {
        return false;
    }
    out = a + b;
    return true;
}

inline bool checked_mul(std::size_t a, std::size_t b, std::size_t& out) noexcept {
    if (a != 0 && b > std::numeric_limits<std::size_t>::max() / a) {
        return false;
    }
    out = a * b;
    return true;
}

} // namespace detail

struct Complex {
    float re = 0.0f;
    float im = 0.0f;

    constexpr Complex() noexcept = default;
    constexpr Complex(float r, float i) noexcept : re(r), im(i) {}

    constexpr Complex conj() const noexcept { return Complex(re, -im); }

    float abs() const noexcept {
        if (!detail::is_finite(re) || !detail::is_finite(im)) {
            return 0.0f;
        }
        double m = std::hypot(static_cast<double>(re), static_cast<double>(im));
        constexpr double fmax = static_cast<double>(std::numeric_limits<float>::max());
        if (!detail::is_finite_double(m)) {
            return 0.0f;
        }
        if (m > fmax) {
            return std::numeric_limits<float>::max();
        }
        return static_cast<float>(m);
    }

    float arg() const noexcept {
        if (!detail::is_finite(re) || !detail::is_finite(im)) {
            return 0.0f;
        }
        return static_cast<float>(std::atan2(static_cast<double>(im), static_cast<double>(re)));
    }

    constexpr Complex operator+(const Complex& o) const noexcept {
        return Complex(re + o.re, im + o.im);
    }
    constexpr Complex operator-(const Complex& o) const noexcept {
        return Complex(re - o.re, im - o.im);
    }
    constexpr Complex operator*(const Complex& o) const noexcept {
        return Complex(re * o.re - im * o.im, re * o.im + im * o.re);
    }
    Complex operator/(const Complex& o) const {
        if (!detail::is_finite(re) || !detail::is_finite(im) ||
            !detail::is_finite(o.re) || !detail::is_finite(o.im)) {
            detail::fail(Error::InvalidInput);
        }
        double br = static_cast<double>(o.re);
        double bi = static_cast<double>(o.im);
        double denom = br * br + bi * bi;
        if (denom == 0.0 || !detail::is_finite_double(denom)) {
            detail::fail(Error::DivisionByZero);
        }
        double ar = static_cast<double>(re);
        double ai = static_cast<double>(im);
        double re_d = (ar * br + ai * bi) / denom;
        double im_d = (ai * br - ar * bi) / denom;
        constexpr double fmax = static_cast<double>(std::numeric_limits<float>::max());
        if (!detail::is_finite_double(re_d) || !detail::is_finite_double(im_d) ||
            std::abs(re_d) > fmax || std::abs(im_d) > fmax) {
            detail::fail(Error::InvalidInput);
        }
        return Complex(static_cast<float>(re_d), static_cast<float>(im_d));
    }

    constexpr bool operator==(const Complex& o) const noexcept {
        return re == o.re && im == o.im;
    }
    constexpr bool operator!=(const Complex& o) const noexcept {
        return !(*this == o);
    }
};

class ComplexTensor {
public:
    ComplexTensor() = default;

    static ComplexTensor zeros(std::size_t n) {
        if (n > MAX_SAMPLES) {
            detail::fail(Error::InvalidInput);
        }
        std::size_t floats = 0;
        if (!detail::checked_mul(n, 2u, floats)) {
            detail::fail(Error::InvalidInput);
        }
        return ComplexTensor(std::vector<float>(floats, 0.0f));
    }

    static ComplexTensor from_real(const float* real, std::size_t n) {
        if (n > MAX_SAMPLES) {
            detail::fail(Error::InvalidInput);
        }
        if (n == 0) {
            return ComplexTensor();
        }
        if (real == nullptr) {
            detail::fail(Error::InvalidInput);
        }
        for (std::size_t i = 0; i < n; ++i) {
            if (!detail::is_finite(real[i])) {
                detail::fail(Error::InvalidInput);
            }
        }
        std::size_t floats = 0;
        if (!detail::checked_mul(n, 2u, floats)) {
            detail::fail(Error::InvalidInput);
        }
        std::vector<float> buf(floats, 0.0f);
        for (std::size_t i = 0; i < n; ++i) {
            buf[2u * i] = real[i];
            buf[2u * i + 1u] = 0.0f;
        }
        return ComplexTensor(std::move(buf));
    }

    static ComplexTensor from_real(const std::vector<float>& real) {
        return from_real(real.empty() ? nullptr : real.data(), real.size());
    }

    static ComplexTensor from_real_imag(const float* real, std::size_t real_len,
                                        const float* imag, std::size_t imag_len) {
        if (real_len != imag_len) {
            detail::fail(Error::LengthMismatch);
        }
        if (real_len > MAX_SAMPLES) {
            detail::fail(Error::InvalidInput);
        }
        if (real_len == 0) {
            return ComplexTensor();
        }
        if (real == nullptr || imag == nullptr) {
            detail::fail(Error::InvalidInput);
        }
        for (std::size_t i = 0; i < real_len; ++i) {
            if (!detail::is_finite(real[i]) || !detail::is_finite(imag[i])) {
                detail::fail(Error::InvalidInput);
            }
        }
        std::size_t floats = 0;
        if (!detail::checked_mul(real_len, 2u, floats)) {
            detail::fail(Error::InvalidInput);
        }
        std::vector<float> buf(floats);
        for (std::size_t i = 0; i < real_len; ++i) {
            buf[2u * i] = real[i];
            buf[2u * i + 1u] = imag[i];
        }
        return ComplexTensor(std::move(buf));
    }

    static ComplexTensor from_real_imag(const std::vector<float>& real,
                                        const std::vector<float>& imag) {
        return from_real_imag(real.empty() ? nullptr : real.data(), real.size(),
                              imag.empty() ? nullptr : imag.data(), imag.size());
    }

    static ComplexTensor from_interleaved(const float* data, std::size_t float_len) {
        if ((float_len % 2u) != 0u) {
            detail::fail(Error::OddInterleavedLength);
        }
        std::size_t n = float_len / 2u;
        if (n > MAX_SAMPLES) {
            detail::fail(Error::InvalidInput);
        }
        if (n == 0) {
            return ComplexTensor();
        }
        if (data == nullptr) {
            detail::fail(Error::InvalidInput);
        }
        for (std::size_t i = 0; i < float_len; ++i) {
            if (!detail::is_finite(data[i])) {
                detail::fail(Error::InvalidInput);
            }
        }
        return ComplexTensor(std::vector<float>(data, data + float_len));
    }

    static ComplexTensor from_interleaved(std::vector<float> data) {
        if ((data.size() % 2u) != 0u) {
            detail::fail(Error::OddInterleavedLength);
        }
        std::size_t n = data.size() / 2u;
        if (n > MAX_SAMPLES) {
            detail::fail(Error::InvalidInput);
        }
        for (float x : data) {
            if (!detail::is_finite(x)) {
                detail::fail(Error::InvalidInput);
            }
        }
        return ComplexTensor(std::move(data));
    }

    std::size_t len() const noexcept { return interleaved_.size() / 2u; }
    bool is_empty() const noexcept { return interleaved_.empty(); }
    std::array<std::size_t, 2> shape() const noexcept { return {len(), 2u}; }

    const std::vector<float>& as_interleaved() const noexcept { return interleaved_; }
    std::vector<float> into_interleaved() && noexcept { return std::move(interleaved_); }

    Complex at(std::size_t idx) const {
        if (idx >= len()) {
            detail::fail(Error::OutOfRange);
        }
        return Complex(interleaved_[2u * idx], interleaved_[2u * idx + 1u]);
    }

    void set(std::size_t idx, Complex val) {
        if (idx >= len()) {
            detail::fail(Error::OutOfRange);
        }
        if (!detail::is_finite(val.re) || !detail::is_finite(val.im)) {
            detail::fail(Error::InvalidInput);
        }
        interleaved_[2u * idx] = val.re;
        interleaved_[2u * idx + 1u] = val.im;
    }

    std::vector<float> real() const {
        std::size_t n = len();
        std::vector<float> out(n);
        for (std::size_t i = 0; i < n; ++i) {
            out[i] = interleaved_[2u * i];
        }
        return out;
    }

    std::vector<float> imag() const {
        std::size_t n = len();
        std::vector<float> out(n);
        for (std::size_t i = 0; i < n; ++i) {
            out[i] = interleaved_[2u * i + 1u];
        }
        return out;
    }

    std::vector<float> magnitude() const {
        std::size_t n = len();
        std::vector<float> out(n);
        constexpr double fmax = static_cast<double>(std::numeric_limits<float>::max());
        for (std::size_t i = 0; i < n; ++i) {
            double m = std::hypot(static_cast<double>(interleaved_[2u * i]),
                                  static_cast<double>(interleaved_[2u * i + 1u]));
            if (!detail::is_finite_double(m) || m > fmax) {
                detail::fail(Error::InvalidInput);
            }
            out[i] = static_cast<float>(m);
        }
        return out;
    }

    std::vector<float> phase() const {
        std::size_t n = len();
        std::vector<float> out(n);
        for (std::size_t i = 0; i < n; ++i) {
            out[i] = static_cast<float>(std::atan2(
                static_cast<double>(interleaved_[2u * i + 1u]),
                static_cast<double>(interleaved_[2u * i])));
        }
        return out;
    }

    ComplexTensor conjugate() const {
        std::vector<float> out = interleaved_;
        for (std::size_t i = 1u; i < out.size(); i += 2u) {
            out[i] = -out[i];
        }
        return ComplexTensor(std::move(out));
    }

    ComplexTensor scale(float factor) const {
        if (!detail::is_finite(factor)) {
            detail::fail(Error::InvalidInput);
        }
        std::vector<float> out(interleaved_.size());
        for (std::size_t i = 0; i < interleaved_.size(); ++i) {
            float v = interleaved_[i] * factor;
            if (!detail::is_finite(v)) {
                detail::fail(Error::InvalidInput);
            }
            out[i] = v;
        }
        return ComplexTensor(std::move(out));
    }

    ComplexTensor add(const ComplexTensor& other) const {
        if (len() != other.len()) {
            detail::fail(Error::LengthMismatch);
        }
        std::vector<float> out(interleaved_.size());
        for (std::size_t i = 0; i < interleaved_.size(); ++i) {
            float v = interleaved_[i] + other.interleaved_[i];
            if (!detail::is_finite(v)) {
                detail::fail(Error::InvalidInput);
            }
            out[i] = v;
        }
        return ComplexTensor(std::move(out));
    }

    ComplexTensor sub(const ComplexTensor& other) const {
        if (len() != other.len()) {
            detail::fail(Error::LengthMismatch);
        }
        std::vector<float> out(interleaved_.size());
        for (std::size_t i = 0; i < interleaved_.size(); ++i) {
            float v = interleaved_[i] - other.interleaved_[i];
            if (!detail::is_finite(v)) {
                detail::fail(Error::InvalidInput);
            }
            out[i] = v;
        }
        return ComplexTensor(std::move(out));
    }

    ComplexTensor mul(const ComplexTensor& other) const {
        if (len() != other.len()) {
            detail::fail(Error::LengthMismatch);
        }
        std::size_t n = len();
        std::vector<float> out(2u * n);
        for (std::size_t i = 0; i < n; ++i) {
            float ar = interleaved_[2u * i];
            float ai = interleaved_[2u * i + 1u];
            float br = other.interleaved_[2u * i];
            float bi = other.interleaved_[2u * i + 1u];
            float re = ar * br - ai * bi;
            float im = ar * bi + ai * br;
            if (!detail::is_finite(re) || !detail::is_finite(im)) {
                detail::fail(Error::InvalidInput);
            }
            out[2u * i] = re;
            out[2u * i + 1u] = im;
        }
        return ComplexTensor(std::move(out));
    }

    ComplexTensor div(const ComplexTensor& other) const {
        if (len() != other.len()) {
            detail::fail(Error::LengthMismatch);
        }
        std::size_t n = len();
        std::vector<float> out(2u * n);
        constexpr double fmax = static_cast<double>(std::numeric_limits<float>::max());
        for (std::size_t i = 0; i < n; ++i) {
            double ar = static_cast<double>(interleaved_[2u * i]);
            double ai = static_cast<double>(interleaved_[2u * i + 1u]);
            double br = static_cast<double>(other.interleaved_[2u * i]);
            double bi = static_cast<double>(other.interleaved_[2u * i + 1u]);
            double denom = br * br + bi * bi;
            if (denom == 0.0 || !detail::is_finite_double(denom)) {
                detail::fail(Error::DivisionByZero);
            }
            double re_d = (ar * br + ai * bi) / denom;
            double im_d = (ai * br - ar * bi) / denom;
            if (!detail::is_finite_double(re_d) || !detail::is_finite_double(im_d) ||
                std::abs(re_d) > fmax || std::abs(im_d) > fmax) {
                detail::fail(Error::InvalidInput);
            }
            out[2u * i] = static_cast<float>(re_d);
            out[2u * i + 1u] = static_cast<float>(im_d);
        }
        return ComplexTensor(std::move(out));
    }

    bool approx_equals(const ComplexTensor& other, float eps = 1e-6f) const {
        if (!detail::is_finite(eps) || eps < 0.0f) {
            detail::fail(Error::InvalidInput);
        }
        if (len() != other.len()) {
            return false;
        }
        for (std::size_t i = 0; i < interleaved_.size(); ++i) {
            float d = interleaved_[i] - other.interleaved_[i];
            if (d < 0.0f) d = -d;
            if (d > eps) return false;
        }
        return true;
    }

    bool operator==(const ComplexTensor& o) const noexcept {
        return interleaved_ == o.interleaved_;
    }
    bool operator!=(const ComplexTensor& o) const noexcept {
        return !(*this == o);
    }

private:
    explicit ComplexTensor(std::vector<float> buf) noexcept
        : interleaved_(std::move(buf)) {}

    std::vector<float> interleaved_;
};

using ComplexArray = ComplexTensor;

} // namespace dsp_complex
} // namespace ca

#endif // CA_DSP_COMPLEX_HPP
