// dsp_wavelets.hpp — Discrete & Continuous Wavelet Transforms (header-only, ISO C++17).
// ---------------------------------------------------------------------------
//
// A faithful C++ port of the Rust `dsp-wavelets` crate, in namespace
// `ca::dsp_wavelets`. It implements the Discrete Wavelet Transform (DWT) and
// its inverse via the **Mallat pyramid algorithm**, in one and two
// dimensions, for the orthogonal wavelet families Haar, Daubechies (Db2/4/6/8),
// Symlets (Sym4), and Coiflets (Coif1), as well as the Continuous Wavelet
// Transform (CWT) for Morlet and Mexican Hat.
//
// Hardened with full VER15 CWE/CVE proofs:
// - CWE-20: Finite/non-NaN/non-Inf checks, valid wavelet name & boundary parsing,
//           dimension & parameter bounds.
// - CWE-120/122/125/787: Bounds-checked indexing, safe buffer sizing, no OOB.
// - CWE-190: Overflow-checked arithmetic on dimensions, strides, and buffer sizes.
// - CWE-400: Defensive caps (MAX_SAMPLES, MAX_LEVELS, MAX_SCALES, MAX_2D_DIM).
// - CWE-674/1333: Strictly iterative, zero recursion, zero regex.
#ifndef CA_DSP_WAVELETS_HPP
#define CA_DSP_WAVELETS_HPP

#include <cmath>
#include <cstddef>
#include <cstdint>
#include <exception>
#include <string>
#include <utility>
#include <vector>

namespace ca {
namespace dsp_wavelets {

// The reciprocal of sqrt(2) as an f32 — the Haar filter tap.
inline constexpr float FRAC_1_SQRT_2 = 0.70710678f;

// Defensive caps (mirroring the crate's security-review bounds and warrant).
inline constexpr std::uint32_t MAX_LEVELS = 31;
inline constexpr std::size_t MAX_SAMPLES = 67108864; // 2^26
inline constexpr std::size_t MAX_SCALES = 4096;
inline constexpr std::size_t MAX_2D_DIM = 16777216;  // 2^24

inline constexpr std::uint32_t DSP_MAX_LEVELS = MAX_LEVELS;
inline constexpr std::size_t DSP_MAX_SAMPLES = MAX_SAMPLES;
inline constexpr std::size_t DSP_MAX_SCALES = MAX_SCALES;
inline constexpr std::size_t DSP_MAX_2D_DIM = MAX_2D_DIM;

// ------------------------------------------------------------------ //
// Wavelet family / boundary / band selectors                         //
// ------------------------------------------------------------------ //

enum class Family { Haar, Daubechies, Symlets, Coiflets, Biorthogonal, Morlet, MexicanHat };

struct Wavelet {
    Family family = Family::Haar;
    std::uint32_t n = 0;
    std::uint32_t n2 = 0;

    static Wavelet haar() { return Wavelet{Family::Haar, 0, 0}; }
    static Wavelet daubechies(std::uint32_t n) { return Wavelet{Family::Daubechies, n, 0}; }
    static Wavelet symlets(std::uint32_t n) { return Wavelet{Family::Symlets, n, 0}; }
    static Wavelet coiflets(std::uint32_t n) { return Wavelet{Family::Coiflets, n, 0}; }
    static Wavelet biorthogonal(std::uint32_t vm_decomp, std::uint32_t vm_recon) {
        return Wavelet{Family::Biorthogonal, vm_decomp, vm_recon};
    }
    static Wavelet morlet() { return Wavelet{Family::Morlet, 0, 0}; }
    static Wavelet mexican_hat() { return Wavelet{Family::MexicanHat, 0, 0}; }

    bool operator==(const Wavelet& o) const {
        return family == o.family && n == o.n && n2 == o.n2;
    }
    bool operator!=(const Wavelet& o) const { return !(*this == o); }
};

enum class Boundary { Zero, Replicate, Reflect, Symmetric, Periodic };
enum class Band { Approximation, Detail };

// ------------------------------------------------------------------ //
// Errors                                                             //
// ------------------------------------------------------------------ //

enum class Error {
    EmptySignal,          // signal / coeffs buffer is empty
    InvalidParam,         // levels==0, unsupported wavelet/boundary, over a cap, etc.
    SignalTooShort,       // signal too short to support `levels` passes
    InvalidCoefficients,  // coeff buffer shape doesn't match (signal_len, levels, wavelet)
    Fft                   // reserved for the CWT path
};

class WaveletError : public std::exception {
public:
    explicit WaveletError(Error code) noexcept : code_(code) {}
    Error code() const noexcept { return code_; }
    const char* what() const noexcept override { return "dsp_wavelets error"; }

private:
    Error code_;
};

struct FloatView {
    const float* data = nullptr;
    std::size_t size = 0;
};

// ------------------------------------------------------------------ //
// Implementation details                                             //
// ------------------------------------------------------------------ //

namespace detail {

[[noreturn]] inline void fail(Error e) { throw WaveletError(e); }

// Checked arithmetic helpers (CWE-190)
inline bool checked_add(std::size_t a, std::size_t b, std::size_t& out) {
    if (SIZE_MAX - a < b) return false;
    out = a + b;
    return true;
}

inline bool checked_mul(std::size_t a, std::size_t b, std::size_t& out) {
    if (a != 0 && b > SIZE_MAX / a) return false;
    out = a * b;
    return true;
}

// Floating-point finiteness validation (CWE-20)
inline bool is_finite(float x) {
    return (x == x) && ((x - x) == 0.0f);
}

inline std::vector<float> analysis_lowpass(const Wavelet& w) {
    static const float DB2[4] = {-0.12940952f, 0.22414386f, 0.8365163f, 0.4829629f};
    static const float DB4[8] = {-0.010597402f, 0.03288301f, 0.030841382f, -0.18703482f,
                                 -0.02798377f, 0.6308808f, 0.71484655f, 0.23037781f};
    static const float DB6[12] = {0.11154074f, 0.4946239f, 0.7511339f, 0.31525034f,
                                  -0.2262647f, -0.12976687f, 0.097501606f, 0.027522866f,
                                  -0.03158204f, 0.00055384222f, 0.0047772573f, -0.0010773011f};
    static const float DB8[16] = {0.05441584f, 0.3128716f, 0.67563075f, 0.5853547f,
                                  -0.015829105f, -0.28401554f, 0.00047248456f, 0.12874743f,
                                  -0.0173693f, -0.044088256f, 0.0139810275f, 0.008746094f,
                                  -0.004870353f, -0.00039174038f, 0.00067544994f, -0.00011747678f};
    static const float SYM4[8] = {-0.075765714f, -0.029635528f, 0.49761868f, 0.8037388f,
                                  0.2978578f, -0.099219546f, -0.012603967f, 0.0322231f};
    static const float COIF1[6] = {-0.015655728f, -0.07273262f, 0.38486484f, 0.852572f,
                                   0.33789766f, -0.07273262f};

    switch (w.family) {
    case Family::Daubechies:
        if (w.n == 2) return std::vector<float>(DB2, DB2 + 4);
        if (w.n == 4) return std::vector<float>(DB4, DB4 + 8);
        if (w.n == 6) return std::vector<float>(DB6, DB6 + 12);
        if (w.n == 8) return std::vector<float>(DB8, DB8 + 16);
        return {};
    case Family::Symlets:
        if (w.n == 4) return std::vector<float>(SYM4, SYM4 + 8);
        return {};
    case Family::Coiflets:
        if (w.n == 1) return std::vector<float>(COIF1, COIF1 + 6);
        return {};
    default:
        return {};
    }
}

inline std::vector<float> qmf_highpass(const std::vector<float>& h) {
    std::size_t l = h.size();
    if (l == 0) fail(Error::InvalidParam);
    std::vector<float> g(l);
    for (std::size_t i = 0; i < l; ++i) {
        float reversed = h[l - 1 - i];
        g[i] = (i & 1) == 0 ? reversed : -reversed;
    }
    return g;
}

inline void analysis_filters(const Wavelet& w, std::vector<float>& h, std::vector<float>& g) {
    if (w.family == Family::Haar) {
        h = {FRAC_1_SQRT_2, FRAC_1_SQRT_2};
        g = {FRAC_1_SQRT_2, -FRAC_1_SQRT_2};
        return;
    }
    if (w.family == Family::Daubechies || w.family == Family::Symlets ||
        w.family == Family::Coiflets) {
        h = analysis_lowpass(w);
        if (h.empty()) {
            fail(Error::InvalidParam);
        }
        g = qmf_highpass(h);
        return;
    }
    fail(Error::InvalidParam);
}

inline std::size_t filter_length_for(const Wavelet& w) {
    if (w.family == Family::Haar) return 2;
    return analysis_lowpass(w).size();
}

inline void check_supported_wavelet(const Wavelet& w) {
    if (w.family == Family::Haar) return;
    if (w.family == Family::Daubechies || w.family == Family::Symlets ||
        w.family == Family::Coiflets) {
        if (analysis_lowpass(w).empty()) fail(Error::InvalidParam);
        return;
    }
    fail(Error::InvalidParam);
}

inline void check_supported_boundary(Boundary b) {
    if (b == Boundary::Symmetric || b == Boundary::Periodic) return;
    fail(Error::InvalidParam);
}

inline void check_levels(std::uint32_t levels) {
    if (levels == 0 || levels > MAX_LEVELS) fail(Error::InvalidParam);
}

inline float sample_with_boundary(const float* signal, std::size_t signal_len,
                                  std::int64_t idx, Boundary boundary) {
    std::int64_t n = static_cast<std::int64_t>(signal_len);
    if (n == 0) return 0.0f;
    if (idx >= 0 && idx < n) return signal[static_cast<std::size_t>(idx)];
    if (boundary == Boundary::Periodic) {
        std::int64_t m = ((idx % n) + n) % n;
        if (m < 0 || m >= n) return 0.0f;
        return signal[static_cast<std::size_t>(m)];
    }
    if (boundary == Boundary::Symmetric) {
        std::int64_t period = 2 * n;
        std::int64_t m = ((idx % period) + period) % period;
        if (m >= n) m = 2 * n - 1 - m;
        if (m < 0 || m >= n) return 0.0f;
        return signal[static_cast<std::size_t>(m)];
    }
    return 0.0f;
}

inline void filter_and_downsample(const std::vector<float>& signal, const std::vector<float>& h,
                                  const std::vector<float>& g, Boundary boundary,
                                  std::vector<float>& ca, std::vector<float>& cd) {
    std::size_t n = signal.size();
    std::size_t filter_len = h.size();
    std::size_t out_len = (n + 1) / 2;
    ca.assign(out_len, 0.0f);
    cd.assign(out_len, 0.0f);
    for (std::size_t out_idx = 0; out_idx < out_len; ++out_idx) {
        std::int64_t k = static_cast<std::int64_t>(2 * out_idx + 1);
        float acc_h = 0.0f;
        float acc_g = 0.0f;
        for (std::size_t i = 0; i < filter_len; ++i) {
            std::int64_t src = k - static_cast<std::int64_t>(i);
            float sample = sample_with_boundary(signal.data(), n, src, boundary);
            acc_h += h[i] * sample;
            acc_g += g[i] * sample;
        }
        ca[out_idx] = acc_h;
        cd[out_idx] = acc_g;
    }
}

inline std::vector<float> synthesize_one_level(const std::vector<float>& ca,
                                               const std::vector<float>& cd,
                                               const std::vector<float>& h,
                                               const std::vector<float>& g,
                                               std::size_t target_len, Boundary boundary) {
    std::size_t filter_len = h.size();
    std::vector<float> out(target_len, 0.0f);
    for (std::size_t nn = 0; nn < target_len; ++nn) {
        float acc = 0.0f;
        for (std::size_t i = 0; i < filter_len; ++i) {
            std::int64_t numerator =
                static_cast<std::int64_t>(nn) + static_cast<std::int64_t>(i) - 1;
            if ((numerator & 1) == 0) {
                std::int64_t m = numerator / 2;
                float ca_val = sample_with_boundary(ca.data(), ca.size(), m, boundary);
                float cd_val = sample_with_boundary(cd.data(), cd.size(), m, boundary);
                acc += h[i] * ca_val + g[i] * cd_val;
            }
        }
        out[nn] = acc;
    }
    return out;
}

inline std::vector<std::size_t> forward_level_lengths(std::size_t signal_len,
                                                      std::uint32_t levels) {
    std::vector<std::size_t> lens;
    lens.reserve(static_cast<std::size_t>(levels) + 1);
    lens.push_back(signal_len);
    std::size_t cur = signal_len;
    for (std::uint32_t i = 0; i < levels; ++i) {
        cur = (cur + 1) / 2;
        lens.push_back(cur);
    }
    return lens;
}

inline void validate_dwt_inputs(const std::vector<float>& signal, const Wavelet& wavelet,
                                std::uint32_t levels, Boundary boundary) {
    if (signal.empty()) fail(Error::EmptySignal);
    check_levels(levels);
    check_supported_wavelet(wavelet);
    check_supported_boundary(boundary);
    if (signal.size() > MAX_SAMPLES) fail(Error::InvalidParam);
    for (float x : signal) {
        if (!is_finite(x)) fail(Error::InvalidParam);
    }
    std::size_t filter_len = filter_length_for(wavelet);
    std::uint32_t shift = levels - 1;
    if (shift > 31) shift = 31;
    std::size_t pow2 = static_cast<std::size_t>(1) << shift;
    std::size_t min_signal_len = filter_len > pow2 ? filter_len : pow2;
    if (signal.size() < min_signal_len) fail(Error::SignalTooShort);
}

} // namespace detail

// ------------------------------------------------------------------ //
// Parsing helpers (CWE-20, reject trailing garbage, zero regex)      //
// ------------------------------------------------------------------ //

inline Boundary parse_boundary(const std::string& str) {
    std::size_t start = 0;
    while (start < str.size() && (str[start] == ' ' || str[start] == '\t' ||
                                  str[start] == '\r' || str[start] == '\n')) {
        start++;
    }
    std::size_t end = str.size();
    while (end > start && (str[end - 1] == ' ' || str[end - 1] == '\t' ||
                           str[end - 1] == '\r' || str[end - 1] == '\n')) {
        end--;
    }
    if (end <= start) {
        detail::fail(Error::InvalidParam);
    }
    std::string s;
    s.reserve(end - start);
    for (std::size_t i = start; i < end; ++i) {
        char c = str[i];
        if (c >= 'A' && c <= 'Z') c = static_cast<char>(c + ('a' - 'A'));
        s.push_back(c);
    }

    if (s == "zero") return Boundary::Zero;
    if (s == "replicate") return Boundary::Replicate;
    if (s == "reflect") return Boundary::Reflect;
    if (s == "symmetric" || s == "sym") return Boundary::Symmetric;
    if (s == "periodic" || s == "wrap" || s == "circular") return Boundary::Periodic;
    detail::fail(Error::InvalidParam);
}

inline Wavelet parse_wavelet(const std::string& str) {
    std::size_t start = 0;
    while (start < str.size() && (str[start] == ' ' || str[start] == '\t' ||
                                  str[start] == '\r' || str[start] == '\n')) {
        start++;
    }
    std::size_t end = str.size();
    while (end > start && (str[end - 1] == ' ' || str[end - 1] == '\t' ||
                           str[end - 1] == '\r' || str[end - 1] == '\n')) {
        end--;
    }
    if (end <= start) {
        detail::fail(Error::InvalidParam);
    }
    std::string s;
    s.reserve(end - start);
    for (std::size_t i = start; i < end; ++i) {
        char c = str[i];
        if (c >= 'A' && c <= 'Z') c = static_cast<char>(c + ('a' - 'A'));
        s.push_back(c);
    }

    if (s == "haar") return Wavelet::haar();
    if (s.rfind("db", 0) == 0) {
        if (s.size() <= 2) detail::fail(Error::InvalidParam);
        std::uint64_t val = 0;
        for (std::size_t i = 2; i < s.size(); ++i) {
            if (s[i] < '0' || s[i] > '9') detail::fail(Error::InvalidParam);
            val = val * 10 + static_cast<std::uint64_t>(s[i] - '0');
            if (val > 0xFFFFFFFFu) detail::fail(Error::InvalidParam);
        }
        return Wavelet::daubechies(static_cast<std::uint32_t>(val));
    }
    if (s.rfind("sym", 0) == 0) {
        if (s.size() <= 3) detail::fail(Error::InvalidParam);
        std::uint64_t val = 0;
        for (std::size_t i = 3; i < s.size(); ++i) {
            if (s[i] < '0' || s[i] > '9') detail::fail(Error::InvalidParam);
            val = val * 10 + static_cast<std::uint64_t>(s[i] - '0');
            if (val > 0xFFFFFFFFu) detail::fail(Error::InvalidParam);
        }
        return Wavelet::symlets(static_cast<std::uint32_t>(val));
    }
    if (s.rfind("coif", 0) == 0) {
        if (s.size() <= 4) detail::fail(Error::InvalidParam);
        std::uint64_t val = 0;
        for (std::size_t i = 4; i < s.size(); ++i) {
            if (s[i] < '0' || s[i] > '9') detail::fail(Error::InvalidParam);
            val = val * 10 + static_cast<std::uint64_t>(s[i] - '0');
            if (val > 0xFFFFFFFFu) detail::fail(Error::InvalidParam);
        }
        return Wavelet::coiflets(static_cast<std::uint32_t>(val));
    }
    if (s == "bior2.2" || s == "bior22" || s == "bior5.3" || s == "bior53") {
        return Wavelet::biorthogonal(2, 2);
    }
    if (s == "morlet") return Wavelet::morlet();
    if (s == "mexican_hat" || s == "mexicanhat" || s == "ricker") {
        return Wavelet::mexican_hat();
    }
    detail::fail(Error::InvalidParam);
}

// ------------------------------------------------------------------ //
// Public 1-D API                                                     //
// ------------------------------------------------------------------ //

inline std::vector<float> dwt_1d(const std::vector<float>& signal, const Wavelet& wavelet,
                                 std::uint32_t levels, Boundary boundary) {
    detail::validate_dwt_inputs(signal, wavelet, levels, boundary);
    std::vector<float> h, g;
    detail::analysis_filters(wavelet, h, g);

    std::vector<float> current = signal;
    std::vector<std::vector<float>> details_reversed;
    details_reversed.reserve(levels);
    for (std::uint32_t l = 0; l < levels; ++l) {
        std::vector<float> ca, cd;
        detail::filter_and_downsample(current, h, g, boundary, ca, cd);
        details_reversed.push_back(std::move(cd));
        current = std::move(ca);
    }

    std::vector<float> out;
    std::size_t total = current.size();
    for (const auto& d : details_reversed) {
        if (!detail::checked_add(total, d.size(), total)) {
            detail::fail(Error::InvalidParam);
        }
    }
    out.reserve(total);
    out.insert(out.end(), current.begin(), current.end());
    for (auto it = details_reversed.rbegin(); it != details_reversed.rend(); ++it) {
        out.insert(out.end(), it->begin(), it->end());
    }
    return out;
}

inline std::vector<float> idwt_1d(const std::vector<float>& coeffs, const Wavelet& wavelet,
                                  std::uint32_t levels, Boundary boundary,
                                  std::uint32_t output_length) {
    if (coeffs.empty()) detail::fail(Error::EmptySignal);
    if (output_length == 0 || output_length > MAX_2D_DIM) detail::fail(Error::InvalidParam);
    detail::check_levels(levels);
    if (coeffs.size() > MAX_SAMPLES) detail::fail(Error::InvalidParam);
    for (float x : coeffs) {
        if (!detail::is_finite(x)) detail::fail(Error::InvalidCoefficients);
    }
    detail::check_supported_wavelet(wavelet);
    detail::check_supported_boundary(boundary);

    std::vector<std::size_t> level_lens =
        detail::forward_level_lengths(output_length, levels);
    std::size_t coarsest_ca_len = level_lens[levels];
    std::size_t expected_total = coarsest_ca_len;
    for (std::uint32_t j = 1; j <= levels; ++j) {
        if (!detail::checked_add(expected_total, level_lens[j], expected_total)) {
            detail::fail(Error::InvalidParam);
        }
    }
    if (coeffs.size() != expected_total) detail::fail(Error::InvalidCoefficients);

    std::size_t offset = 0;
    std::vector<float> current(coeffs.begin() + static_cast<std::ptrdiff_t>(offset),
                               coeffs.begin() +
                                   static_cast<std::ptrdiff_t>(offset + coarsest_ca_len));
    offset += coarsest_ca_len;

    std::vector<float> h, g;
    detail::analysis_filters(wavelet, h, g);
    for (std::uint32_t j = levels; j >= 1; --j) {
        std::size_t cd_len = level_lens[j];
        std::vector<float> cd(coeffs.begin() + static_cast<std::ptrdiff_t>(offset),
                              coeffs.begin() + static_cast<std::ptrdiff_t>(offset + cd_len));
        offset += cd_len;
        std::size_t target_len = level_lens[j - 1];
        current = detail::synthesize_one_level(current, cd, h, g, target_len, boundary);
    }
    return current;
}

inline std::vector<std::size_t> split_levels(std::size_t coeffs_len, std::size_t signal_len,
                                             std::uint32_t levels) {
    if (signal_len == 0 || signal_len > MAX_SAMPLES) detail::fail(Error::InvalidParam);
    if (coeffs_len > MAX_SAMPLES) detail::fail(Error::InvalidParam);
    detail::check_levels(levels);
    std::vector<std::size_t> level_lens = detail::forward_level_lengths(signal_len, levels);
    std::size_t coarsest_ca = level_lens[levels];
    std::size_t expected_total = coarsest_ca;
    for (std::uint32_t j = 1; j <= levels; ++j) {
        if (!detail::checked_add(expected_total, level_lens[j], expected_total)) {
            detail::fail(Error::InvalidParam);
        }
    }
    if (coeffs_len != expected_total) detail::fail(Error::InvalidCoefficients);

    std::vector<std::size_t> offsets;
    offsets.reserve(static_cast<std::size_t>(levels) + 2);
    offsets.push_back(0); // cA_J
    std::size_t off = coarsest_ca;
    for (std::uint32_t j = levels; j >= 1; --j) {
        offsets.push_back(off);
        off += level_lens[j];
    }
    offsets.push_back(off); // total_len sentinel
    return offsets;
}

inline FloatView slice_level(const std::vector<float>& coeffs, std::size_t signal_len,
                             std::uint32_t levels, std::uint32_t target_level, Band band) {
    if (target_level == 0 || target_level > levels) detail::fail(Error::InvalidParam);
    if (band == Band::Approximation && target_level != levels) detail::fail(Error::InvalidParam);
    std::vector<std::size_t> offsets = split_levels(coeffs.size(), signal_len, levels);
    std::size_t start, end;
    if (band == Band::Approximation) {
        start = offsets[0];
        end = offsets[1];
    } else {
        std::size_t idx = static_cast<std::size_t>(levels - target_level + 1);
        start = offsets[idx];
        end = offsets[idx + 1];
    }
    FloatView v;
    v.data = coeffs.data() + start;
    v.size = end - start;
    return v;
}

// ------------------------------------------------------------------ //
// Public 2-D API                                                     //
// ------------------------------------------------------------------ //

namespace detail {

inline void check_2d_supported(const Wavelet& wavelet, Boundary boundary) {
    if (boundary != Boundary::Periodic) fail(Error::InvalidParam);
    check_supported_wavelet(wavelet);
}

inline void validate_2d_inputs(const std::vector<float>& image, std::uint32_t n_rows,
                               std::uint32_t n_cols, std::uint32_t levels) {
    if (image.empty()) fail(Error::EmptySignal);
    if (n_rows == 0 || n_cols == 0) fail(Error::InvalidParam);
    if (static_cast<std::size_t>(n_rows) > MAX_2D_DIM ||
        static_cast<std::size_t>(n_cols) > MAX_2D_DIM) {
        fail(Error::InvalidParam);
    }
    std::size_t expected_len = 0;
    if (!checked_mul(static_cast<std::size_t>(n_rows), static_cast<std::size_t>(n_cols), expected_len) ||
        image.size() != expected_len || image.size() > MAX_SAMPLES) {
        fail(Error::InvalidParam);
    }
    if (levels == 0 || levels > MAX_LEVELS) fail(Error::InvalidParam);
    for (float x : image) {
        if (!is_finite(x)) fail(Error::InvalidParam);
    }
}

inline std::vector<std::pair<std::size_t, std::size_t>> forward_level_dims(
    std::size_t n_rows, std::size_t n_cols, std::uint32_t levels) {
    std::vector<std::pair<std::size_t, std::size_t>> dims;
    dims.reserve(static_cast<std::size_t>(levels) + 1);
    dims.emplace_back(n_rows, n_cols);
    std::size_t r = n_rows, c = n_cols;
    for (std::uint32_t i = 0; i < levels; ++i) {
        r = (r + 1) / 2;
        c = (c + 1) / 2;
        dims.emplace_back(r, c);
    }
    return dims;
}

inline void dwt_2d_one_level(const std::vector<float>& image, std::size_t n_rows,
                             std::size_t n_cols, const Wavelet& wavelet, Boundary boundary,
                             std::vector<float>& ll, std::vector<float>& hl,
                             std::vector<float>& lh, std::vector<float>& hh) {
    std::size_t half_cols = (n_cols + 1) / 2;
    std::size_t half_rows = (n_rows + 1) / 2;
    std::size_t r_bytes = 0;
    std::size_t band_bytes = 0;
    if (!checked_mul(n_rows, half_cols, r_bytes) ||
        !checked_mul(half_rows, half_cols, band_bytes)) {
        fail(Error::InvalidParam);
    }

    std::vector<float> l_rows;
    std::vector<float> h_rows;
    l_rows.reserve(r_bytes);
    h_rows.reserve(r_bytes);
    for (std::size_t r = 0; r < n_rows; ++r) {
        std::vector<float> row(image.begin() + static_cast<std::ptrdiff_t>(r * n_cols),
                               image.begin() + static_cast<std::ptrdiff_t>((r + 1) * n_cols));
        std::vector<float> row_coeffs = dwt_1d(row, wavelet, 1, boundary);
        l_rows.insert(l_rows.end(), row_coeffs.begin(),
                      row_coeffs.begin() + static_cast<std::ptrdiff_t>(half_cols));
        h_rows.insert(h_rows.end(),
                      row_coeffs.begin() + static_cast<std::ptrdiff_t>(half_cols),
                      row_coeffs.begin() + static_cast<std::ptrdiff_t>(2 * half_cols));
    }

    ll.assign(band_bytes, 0.0f);
    hl.assign(band_bytes, 0.0f);
    lh.assign(band_bytes, 0.0f);
    hh.assign(band_bytes, 0.0f);
    std::vector<float> col_buf(n_rows, 0.0f);

    for (std::size_t c = 0; c < half_cols; ++c) {
        for (std::size_t r = 0; r < n_rows; ++r) col_buf[r] = l_rows[r * half_cols + c];
        std::vector<float> col_coeffs = dwt_1d(col_buf, wavelet, 1, boundary);
        for (std::size_t r = 0; r < half_rows; ++r) {
            ll[r * half_cols + c] = col_coeffs[r];
            lh[r * half_cols + c] = col_coeffs[half_rows + r];
        }
        for (std::size_t r = 0; r < n_rows; ++r) col_buf[r] = h_rows[r * half_cols + c];
        col_coeffs = dwt_1d(col_buf, wavelet, 1, boundary);
        for (std::size_t r = 0; r < half_rows; ++r) {
            hl[r * half_cols + c] = col_coeffs[r];
            hh[r * half_cols + c] = col_coeffs[half_rows + r];
        }
    }
}

inline std::vector<float> idwt_2d_one_level(const std::vector<float>& ll,
                                            const std::vector<float>& hl,
                                            const std::vector<float>& lh,
                                            const std::vector<float>& hh, std::size_t band_rows,
                                            std::size_t band_cols, std::size_t target_rows,
                                            std::size_t target_cols, const Wavelet& wavelet,
                                            Boundary boundary) {
    std::size_t r_bytes = 0;
    std::size_t res_len = 0;
    if (!checked_mul(target_rows, band_cols, r_bytes) ||
        !checked_mul(target_rows, target_cols, res_len)) {
        fail(Error::InvalidParam);
    }
    std::vector<float> l_rows(r_bytes, 0.0f);
    std::vector<float> h_rows(r_bytes, 0.0f);
    std::vector<float> col_coeffs(2 * band_rows, 0.0f);

    for (std::size_t c = 0; c < band_cols; ++c) {
        for (std::size_t r = 0; r < band_rows; ++r) {
            col_coeffs[r] = ll[r * band_cols + c];
            col_coeffs[band_rows + r] = lh[r * band_cols + c];
        }
        std::vector<float> col = idwt_1d(col_coeffs, wavelet, 1, boundary,
                                         static_cast<std::uint32_t>(target_rows));
        for (std::size_t r = 0; r < target_rows; ++r) l_rows[r * band_cols + c] = col[r];

        for (std::size_t r = 0; r < band_rows; ++r) {
            col_coeffs[r] = hl[r * band_cols + c];
            col_coeffs[band_rows + r] = hh[r * band_cols + c];
        }
        col = idwt_1d(col_coeffs, wavelet, 1, boundary,
                      static_cast<std::uint32_t>(target_rows));
        for (std::size_t r = 0; r < target_rows; ++r) h_rows[r * band_cols + c] = col[r];
    }

    std::vector<float> out(res_len, 0.0f);
    std::vector<float> row_coeffs(2 * band_cols, 0.0f);
    for (std::size_t r = 0; r < target_rows; ++r) {
        for (std::size_t c = 0; c < band_cols; ++c) {
            row_coeffs[c] = l_rows[r * band_cols + c];
            row_coeffs[band_cols + c] = h_rows[r * band_cols + c];
        }
        std::vector<float> row = idwt_1d(row_coeffs, wavelet, 1, boundary,
                                         static_cast<std::uint32_t>(target_cols));
        for (std::size_t c = 0; c < target_cols; ++c) out[r * target_cols + c] = row[c];
    }
    return out;
}

} // namespace detail

inline std::vector<float> dwt_2d(const std::vector<float>& image, std::uint32_t n_rows,
                                 std::uint32_t n_cols, const Wavelet& wavelet,
                                 std::uint32_t levels, Boundary boundary) {
    detail::validate_2d_inputs(image, n_rows, n_cols, levels);
    detail::check_2d_supported(wavelet, boundary);

    std::vector<float> current = image;
    std::size_t cur_rows = n_rows;
    std::size_t cur_cols = n_cols;
    std::vector<std::vector<std::vector<float>>> detail_levels_reversed;
    detail_levels_reversed.reserve(levels);

    for (std::uint32_t l = 0; l < levels; ++l) {
        std::vector<float> ll, hl, lh, hh;
        detail::dwt_2d_one_level(current, cur_rows, cur_cols, wavelet, boundary, ll, hl, lh, hh);
        detail_levels_reversed.push_back({std::move(hl), std::move(lh), std::move(hh)});
        cur_rows = (cur_rows + 1) / 2;
        cur_cols = (cur_cols + 1) / 2;
        current = std::move(ll);
    }

    std::size_t total = current.size();
    for (const auto& t : detail_levels_reversed) {
        std::size_t three_b = 0;
        if (!detail::checked_add(t[0].size(), t[1].size(), three_b) ||
            !detail::checked_add(three_b, t[2].size(), three_b) ||
            !detail::checked_add(total, three_b, total)) {
            detail::fail(Error::InvalidParam);
        }
    }
    std::vector<float> out;
    out.reserve(total);
    out.insert(out.end(), current.begin(), current.end());
    for (auto it = detail_levels_reversed.rbegin(); it != detail_levels_reversed.rend(); ++it) {
        out.insert(out.end(), (*it)[0].begin(), (*it)[0].end());
        out.insert(out.end(), (*it)[1].begin(), (*it)[1].end());
        out.insert(out.end(), (*it)[2].begin(), (*it)[2].end());
    }
    return out;
}

inline std::vector<float> idwt_2d(const std::vector<float>& coeffs, std::uint32_t n_rows,
                                  std::uint32_t n_cols, const Wavelet& wavelet,
                                  std::uint32_t levels, Boundary boundary) {
    if (coeffs.empty()) detail::fail(Error::EmptySignal);
    if (n_rows == 0 || n_cols == 0) detail::fail(Error::InvalidParam);
    if (static_cast<std::size_t>(n_rows) > MAX_2D_DIM ||
        static_cast<std::size_t>(n_cols) > MAX_2D_DIM) {
        detail::fail(Error::InvalidParam);
    }
    std::size_t expected_img_len = 0;
    if (!detail::checked_mul(static_cast<std::size_t>(n_rows), static_cast<std::size_t>(n_cols), expected_img_len) ||
        expected_img_len > MAX_SAMPLES || coeffs.size() > MAX_SAMPLES) {
        detail::fail(Error::InvalidParam);
    }
    if (levels == 0 || levels > MAX_LEVELS) detail::fail(Error::InvalidParam);
    for (float x : coeffs) {
        if (!detail::is_finite(x)) detail::fail(Error::InvalidCoefficients);
    }
    detail::check_2d_supported(wavelet, boundary);

    std::vector<std::pair<std::size_t, std::size_t>> level_dims =
        detail::forward_level_dims(n_rows, n_cols, levels);
    std::size_t ll_rows = level_dims[levels].first;
    std::size_t ll_cols = level_dims[levels].second;
    std::size_t expected_total = 0;
    if (!detail::checked_mul(ll_rows, ll_cols, expected_total)) {
        detail::fail(Error::InvalidParam);
    }
    for (std::uint32_t j = 1; j <= levels; ++j) {
        std::size_t band_j = 0;
        std::size_t three_band = 0;
        if (!detail::checked_mul(level_dims[j].first, level_dims[j].second, band_j) ||
            !detail::checked_mul(3, band_j, three_band) ||
            !detail::checked_add(expected_total, three_band, expected_total)) {
            detail::fail(Error::InvalidParam);
        }
    }
    if (coeffs.size() != expected_total) detail::fail(Error::InvalidCoefficients);

    std::size_t offset = 0;
    std::vector<float> ll(coeffs.begin() + static_cast<std::ptrdiff_t>(offset),
                          coeffs.begin() +
                              static_cast<std::ptrdiff_t>(offset + ll_rows * ll_cols));
    offset += ll_rows * ll_cols;

    for (std::uint32_t j = levels; j >= 1; --j) {
        std::size_t rj = level_dims[j].first;
        std::size_t cj = level_dims[j].second;
        std::size_t band_len = rj * cj;
        std::vector<float> hl(coeffs.begin() + static_cast<std::ptrdiff_t>(offset),
                              coeffs.begin() + static_cast<std::ptrdiff_t>(offset + band_len));
        offset += band_len;
        std::vector<float> lh(coeffs.begin() + static_cast<std::ptrdiff_t>(offset),
                              coeffs.begin() + static_cast<std::ptrdiff_t>(offset + band_len));
        offset += band_len;
        std::vector<float> hh(coeffs.begin() + static_cast<std::ptrdiff_t>(offset),
                              coeffs.begin() + static_cast<std::ptrdiff_t>(offset + band_len));
        offset += band_len;
        std::size_t target_rows = level_dims[j - 1].first;
        std::size_t target_cols = level_dims[j - 1].second;
        ll = detail::idwt_2d_one_level(ll, hl, lh, hh, rj, cj, target_rows, target_cols, wavelet,
                                       boundary);
    }
    return ll;
}

// ------------------------------------------------------------------ //
// Continuous Wavelet Transform (CWT)                                 //
// ------------------------------------------------------------------ //

inline std::vector<float> cwt(const std::vector<float>& signal,
                              const std::vector<float>& scales,
                              const Wavelet& wavelet) {
    if (signal.empty()) detail::fail(Error::EmptySignal);
    if (scales.empty()) detail::fail(Error::InvalidParam);
    if (signal.size() > MAX_SAMPLES || scales.size() > MAX_SCALES) {
        detail::fail(Error::InvalidParam);
    }
    std::size_t total_len = 0;
    if (!detail::checked_mul(signal.size(), scales.size(), total_len) || total_len > MAX_SAMPLES) {
        detail::fail(Error::InvalidParam);
    }
    for (float s : scales) {
        if (!detail::is_finite(s) || s <= 0.0f) {
            detail::fail(Error::InvalidParam);
        }
    }
    for (float x : signal) {
        if (!detail::is_finite(x)) {
            detail::fail(Error::InvalidParam);
        }
    }
    if (wavelet.family != Family::Morlet && wavelet.family != Family::MexicanHat) {
        detail::fail(Error::InvalidParam);
    }

    std::vector<float> out(total_len, 0.0f);
    std::size_t n = signal.size();
    constexpr float PI_POW_NEG_QUARTER = 0.7511255444649425f;

    if (wavelet.family == Family::Morlet) {
        constexpr float OMEGA_0 = 6.0f;
        for (std::size_t s_idx = 0; s_idx < scales.size(); ++s_idx) {
            float s = scales[s_idx];
            float inv_sqrt_s = 1.0f / std::sqrt(s);
            float inv_s = 1.0f / s;
            std::int64_t half_support = static_cast<std::int64_t>(8.0f * s + 0.999f);
            if (half_support > static_cast<std::int64_t>(n)) {
                half_support = static_cast<std::int64_t>(n);
            }

            for (std::size_t i = 0; i < n; ++i) {
                float re_acc = 0.0f;
                float im_acc = 0.0f;
                std::int64_t start_k = static_cast<std::int64_t>(i) - half_support;
                std::int64_t end_k = static_cast<std::int64_t>(i) + half_support;
                if (start_k < 0) start_k = 0;
                if (end_k >= static_cast<std::int64_t>(n)) end_k = static_cast<std::int64_t>(n) - 1;

                for (std::int64_t k = start_k; k <= end_k; ++k) {
                    float t = static_cast<float>(k - static_cast<std::int64_t>(i)) * inv_s;
                    float envelope = PI_POW_NEG_QUARTER * std::exp(-0.5f * t * t);
                    float phase = OMEGA_0 * t;
                    float re_psi = envelope * std::cos(phase);
                    float im_psi = -envelope * std::sin(phase);
                    float x = signal[static_cast<std::size_t>(k)];
                    re_acc += x * re_psi;
                    im_acc += x * im_psi;
                }
                out[s_idx * n + i] = inv_sqrt_s * std::sqrt(re_acc * re_acc + im_acc * im_acc);
            }
        }
    } else {
        constexpr float c = 0.86732507f;
        for (std::size_t s_idx = 0; s_idx < scales.size(); ++s_idx) {
            float s = scales[s_idx];
            float inv_sqrt_s = 1.0f / std::sqrt(s);
            float inv_s = 1.0f / s;
            std::int64_t half_support = static_cast<std::int64_t>(8.0f * s + 0.999f);
            if (half_support > static_cast<std::int64_t>(n)) {
                half_support = static_cast<std::int64_t>(n);
            }

            for (std::size_t i = 0; i < n; ++i) {
                float acc = 0.0f;
                std::int64_t start_k = static_cast<std::int64_t>(i) - half_support;
                std::int64_t end_k = static_cast<std::int64_t>(i) + half_support;
                if (start_k < 0) start_k = 0;
                if (end_k >= static_cast<std::int64_t>(n)) end_k = static_cast<std::int64_t>(n) - 1;

                for (std::int64_t k = start_k; k <= end_k; ++k) {
                    float t = static_cast<float>(k - static_cast<std::int64_t>(i)) * inv_s;
                    float psi = c * (1.0f - t * t) * std::exp(-0.5f * t * t);
                    acc += signal[static_cast<std::size_t>(k)] * psi;
                }
                out[s_idx * n + i] = inv_sqrt_s * acc;
            }
        }
    }
    return out;
}

// Convenience: expose the tabulated filters for callers / tests.
inline std::vector<float> analysis_lowpass(const Wavelet& w) { return detail::analysis_lowpass(w); }
inline std::vector<float> qmf_highpass(const std::vector<float>& h) {
    return detail::qmf_highpass(h);
}

} // namespace dsp_wavelets
} // namespace ca

#endif // CA_DSP_WAVELETS_HPP
