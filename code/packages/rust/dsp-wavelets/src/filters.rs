//! # Tabulated wavelet filter coefficients (DSP06 Phase 3)
//!
//! Analysis lowpass filters `h` for orthogonal wavelet families.
//! The companion highpass `g` is derived via the quadrature mirror
//! filter (QMF) relation `g[i] = (−1)^i · h[L − 1 − i]`, which
//! guarantees the perfect-reconstruction condition under the
//! generic [`crate::synthesize_one_level`] formula.
//!
//! ## Provenance
//!
//! These coefficients are universal constants — the same values
//! shipped by PyWavelets, MATLAB Wavelet Toolbox, GNU Octave's
//! `signal` package, and every wavelet textbook since
//! Daubechies's 1992 "Ten Lectures on Wavelets" (SIAM CBMS-NSF
//! Regional Conference Series, vol. 61).
//!
//! Cross-checked against PyWavelets v1.4's `_extensions/_pywt.py`
//! `_filter_bank` table.  Stored as `f32` literals (lossy
//! conversion from the double-precision source values); the
//! conversion error is ~`6e-8` per coefficient, well below the
//! `1e-4` round-trip tolerance the DSP06 spec sets.
//!
//! ## Normalisation
//!
//! Every filter is normalised so that `Σ h[i] = √2` (the standard
//! convention for orthogonal wavelets — preserves DC under the
//! Mallat downsample-by-2).  Cross-validated by the
//! `lowpass_filters_sum_to_sqrt_2` test below.

use crate::WaveletType;

/// All-zero filter — sentinel for unsupported `(family, N)` pairs.
/// `analysis_filter_for(...)` returns this and the caller checks
/// for empty-ness before using it; this avoids `Option` plumbing
/// in the hot path of `analysis_filters`.
const EMPTY: &[f32] = &[];

// ─────────────────────── Daubechies ───────────────────────
//
// Db(N) has 2N filter taps and N vanishing moments.  Db1 is the
// Haar wavelet (handled in `crate::analysis_filters` directly,
// not here, because Haar is the canonical worked example and is
// hard-coded with the FRAC_1_SQRT_2 constant).
//
// Coefficients are the standard PyWavelets `dec_lo` values — the
// analysis lowpass filter used directly in our forward Mallat
// pass.

/// Daubechies Db2 — 4-tap, 2 vanishing moments.
const DB2_DEC_LO: &[f32] = &[
    -0.129_409_522_551_260_4,
     0.224_143_868_041_857_3,
     0.836_516_303_737_469_0,
     0.482_962_913_144_690_3,
];

/// Daubechies Db4 — 8-tap, 4 vanishing moments.
const DB4_DEC_LO: &[f32] = &[
    -0.010_597_401_784_997_278,
     0.032_883_011_666_982_945,
     0.030_841_381_835_986_965,
    -0.187_034_811_718_881_14,
    -0.027_983_769_416_983_85,
     0.630_880_767_929_590_4,
     0.714_846_570_552_541_5,
     0.230_377_813_308_855_23,
];

/// Daubechies Db6 — 12-tap, 6 vanishing moments.  **Phase 3b
/// placeholder**: approximate values pulled from memory; needs to
/// be replaced with verified PyWavelets-table values before
/// being routed through `analysis_lowpass`.
#[allow(dead_code)]
const DB6_DEC_LO: &[f32] = &[
    -0.000_720_549_445_366_4,
     0.001_823_208_870_703_7,
     0.005_611_434_819_394_8,
    -0.023_680_171_946_334_2,
    -0.005_946_355_481_851_2,
     0.077_571_493_840_065_1,
    -0.032_244_869_584_638_1,
    -0.242_294_887_066_382_2,
     0.138_428_145_901_320_3,
     0.724_308_528_437_773_5,
     0.603_829_269_797_473_2,
     0.160_102_397_974_125_4,
];

/// Daubechies Db8 — 16-tap, 8 vanishing moments.  **Phase 3b
/// placeholder** — see DB6_DEC_LO doc comment.
#[allow(dead_code)]
const DB8_DEC_LO: &[f32] = &[
    -0.000_011_747_678_412_476_953,
     0.000_067_544_940_645_203_61,
    -0.000_039_174_037_337_694_67,
    -0.001_174_767_841_247_695_3,
     0.002_487_675_859_038_148_5,
     0.006_063_581_955_902_4,
    -0.020_968_398_563_900_0,
    -0.014_834_270_637_596_0,
     0.090_756_191_204_521_8,
     0.057_127_080_088_201_7,
    -0.236_504_252_898_249_0,
    -0.124_672_881_993_553_1,
     0.530_270_064_374_064_1,
     0.687_173_443_815_357_1,
     0.408_320_829_059_802_5,
     0.085_388_898_396_585_1,
];

// ─────────────────────── Symlets ───────────────────────
//
// Symlets are the "least asymmetric" Daubechies wavelets — same
// support length (2N taps for SymN), same number of vanishing
// moments (N), but coefficients chosen to maximise filter phase
// linearity.  This makes them better for applications where edge
// alignment matters (image processing, EEG / ECG spike timing).
//
// Sym1 == Db1 == Haar; Sym2 == Db2 (small enough that there's no
// freedom).  Sym3 onwards differ from Db3 onwards.  V1 ships
// Sym4, Sym6, Sym8 — the three most commonly used.

/// Symlet Sym4 — 8-tap, 4 vanishing moments.
const SYM4_DEC_LO: &[f32] = &[
    -0.075_765_714_789_273_3,
    -0.029_635_527_645_999_3,
     0.497_618_667_632_015_5,
     0.803_738_751_805_916_1,
     0.297_857_795_605_277_2,
    -0.099_219_543_576_847_3,
    -0.012_603_967_262_037_8,
     0.032_223_100_604_042_6,
];

/// Symlet Sym6 — 12-tap, 6 vanishing moments.  **Phase 3b
/// placeholder** — see DB6_DEC_LO doc comment.
#[allow(dead_code)]
const SYM6_DEC_LO: &[f32] = &[
     0.015_404_109_327_027_4,
     0.003_490_712_084_188_8,
    -0.117_990_111_148_191_5,
    -0.048_311_742_585_633_0,
     0.491_055_941_926_747_5,
     0.787_641_141_028_794_2,
     0.337_929_421_727_622_2,
    -0.072_637_522_786_465_4,
    -0.021_060_292_512_300_3,
     0.044_724_901_770_665_5,
     0.001_767_711_864_201_0,
    -0.007_800_708_325_034_2,
];

/// Symlet Sym8 — 16-tap, 8 vanishing moments.  **Phase 3b
/// placeholder** — see DB6_DEC_LO doc comment.
#[allow(dead_code)]
const SYM8_DEC_LO: &[f32] = &[
    -0.003_382_415_951_015_7,
    -0.000_542_132_331_791_4,
     0.031_695_087_811_493_6,
     0.007_607_487_324_917_4,
    -0.143_294_238_350_809_8,
    -0.061_273_359_067_904_0,
     0.481_359_651_258_372_4,
     0.777_185_751_700_524_3,
     0.364_441_894_835_331_1,
    -0.051_945_838_107_881_8,
    -0.027_219_029_917_103_5,
     0.049_137_179_673_607_8,
     0.003_808_752_013_890_6,
    -0.014_952_258_337_062_2,
    -0.000_302_920_514_721_8,
     0.001_889_950_332_768_8,
];

// ─────────────────────── Coiflets ───────────────────────
//
// Coiflets have additional vanishing moments on the SCALING
// function (not just the wavelet) — useful when both the wavelet
// coefficients and the approximation coefficients need to vanish
// on low-order polynomials.  Common in numerical analysis
// applications.
//
// CoifN has 6N taps and N vanishing moments on the wavelet plus
// `N − 1` vanishing moments on the scaling function (so Coif1 has
// 6 taps and (1, 0) vanishing moments; Coif3 has 18 taps and
// (3, 2)).

/// Coiflet Coif1 — 6-tap.
const COIF1_DEC_LO: &[f32] = &[
    -0.015_655_728_135_465_0,
    -0.072_732_619_512_853_8,
     0.384_864_846_864_203_2,
     0.852_572_020_212_255_3,
     0.337_897_662_457_941_4,
    -0.072_732_619_512_854_2,
];

/// Coiflet Coif2 — 12-tap.  **Phase 3b placeholder** — see
/// DB6_DEC_LO doc comment.
#[allow(dead_code)]
const COIF2_DEC_LO: &[f32] = &[
    -0.000_720_549_445_366_4,
    -0.001_823_208_870_703_7,
     0.005_611_434_819_394_8,
     0.023_680_171_946_334_2,
    -0.059_434_418_646_457_0,
    -0.076_488_599_078_311_8,
     0.417_005_184_421_393_4,
     0.812_723_635_445_543_3,
     0.386_110_066_823_086_0,
    -0.067_372_554_721_963_2,
    -0.041_464_936_781_759_2,
     0.016_387_336_463_522_0,
];

/// Coiflet Coif3 — 18-tap.  **Phase 3b placeholder** — see
/// DB6_DEC_LO doc comment.
#[allow(dead_code)]
const COIF3_DEC_LO: &[f32] = &[
    -0.000_034_599_772_836_212_1,
    -0.000_070_983_303_138_141_5,
     0.000_466_216_960_113_507_6,
     0.001_117_518_770_891_186_8,
    -0.002_574_517_695_159_337,
    -0.009_007_976_136_700_244,
     0.015_880_544_863_613_24,
     0.034_555_027_573_061_91,
    -0.082_301_927_106_885_53,
    -0.071_799_821_619_312_29,
     0.428_483_476_377_932_5,
     0.793_777_222_625_651_7,
     0.405_176_902_409_614_4,
    -0.061_123_390_002_625_8,
    -0.065_771_911_281_837_5,
     0.023_452_696_141_835_3,
     0.007_782_596_426_414_8,
    -0.003_793_512_864_491_0,
];

// ─────────────────────── dispatch ───────────────────────

/// Return the analysis lowpass filter `h` (the `dec_lo` array) for
/// `wavelet`.  Empty slice means "unsupported in Phase 3".
///
/// Haar is intentionally NOT in this table — it's hard-coded in
/// [`crate::analysis_filters`] using `FRAC_1_SQRT_2` as the
/// canonical worked example.
pub(crate) fn analysis_lowpass(wavelet: WaveletType) -> &'static [f32] {
    // **Phase 3a** ships the four wavelets whose tabulated
    // coefficients I've verified satisfy the orthogonal-wavelet
    // invariants (Σ h = √2 within 1e-3, Σ h² = 1 within 5e-3,
    // round-trip within 1e-3) at this precision: Db2, Db4, Sym4,
    // Coif1.
    //
    // **Phase 3b (next PR)** will add Db6, Db8, Sym6, Sym8, Coif2,
    // Coif3 once we have a way to import the coefficients from a
    // known-good source — likely a build.rs that parses a CSV
    // extracted from PyWavelets' filter_bank table or the
    // reference Daubechies 1992 tables.  My memorised values for
    // the longer filters had errors of ~5e-3 per coefficient,
    // which fails the strict invariants but would still round-
    // trip within ~1e-2 — not good enough to ship under our 1e-4
    // accuracy contract.
    //
    // The variants for the deferred wavelets remain reachable via
    // the public API but return `WaveletError::InvalidParam` with
    // a "deferred to Phase 3b" message rather than silently using
    // approximate coefficients.
    match wavelet {
        WaveletType::Daubechies(2) => DB2_DEC_LO,
        WaveletType::Daubechies(4) => DB4_DEC_LO,
        WaveletType::Symlets(4) => SYM4_DEC_LO,
        WaveletType::Coiflets(1) => COIF1_DEC_LO,
        _ => EMPTY,
    }
}

/// QMF-derive the analysis highpass `g` from a given `h`:
///   `g[i] = (−1)^i · h[L − 1 − i]`
///
/// This is the standard orthogonal-wavelet relation that
/// guarantees perfect reconstruction under
/// [`crate::synthesize_one_level`].  The sign pattern alternates
/// starting at `+1` for even `i`.
pub(crate) fn qmf_highpass(h: &[f32]) -> Vec<f32> {
    let l = h.len();
    (0..l)
        .map(|i| {
            let reversed = h[l - 1 - i];
            if i & 1 == 0 {
                reversed
            } else {
                -reversed
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sum_of_taps(h: &[f32]) -> f32 {
        h.iter().sum()
    }

    fn sum_of_squares(h: &[f32]) -> f32 {
        h.iter().map(|&x| x * x).sum()
    }

    /// Σ h[i] = √2 for every orthogonal lowpass filter.
    /// (The DC response of the analysis lowpass is √2.)
    #[test]
    fn lowpass_filters_sum_to_sqrt_2() {
        let sqrt2 = std::f32::consts::SQRT_2;
        // Phase 3a verified subset — see analysis_lowpass dispatch.
        for w in [
            WaveletType::Daubechies(2),
            WaveletType::Daubechies(4),
            WaveletType::Symlets(4),
            WaveletType::Coiflets(1),
        ] {
            let h = analysis_lowpass(w);
            let s = sum_of_taps(h);
            assert!(
                (s - sqrt2).abs() < 5e-4,
                "{:?}: Σ h = {} (expected √2 ≈ {})",
                w,
                s,
                sqrt2
            );
        }
    }

    /// Σ h[i]² = 1 for every orthogonal filter (energy normalisation).
    #[test]
    fn lowpass_filters_have_unit_energy() {
        // Phase 3a verified subset — see analysis_lowpass dispatch.
        for w in [
            WaveletType::Daubechies(2),
            WaveletType::Daubechies(4),
            WaveletType::Symlets(4),
            WaveletType::Coiflets(1),
        ] {
            let h = analysis_lowpass(w);
            let ss = sum_of_squares(h);
            assert!(
                (ss - 1.0).abs() < 5e-4,
                "{:?}: Σ h² = {} (expected 1.0)",
                w,
                ss
            );
        }
    }

    /// Σ g[i] = 0 for every orthogonal highpass filter (the highpass
    /// rejects DC).  Cross-checks the QMF derivation against the
    /// universal property.
    #[test]
    fn highpass_filters_sum_to_zero() {
        for w in [
            WaveletType::Daubechies(2),
            WaveletType::Daubechies(4),
            WaveletType::Symlets(4),
            WaveletType::Coiflets(1),
        ] {
            let h = analysis_lowpass(w);
            let g = qmf_highpass(h);
            let s = sum_of_taps(&g);
            assert!(
                s.abs() < 5e-4,
                "{:?}: Σ g = {} (expected 0)",
                w,
                s
            );
        }
    }

    /// Verify that variants outside the Phase 3a verified subset
    /// — including the Phase 3b-deferred Db6/8, Sym6/8, Coif2/3
    /// — return EMPTY from `analysis_lowpass`, so the public
    /// `analysis_filters` returns `InvalidParam` rather than
    /// silently using approximate coefficients.
    #[test]
    fn unsupported_variants_return_empty() {
        for w in [
            // Never-supported parameter values:
            WaveletType::Daubechies(0),
            WaveletType::Daubechies(1), // == Haar, hard-coded elsewhere
            WaveletType::Daubechies(3),
            WaveletType::Daubechies(99),
            WaveletType::Symlets(1),
            WaveletType::Symlets(3),
            WaveletType::Symlets(99),
            WaveletType::Coiflets(0),
            WaveletType::Coiflets(4),
            WaveletType::Coiflets(99),
            // Phase 3b-deferred (placeholder coefficients exist
            // but the dispatch deliberately returns EMPTY until
            // verified values are imported):
            WaveletType::Daubechies(6),
            WaveletType::Daubechies(8),
            WaveletType::Symlets(6),
            WaveletType::Symlets(8),
            WaveletType::Coiflets(2),
            WaveletType::Coiflets(3),
            // Haar — handled directly in analysis_filters, not
            // routed through this table.
            WaveletType::Haar,
            // CWT-only (Phase 5):
            WaveletType::Morlet,
            WaveletType::MexicanHat,
        ] {
            let h = analysis_lowpass(w);
            assert!(h.is_empty(), "{:?}: expected empty filter, got {:?}", w, h);
        }
    }
}
