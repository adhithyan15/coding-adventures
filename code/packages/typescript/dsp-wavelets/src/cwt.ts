import { MAX_SAMPLES, MAX_SCALES } from "./constants.js";
import {
  EmptySignalError,
  InvalidParamError,
  safeFormatValue,
  validateFloats,
  validateSequence,
} from "./errors.js";
import { WaveletFamily } from "./types.js";
import { parseWavelet, Wavelet } from "./wavelet.js";

/**
 * Compute the Continuous Wavelet Transform (CWT).
 *
 * Returns a flattened row-major [scales.length, signal.length] Float64Array.
 */
export function cwt(
  signal: ArrayLike<number>,
  scales: ArrayLike<number>,
  wavelet: Wavelet | string = "morlet"
): Float64Array {
  const n = validateSequence(signal, "signal", MAX_SAMPLES, EmptySignalError, InvalidParamError);
  const numScales = validateSequence(scales, "scales", MAX_SCALES, InvalidParamError, InvalidParamError);

  const totalLen = numScales * n;
  if (!Number.isSafeInteger(totalLen) || totalLen > MAX_SAMPLES) {
    throw new InvalidParamError("CWT output length exceeds MAX_SAMPLES");
  }

  for (let idx = 0; idx < numScales; idx++) {
    const raw: unknown = scales[idx];
    if (
      typeof raw !== "number" ||
      !Number.isFinite(raw) ||
      raw <= 0.0 ||
      !Number.isFinite(1.0 / raw) ||
      !Number.isFinite(1.0 / Math.sqrt(raw))
    ) {
      throw new InvalidParamError(
        `scales must be positive and have finite reciprocals; got ${safeFormatValue(raw)} at index ${idx}`
      );
    }
  }

  validateFloats(signal, "signal");

  const w = parseWavelet(wavelet);
  if (w.family !== WaveletFamily.MORLET && w.family !== WaveletFamily.RICKER) {
    throw new InvalidParamError(
      `CWT supports Morlet and Ricker wavelets only; got ${safeFormatValue(w.name)}`
    );
  }

  const piPowNegQuarter = Math.pow(Math.PI, -0.25);
  const out = new Float64Array(totalLen);

  if (w.family === WaveletFamily.MORLET) {
    // psi(t) = pi^(-1/4) * e^(i omega_0 t) * e^(-t^2 / 2), omega_0 = 6.0
    const omega0 = 6.0;
    let outIdx = 0;
    for (let sIdx = 0; sIdx < scales.length; sIdx++) {
      const s = Number(scales[sIdx]);
      const invSqrtS = 1.0 / Math.sqrt(s);
      const invS = 1.0 / s;
      const halfSupport = Math.min(Math.ceil(8.0 * s), n);

      for (let i = 0; i < n; i++) {
        let reAcc = 0.0;
        let imAcc = 0.0;
        const startK = Math.max(0, i - halfSupport);
        const endK = Math.min(n - 1, i + halfSupport);

        for (let k = startK; k <= endK; k++) {
          const t = (k - i) * invS;
          const envelope = piPowNegQuarter * Math.exp(-0.5 * t * t);
          const phase = omega0 * t;
          const rePsi = envelope * Math.cos(phase);
          const imPsi = -envelope * Math.sin(phase);
          const x = Number(signal[k]);
          reAcc += x * rePsi;
          imAcc += x * imPsi;
        }

        const mag = invSqrtS * Math.sqrt(reAcc * reAcc + imAcc * imAcc);
        out[outIdx++] = mag;
      }
    }
  } else if (w.family === WaveletFamily.RICKER) {
    // psi(t) = 2/sqrt(3) * pi^(-1/4) * (1 - t^2) * e^(-t^2 / 2)
    const c = (2.0 / Math.sqrt(3.0)) * piPowNegQuarter;
    let outIdx = 0;
    for (let sIdx = 0; sIdx < scales.length; sIdx++) {
      const s = Number(scales[sIdx]);
      const invSqrtS = 1.0 / Math.sqrt(s);
      const invS = 1.0 / s;
      const halfSupport = Math.min(Math.ceil(8.0 * s), n);

      for (let i = 0; i < n; i++) {
        let acc = 0.0;
        const startK = Math.max(0, i - halfSupport);
        const endK = Math.min(n - 1, i + halfSupport);

        for (let k = startK; k <= endK; k++) {
          const t = (k - i) * invS;
          const psi = c * (1.0 - t * t) * Math.exp(-0.5 * t * t);
          acc += Number(signal[k]) * psi;
        }

        out[outIdx++] = invSqrtS * acc;
      }
    }
  }

  return out;
}
