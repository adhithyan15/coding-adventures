import { validateDctInputs, validateSignal } from "./dct.js";
import { InvalidInputError } from "./errors.js";
import { DctNorm, parseNorm } from "./types.js";

export enum DstType {
  I = 1,
  II = 2,
  III = 3,
  IV = 4,
}

const DST_TYPE_MAP = new Map<string, DstType>([
  ["1", DstType.I],
  ["i", DstType.I],
  ["2", DstType.II],
  ["ii", DstType.II],
  ["3", DstType.III],
  ["iii", DstType.III],
  ["4", DstType.IV],
  ["iv", DstType.IV],
]);

export function parseDstType(dstType?: DstType | string | number): DstType {
  if (dstType === undefined) {
    return DstType.II;
  }
  if (typeof dstType === "number") {
    if (Number.isInteger(dstType) && dstType >= 1 && dstType <= 4) return dstType as DstType;
  }
  if (typeof dstType === "string") {
    const found = DST_TYPE_MAP.get(dstType.trim().toLowerCase());
    if (found !== undefined) return found;
  }
  throw new InvalidInputError(`unknown DST type: ${dstType}`);
}

export function dstI(
  signal: Float64Array | ArrayLike<number>,
  n: number,
  norm: DctNorm
): Float64Array {
  validateDctInputs(signal, n, 2);
  const denom = n + 1.0;
  const out = new Float64Array(n);
  for (let k = 0; k < n; k++) {
    let acc = 0.0;
    const kFactor = k + 1.0;
    for (let m = 0; m < n; m++) {
      const theta = (Math.PI * kFactor * (m + 1.0)) / denom;
      acc += Number(signal[m]) * Math.sin(theta);
    }
    out[k] = 2.0 * acc;
  }

  if (norm === DctNorm.ORTHO) {
    const scale = Math.sqrt(2.0 / denom) * 0.5;
    for (let i = 0; i < n; i++) {
      out[i] *= scale;
    }
  }
  return out;
}

export function dstIi(
  signal: Float64Array | ArrayLike<number>,
  n: number,
  norm: DctNorm
): Float64Array {
  validateDctInputs(signal, n, 1);
  const twoN = 2.0 * n;
  const out = new Float64Array(n);
  for (let k = 0; k < n; k++) {
    let acc = 0.0;
    const kFactor = k + 1.0;
    for (let m = 0; m < n; m++) {
      const theta = (Math.PI * kFactor * (2.0 * m + 1.0)) / twoN;
      acc += Number(signal[m]) * Math.sin(theta);
    }
    out[k] = 2.0 * acc;
  }

  if (norm === DctNorm.ORTHO) {
    const sk = Math.sqrt(1.0 / (2.0 * n));
    const sLast = Math.sqrt(1.0 / (4.0 * n));
    for (let i = 0; i < n - 1; i++) {
      out[i] *= sk;
    }
    out[n - 1] *= sLast;
  }
  return out;
}

export function dstIii(
  signal: Float64Array | ArrayLike<number>,
  n: number,
  norm: DctNorm
): Float64Array {
  validateDctInputs(signal, n, 1);
  const twoN = 2.0 * n;
  const out = new Float64Array(n);
  const xLast = Number(signal[n - 1]);

  for (let k = 0; k < n; k++) {
    const sign = k % 2 === 0 ? 1.0 : -1.0;
    let acc = sign * xLast;
    const kFactor = 2.0 * k + 1.0;
    for (let m = 0; m < n - 1; m++) {
      const theta = (Math.PI * kFactor * (m + 1.0)) / twoN;
      acc += 2.0 * Number(signal[m]) * Math.sin(theta);
    }
    out[k] = acc;
  }

  if (norm === DctNorm.ORTHO) {
    const sk = Math.sqrt(1.0 / (2.0 * n));
    const s0 = Math.sqrt(1.0 / (4.0 * n));
    out[0] *= s0;
    for (let i = 1; i < n; i++) {
      out[i] *= sk;
    }
  }
  return out;
}

export function dstIv(
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
      acc += Number(signal[m]) * Math.sin(theta);
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

export function dst(
  signal: Float64Array | ArrayLike<number>,
  dstType?: DstType | number | string,
  norm?: DctNorm | string | number
): Float64Array {
  const n = validateSignal(signal);
  const t = parseDstType(dstType);
  const nm = parseNorm(norm);

  if (t === DstType.I) {
    if (n < 2) {
      throw new InvalidInputError("DST-I requires N >= 2");
    }
    return dstI(signal, n, nm);
  }
  if (t === DstType.II) return dstIi(signal, n, nm);
  if (t === DstType.III) return dstIii(signal, n, nm);
  if (t === DstType.IV) return dstIv(signal, n, nm);

  throw new InvalidInputError(`unsupported DST type: ${t}`);
}

export function idst(
  coeffs: Float64Array | ArrayLike<number>,
  dstType?: DstType | number | string,
  norm?: DctNorm | string | number
): Float64Array {
  const n = validateSignal(coeffs);
  const t = parseDstType(dstType !== undefined ? dstType : DstType.III);
  const nm = parseNorm(norm);

  if (t === DstType.III) {
    let scaled: Float64Array;
    if (nm === DctNorm.ORTHO) {
      const sk = Math.sqrt(2.0 * n);
      const sLast = Math.sqrt(4.0 * n);
      scaled = new Float64Array(n);
      for (let i = 0; i < n - 1; i++) {
        scaled[i] = Number(coeffs[i]) * sk;
      }
      scaled[n - 1] = Number(coeffs[n - 1]) * sLast;
    } else {
      scaled = new Float64Array(n);
      for (let i = 0; i < n; i++) {
        scaled[i] = Number(coeffs[i]);
      }
    }
    const raw = dstIii(scaled, n, DctNorm.NONE);
    const invScale = 1.0 / (2.0 * n);
    for (let i = 0; i < n; i++) {
      raw[i] *= invScale;
    }
    return raw;
  }

  if (t === DstType.II) {
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
    const raw = dstIi(scaled, n, DctNorm.NONE);
    const invScale = 1.0 / (2.0 * n);
    for (let i = 0; i < n; i++) {
      raw[i] *= invScale;
    }
    return raw;
  }

  if (t === DstType.I) {
    if (n < 2) {
      throw new InvalidInputError("DST-I requires N >= 2");
    }
    if (nm === DctNorm.ORTHO) {
      return dstI(coeffs, n, DctNorm.ORTHO);
    } else {
      const raw = dstI(coeffs, n, DctNorm.NONE);
      const scale = 1.0 / (2.0 * (n + 1));
      for (let i = 0; i < n; i++) {
        raw[i] *= scale;
      }
      return raw;
    }
  }

  if (t === DstType.IV) {
    if (nm === DctNorm.ORTHO) {
      return dstIv(coeffs, n, DctNorm.ORTHO);
    } else {
      const raw = dstIv(coeffs, n, DctNorm.NONE);
      const scale = 1.0 / (2.0 * n);
      for (let i = 0; i < n; i++) {
        raw[i] *= scale;
      }
      return raw;
    }
  }

  throw new InvalidInputError(`unsupported DST type: ${t}`);
}
