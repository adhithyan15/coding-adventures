# Changelog

## Unreleased

### Added
- Added public Biquad/SOS characterization tests for reciprocal normalization overflow, output and hidden-delay overflow, partial state commit after invalid input, and SOS section-order behavior.
- Aligned the AF-02 fixture test loader with decoded finite binary64 equality for version and chunk-length fields, including decimal/exponent spellings and malformed controls.
- Added public FIR designer contract regressions for cutoff endpoints, tap-cap boundaries, lane-specific windows, and validation error precedence.
- Added strict shared AF-02 Biquad/SOS fixture checks, including all 32,768 impulse chunk partitions, reset replay, 64 seeded partition-property samples, and 128 seeded malformed-fixture fuzz samples through public exports.

### Fixed
- Avoid reciprocal overflow during low-pass and band-pass FIR normalization when the finite nonzero normalization denominator has a nonfinite reciprocal; retain legacy multiplication when the reciprocal is finite.
- `dsp-filters.warrant` was invalid Warrant (`max` is not a builtin); the iir steps bound now uses `imax`. Document only.
- Removed the false FIR round-trip claim, common byte-length caps, and design `no_panic` claims pending lane-specific sample framing and typed error behavior. Added public FIR regressions for `[2] * [2] = [4]` and the one-tap identity. Warrant declarations remain obligations, not executed evidence.

## 0.1.0

- Initial release of `@coding-adventures/dsp-filters`.
- `Biquad` Audio EQ Cookbook implementation with Direct-Form II Transposed structure.
- `lfilter` and `iir` Direct-Form II Transposed IIR filtering.
- `sosfilt` Second-Order Sections cascade.
- `filtfilt` zero-phase forward-backward filtering.
- Direct FIR filtering (`fir`) and FFT-accelerated fast convolution (`firFft` / `fir_fft`).
- Classical filter design: `firwin`, `designBandPass`, `designLowPass`, `designHighPass`, `butterworthLowpass`, `butterworthHighpass`, `butterworthBandpass`, `chebyshev1Lowpass`.
- VER15 security hardening: CWE-20, CWE-400, CWE-674, CWE-1321, CWE-1333 compliance.
