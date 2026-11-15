import { parseWaveletBoundary, sampleWithBoundary } from "./boundary.js";
import { MAX_SAMPLES, MAX_WAVELET_LEVELS } from "./constants.js";
import {
  EmptySignalError,
  InvalidCoefficientsError,
  InvalidParamError,
  SignalTooShortError,
  validateFloats,
} from "./errors.js";
import { WaveletBoundary } from "./types.js";
import {
  parseWavelet,
  requireOrthogonalWavelet,
  Wavelet,
} from "./wavelet.js";


/**
 * Compute one level forward DWT: filter and downsample by 2.
 */
export function filterAndDownsample(
  signal: ArrayLike<number>,
  h: ArrayLike<number>,
  g: ArrayLike<number>,
  boundary: WaveletBoundary
): [Float64Array, Float64Array] {
  const n = signal.length;
  const lenH = h.length;
  const lenG = g.length;
  const outLen = Math.floor((n + 1) / 2);

  const ca = new Float64Array(outLen);
  const cd = new Float64Array(outLen);

  for (let outIdx = 0; outIdx < outLen; outIdx++) {
    const k = 2 * outIdx + 1;
    let accH = 0.0;
    for (let i = 0; i < lenH; i++) {
      const s = sampleWithBoundary(signal, k - i, boundary);
      accH += Number(h[i]) * s;
    }
    let accG = 0.0;
    for (let i = 0; i < lenG; i++) {
      const s = sampleWithBoundary(signal, k - i, boundary);
      accG += Number(g[i]) * s;
    }
    ca[outIdx] = accH;
    cd[outIdx] = accG;
  }

  return [ca, cd];
}

/**
 * Generic synthesis for one level: upsample by 2 and filter with synthesis pair.
 */
export function synthesizeOneLevel(
  ca: ArrayLike<number>,
  cd: ArrayLike<number>,
  h: ArrayLike<number>,
  g: ArrayLike<number>,
  targetLen: number,
  boundary: WaveletBoundary
): Float64Array {
  const lenH = h.length;
  const lenG = g.length;
  const out = new Float64Array(targetLen);

  for (let n = 0; n < targetLen; n++) {
    let acc = 0.0;
    for (let i = 0; i < lenH; i++) {
      const num = n + i - 1;
      if (num % 2 === 0) {
        const m = Math.floor(num / 2);
        const caVal = sampleWithBoundary(ca, m, boundary);
        acc += Number(h[i]) * caVal;
      }
    }
    for (let i = 0; i < lenG; i++) {
      const num = n + i - 1;
      if (num % 2 === 0) {
        const m = Math.floor(num / 2);
        const cdVal = sampleWithBoundary(cd, m, boundary);
        acc += Number(g[i]) * cdVal;
      }
    }
    out[n] = acc;
  }

  return out;
}

/**
 * Compute single-level 1-D discrete wavelet transform: returns [cA, cD].
 */
export function dwt(
  signal: ArrayLike<number>,
  wavelet: Wavelet | string = "haar",
  boundary: WaveletBoundary | string = WaveletBoundary.SYMMETRIC
): [Float64Array, Float64Array] {
  if (signal.length === 0) {
    throw new EmptySignalError("signal must be non-empty");
  }
  if (signal.length > MAX_SAMPLES) {
    throw new InvalidParamError(`signal length exceeds MAX_SAMPLES ${MAX_SAMPLES}`);
  }
  const b = parseWaveletBoundary(boundary);
  validateFloats(signal, "signal");

  const w = parseWavelet(wavelet);
  requireOrthogonalWavelet(w, "DWT");

  return filterAndDownsample(signal, w.decLo, w.decHi, b);
}

/**
 * Compute single-level 1-D inverse discrete wavelet transform.
 */
export function idwt(
  ca: ArrayLike<number>,
  cd: ArrayLike<number>,
  wavelet: Wavelet | string = "haar",
  boundary: WaveletBoundary | string = WaveletBoundary.SYMMETRIC,
  targetLen?: number
): Float64Array {
  if (ca.length === 0 || cd.length === 0) {
    throw new EmptySignalError("cA and cD must be non-empty");
  }
  if (ca.length !== cd.length) {
    throw new InvalidCoefficientsError("cA and cD must have equal lengths");
  }
  if (ca.length > MAX_SAMPLES || cd.length > MAX_SAMPLES) {
    throw new InvalidParamError(`subband length exceeds MAX_SAMPLES ${MAX_SAMPLES}`);
  }

  const tLen = targetLen !== undefined ? targetLen : 2 * ca.length;
  if (!Number.isSafeInteger(tLen) || tLen <= 0 || tLen > MAX_SAMPLES) {
    throw new InvalidParamError(
      `targetLen exceeds MAX_SAMPLES ${MAX_SAMPLES} or is non-positive`
    );
  }

  const b = parseWaveletBoundary(boundary);
  validateFloats(ca, "ca", InvalidCoefficientsError);
  validateFloats(cd, "cd", InvalidCoefficientsError);

  const w = parseWavelet(wavelet);
  requireOrthogonalWavelet(w, "DWT");

  return synthesizeOneLevel(ca, cd, w.recLo, w.recHi, tLen, b);
}

/**
 * Compute lengths [len_0, len_1, ..., len_levels] where len_0 = initialLen.
 */
export function forwardLevelLengths(initialLen: number, levels: number): number[] {
  if (!Number.isSafeInteger(initialLen) || initialLen <= 0 || initialLen > MAX_SAMPLES) {
    throw new InvalidParamError(`initialLen must be positive and <= MAX_SAMPLES; got ${initialLen}`);
  }
  if (!Number.isSafeInteger(levels) || levels <= 0 || levels > MAX_WAVELET_LEVELS) {
    throw new InvalidParamError(`levels must be > 0 and <= MAX_WAVELET_LEVELS; got ${levels}`);
  }
  const lens = [initialLen];
  let curr = initialLen;
  for (let i = 0; i < levels; i++) {
    curr = Math.floor((curr + 1) / 2);
    lens.push(curr);
  }
  return lens;
}

/**
 * Forward 1-D DWT returning flattened [cA_J | cD_J | ... | cD_1] layout.
 */
export function dwt1d(
  signal: ArrayLike<number>,
  wavelet: Wavelet | string = "haar",
  levels: number = 1,
  boundary: WaveletBoundary | string = WaveletBoundary.SYMMETRIC
): Float64Array {
  if (signal.length === 0) {
    throw new EmptySignalError("signal must be non-empty");
  }
  if (!Number.isSafeInteger(levels) || levels <= 0) {
    throw new InvalidParamError(`levels must be > 0; got ${levels}`);
  }
  if (levels > MAX_WAVELET_LEVELS) {
    throw new InvalidParamError(
      `levels ${levels} exceeds MAX_WAVELET_LEVELS ${MAX_WAVELET_LEVELS}`
    );
  }
  if (signal.length > MAX_SAMPLES) {
    throw new InvalidParamError(`signal length exceeds MAX_SAMPLES ${MAX_SAMPLES}`);
  }
  const b = parseWaveletBoundary(boundary);
  validateFloats(signal, "signal");

  const w = parseWavelet(wavelet);
  requireOrthogonalWavelet(w, "DWT");

  if (signal.length < (1 << (levels - 1))) {
    throw new SignalTooShortError(
      `signal length ${signal.length} too short for ${levels} levels`
    );
  }

  let current: Float64Array = Float64Array.from(signal);
  const detailsReversed: Float64Array[] = [];

  for (let l = 0; l < levels; l++) {
    const [ca, cd] = filterAndDownsample(current, w.decLo, w.decHi, b);
    detailsReversed.push(cd);
    current = ca;
  }

  let totalLen = current.length;
  for (const d of detailsReversed) {
    totalLen += d.length;
  }

  const out = new Float64Array(totalLen);
  out.set(current, 0);

  let offset = current.length;
  for (let i = detailsReversed.length - 1; i >= 0; i--) {
    const d = detailsReversed[i];
    out.set(d, offset);
    offset += d.length;
  }

  return out;
}

/**
 * Inverse 1-D DWT reversing dwt1d via synthesis filter bank.
 */
export function idwt1d(
  coeffs: ArrayLike<number>,
  wavelet: Wavelet | string = "haar",
  levels: number = 1,
  boundary: WaveletBoundary | string = WaveletBoundary.SYMMETRIC,
  outputLength: number = 0
): Float64Array {
  if (coeffs.length === 0) {
    throw new EmptySignalError("coeffs must be non-empty");
  }
  if (!Number.isSafeInteger(outputLength) || outputLength <= 0) {
    throw new InvalidParamError("outputLength must be a positive safe integer");
  }
  if (outputLength > MAX_SAMPLES || coeffs.length > MAX_SAMPLES) {
    throw new InvalidParamError("length exceeds MAX_SAMPLES");
  }
  if (!Number.isSafeInteger(levels) || levels <= 0 || levels > MAX_WAVELET_LEVELS) {
    throw new InvalidParamError(`invalid levels ${levels}`);
  }
  const b = parseWaveletBoundary(boundary);
  validateFloats(coeffs, "coeffs", InvalidCoefficientsError);

  const w = parseWavelet(wavelet);
  requireOrthogonalWavelet(w, "DWT");

  const levelLens = forwardLevelLengths(outputLength, levels);
  const coarsestCaLen = levelLens[levels];

  let expectedTotal = coarsestCaLen;
  for (let j = 1; j <= levels; j++) {
    expectedTotal += levelLens[j];
  }

  if (coeffs.length !== expectedTotal) {
    throw new InvalidCoefficientsError(
      `coeffs length ${coeffs.length} != expected ${expectedTotal} for outputLength=${outputLength}, levels=${levels}`
    );
  }

  let offset = 0;
  let current: Float64Array = Float64Array.from(
    Array.prototype.slice.call(coeffs, offset, offset + coarsestCaLen)
  );
  offset += coarsestCaLen;

  for (let j = levels; j >= 1; j--) {
    const cdLen = levelLens[j];
    const cd = Float64Array.from(
      Array.prototype.slice.call(coeffs, offset, offset + cdLen)
    );
    offset += cdLen;
    const targetLen = levelLens[j - 1];
    current = synthesizeOneLevel(current, cd, w.recLo, w.recHi, targetLen, b);
  }

  return current;
}

/**
 * Forward 2-D DWT quad-tree decomposition for row-major flattened images.
 */
export function dwt2d(
  image: ArrayLike<number>,
  nRows: number,
  nCols: number,
  wavelet: Wavelet | string = "haar",
  levels: number = 1,
  boundary: WaveletBoundary | string = WaveletBoundary.PERIODIC
): Float64Array {
  if (image.length === 0) {
    throw new EmptySignalError("image must be non-empty");
  }
  if (
    !Number.isSafeInteger(nRows) ||
    !Number.isSafeInteger(nCols) ||
    nRows <= 0 ||
    nCols <= 0
  ) {
    throw new InvalidParamError("image dimensions must be positive safe integers");
  }
  const totalPixels = nRows * nCols;
  if (totalPixels > MAX_SAMPLES || image.length !== totalPixels) {
    throw new InvalidParamError("image size mismatch or exceeds MAX_SAMPLES");
  }
  if (!Number.isSafeInteger(levels) || levels <= 0 || levels > MAX_WAVELET_LEVELS) {
    throw new InvalidParamError(`invalid levels ${levels}`);
  }
  const b = parseWaveletBoundary(boundary);
  validateFloats(image, "image");

  const w = parseWavelet(wavelet);
  requireOrthogonalWavelet(w, "DWT");

  let current = Float64Array.from(image);
  let curRows = nRows;
  let curCols = nCols;
  const detailsReversed: Array<[Float64Array, Float64Array, Float64Array]> = [];

  for (let l = 0; l < levels; l++) {
    const halfRows = Math.floor((curRows + 1) / 2);
    const halfCols = Math.floor((curCols + 1) / 2);

    // Row pass
    const lRows = new Float64Array(curRows * halfCols);
    const hRows = new Float64Array(curRows * halfCols);
    const rowBuf = new Float64Array(curCols);

    for (let r = 0; r < curRows; r++) {
      const rowStart = r * curCols;
      for (let c = 0; c < curCols; c++) {
        rowBuf[c] = current[rowStart + c];
      }
      const [ca, cd] = filterAndDownsample(rowBuf, w.decLo, w.decHi, b);
      lRows.set(ca, r * halfCols);
      hRows.set(cd, r * halfCols);
    }

    // Column pass
    const ll = new Float64Array(halfRows * halfCols);
    const hl = new Float64Array(halfRows * halfCols);
    const lh = new Float64Array(halfRows * halfCols);
    const hh = new Float64Array(halfRows * halfCols);

    const colBuf = new Float64Array(curRows);
    for (let c = 0; c < halfCols; c++) {
      // L columns -> LL and LH
      for (let r = 0; r < curRows; r++) {
        colBuf[r] = lRows[r * halfCols + c];
      }
      const [caL, cdL] = filterAndDownsample(colBuf, w.decLo, w.decHi, b);
      for (let r = 0; r < halfRows; r++) {
        ll[r * halfCols + c] = caL[r];
        lh[r * halfCols + c] = cdL[r];
      }

      // H columns -> HL and HH
      for (let r = 0; r < curRows; r++) {
        colBuf[r] = hRows[r * halfCols + c];
      }
      const [caH, cdH] = filterAndDownsample(colBuf, w.decLo, w.decHi, b);
      for (let r = 0; r < halfRows; r++) {
        hl[r * halfCols + c] = caH[r];
        hh[r * halfCols + c] = cdH[r];
      }
    }

    detailsReversed.push([hl, lh, hh]);
    curRows = halfRows;
    curCols = halfCols;
    current = ll;
  }

  let totalOutLen = current.length;
  for (const [hl, lh, hh] of detailsReversed) {
    totalOutLen += hl.length + lh.length + hh.length;
  }

  const out = new Float64Array(totalOutLen);
  out.set(current, 0);

  let offset = current.length;
  for (let i = detailsReversed.length - 1; i >= 0; i--) {
    const [hl, lh, hh] = detailsReversed[i];
    out.set(hl, offset);
    offset += hl.length;
    out.set(lh, offset);
    offset += lh.length;
    out.set(hh, offset);
    offset += hh.length;
  }

  return out;
}

/**
 * Inverse 2-D DWT quad-tree reconstruction.
 */
export function idwt2d(
  coeffs: ArrayLike<number>,
  nRows: number,
  nCols: number,
  wavelet: Wavelet | string = "haar",
  levels: number = 1,
  boundary: WaveletBoundary | string = WaveletBoundary.PERIODIC
): Float64Array {
  if (coeffs.length === 0) {
    throw new EmptySignalError("coeffs must be non-empty");
  }
  if (
    !Number.isSafeInteger(nRows) ||
    !Number.isSafeInteger(nCols) ||
    nRows <= 0 ||
    nCols <= 0
  ) {
    throw new InvalidParamError("image dimensions must be positive safe integers");
  }
  const totalPixels = nRows * nCols;
  if (totalPixels > MAX_SAMPLES) {
    throw new InvalidParamError("image dimensions exceed MAX_SAMPLES");
  }
  if (!Number.isSafeInteger(levels) || levels <= 0 || levels > MAX_WAVELET_LEVELS) {
    throw new InvalidParamError(`invalid levels ${levels}`);
  }
  const b = parseWaveletBoundary(boundary);
  validateFloats(coeffs, "coeffs", InvalidCoefficientsError);

  const w = parseWavelet(wavelet);
  requireOrthogonalWavelet(w, "DWT");

  const levelDims: Array<[number, number]> = [[nRows, nCols]];
  let rDim = nRows;
  let cDim = nCols;
  for (let l = 0; l < levels; l++) {
    rDim = Math.floor((rDim + 1) / 2);
    cDim = Math.floor((cDim + 1) / 2);
    levelDims.push([rDim, cDim]);
  }

  const [llRows, llCols] = levelDims[levels];
  let expectedTotal = llRows * llCols;
  for (let l = 1; l <= levels; l++) {
    const [rj, cj] = levelDims[l];
    expectedTotal += 3 * rj * cj;
  }

  if (coeffs.length !== expectedTotal) {
    throw new InvalidCoefficientsError(
      `coeffs length ${coeffs.length} != expected ${expectedTotal}`
    );
  }

  let offset = 0;
  let ll = Float64Array.from(
    Array.prototype.slice.call(coeffs, offset, offset + llRows * llCols)
  );
  offset += llRows * llCols;

  for (let j = levels; j >= 1; j--) {
    const [rj, cj] = levelDims[j];
    const bandLen = rj * cj;
    const hl = Float64Array.from(
      Array.prototype.slice.call(coeffs, offset, offset + bandLen)
    );
    offset += bandLen;
    const lh = Float64Array.from(
      Array.prototype.slice.call(coeffs, offset, offset + bandLen)
    );
    offset += bandLen;
    const hh = Float64Array.from(
      Array.prototype.slice.call(coeffs, offset, offset + bandLen)
    );
    offset += bandLen;

    const [targetRows, targetCols] = levelDims[j - 1];

    // Inverse column pass
    const lRows = new Float64Array(targetRows * cj);
    const hRows = new Float64Array(targetRows * cj);
    const caBuf = new Float64Array(rj);
    const cdBuf = new Float64Array(rj);

    for (let c = 0; c < cj; c++) {
      for (let r = 0; r < rj; r++) {
        caBuf[r] = ll[r * cj + c];
        cdBuf[r] = lh[r * cj + c];
      }
      const colL = synthesizeOneLevel(caBuf, cdBuf, w.recLo, w.recHi, targetRows, b);
      for (let r = 0; r < targetRows; r++) {
        lRows[r * cj + c] = colL[r];
      }

      for (let r = 0; r < rj; r++) {
        caBuf[r] = hl[r * cj + c];
        cdBuf[r] = hh[r * cj + c];
      }
      const colH = synthesizeOneLevel(caBuf, cdBuf, w.recLo, w.recHi, targetRows, b);
      for (let r = 0; r < targetRows; r++) {
        hRows[r * cj + c] = colH[r];
      }
    }

    // Inverse row pass
    const reconstructedBand = new Float64Array(targetRows * targetCols);
    const rowCa = new Float64Array(cj);
    const rowCd = new Float64Array(cj);

    for (let r = 0; r < targetRows; r++) {
      for (let c = 0; c < cj; c++) {
        rowCa[c] = lRows[r * cj + c];
        rowCd[c] = hRows[r * cj + c];
      }
      const rowRec = synthesizeOneLevel(rowCa, rowCd, w.recLo, w.recHi, targetCols, b);
      reconstructedBand.set(rowRec, r * targetCols);
    }

    ll = reconstructedBand;
  }

  return ll;
}

/** Snake-case aliases */
export const dwt_1d = dwt1d;
export const idwt_1d = idwt1d;
export const dwt_2d = dwt2d;
export const idwt_2d = idwt2d;
export const forward_level_lengths = forwardLevelLengths;
