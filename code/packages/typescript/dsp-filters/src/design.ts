import { buildWindow } from "@coding-adventures/dsp-fft";
import { MAX_FILTER_TAPS } from "./constants.js";
import { InvalidCoefficientError } from "./errors.js";

export interface FilterCoefficients {
  b: Float64Array;
  a: Float64Array;
}

export interface FirwinOptions {
  window?: string;
  passZero?: boolean;
}

// ─────────────────────────── FIR Filter Designs ───────────────────────────

/**
 * Design a linear-phase low-pass FIR filter via windowed-sinc.
 *
 * cutoffNorm: normalised cutoff in [0.0, 0.5] (0.5 is Nyquist).
 * numTaps: filter length — must be odd and >= 1.
 * window: window name string ('hamming', 'hann', 'blackman', 'rectangular', 'kaiser').
 *
 * Returns kernel normalized so DC gain is 1.0.
 */
export function designLowPass(
  cutoffNorm: number,
  numTaps: number,
  window: string = "hamming"
): Float64Array {
  if (!Number.isFinite(numTaps) || !Number.isInteger(numTaps) || numTaps < 1 || numTaps % 2 === 0) {
    throw new InvalidCoefficientError(
      `numTaps must be odd and >= 1; got ${numTaps}`
    );
  }
  if (numTaps > MAX_FILTER_TAPS) {
    throw new InvalidCoefficientError(
      `numTaps ${numTaps} exceeds MAX_FILTER_TAPS (${MAX_FILTER_TAPS})`
    );
  }
  if (!Number.isFinite(cutoffNorm) || cutoffNorm < 0.0 || cutoffNorm > 0.5) {
    throw new InvalidCoefficientError(
      `cutoffNorm must be finite in [0.0, 0.5]; got ${cutoffNorm}`
    );
  }

  const win = buildWindow(window, numTaps);
  const m = numTaps - 1;
  const centre = m * 0.5;
  const fc = cutoffNorm;

  const h = new Float64Array(numTaps);
  for (let k = 0; k < numTaps; k++) {
    const x = k - centre;
    const val =
      x === 0.0
        ? 2.0 * fc
        : Math.sin(2.0 * Math.PI * fc * x) / (Math.PI * x);
    h[k] = val * win[k];
  }

  // Normalise DC gain = 1.0
  let sum = 0.0;
  for (let k = 0; k < numTaps; k++) {
    sum += h[k];
  }
  if (Math.abs(sum) > 0.0) {
    const invSum = 1.0 / sum;
    for (let k = 0; k < numTaps; k++) {
      h[k] *= invSum;
    }
  }

  return h;
}

/**
 * Design a linear-phase high-pass FIR filter via spectral inversion.
 */
export function designHighPass(
  cutoffNorm: number,
  numTaps: number,
  window: string = "hamming"
): Float64Array {
  const lp = designLowPass(cutoffNorm, numTaps, window);
  const hp = new Float64Array(numTaps);
  for (let k = 0; k < numTaps; k++) {
    hp[k] = -lp[k];
  }
  const centre = (numTaps - 1) >> 1;
  hp[centre] += 1.0;
  return hp;
}

/**
 * Design a linear-phase band-pass FIR filter via windowed-sinc.
 */
export function designBandPass(
  lowCutoffNorm: number,
  highCutoffNorm: number,
  numTaps: number,
  window: string = "hamming"
): Float64Array {
  if (!Number.isFinite(numTaps) || !Number.isInteger(numTaps) || numTaps < 1 || numTaps % 2 === 0) {
    throw new InvalidCoefficientError(
      `numTaps must be odd and >= 1; got ${numTaps}`
    );
  }
  if (numTaps > MAX_FILTER_TAPS) {
    throw new InvalidCoefficientError(
      `numTaps ${numTaps} exceeds MAX_FILTER_TAPS (${MAX_FILTER_TAPS})`
    );
  }
  if (!Number.isFinite(lowCutoffNorm) || !Number.isFinite(highCutoffNorm)) {
    throw new InvalidCoefficientError("cutoff frequencies must be finite");
  }
  if (!(0.0 <= lowCutoffNorm && lowCutoffNorm < highCutoffNorm && highCutoffNorm <= 0.5)) {
    throw new InvalidCoefficientError(
      `cutoffs must satisfy 0.0 <= low < high <= 0.5; got low=${lowCutoffNorm}, high=${highCutoffNorm}`
    );
  }

  const win = buildWindow(window, numTaps);
  const m = numTaps - 1;
  const centre = m * 0.5;
  const f1 = lowCutoffNorm;
  const f2 = highCutoffNorm;

  const h = new Float64Array(numTaps);
  for (let k = 0; k < numTaps; k++) {
    const x = k - centre;
    const val =
      x === 0.0
        ? 2.0 * (f2 - f1)
        : (Math.sin(2.0 * Math.PI * f2 * x) - Math.sin(2.0 * Math.PI * f1 * x)) /
          (Math.PI * x);
    h[k] = val * win[k];
  }

  // Normalise for unit peak passband gain at centre frequency f0
  const f0 = (f1 + f2) * 0.5;
  let gain = 0.0;
  for (let k = 0; k < numTaps; k++) {
    const x = k - centre;
    gain += h[k] * Math.cos(2.0 * Math.PI * f0 * x);
  }

  const gainAbs = Math.abs(gain);
  if (gainAbs > 0.0) {
    const invG = 1.0 / gainAbs;
    for (let k = 0; k < numTaps; k++) {
      h[k] *= invG;
    }
  }

  return h;
}

/**
 * Windowed FIR filter design matching scipy.signal.firwin.
 *
 * numTaps: length of filter (must be odd).
 * cutoff: cutoff frequency in (0, 0.5), or [low, high] for bandpass/bandstop.
 */
export function firwin(
  numTaps: number,
  cutoff: number | [number, number],
  options?: FirwinOptions
): Float64Array {
  const win = options?.window ?? "hamming";
  if (typeof cutoff === "number") {
    const passZero = options?.passZero !== false;
    if (passZero) {
      return designLowPass(cutoff, numTaps, win);
    } else {
      return designHighPass(cutoff, numTaps, win);
    }
  }

  if (Array.isArray(cutoff) && cutoff.length === 2) {
    const [low, high] = cutoff;
    const passZero = options?.passZero === true;
    if (!passZero) {
      return designBandPass(low, high, numTaps, win);
    } else {
      // Bandstop: spectral inversion of bandpass
      const bp = designBandPass(low, high, numTaps, win);
      const bs = new Float64Array(numTaps);
      for (let k = 0; k < numTaps; k++) {
        bs[k] = -bp[k];
      }
      const centre = (numTaps - 1) >> 1;
      bs[centre] += 1.0;
      return bs;
    }
  }

  throw new InvalidCoefficientError("cutoff must be a number or [low, high] pair");
}

// ─────────────────────────── IIR Filter Designs ───────────────────────────

/**
 * Design a Butterworth low-pass IIR filter (orders 1 and 2).
 * Returns { b, a } coefficient vectors.
 */
export function butterworthLowpass(
  order: number,
  cutoffNorm: number
): FilterCoefficients {
  if (order !== 1 && order !== 2) {
    throw new InvalidCoefficientError(
      `Butterworth supports orders 1 and 2; got ${order}`
    );
  }
  if (!Number.isFinite(cutoffNorm) || cutoffNorm <= 0.0 || cutoffNorm >= 0.5) {
    throw new InvalidCoefficientError(
      `cutoffNorm must be in (0.0, 0.5); got ${cutoffNorm}`
    );
  }

  const k = Math.tan(Math.PI * cutoffNorm);

  if (order === 1) {
    const alpha = k / (1.0 + k);
    return {
      b: new Float64Array([alpha, alpha]),
      a: new Float64Array([1.0, 2.0 * alpha - 1.0]),
    };
  }

  // order === 2
  const omega0 = 2.0 * Math.PI * cutoffNorm;
  const sinW0 = Math.sin(omega0);
  const cosW0 = Math.cos(omega0);
  const q = 1.0 / Math.SQRT2;
  const alpha = sinW0 / (2.0 * q);
  const b0 = (1.0 - cosW0) * 0.5;
  const b1 = 1.0 - cosW0;
  const b2 = b0;
  const a0 = 1.0 + alpha;
  const a1 = -2.0 * cosW0;
  const a2 = 1.0 - alpha;

  const invA0 = 1.0 / a0;
  return {
    b: new Float64Array([b0 * invA0, b1 * invA0, b2 * invA0]),
    a: new Float64Array([1.0, a1 * invA0, a2 * invA0]),
  };
}

/**
 * Design a Butterworth high-pass IIR filter (orders 1 and 2).
 * Returns { b, a } coefficient vectors.
 */
export function butterworthHighpass(
  order: number,
  cutoffNorm: number
): FilterCoefficients {
  if (order !== 1 && order !== 2) {
    throw new InvalidCoefficientError(
      `Butterworth supports orders 1 and 2; got ${order}`
    );
  }
  if (!Number.isFinite(cutoffNorm) || cutoffNorm <= 0.0 || cutoffNorm >= 0.5) {
    throw new InvalidCoefficientError(
      `cutoffNorm must be in (0.0, 0.5); got ${cutoffNorm}`
    );
  }

  const k = Math.tan(Math.PI * cutoffNorm);

  if (order === 1) {
    const alpha = 1.0 / (1.0 + k);
    return {
      b: new Float64Array([alpha, -alpha]),
      a: new Float64Array([1.0, (k - 1.0) / (k + 1.0)]),
    };
  }

  // order === 2
  const omega0 = 2.0 * Math.PI * cutoffNorm;
  const sinW0 = Math.sin(omega0);
  const cosW0 = Math.cos(omega0);
  const q = 1.0 / Math.SQRT2;
  const alpha = sinW0 / (2.0 * q);
  const b0 = (1.0 + cosW0) * 0.5;
  const b1 = -(1.0 + cosW0);
  const b2 = b0;
  const a0 = 1.0 + alpha;
  const a1 = -2.0 * cosW0;
  const a2 = 1.0 - alpha;

  const invA0 = 1.0 / a0;
  return {
    b: new Float64Array([b0 * invA0, b1 * invA0, b2 * invA0]),
    a: new Float64Array([1.0, a1 * invA0, a2 * invA0]),
  };
}

/**
 * Design a Butterworth band-pass IIR filter (orders 1 and 2).
 * Returns { b, a } coefficient vectors.
 */
export function butterworthBandpass(
  order: number,
  lowCutoffNorm: number,
  highCutoffNorm: number
): FilterCoefficients {
  if (order !== 1 && order !== 2) {
    throw new InvalidCoefficientError(
      `Butterworth bandpass supports orders 1 and 2; got ${order}`
    );
  }
  if (!Number.isFinite(lowCutoffNorm) || !Number.isFinite(highCutoffNorm)) {
    throw new InvalidCoefficientError("cutoff frequencies must be finite");
  }
  if (!(0.0 < lowCutoffNorm && lowCutoffNorm < highCutoffNorm && highCutoffNorm < 0.5)) {
    throw new InvalidCoefficientError(
      `cutoffs must satisfy 0.0 < low < high < 0.5; got low=${lowCutoffNorm}, high=${highCutoffNorm}`
    );
  }

  const omega1 = Math.tan(Math.PI * lowCutoffNorm);
  const omega2 = Math.tan(Math.PI * highCutoffNorm);
  const bBand = omega2 - omega1;
  const omega0Sq = omega1 * omega2;

  const a0 = 1.0 + bBand + omega0Sq;
  const invA0 = 1.0 / a0;
  const b0 = bBand * invA0;
  const b1 = 0.0;
  const b2 = -bBand * invA0;
  const a1 = (2.0 * (omega0Sq - 1.0)) * invA0;
  const a2 = (1.0 - bBand + omega0Sq) * invA0;

  if (order === 1) {
    return {
      b: new Float64Array([b0, b1, b2]),
      a: new Float64Array([1.0, a1, a2]),
    };
  }

  // order === 2: cascade two order-1 sections (convolving polynomials)
  // b_2nd = b * b
  const bRes = new Float64Array(5);
  bRes[0] = b0 * b0;
  bRes[1] = 2.0 * b0 * b1;
  bRes[2] = 2.0 * b0 * b2 + b1 * b1;
  bRes[3] = 2.0 * b1 * b2;
  bRes[4] = b2 * b2;

  // a_2nd = a * a
  const aRes = new Float64Array(5);
  aRes[0] = 1.0;
  aRes[1] = 2.0 * a1;
  aRes[2] = a1 * a1 + 2.0 * a2;
  aRes[3] = 2.0 * a1 * a2;
  aRes[4] = a2 * a2;

  return { b: bRes, a: aRes };
}

/**
 * Design a Chebyshev Type I low-pass IIR filter (orders 1 and 2).
 * Returns { b, a } coefficient vectors.
 */
export function chebyshev1Lowpass(
  order: number,
  rippleDb: number,
  cutoffNorm: number
): FilterCoefficients {
  if (order !== 1 && order !== 2) {
    throw new InvalidCoefficientError(
      `Chebyshev-I supports orders 1 and 2; got ${order}`
    );
  }
  if (!Number.isFinite(rippleDb) || rippleDb <= 0.0) {
    throw new InvalidCoefficientError(
      `rippleDb must be finite and > 0; got ${rippleDb}`
    );
  }
  if (!Number.isFinite(cutoffNorm) || cutoffNorm <= 0.0 || cutoffNorm >= 0.5) {
    throw new InvalidCoefficientError(
      `cutoffNorm must be in (0.0, 0.5); got ${cutoffNorm}`
    );
  }

  const k = Math.tan(Math.PI * cutoffNorm);
  const eps = Math.sqrt(Math.pow(10.0, rippleDb * 0.1) - 1.0);
  const mu =
    (1.0 / order) *
    Math.log(1.0 / eps + Math.sqrt(1.0 / (eps * eps) + 1.0));

  if (order === 1) {
    const sinhMu = Math.sinh(mu);
    const alpha = (k * sinhMu) / (1.0 + k * sinhMu);
    return {
      b: new Float64Array([alpha, alpha]),
      a: new Float64Array([1.0, 2.0 * alpha - 1.0]),
    };
  }

  // order === 2
  const u = Math.sinh(mu) / Math.SQRT2;
  const v = Math.cosh(mu) / Math.SQRT2;
  const sumSq = u * u + v * v;
  const na = sumSq / Math.sqrt(1.0 + eps * eps);

  const a0 = 1.0 + 2.0 * u * k + sumSq * k * k;
  const a1 = 2.0 * (sumSq * k * k - 1.0);
  const a2 = 1.0 - 2.0 * u * k + sumSq * k * k;

  const b0 = na * k * k;
  const b1 = 2.0 * b0;
  const b2 = b0;

  const invA0 = 1.0 / a0;
  return {
    b: new Float64Array([b0 * invA0, b1 * invA0, b2 * invA0]),
    a: new Float64Array([1.0, a1 * invA0, a2 * invA0]),
  };
}
