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

/**
 * Maximum Kaiser window beta parameter (I_0(x) overflows IEEE-754 binary64 around 709.78).
 */
export const MAX_KAISER_BETA: number = 700.0;
