/**
 * Defensive resource bounds (CWE-400).
 */

/** Maximum supported FFT length (2^20 = 1,048,576). */
export const MAX_FFT_LEN: number = 1 << 20;

/** Maximum supported signal / output length in samples (2^24 = 16,777,216). */
export const MAX_SAMPLES: number = 1 << 24;

/** Maximum supported frames in spectrogram (2^20 = 1,048,576). */
export const MAX_FRAMES: number = 1 << 20;

/** Maximum supported mel filterbank bins (2^16 = 65,536). */
export const MAX_MELS: number = 1 << 16;
