/**
 * Signal boundary extension modes outside [0, N) support.
 */
export enum WaveletBoundary {
  ZERO = "zero",
  REPLICATE = "replicate",
  REFLECT = "reflect",
  SYMMETRIC = "symmetric",
  PERIODIC = "periodic",
}

export const Zero = WaveletBoundary.ZERO;
export const Replicate = WaveletBoundary.REPLICATE;
export const Reflect = WaveletBoundary.REFLECT;
export const Symmetric = WaveletBoundary.SYMMETRIC;
export const Periodic = WaveletBoundary.PERIODIC;

/**
 * Supported wavelet families.
 */
export enum WaveletFamily {
  HAAR = "haar",
  DAUBECHIES = "db",
  SYMLETS = "sym",
  COIFLETS = "coif",
  BIORTHOGONAL = "bior",
  MORLET = "morlet",
  RICKER = "ricker",
}
