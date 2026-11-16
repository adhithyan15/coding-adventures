/**
 * Defensive resource bounds (CWE-400).
 */

/** Maximum signal / image samples: 2^24 = 16,777,216 samples (~64 MB of float32 / ~128 MB of float64). */
export const MAX_SAMPLES: number = 1 << 24;
export const MAX_SIGNAL_LEN: number = MAX_SAMPLES;

/** Maximum convolution kernel length: 2^20 = 1,048,576 taps. */
export const MAX_KERNEL_LEN: number = 1 << 20;
