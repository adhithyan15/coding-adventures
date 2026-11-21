import { MAX_SAMPLES } from "./constants.js";
import {
  InvalidParamError,
  safeFormatValue,
  validateFloats,
  validateSequence,
} from "./errors.js";
import { WaveletFamily } from "./types.js";

const MAX_WAVELET_NAME_LEN = 64;

const VALID_WAVELET_FAMILIES = new Set<WaveletFamily>([
  WaveletFamily.HAAR,
  WaveletFamily.DAUBECHIES,
  WaveletFamily.SYMLETS,
  WaveletFamily.COIFLETS,
  WaveletFamily.BIORTHOGONAL,
  WaveletFamily.MORLET,
  WaveletFamily.RICKER,
]);

const DB_FILTER_TABLES = new Map<number, number[]>();

export const FRAC_1_SQRT_2 = 1.0 / Math.SQRT2;
export const SQRT_2 = Math.SQRT2;

export const DB2_DEC_LO = [
  -0.12940952,
   0.22414386,
   0.8365163,
   0.4829629,
];

export const DB4_DEC_LO = [
  -0.010597402,
   0.03288301,
   0.030841382,
  -0.18703482,
  -0.02798377,
   0.6308808,
   0.71484655,
   0.23037781,
];

export const DB6_DEC_LO = [
   0.11154074,
   0.4946239,
   0.7511339,
   0.31525034,
  -0.2262647,
  -0.12976687,
   0.097501606,
   0.027522866,
  -0.03158204,
   0.0005538422,
   0.0047772573,
  -0.0010773011,
];

export const DB8_DEC_LO = [
   0.05441584,
   0.3128716,
   0.67563075,
   0.5853547,
  -0.015829105,
  -0.28401554,
   0.00047248456,
   0.12874743,
  -0.0173693,
  -0.044088256,
   0.0139810275,
   0.008746094,
  -0.004870353,
  -0.00039174038,
   0.0006754494,
  -0.00011747678,
];

export const SYM4_DEC_LO = [
  -0.075765714,
  -0.029635528,
   0.49761868,
   0.8037388,
   0.2978578,
  -0.099219546,
  -0.012603967,
   0.0322231,
];

export const COIF1_DEC_LO = [
  -0.015655728,
  -0.07273262,
   0.38486484,
   0.852572,
   0.33789766,
  -0.07273262,
];

export const BIOR2_2_DEC_LO = [
  -0.125 * SQRT_2,
   0.25 * SQRT_2,
   0.75 * SQRT_2,
   0.25 * SQRT_2,
  -0.125 * SQRT_2,
];

export const BIOR2_2_REC_LO = [
  0.5 * FRAC_1_SQRT_2,
  1.0 * FRAC_1_SQRT_2,
  0.5 * FRAC_1_SQRT_2,
];

/**
 * Derive QMF analysis highpass g from lowpass h: g[i] = (-1)^i * h[L - 1 - i].
 */
export function qmfHighpass(h: ArrayLike<number>): Float64Array {
  const len = validateSequence(h, "h", MAX_SAMPLES, null, InvalidParamError);
  validateFloats(h, "h", InvalidParamError);
  const out = new Float64Array(len);
  for (let i = 0; i < len; i++) {
    const rev = h[len - 1 - i];
    out[i] = i % 2 === 0 ? rev : -rev;
  }
  return out;
}

/**
 * Wavelet filter bank definition.
 */
export class Wavelet {
  readonly name: string;
  readonly family: WaveletFamily;
  readonly decLo: Float64Array;
  readonly decHi: Float64Array;
  readonly recLo: Float64Array;
  readonly recHi: Float64Array;

  constructor(
    name: string,
    family: WaveletFamily,
    decLo: ArrayLike<number>,
    decHi: ArrayLike<number>,
    recLo?: ArrayLike<number>,
    recHi?: ArrayLike<number>
  ) {
    if (typeof name !== "string" || name.length === 0 || name.length > MAX_WAVELET_NAME_LEN) {
      throw new InvalidParamError(
        `wavelet name must be a non-empty string of length <= ${MAX_WAVELET_NAME_LEN}; got ${safeFormatValue(name)}`
      );
    }
    if (!VALID_WAVELET_FAMILIES.has(family)) {
      throw new InvalidParamError(`unsupported wavelet family: ${safeFormatValue(family)}`);
    }
    const decLoLen = validateSequence(decLo, "decLo", MAX_SAMPLES, null, InvalidParamError);
    validateFloats(decLo, "decLo", InvalidParamError);
    const decHiLen = validateSequence(decHi, "decHi", MAX_SAMPLES, null, InvalidParamError);
    validateFloats(decHi, "decHi", InvalidParamError);
    const isDiscrete = family !== WaveletFamily.MORLET && family !== WaveletFamily.RICKER;
    if (isDiscrete && (decLoLen === 0 || decHiLen === 0)) {
      throw new InvalidParamError("discrete wavelet filter banks decLo and decHi must be non-empty");
    }
    if (recLo !== undefined) {
      const recLoLen = validateSequence(recLo, "recLo", MAX_SAMPLES, null, InvalidParamError);
      validateFloats(recLo, "recLo", InvalidParamError);
      if (isDiscrete && recLoLen === 0) {
        throw new InvalidParamError("discrete wavelet filter bank recLo must be non-empty");
      }
    }
    if (recHi !== undefined) {
      const recHiLen = validateSequence(recHi, "recHi", MAX_SAMPLES, null, InvalidParamError);
      validateFloats(recHi, "recHi", InvalidParamError);
      if (isDiscrete && recHiLen === 0) {
        throw new InvalidParamError("discrete wavelet filter bank recHi must be non-empty");
      }
    }
    this.name = name;
    this.family = family;
    this.decLo = Float64Array.from(decLo);
    this.decHi = Float64Array.from(decHi);
    this.recLo = recLo ? Float64Array.from(recLo) : Float64Array.from(decLo);
    this.recHi = recHi ? Float64Array.from(recHi) : Float64Array.from(decHi);
  }

  get filterLength(): number {
    return this.decLo.length;
  }

  static haar(): Wavelet {
    const h = [FRAC_1_SQRT_2, FRAC_1_SQRT_2];
    const g = [FRAC_1_SQRT_2, -FRAC_1_SQRT_2];
    return new Wavelet("haar", WaveletFamily.HAAR, h, g);
  }

  static db(n: number): Wavelet {
    if (!Number.isSafeInteger(n)) {
      throw new InvalidParamError(
        `unsupported Daubechies order ${safeFormatValue(n)}; choose from [1, 2, 4, 6, 8]`
      );
    }
    if (n === 1) {
      return Wavelet.haar();
    }
    if (DB_FILTER_TABLES.size === 0) {
      DB_FILTER_TABLES.set(2, DB2_DEC_LO);
      DB_FILTER_TABLES.set(4, DB4_DEC_LO);
      DB_FILTER_TABLES.set(6, DB6_DEC_LO);
      DB_FILTER_TABLES.set(8, DB8_DEC_LO);
    }
    const h = DB_FILTER_TABLES.get(n);
    if (!h) {
      throw new InvalidParamError(
        `unsupported Daubechies order ${n}; choose from [1, 2, 4, 6, 8]`
      );
    }
    const g = qmfHighpass(h);
    return new Wavelet(`db${n}`, WaveletFamily.DAUBECHIES, h, g);
  }

  static sym(n: number): Wavelet {
    if (n === 2) {
      return Wavelet.db(2);
    }
    if (n === 4) {
      const h = SYM4_DEC_LO;
      const g = qmfHighpass(h);
      return new Wavelet("sym4", WaveletFamily.SYMLETS, h, g);
    }
    throw new InvalidParamError(
      `unsupported Symlet order ${safeFormatValue(n)}; choose from [2, 4]`
    );
  }

  static coif(n: number): Wavelet {
    if (n === 1) {
      const h = COIF1_DEC_LO;
      const g = qmfHighpass(h);
      return new Wavelet("coif1", WaveletFamily.COIFLETS, h, g);
    }
    throw new InvalidParamError(`unsupported Coiflet order ${safeFormatValue(n)}; choose from [1]`);
  }

  static bior(decomp: number = 2, recon: number = 2): Wavelet {
    if ((decomp === 2 && recon === 2) || (decomp === 5 && recon === 3)) {
      const h = BIOR2_2_DEC_LO;
      const g = qmfHighpass(BIOR2_2_REC_LO);
      const recH = BIOR2_2_REC_LO;
      const recG = qmfHighpass(h);
      return new Wavelet("bior2.2", WaveletFamily.BIORTHOGONAL, h, g, recH, recG);
    }
    throw new InvalidParamError(
      `unsupported Biorthogonal pair (${safeFormatValue(decomp)}, ${safeFormatValue(recon)})`
    );
  }

  static morlet(): Wavelet {
    return new Wavelet("morlet", WaveletFamily.MORLET, [], []);
  }

  static ricker(): Wavelet {
    return new Wavelet("ricker", WaveletFamily.RICKER, [], []);
  }

  static mexicanHat(): Wavelet {
    return Wavelet.ricker();
  }
}

function parseExactDigits(str: string): number | null {
  if (str.length === 0) return null;
  for (let i = 0; i < str.length; i++) {
    const code = str.charCodeAt(i);
    if (code < 48 || code > 57) {
      return null;
    }
  }
  const num = Number(str);
  return Number.isSafeInteger(num) ? num : null;
}

/**
 * Enforce orthogonal wavelet requirement across transforms (dwt, wavedec, swt).
 */
export function requireOrthogonalWavelet(w: Wavelet, opName = "DWT"): void {
  if (
    w.family !== WaveletFamily.HAAR &&
    w.family !== WaveletFamily.DAUBECHIES &&
    w.family !== WaveletFamily.SYMLETS &&
    w.family !== WaveletFamily.COIFLETS
  ) {
    throw new InvalidParamError(
      `${opName} supports orthogonal wavelets only; got ${safeFormatValue(w.name)}`
    );
  }
}

export const requireOrthogonal = requireOrthogonalWavelet;

/**
 * Resolve a Wavelet instance from an existing object or string name.
 */
export function parseWavelet(w: Wavelet | string): Wavelet {
  if (w instanceof Wavelet) {
    return w;
  }
  if (typeof w === "string" && w.length <= MAX_WAVELET_NAME_LEN) {
    const s = w.trim().toLowerCase();
    if (s === "haar") {
      return Wavelet.haar();
    }
    if (s.startsWith("db")) {
      const num = parseExactDigits(s.slice(2));
      if (num !== null) {
        return Wavelet.db(num);
      }
    } else if (s.startsWith("sym")) {
      const num = parseExactDigits(s.slice(3));
      if (num !== null) {
        return Wavelet.sym(num);
      }
    } else if (s.startsWith("coif")) {
      const num = parseExactDigits(s.slice(4));
      if (num !== null) {
        return Wavelet.coif(num);
      }
    } else if (
      s === "bior2.2" ||
      s === "bior22" ||
      s === "bior5.3" ||
      s === "bior53"
    ) {
      return Wavelet.bior(2, 2);
    } else if (s === "morlet") {
      return Wavelet.morlet();
    } else if (s === "ricker" || s === "mexican_hat" || s === "mexicanhat") {
      return Wavelet.ricker();
    }
  }
  throw new InvalidParamError(`unsupported wavelet identifier: ${safeFormatValue(w)}`);
}
