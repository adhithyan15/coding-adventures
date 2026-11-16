/**
 * Constants for FFT bounds.
 *
 * Conforms to DSP01 and CWE-400 resource bounding.
 */

/**
 * Maximum 1-D FFT transform length (1,048,576 complex bins = 2^20).
 */
export const MAX_FFT_LEN: number = 1 << 20;

/**
 * Maximum number of float samples allowed across any buffer (16,777,216 floats = 2^24).
 */
export const MAX_SAMPLES: number = 1 << 24;
