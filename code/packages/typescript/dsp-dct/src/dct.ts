import { bluesteinScalar, Direction, fftScalar, ifftScalar } from "@coding-adventures/dsp-fft";
import { MAX_DCT_LEN, MAX_SAMPLES } from "./constants.js";
import { EmptyInputError, InvalidInputError } from "./errors.js";
import { DctNorm, DctType, parseNorm, parseType } from "./types.js";

export function validateSignal(signal: Float64Array | ArrayLike<number>): number {
  const n = signal.length;
  if (n === 0) {
    throw new EmptyInputError("input signal must not be empty");
  }
  if (n > MAX_DCT_LEN) {
    throw new InvalidInputError(
      `input length ${n} exceeds MAX_DCT_LEN (${MAX_DCT_LEN})`
    );
  }
  if (n > MAX_SAMPLES) {
    throw new InvalidInputError(
      `input length ${n} exceeds MAX_SAMPLES (${MAX_SAMPLES})`
    );
  }

  for (let i = 0; i < n; i++) {
    const val = Number(signal[i]);
    if (!Number.isFinite(val)) {
      throw new InvalidInputError("input contains non-finite float (NaN or Inf)");
    }
  }
  return n;
}

export function validateDctInputs(
  signal: Float64Array | ArrayLike<number>,
  n: number,
  minLen: number = 1
): void {
  if (!Number.isFinite(n) || !Number.isInteger(n) || n < minLen) {
    throw new InvalidInputError(`input length must be an integer >= ${minLen}; got ${n}`);
  }
  if (n > MAX_DCT_LEN) {
    throw new InvalidInputError(`input length ${n} exceeds MAX_DCT_LEN (${MAX_DCT_LEN})`);
  }
  if (n > MAX_SAMPLES) {
    throw new InvalidInputError(`input length ${n} exceeds MAX_SAMPLES (${MAX_SAMPLES})`);
  }
  if (signal.length < n) {
    throw new InvalidInputError(`signal length ${signal.length} is less than specified n ${n}`);
  }
  for (let i = 0; i < n; i++) {
    const val = Number(signal[i]);
    if (!Number.isFinite(val)) {
      throw new InvalidInputError("input contains non-finite float (NaN or Inf)");
    }
  }
}

/**
 * Compute forward DCT-II in O(N log N) via Makhoul's reduction.
 */
export function dctIiViaFft(
  signal: Float64Array | ArrayLike<number>,
  n: number,
  norm: DctNorm
): Float64Array {
  validateDctInputs(signal, n, 1);

  // Step 1: Makhoul shuffle
  const halfEven = (n + 1) >> 1;
  const halfOdd = n >> 1;
  const y = new Float64Array(n);
  for (let m = 0; m < halfEven; m++) {
    y[m] = Number(signal[2 * m]);
  }
  for (let m = 0; m < halfOdd; m++) {
    y[n - 1 - m] = Number(signal[2 * m + 1]);
  }

  // Step 2: FFT of length N (interleaved complex with imag=0)
  const interleaved = new Float64Array(2 * n);
  for (let m = 0; m < n; m++) {
    interleaved[2 * m] = y[m];
    interleaved[2 * m + 1] = 0.0;
  }

  const isPow2 = (n & (n - 1)) === 0 && n >= 1;
  const spectrum = isPow2
    ? fftScalar(interleaved)
    : bluesteinScalar(interleaved, Direction.FORWARD);

  // Step 3: Twiddle multiply, take real part, double
  const twoN = 2.0 * n;
  const out = new Float64Array(n);
  for (let k = 0; k < n; k++) {
    const theta = (Math.PI * k) / twoN;
    const cosT = Math.cos(theta);
    const sinT = Math.sin(theta);
    const yRe = spectrum[2 * k];
    const yIm = spectrum[2 * k + 1];
    out[k] = 2.0 * (yRe * cosT + yIm * sinT);
  }

  // Step 4: Normalise
  if (norm === DctNorm.ORTHO) {
    const s0 = Math.sqrt(1.0 / (4.0 * n));
    const sk = Math.sqrt(1.0 / (2.0 * n));
    out[0] *= s0;
    for (let i = 1; i < n; i++) {
      out[i] *= sk;
    }
  }

  return out;
}

/**
 * Fast un-normalised DCT-III in O(N log N) via IFFT (inverse Makhoul).
 */
export function dctIiiViaFft(
  coeffs: Float64Array | ArrayLike<number>,
  n: number
): Float64Array {
  validateDctInputs(coeffs, n, 1);

  if (n === 1) {
    return new Float64Array([Number(coeffs[0])]);
  }

  const interleaved = new Float64Array(2 * n);
  interleaved[0] = Number(coeffs[0]);
  interleaved[1] = 0.0;

  const twoN = 2.0 * n;
  for (let k = 1; k < n; k++) {
    const theta = (Math.PI * k) / twoN;
    const cosT = Math.cos(theta);
    const sinT = Math.sin(theta);
    const reK = Number(coeffs[k]);
    const imK = -Number(coeffs[n - k]);
    interleaved[2 * k] = reK * cosT - imK * sinT;
    interleaved[2 * k + 1] = reK * sinT + imK * cosT;
  }

  const isPow2 = (n & (n - 1)) === 0 && n >= 1;
  const y = isPow2
    ? ifftScalar(interleaved)
    : bluesteinScalar(interleaved, Direction.INVERSE);

  const out = new Float64Array(n);
  const halfEven = (n + 1) >> 1;
  const halfOdd = n >> 1;
  for (let m = 0; m < halfEven; m++) {
    out[2 * m] = y[2 * m] * n;
  }
  for (let m = 0; m < halfOdd; m++) {
    out[2 * m + 1] = y[2 * (n - 1 - m)] * n;
  }

  return out;
}

/**
 * Iterative DCT-I (N >= 2).
 */
export function dctI(
  signal: Float64Array | ArrayLike<number>,
  n: number,
  norm: DctNorm
): Float64Array {
  validateDctInputs(signal, n, 2);

  const denom = n - 1;
  if (norm === DctNorm.ORTHO) {
    const invSqrt2 = 1.0 / Math.sqrt(2.0);
    const xScaled = new Float64Array(n);
    for (let i = 0; i < n; i++) {
      xScaled[i] = Number(signal[i]);
    }
    xScaled[0] *= invSqrt2;
    xScaled[n - 1] *= invSqrt2;

    const out = new Float64Array(n);
    const scale = Math.sqrt(2.0 / denom);
    for (let k = 0; k < n; k++) {
      const wK = k === 0 || k === n - 1 ? invSqrt2 : 1.0;
      let acc = 0.0;
      for (let m = 0; m < n; m++) {
        const theta = (Math.PI * k * m) / denom;
        acc += xScaled[m] * Math.cos(theta);
      }
      out[k] = scale * wK * acc;
    }
    return out;
  } else {
    const out = new Float64Array(n);
    const x0 = Number(signal[0]);
    const xLast = Number(signal[n - 1]);
    for (let k = 0; k < n; k++) {
      const sign = k % 2 === 0 ? 1.0 : -1.0;
      let acc = x0 + sign * xLast;
      for (let m = 1; m < n - 1; m++) {
        const theta = (Math.PI * k * m) / denom;
        acc += 2.0 * Number(signal[m]) * Math.cos(theta);
      }
      out[k] = acc;
    }
    return out;
  }
}

/**
 * Iterative DCT-IV.
 */
export function dctIv(
  signal: Float64Array | ArrayLike<number>,
  n: number,
  norm: DctNorm
): Float64Array {
  validateDctInputs(signal, n, 1);

  const fourN = 4.0 * n;
  const out = new Float64Array(n);
  for (let k = 0; k < n; k++) {
    let acc = 0.0;
    const kFactor = 2.0 * k + 1.0;
    for (let m = 0; m < n; m++) {
      const theta = (Math.PI * kFactor * (2.0 * m + 1.0)) / fourN;
      acc += Number(signal[m]) * Math.cos(theta);
    }
    out[k] = 2.0 * acc;
  }

  if (norm === DctNorm.ORTHO) {
    const scale = Math.sqrt(2.0 / n) * 0.5;
    for (let i = 0; i < n; i++) {
      out[i] *= scale;
    }
  }

  return out;
}

/**
 * Forward Discrete Cosine Transform.
 * Supports DCT types I, II, III, IV and norms None and Ortho.
 */
export function dct(
  signal: Float64Array | ArrayLike<number>,
  dctType?: DctType | number | string,
  norm?: DctNorm | string | number
): Float64Array {
  const n = validateSignal(signal);
  const t = parseType(dctType);
  const nm = parseNorm(norm);

  if (t === DctType.II) {
    return dctIiViaFft(signal, n, nm);
  }
  if (t === DctType.III) {
    const unnorm = dctIiiViaFft(signal, n);
    if (nm === DctNorm.ORTHO) {
      const s0 = Math.sqrt(1.0 / (4.0 * n));
      const sk = Math.sqrt(1.0 / (2.0 * n));
      unnorm[0] *= s0;
      for (let i = 1; i < n; i++) {
        unnorm[i] *= sk;
      }
    }
    return unnorm;
  }
  if (t === DctType.I) {
    return dctI(signal, n, nm);
  }
  if (t === DctType.IV) {
    return dctIv(signal, n, nm);
  }

  throw new InvalidInputError(`unsupported DCT type: ${t}`);
}

/**
 * Inverse Discrete Cosine Transform.
 * Passing DctType.III with the same norm inverts DctType.II.
 * Passing DctType.II with the same norm inverts DctType.III.
 */
export function idct(
  coeffs: Float64Array | ArrayLike<number>,
  dctType?: DctType | number | string,
  norm?: DctNorm | string | number
): Float64Array {
  const n = validateSignal(coeffs);
  const t = parseType(dctType !== undefined ? dctType : DctType.III);
  const nm = parseNorm(norm);

  if (t === DctType.III) {
    // Invert DCT-II
    let scaled: Float64Array;
    if (nm === DctNorm.ORTHO) {
      const s0 = Math.sqrt(4.0 * n);
      const sk = Math.sqrt(2.0 * n);
      scaled = new Float64Array(n);
      scaled[0] = Number(coeffs[0]) * s0;
      for (let i = 1; i < n; i++) {
        scaled[i] = Number(coeffs[i]) * sk;
      }
    } else {
      scaled = new Float64Array(n);
      for (let i = 0; i < n; i++) {
        scaled[i] = Number(coeffs[i]);
      }
    }

    const raw = dctIiiViaFft(scaled, n);
    const invScale = 1.0 / (2.0 * n);
    for (let i = 0; i < n; i++) {
      raw[i] *= invScale;
    }
    return raw;
  }

  if (t === DctType.II) {
    // Invert DCT-III
    let scaled: Float64Array;
    if (nm === DctNorm.ORTHO) {
      const s0 = Math.sqrt(4.0 * n);
      const sk = Math.sqrt(2.0 * n);
      scaled = new Float64Array(n);
      scaled[0] = Number(coeffs[0]) * s0;
      for (let i = 1; i < n; i++) {
        scaled[i] = Number(coeffs[i]) * sk;
      }
    } else {
      scaled = new Float64Array(n);
      for (let i = 0; i < n; i++) {
        scaled[i] = Number(coeffs[i]);
      }
    }

    const raw = dctIiViaFft(scaled, n, DctNorm.NONE);
    const invScale = 1.0 / (2.0 * n);
    for (let i = 0; i < n; i++) {
      raw[i] *= invScale;
    }
    return raw;
  }

  if (t === DctType.I) {
    // DCT-I is self-inverse up to 2(N-1) scale
    if (nm === DctNorm.ORTHO) {
      return dctI(coeffs, n, DctNorm.ORTHO);
    } else {
      const raw = dctI(coeffs, n, DctNorm.NONE);
      const scale = 1.0 / (2.0 * (n - 1));
      for (let i = 0; i < n; i++) {
        raw[i] *= scale;
      }
      return raw;
    }
  }

  if (t === DctType.IV) {
    // DCT-IV is self-inverse up to 2N scale
    if (nm === DctNorm.ORTHO) {
      return dctIv(coeffs, n, DctNorm.ORTHO);
    } else {
      const raw = dctIv(coeffs, n, DctNorm.NONE);
      const scale = 1.0 / (2.0 * n);
      for (let i = 0; i < n; i++) {
        raw[i] *= scale;
      }
      return raw;
    }
  }

  throw new InvalidInputError(`unsupported DCT type: ${t}`);
}
