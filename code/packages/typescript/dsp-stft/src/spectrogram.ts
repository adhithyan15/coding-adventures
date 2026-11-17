import { InvalidParamError } from "./errors.js";
import { stft } from "./stft.js";
import { WindowType } from "./types.js";

/**
 * Compute the power spectrogram |STFT|^2 for signal.
 *
 * Returns a flattened row-major [numFrames, nFft / 2 + 1] Float64Array.
 */
export function powerSpectrogram(
  signal: ArrayLike<number>,
  nFft: number,
  hopLength: number,
  window: WindowType | string = WindowType.HANN
): Float64Array {
  const complexSpec = stft(signal, nFft, hopLength, window);
  const bins = Math.floor(nFft / 2) + 1;
  const frameFloats = bins * 2;
  const numFrames = complexSpec.length / frameFloats;

  const out = new Float64Array(numFrames * bins);
  for (let m = 0; m < numFrames; m++) {
    const inOffset = m * frameFloats;
    const outOffset = m * bins;
    for (let k = 0; k < bins; k++) {
      const re = complexSpec[inOffset + 2 * k];
      const im = complexSpec[inOffset + 2 * k + 1];
      out[outOffset + k] = re * re + im * im;
    }
  }
  return out;
}

/**
 * Compute the power spectrogram |STFT|^2 (matching Rust dsp_stft::spectrogram).
 */
export function spectrogram(
  signal: ArrayLike<number>,
  nFft: number,
  hopLength: number,
  window: WindowType | string = WindowType.HANN
): Float64Array {
  return powerSpectrogram(signal, nFft, hopLength, window);
}

/**
 * Compute log-power spectrogram: ln(|STFT|^2 + eps).
 */
export function logSpectrogram(
  signal: ArrayLike<number>,
  nFft: number,
  hopLength: number,
  window: WindowType | string = WindowType.HANN,
  eps: number = 1e-10
): Float64Array {
  if (!Number.isFinite(eps) || eps <= 0.0) {
    throw new InvalidParamError(`eps must be positive and finite; got ${eps}`);
  }
  const power = powerSpectrogram(signal, nFft, hopLength, window);
  const out = new Float64Array(power.length);
  for (let i = 0; i < power.length; i++) {
    out[i] = Math.log(power[i] + eps);
  }
  return out;
}
