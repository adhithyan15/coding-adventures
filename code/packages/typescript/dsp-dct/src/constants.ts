/**
 * Constants for DCT bounds.
 *
 * Conforms to DSP02 and CWE-400 resource bounding.
 */

/**
 * Maximum 1-D DCT transform length (1,048,576 samples = 2^20).
 */
export const MAX_DCT_LEN: number = 1 << 20;

/**
 * Maximum total samples allowed across 1-D or 2-D buffers (16,777,216 samples = 2^24).
 */
export const MAX_SAMPLES: number = 1 << 24;
