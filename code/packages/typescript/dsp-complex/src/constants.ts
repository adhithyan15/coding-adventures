/**
 * Constants for complex number and array bounds.
 *
 * Conforms to DSP00, DSP01, and CWE-400 resource bounding.
 */

/**
 * Maximum number of complex elements in a ComplexArray (16,777,216 samples = 2^24).
 * Equivalent to 268,435,456 bytes for Float64 interleaved storage.
 */
export const MAX_SAMPLES: number = 1 << 24;
