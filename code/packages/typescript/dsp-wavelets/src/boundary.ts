import { InvalidParamError } from "./errors.js";
import { WaveletBoundary } from "./types.js";

/**
 * Parse and normalize wavelet boundary mode from enum or case-insensitive string.
 */
export function parseWaveletBoundary(
  boundary: WaveletBoundary | string
): WaveletBoundary {
  if (typeof boundary !== "string") {
    throw new InvalidParamError(`unsupported boundary mode: ${String(boundary)}`);
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
      throw new InvalidParamError(`unsupported boundary mode: ${boundary}`);
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
  const n = signal.length;
  if (n === 0) {
    return 0.0;
  }
  if (idx >= 0 && idx < n) {
    return Number(signal[idx]);
  }

  const bMode = parseWaveletBoundary(boundary);
  switch (bMode) {
    case WaveletBoundary.ZERO:
      return 0.0;
    case WaveletBoundary.REPLICATE: {
      const clamped = Math.max(0, Math.min(n - 1, idx));
      return Number(signal[clamped]);
    }
    case WaveletBoundary.REFLECT: {
      if (n === 1) {
        return Number(signal[0]);
      }
      const period = 2 * (n - 1);
      let m = ((idx % period) + period) % period;
      if (m >= n) {
        m = period - m;
      }
      return Number(signal[m]);
    }
    case WaveletBoundary.SYMMETRIC: {
      const period = 2 * n;
      let m = ((idx % period) + period) % period;
      if (m >= n) {
        m = 2 * n - 1 - m;
      }
      return Number(signal[m]);
    }
    case WaveletBoundary.PERIODIC: {
      const m = ((idx % n) + n) % n;
      return Number(signal[m]);
    }
  }
}
