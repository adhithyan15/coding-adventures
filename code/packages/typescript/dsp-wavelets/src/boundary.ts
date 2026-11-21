import { MAX_SAMPLES } from "./constants.js";
import { InvalidParamError, safeFormatValue, validateSequence } from "./errors.js";
import { WaveletBoundary } from "./types.js";

const MAX_BOUNDARY_STR_LEN = 64;

/**
 * Parse and normalize wavelet boundary mode from enum or case-insensitive string.
 */
export function parseWaveletBoundary(
  boundary: WaveletBoundary | string
): WaveletBoundary {
  if (typeof boundary !== "string" || boundary.length > MAX_BOUNDARY_STR_LEN) {
    throw new InvalidParamError(`unsupported boundary mode: ${safeFormatValue(boundary)}`);
  }
  const norm = boundary.trim().toLowerCase();
  switch (norm) {
    case "zero":
      return WaveletBoundary.ZERO;
    case "replicate":
      return WaveletBoundary.REPLICATE;
    case "reflect":
      return WaveletBoundary.REFLECT;
    case "symmetric":
    case "sym":
      return WaveletBoundary.SYMMETRIC;
    case "periodic":
    case "wrap":
    case "circular":
      return WaveletBoundary.PERIODIC;
    default:
      throw new InvalidParamError(`unsupported boundary mode: ${safeFormatValue(boundary)}`);
  }
}

/**
 * Sample signal[idx] with the requested boundary extension mode.
 */
export function sampleWithBoundary(
  signal: ArrayLike<number>,
  idx: number,
  boundary: WaveletBoundary | string
): number {
  const n = validateSequence(signal, "signal", MAX_SAMPLES, null, InvalidParamError);
  if (!Number.isSafeInteger(idx)) {
    throw new InvalidParamError(`index must be a safe integer; got ${safeFormatValue(idx)}`);
  }
  const bMode = parseWaveletBoundary(boundary);
  if (n === 0) {
    return 0.0;
  }
  let mapped: number;
  if (idx >= 0 && idx < n) {
    mapped = idx;
  } else {
    switch (bMode) {
      case WaveletBoundary.ZERO:
        return 0.0;
      case WaveletBoundary.REPLICATE:
        mapped = Math.max(0, Math.min(n - 1, idx));
        break;
      case WaveletBoundary.REFLECT: {
        if (n === 1) {
          mapped = 0;
        } else {
          const period = 2 * (n - 1);
          let m = ((idx % period) + period) % period;
          if (m >= n) {
            m = period - m;
          }
          mapped = m;
        }
        break;
      }
      case WaveletBoundary.SYMMETRIC: {
        const period = 2 * n;
        let m = ((idx % period) + period) % period;
        if (m >= n) {
          m = 2 * n - 1 - m;
        }
        mapped = m;
        break;
      }
      case WaveletBoundary.PERIODIC:
        mapped = ((idx % n) + n) % n;
        break;
    }
  }
  const val: unknown = signal[mapped];
  if (typeof val !== "number" || !Number.isFinite(val)) {
    throw new InvalidParamError(
      `non-finite float in signal at index ${mapped}: ${safeFormatValue(val)}`
    );
  }
  return val;
}
