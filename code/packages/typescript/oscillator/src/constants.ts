/**
 * Global constants and CWE-400 bounds for oscillator operations.
 */

export const MAX_SAMPLE_RATE_HZ = 192_000.0;
export const MAX_FREQ_HZ = 96_000.0;
export const MAX_DURATION_SECONDS = 60.0;
export const MAX_SAMPLES = 16_777_216; // 2^24 samples
export const INTEGER_TOLERANCE = 1e-9;
export const TWO_PI = 2.0 * Math.PI;
