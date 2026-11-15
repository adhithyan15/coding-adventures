/**
 * Defensive resource bounds (CWE-400).
 */

/** Maximum supported signal / output / image length in samples (2^24 = 16,777,216). */
export const MAX_SAMPLES: number = 1 << 24;

/** Maximum supported wavelet decomposition levels (pyramid depth). */
export const MAX_WAVELET_LEVELS: number = 31;

/** Maximum continuous wavelet transform scales. */
export const MAX_SCALES: number = 4096;
