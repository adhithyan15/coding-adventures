/**
 * Constants for filter bounds.
 *
 * Conforms to DSP03 and CWE-400 resource bounding.
 */

/**
 * Maximum filter taps for FIR filters and IIR polynomials (65,536 taps = 2^16).
 */
export const MAX_FILTER_TAPS: number = 1 << 16;
export const MAX_TAPS: number = MAX_FILTER_TAPS;

/**
 * Maximum signal length in samples (16,777,216 samples = 2^24).
 */
export const MAX_SIGNAL_LEN: number = 1 << 24;
export const MAX_SAMPLES: number = MAX_SIGNAL_LEN;

/**
 * Maximum IIR filter order / SOS section count (1,024 = 2^10).
 */
export const MAX_ORDER: number = 1 << 10;
