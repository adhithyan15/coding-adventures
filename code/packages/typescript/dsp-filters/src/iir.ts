import { MAX_FILTER_TAPS, MAX_ORDER, MAX_SIGNAL_LEN } from "./constants.js";
import {
  EmptyKernelError,
  EmptySignalError,
  InvalidCoefficientError,
} from "./errors.js";

/**
 * Compute IIR filtering using Direct-Form-II Transposed structure.
 * Matches scipy.signal.lfilter(b, a, x).
 *
 * b: numerator / feed-forward polynomial coefficients.
 * a: denominator / feedback polynomial coefficients.
 * signal: real input sequence of length N.
 */
export function lfilter(
  b: Float64Array | ArrayLike<number>,
  a: Float64Array | ArrayLike<number>,
  signal: Float64Array | ArrayLike<number>
): Float64Array {
  const n = signal.length;
  if (n === 0) {
    throw new EmptySignalError("filter signal must be non-empty");
  }
  const m = b.length;
  const nCoef = a.length;
  if (m === 0 || nCoef === 0) {
    throw new EmptyKernelError("filter coefficients b and a must be non-empty");
  }

  if (n > MAX_SIGNAL_LEN) {
    throw new InvalidCoefficientError(
      `signal length ${n} exceeds MAX_SIGNAL_LEN (${MAX_SIGNAL_LEN})`
    );
  }
  if (m > MAX_FILTER_TAPS || nCoef > MAX_FILTER_TAPS) {
    throw new InvalidCoefficientError(
      `coefficient length exceeds MAX_FILTER_TAPS (${MAX_FILTER_TAPS})`
    );
  }

  const a0 = Number(a[0]);
  if (a0 === 0.0 || !Number.isFinite(a0)) {
    throw new InvalidCoefficientError(`a[0] must be non-zero and finite; got ${a0}`);
  }

  const order = Math.max(m, nCoef) - 1;
  if (order > MAX_ORDER) {
    throw new InvalidCoefficientError(
      `filter order ${order} exceeds MAX_ORDER (${MAX_ORDER})`
    );
  }

  for (let i = 0; i < n; i++) {
    if (!Number.isFinite(Number(signal[i]))) {
      throw new InvalidCoefficientError("non-finite float in signal");
    }
  }
  for (let i = 0; i < m; i++) {
    if (!Number.isFinite(Number(b[i]))) {
      throw new InvalidCoefficientError("non-finite float in b");
    }
  }
  for (let i = 0; i < nCoef; i++) {
    if (!Number.isFinite(Number(a[i]))) {
      throw new InvalidCoefficientError("non-finite float in a");
    }
  }

  const invA0 = 1.0 / a0;
  const bScaled = new Float64Array(order + 1);
  for (let k = 0; k < m; k++) {
    bScaled[k] = Number(b[k]) * invA0;
  }
  const aScaled = new Float64Array(order + 1);
  for (let k = 0; k < nCoef; k++) {
    aScaled[k] = Number(a[k]) * invA0;
  }

  const state = new Float64Array(order);
  const out = new Float64Array(n);

  for (let i = 0; i < n; i++) {
    const xVal = Number(signal[i]);
    const y = bScaled[0] * xVal + (order > 0 ? state[0] : 0.0);
    out[i] = y;

    if (order > 0) {
      for (let k = 0; k < order - 1; k++) {
        state[k] =
          bScaled[k + 1] * xVal - aScaled[k + 1] * y + state[k + 1];
      }
      state[order - 1] = bScaled[order] * xVal - aScaled[order] * y;
    }
  }

  return out;
}

/**
 * IIR filtering alias matching (signal, b, a) convention from warrant specification.
 */
export function iir(
  signal: Float64Array | ArrayLike<number>,
  b: Float64Array | ArrayLike<number>,
  a: Float64Array | ArrayLike<number>
): Float64Array {
  return lfilter(b, a, signal);
}

/**
 * Forward-backward zero-phase digital filtering.
 * Matches scipy.signal.filtfilt.
 */
export function filtfilt(
  b: Float64Array | ArrayLike<number>,
  a: Float64Array | ArrayLike<number>,
  signal: Float64Array | ArrayLike<number>
): Float64Array {
  const n = signal.length;
  if (n === 0) {
    throw new EmptySignalError("signal must not be empty");
  }
  const m = b.length;
  const nCoef = a.length;
  if (m === 0 || nCoef === 0) {
    throw new EmptyKernelError("filter coefficients b and a must be non-empty");
  }

  if (n > MAX_SIGNAL_LEN) {
    throw new InvalidCoefficientError(
      `signal length ${n} exceeds MAX_SIGNAL_LEN (${MAX_SIGNAL_LEN})`
    );
  }
  if (m > MAX_FILTER_TAPS || nCoef > MAX_FILTER_TAPS) {
    throw new InvalidCoefficientError(
      `coefficient length exceeds MAX_FILTER_TAPS (${MAX_FILTER_TAPS})`
    );
  }

  const a0 = Number(a[0]);
  if (a0 === 0.0 || !Number.isFinite(a0)) {
    throw new InvalidCoefficientError(`a[0] must be non-zero and finite; got ${a0}`);
  }

  const order = Math.max(m, nCoef) - 1;
  if (order > MAX_ORDER) {
    throw new InvalidCoefficientError(
      `filter order ${order} exceeds MAX_ORDER (${MAX_ORDER})`
    );
  }

  const maxCoeffLen = Math.max(m, nCoef);
  const padLen = 3 * maxCoeffLen;
  const usePadding = n > padLen;
  if (usePadding && n + 2 * padLen > MAX_SIGNAL_LEN) {
    throw new InvalidCoefficientError(
      `padded signal length ${n + 2 * padLen} exceeds MAX_SIGNAL_LEN (${MAX_SIGNAL_LEN})`
    );
  }

  // Pre-validate all numeric elements for finite floats before allocation
  for (let i = 0; i < n; i++) {
    if (!Number.isFinite(Number(signal[i]))) {
      throw new InvalidCoefficientError("non-finite float in signal");
    }
  }
  for (let i = 0; i < m; i++) {
    if (!Number.isFinite(Number(b[i]))) {
      throw new InvalidCoefficientError("non-finite float in b");
    }
  }
  for (let i = 0; i < nCoef; i++) {
    if (!Number.isFinite(Number(a[i]))) {
      throw new InvalidCoefficientError("non-finite float in a");
    }
  }

  let padded: Float64Array;

  if (usePadding) {
    padded = new Float64Array(n + 2 * padLen);
    const x0 = Number(signal[0]);
    const xEnd = Number(signal[n - 1]);

    // Leading reflection: 2*x0 - x[padLen..1]
    for (let i = 0; i < padLen; i++) {
      padded[i] = 2.0 * x0 - Number(signal[padLen - i]);
    }
    // Main signal
    for (let i = 0; i < n; i++) {
      padded[padLen + i] = Number(signal[i]);
    }
    // Trailing reflection: 2*xEnd - x[n-2..n-padLen-1]
    for (let i = 0; i < padLen; i++) {
      padded[padLen + n + i] = 2.0 * xEnd - Number(signal[n - 2 - i]);
    }
  } else {
    padded = new Float64Array(n);
    for (let i = 0; i < n; i++) {
      padded[i] = Number(signal[i]);
    }
  }

  // Forward pass
  const forward = lfilter(b, a, padded);

  // Reverse buffer
  const revPadded = new Float64Array(forward.length);
  const fLen = forward.length;
  for (let i = 0; i < fLen; i++) {
    revPadded[i] = forward[fLen - 1 - i];
  }

  // Backward pass
  const backward = lfilter(b, a, revPadded);

  // Reverse back
  const out = new Float64Array(n);
  if (usePadding) {
    for (let i = 0; i < n; i++) {
      out[i] = backward[fLen - 1 - (padLen + i)];
    }
  } else {
    for (let i = 0; i < n; i++) {
      out[i] = backward[n - 1 - i];
    }
  }

  return out;
}
