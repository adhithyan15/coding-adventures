import { MAX_FFT_LEN, MAX_KAISER_BETA } from "./constants.js";
import { InvalidInputError, safeFormatValue } from "./errors.js";

const MAX_ENUM_STRING_LENGTH = 64;
const MAX_BESSEL_ITERATIONS = 1024;

export enum WindowType {
  RECTANGULAR = "rectangular",
  HANN = "hann",
  HAMMING = "hamming",
  BLACKMAN = "blackman",
  KAISER = "kaiser",
}

const WINDOW_TYPE_MAP = new Map<string, WindowType>([
  ["rectangular", WindowType.RECTANGULAR],
  ["hann", WindowType.HANN],
  ["hanning", WindowType.HANN],
  ["hamming", WindowType.HAMMING],
  ["blackman", WindowType.BLACKMAN],
  ["kaiser", WindowType.KAISER],
]);

function parseWindowType(type: WindowType | string): WindowType {
  if (typeof type !== "string" || type.length > MAX_ENUM_STRING_LENGTH) {
    throw new InvalidInputError(`unknown window type: ${safeFormatValue(type)}`);
  }
  const lowered = type.trim().toLowerCase();
  const found = WINDOW_TYPE_MAP.get(lowered);
  if (!found) {
    throw new InvalidInputError(`unknown window type: ${safeFormatValue(type)}`);
  }
  return found;
}

/**
 * Compute zero-th order modified Bessel function of the first kind I_0(x).
 * Strictly iterative series summation (CWE-674).
 */
export function besselI0(x: number): number {
  if (typeof x !== "number" || !Number.isFinite(x)) {
    throw new InvalidInputError("besselI0 input must be finite");
  }
  if (Math.abs(x) > MAX_KAISER_BETA) {
    throw new InvalidInputError(
      `besselI0 input magnitude exceeds MAX_KAISER_BETA (${MAX_KAISER_BETA})`
    );
  }
  let sum = 1.0;
  let term = 1.0;
  const halfX = x * 0.5;
  for (let k = 1; k < MAX_BESSEL_ITERATIONS; k++) {
    const factor = halfX / k;
    term *= factor * factor;
    sum += term;
    if (!Number.isFinite(sum)) {
      throw new InvalidInputError("besselI0 overflowed to non-finite value");
    }
    if (term < sum * 1e-16) {
      break;
    }
  }
  return sum;
}

export interface WindowOptions {
  beta?: number;
  symmetric?: boolean;
}

export function buildWindow(
  type: WindowType | string,
  n: number,
  options?: WindowOptions
): Float64Array {
  if (typeof n !== "number" || !Number.isSafeInteger(n) || n < 1) {
    throw new InvalidInputError("window length must be a positive integer");
  }
  if (n > MAX_FFT_LEN) {
    throw new InvalidInputError(
      `window length ${n} exceeds MAX_FFT_LEN (${MAX_FFT_LEN})`
    );
  }

  const wType = parseWindowType(type);
  if (wType === WindowType.KAISER && options?.beta !== undefined) {
    if (
      typeof options.beta !== "number" ||
      !Number.isFinite(options.beta) ||
      options.beta < 0 ||
      options.beta > MAX_KAISER_BETA
    ) {
      throw new InvalidInputError(
        `Kaiser beta must be a finite number in [0, ${MAX_KAISER_BETA}]`
      );
    }
  }

  if (n === 1) {
    return new Float64Array([1.0]);
  }

  const symmetric = options?.symmetric !== false;
  const m = symmetric ? n - 1 : n;
  const out = new Float64Array(n);
  const twoPi = 2.0 * Math.PI;
  const fourPi = 4.0 * Math.PI;

  switch (wType) {
    case WindowType.RECTANGULAR: {
      out.fill(1.0);
      break;
    }
    case WindowType.HANN: {
      for (let i = 0; i < n; i++) {
        out[i] = 0.5 * (1.0 - Math.cos((twoPi * i) / m));
      }
      break;
    }
    case WindowType.HAMMING: {
      for (let i = 0; i < n; i++) {
        out[i] = 0.54 - 0.46 * Math.cos((twoPi * i) / m);
      }
      break;
    }
    case WindowType.BLACKMAN: {
      for (let i = 0; i < n; i++) {
        out[i] =
          0.42 -
          0.5 * Math.cos((twoPi * i) / m) +
          0.08 * Math.cos((fourPi * i) / m);
      }
      break;
    }
    case WindowType.KAISER: {
      const beta = options?.beta ?? 5.0;
      if (!Number.isFinite(beta) || beta < 0) {
        throw new InvalidInputError("Kaiser beta must be a non-negative finite number");
      }
      const denom = besselI0(beta);
      const alpha = (n - 1) * 0.5;
      for (let i = 0; i < n; i++) {
        const r = (i - alpha) / alpha;
        const rad = Math.max(0.0, 1.0 - r * r);
        out[i] = besselI0(beta * Math.sqrt(rad)) / denom;
      }
      break;
    }
  }

  return out;
}

export function hann(n: number, symmetric: boolean = true): Float64Array {
  return buildWindow(WindowType.HANN, n, { symmetric });
}

export function hamming(n: number, symmetric: boolean = true): Float64Array {
  return buildWindow(WindowType.HAMMING, n, { symmetric });
}

export function blackman(n: number, symmetric: boolean = true): Float64Array {
  return buildWindow(WindowType.BLACKMAN, n, { symmetric });
}

export function kaiser(n: number, beta: number = 5.0): Float64Array {
  return buildWindow(WindowType.KAISER, n, { beta });
}

export function rectangular(n: number): Float64Array {
  return buildWindow(WindowType.RECTANGULAR, n);
}
