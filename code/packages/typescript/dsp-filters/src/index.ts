/**
 * @coding-adventures/dsp-filters
 *
 * Digital FIR and IIR filters, second-order sections (biquads), and classical filter design.
 */

export {
  Biquad,
  sosCascade,
  sos_cascade,
  sosfilt,
} from "./biquad.js";
export {
  MAX_FILTER_TAPS,
  MAX_ORDER,
  MAX_SAMPLES,
  MAX_SIGNAL_LEN,
  MAX_TAPS,
} from "./constants.js";
export {
  butterworthBandpass,
  butterworthHighpass,
  butterworthLowpass,
  chebyshev1Lowpass,
  designBandPass,
  designHighPass,
  designLowPass,
  firwin,
  type FilterCoefficients,
  type FirwinOptions,
} from "./design.js";
export {
  EmptyKernelError,
  EmptySignalError,
  FilterError,
  FftError,
  InvalidCoefficientError,
} from "./errors.js";
export {
  fir,
  firFft,
  fir_fft,
} from "./fir.js";
export {
  filtfilt,
  iir,
  lfilter,
} from "./iir.js";
