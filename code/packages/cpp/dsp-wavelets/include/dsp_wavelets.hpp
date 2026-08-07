// dsp_wavelets.hpp — Discrete Wavelet Transforms (header-only, ISO C++17).
// ---------------------------------------------------------------------------
//
// A faithful C++ port of the Rust `dsp-wavelets` crate, in namespace
// `ca::dsp_wavelets`.  It implements the Discrete Wavelet Transform (DWT) and
// its inverse via the **Mallat pyramid algorithm**, in one and two
// dimensions, for the orthogonal wavelet families Haar, Daubechies (Db2/4/6/8),
// Symlets (Sym4), and Coiflets (Coif1).
//
// ## What a wavelet transform does
//
// Where a Fourier transform decomposes a signal into fixed-frequency sines,
// a wavelet transform decomposes it into scale-and-position-localised basis
// functions — an adaptive time-frequency tiling that matches how edges and
// transients actually live in signals and images.  One forward step is two
// FIR filter passes (a lowpass `h` and a highpass `g`) followed by
// downsample-by-2:
//
//     x[n] ──┬── lowpass  h ──► ↓2 → cA   (approximation, half length)
//            └── highpass g ──► ↓2 → cD   (detail,         half length)
//
// `levels` of DWT applies the same pair recursively to `cA`, producing the
// flattened layout  [cA_J | cD_J | cD_{J-1} | ... | cD_1].
//
// ## Errors
//
// Where the Rust crate returns `Result<_, WaveletError>`, this port throws a
// `WaveletError` exception carrying an `Error` code (the human-readable
// message strings are not reproduced — the code identifies the failure).
//
// ## Buffers
//
// Signals and coefficient buffers are `std::vector<float>` (mirroring the
// crate's `Vec<f32>`).  `slice_level` returns a borrowed `FloatView` into the
// caller's coefficient buffer.
#ifndef CA_DSP_WAVELETS_HPP
#define CA_DSP_WAVELETS_HPP

#include <cstddef>
#include <cstdint>
#include <exception>
#include <vector>

namespace ca {
namespace dsp_wavelets {

// The reciprocal of sqrt(2) as an f32 — the Haar filter tap.  Hard-coded (no
// <cmath>) so the lane stays pure-ISO with no math-library dependency; the
// literal rounds to the same float as Rust's std::f32::consts::FRAC_1_SQRT_2.
inline constexpr float FRAC_1_SQRT_2 = 0.70710678f;

// Defensive caps (mirrors the crate's security-review bounds): levels is
// bounded so `1u << (levels-1)` cannot overflow, and sample counts are capped
// so a hostile length cannot drive a multi-gigabyte allocation.
inline constexpr std::uint32_t MAX_LEVELS = 31;
inline constexpr std::uint32_t MAX_SAMPLES = 1u << 30;

// ------------------------------------------------------------------ //
// Wavelet family / boundary / band selectors                         //
// ------------------------------------------------------------------ //

enum class Family { Haar, Daubechies, Symlets, Coiflets, Biorthogonal, Morlet, MexicanHat };

// A wavelet selector.  `n` is the family order (e.g. Daubechies(4)); `n2` is
// only meaningful for Biorthogonal (the reconstruction order).
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
    Fft                   // reserved for the CWT path (never thrown here)
};

class WaveletError : public std::exception {
public:
    explicit WaveletError(Error code) noexcept : code_(code) {}
    Error code() const noexcept { return code_; }
    const char* what() const noexcept override { return "dsp_wavelets error"; }

private:
    Error code_;
};

// A non-owning view over a slice of a coefficient buffer.
struct FloatView {
    const float* data = nullptr;
    std::size_t size = 0;
};

// ------------------------------------------------------------------ //
// Filter tables (tabulated orthogonal-wavelet coefficients)          //
// ------------------------------------------------------------------ //

namespace detail {

[[noreturn]] inline void fail(Error e) { throw WaveletError(e); }

// Analysis lowpass filter `h` (the `dec_lo` array) for `wavelet`.  Returns an
// empty vector for any (family, order) pair not shipped here — Haar included,
// since Haar is hard-coded in analysis_filters as the canonical worked example.
// Coefficients are the standard PyWavelets `dec_lo` values, stored as f32.
inline std::vector<float> analysis_lowpass(const Wavelet& w) {
    // Daubechies Db2 — 4-tap, 2 vanishing moments.
    static const float DB2[4] = {-0.12940952f, 0.22414386f, 0.8365163f, 0.4829629f};
    // Daubechies Db4 — 8-tap.
    static const float DB4[8] = {-0.010597402f, 0.03288301f, 0.030841382f, -0.18703482f,
                                 -0.02798377f, 0.6308808f, 0.71484655f, 0.23037781f};
    // Daubechies Db6 — 12-tap.
    static const float DB6[12] = {0.11154074f, 0.4946239f, 0.7511339f, 0.31525034f,
                                  -0.2262647f, -0.12976687f, 0.097501606f, 0.027522866f,
                                  -0.03158204f, 0.00055384222f, 0.0047772573f, -0.0010773011f};
    // Daubechies Db8 — 16-tap.
    static const float DB8[16] = {0.05441584f, 0.3128716f, 0.67563075f, 0.5853547f,
                                  -0.015829105f, -0.28401554f, 0.00047248456f, 0.12874743f,
                                  -0.0173693f, -0.044088256f, 0.0139810275f, 0.008746094f,
                                  -0.004870353f, -0.00039174038f, 0.00067544994f, -0.00011747678f};
    // Symlet Sym4 — 8-tap.
    static const float SYM4[8] = {-0.075765714f, -0.029635528f, 0.49761868f, 0.8037388f,
                                  0.2978578f, -0.099219546f, -0.012603967f, 0.0322231f};
    // Coiflet Coif1 — 6-tap.
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

// QMF-derive the analysis highpass `g` from `h`:  g[i] = (-1)^i * h[L-1-i].
inline std::vector<float> qmf_highpass(const std::vector<float>& h) {
    std::size_t l = h.size();
    std::vector<float> g(l);
    for (std::size_t i = 0; i < l; ++i) {
        float reversed = h[l - 1 - i];
        g[i] = (i & 1) == 0 ? reversed : -reversed;
    }
    return g;
}

// Analysis filter pair (h, g).  Haar is hard-coded; other orthogonal families
// come from the table with g QMF-derived.  Throws InvalidParam for anything
// unsupported.
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
    if (levels == 0) fail(Error::InvalidParam);
    if (levels > MAX_LEVELS) fail(Error::InvalidParam);
}

// Sample signal[idx] with the requested boundary extension.  idx may be
// negative or >= signal.size(); the boundary rule maps it into [0, size).
inline float sample_with_boundary(const float* signal, std::size_t signal_len,
                                  std::int64_t idx, Boundary boundary) {
    std::int64_t n = static_cast<std::int64_t>(signal_len);
    if (n == 0) return 0.0f;
    if (idx >= 0 && idx < n) return signal[static_cast<std::size_t>(idx)];
    if (boundary == Boundary::Periodic) {
        std::int64_t m = ((idx % n) + n) % n;
        return signal[static_cast<std::size_t>(m)];
    }
    if (boundary == Boundary::Symmetric) {
        std::int64_t period = 2 * n;
        std::int64_t m = ((idx % period) + period) % period;
        if (m >= n) m = 2 * n - 1 - m;
        return signal[static_cast<std::size_t>(m)];
    }
    return 0.0f; // Zero / Replicate / Reflect pre-rejected upstream
}

// One Mallat step: filter with h and g, downsample by 2 (keep odd indices).
inline void filter_and_downsample(const std::vector<float>& signal, const std::vector<float>& h,
                                  const std::vector<float>& g, Boundary boundary,
                                  std::vector<float>& ca, std::vector<float>& cd) {
    std::size_t n = signal.size();
    std::size_t filter_len = h.size();
    std::size_t out_len = (n + 1) / 2; // ceil(n/2)
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

// Generic one-step synthesis for any orthogonal filter pair (uses the analysis
// filters directly — the two reversals of the textbook synthesis filters
// cancel for orthogonal wavelets).
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

// Per-level signal lengths [L_0, L_1, ..., L_J] under ceil(/2) halving.
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
    if (signal.size() > static_cast<std::size_t>(MAX_SAMPLES)) fail(Error::InvalidParam);
    std::size_t filter_len = filter_length_for(wavelet);
    std::uint32_t shift = levels - 1;
    if (shift > 31) shift = 31;
    std::size_t pow2 = static_cast<std::size_t>(1) << shift;
    std::size_t min_signal_len = filter_len > pow2 ? filter_len : pow2;
    if (signal.size() < min_signal_len) fail(Error::SignalTooShort);
}

} // namespace detail

// ------------------------------------------------------------------ //
// Public 1-D API                                                     //
// ------------------------------------------------------------------ //

// Forward 1-D DWT via the Mallat pyramid.  Output layout (flattened):
//   [cA_J | cD_J | cD_{J-1} | ... | cD_1].
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
    for (const auto& d : details_reversed) total += d.size();
    out.reserve(total);
    out.insert(out.end(), current.begin(), current.end());
    for (auto it = details_reversed.rbegin(); it != details_reversed.rend(); ++it) {
        out.insert(out.end(), it->begin(), it->end());
    }
    return out;
}

// Inverse 1-D DWT.  `output_length` recovers the parity bit lost to
// downsampling — pass the original signal length.
inline std::vector<float> idwt_1d(const std::vector<float>& coeffs, const Wavelet& wavelet,
                                  std::uint32_t levels, Boundary boundary,
                                  std::uint32_t output_length) {
    if (coeffs.empty()) detail::fail(Error::EmptySignal);
    if (output_length == 0) detail::fail(Error::InvalidParam);
    if (output_length > MAX_SAMPLES) detail::fail(Error::InvalidParam);
    detail::check_levels(levels);
    if (coeffs.size() > static_cast<std::size_t>(MAX_SAMPLES)) detail::fail(Error::InvalidParam);
    detail::check_supported_wavelet(wavelet);
    detail::check_supported_boundary(boundary);

    std::vector<std::size_t> level_lens =
        detail::forward_level_lengths(output_length, levels);
    std::size_t coarsest_ca_len = level_lens[levels];
    std::size_t expected_total = coarsest_ca_len;
    for (std::uint32_t j = 1; j <= levels; ++j) expected_total += level_lens[j];
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

// Per-band offsets in a flattened dwt_1d buffer:
//   [offset_of_cA_J, offset_of_cD_J, ..., offset_of_cD_1, total_len].
inline std::vector<std::size_t> split_levels(std::size_t coeffs_len, std::size_t signal_len,
                                             std::uint32_t levels) {
    if (signal_len == 0) detail::fail(Error::InvalidParam);
    if (signal_len > static_cast<std::size_t>(MAX_SAMPLES)) detail::fail(Error::InvalidParam);
    detail::check_levels(levels);
    std::vector<std::size_t> level_lens = detail::forward_level_lengths(signal_len, levels);
    std::size_t coarsest_ca = level_lens[levels];
    std::size_t expected_total = coarsest_ca;
    for (std::uint32_t j = 1; j <= levels; ++j) expected_total += level_lens[j];
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

// Borrowed view of the (target_level, band) slice within `coeffs`.
inline FloatView slice_level(const std::vector<float>& coeffs, std::size_t signal_len,
                             std::uint32_t levels, std::uint32_t target_level, Band band) {
    if (target_level == 0) detail::fail(Error::InvalidParam);
    if (target_level > levels) detail::fail(Error::InvalidParam);
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
    switch (wavelet.family) {
    case Family::Haar:
    case Family::Daubechies:
    case Family::Symlets:
    case Family::Coiflets:
        return;
    default:
        fail(Error::InvalidParam);
    }
}

inline void validate_2d_inputs(const std::vector<float>& image, std::uint32_t n_rows,
                               std::uint32_t n_cols, std::uint32_t levels) {
    if (image.empty()) fail(Error::EmptySignal);
    if (n_rows == 0 || n_cols == 0) fail(Error::InvalidParam);
    if (static_cast<std::size_t>(n_rows) > MAX_SAMPLES ||
        static_cast<std::size_t>(n_cols) > MAX_SAMPLES) {
        fail(Error::InvalidParam);
    }
    if (image.size() != static_cast<std::size_t>(n_rows) * static_cast<std::size_t>(n_cols)) {
        fail(Error::InvalidParam);
    }
    if (levels == 0) fail(Error::InvalidParam);
    if (levels > MAX_LEVELS) fail(Error::InvalidParam);
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

// One level of 2-D DWT: row pass then column pass → (LL, HL, LH, HH).
inline void dwt_2d_one_level(const std::vector<float>& image, std::size_t n_rows,
                             std::size_t n_cols, const Wavelet& wavelet, Boundary boundary,
                             std::vector<float>& ll, std::vector<float>& hl,
                             std::vector<float>& lh, std::vector<float>& hh) {
    std::size_t half_cols = (n_cols + 1) / 2;
    std::size_t half_rows = (n_rows + 1) / 2;

    std::vector<float> l_rows;
    std::vector<float> h_rows;
    l_rows.reserve(n_rows * half_cols);
    h_rows.reserve(n_rows * half_cols);
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

    ll.assign(half_rows * half_cols, 0.0f);
    hl.assign(half_rows * half_cols, 0.0f);
    lh.assign(half_rows * half_cols, 0.0f);
    hh.assign(half_rows * half_cols, 0.0f);
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
    std::vector<float> l_rows(target_rows * band_cols, 0.0f);
    std::vector<float> h_rows(target_rows * band_cols, 0.0f);
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

    std::vector<float> out(target_rows * target_cols, 0.0f);
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

// Forward 2-D DWT via separable row-then-column 1-D DWT.  `image` is a
// row-major [n_rows, n_cols] matrix.  Output layout:
//   [LL_J | HL_J | LH_J | HH_J | HL_{J-1} | LH_{J-1} | HH_{J-1} | ... ].
inline std::vector<float> dwt_2d(const std::vector<float>& image, std::uint32_t n_rows,
                                 std::uint32_t n_cols, const Wavelet& wavelet,
                                 std::uint32_t levels, Boundary boundary) {
    detail::validate_2d_inputs(image, n_rows, n_cols, levels);
    detail::check_2d_supported(wavelet, boundary);

    std::vector<float> current = image;
    std::size_t cur_rows = n_rows;
    std::size_t cur_cols = n_cols;
    std::vector<std::vector<std::vector<float>>> detail_levels_reversed; // each: {hl,lh,hh}
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
    for (const auto& t : detail_levels_reversed) total += t[0].size() + t[1].size() + t[2].size();
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

// Inverse 2-D DWT — reverses dwt_2d.
inline std::vector<float> idwt_2d(const std::vector<float>& coeffs, std::uint32_t n_rows,
                                  std::uint32_t n_cols, const Wavelet& wavelet,
                                  std::uint32_t levels, Boundary boundary) {
    if (coeffs.empty()) detail::fail(Error::EmptySignal);
    if (n_rows == 0 || n_cols == 0) detail::fail(Error::InvalidParam);
    if (static_cast<std::size_t>(n_rows) > MAX_SAMPLES ||
        static_cast<std::size_t>(n_cols) > MAX_SAMPLES) {
        detail::fail(Error::InvalidParam);
    }
    if (levels == 0) detail::fail(Error::InvalidParam);
    if (levels > MAX_LEVELS) detail::fail(Error::InvalidParam);
    detail::check_2d_supported(wavelet, boundary);

    std::vector<std::pair<std::size_t, std::size_t>> level_dims =
        detail::forward_level_dims(n_rows, n_cols, levels);
    std::size_t ll_rows = level_dims[levels].first;
    std::size_t ll_cols = level_dims[levels].second;
    std::size_t expected_total = ll_rows * ll_cols;
    for (std::uint32_t j = 1; j <= levels; ++j) {
        expected_total += 3 * level_dims[j].first * level_dims[j].second;
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

// Convenience: expose the tabulated filters for callers / tests.
inline std::vector<float> analysis_lowpass(const Wavelet& w) { return detail::analysis_lowpass(w); }
inline std::vector<float> qmf_highpass(const std::vector<float>& h) {
    return detail::qmf_highpass(h);
}

} // namespace dsp_wavelets
} // namespace ca

#endif // CA_DSP_WAVELETS_HPP
