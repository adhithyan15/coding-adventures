import { irfftScalar } from "@coding-adventures/dsp-fft";
import { MAX_FFT_LEN, MAX_FRAMES, MAX_SAMPLES } from "./constants.js";
import {
  FftError,
  InvalidParamError,
  InvalidSpectrogramError,
  validateFloats,
} from "./errors.js";
import { WindowType } from "./types.js";
import { buildWindow } from "./windows.js";

/**
 * Inverse Short-Time Fourier Transform (ISTFT) via overlap-add (OLA).
 *
 * Reconstructs a time-domain signal from its STFT spectrogram.
 */
export function istft(
  spectrogram: ArrayLike<number>,
  nFft: number,
  hopLength: number,
  window: WindowType | string = WindowType.HANN,
  outputLength?: number
): Float64Array {
  const specLen = spectrogram.length;
  if (specLen === 0) {
    throw new InvalidSpectrogramError("spectrogram is empty");
  }
  if (!Number.isSafeInteger(nFft) || nFft <= 0) {
    throw new InvalidParamError(`nFft must be a positive integer; got ${nFft}`);
  }
  if (!Number.isSafeInteger(hopLength) || hopLength <= 0) {
    throw new InvalidParamError(
      `hopLength must be a positive integer; got ${hopLength}`
    );
  }
  if (outputLength !== undefined) {
    if (!Number.isSafeInteger(outputLength) || outputLength <= 0) {
      throw new InvalidParamError(
        `outputLength must be a positive integer; got ${outputLength}`
      );
    }
    if (outputLength > MAX_SAMPLES) {
      throw new InvalidParamError(
        `outputLength ${outputLength} exceeds MAX_SAMPLES ${MAX_SAMPLES}`
      );
    }
  }
  if (nFft > MAX_FFT_LEN) {
    throw new InvalidParamError(
      `nFft ${nFft} exceeds MAX_FFT_LEN ${MAX_FFT_LEN}`
    );
  }

  const bins = Math.floor(nFft / 2) + 1;
  const frameFloats = bins * 2;

  if (specLen > MAX_SAMPLES) {
    throw new InvalidParamError(
      `spectrogram length ${specLen} exceeds MAX_SAMPLES ${MAX_SAMPLES}`
    );
  }

  if (specLen % frameFloats !== 0) {
    throw new InvalidSpectrogramError(
      `spectrogram length ${specLen} is not a multiple of bins * 2 (${frameFloats})`
    );
  }

  const numFrames = specLen / frameFloats;
  if (numFrames > MAX_FRAMES) {
    throw new InvalidParamError(
      `numFrames ${numFrames} exceeds MAX_FRAMES ${MAX_FRAMES}`
    );
  }

  const outLen = outputLength ?? ((numFrames - 1) * hopLength + nFft);
  if (outLen > MAX_SAMPLES || !Number.isSafeInteger(outLen)) {
    throw new InvalidParamError(
      `outputLength ${outLen} exceeds MAX_SAMPLES ${MAX_SAMPLES}`
    );
  }

  validateFloats(spectrogram, "spectrogram", InvalidSpectrogramError);

  const win = buildWindow(window, nFft);
  const out = new Float64Array(outLen);
  const norm = new Float64Array(outLen);
  const frameSlice = new Float64Array(frameFloats);

  for (let m = 0; m < numFrames; m++) {
    const frameStartGlobal = m * hopLength;
    const frameStartFloats = m * frameFloats;
    for (let k = 0; k < frameFloats; k++) {
      frameSlice[k] = Number(spectrogram[frameStartFloats + k]);
    }
    let frame: Float64Array;
    try {
      frame = irfftScalar(frameSlice, nFft);
    } catch (err) {
      throw new FftError(`IFFT failure in frame ${m}: ${String(err)}`);
    }

    for (let k = 0; k < nFft; k++) {
      const n = frameStartGlobal + k;
      if (n >= outLen) {
        break;
      }
      const w = win[k];
      out[n] += frame[k] * w;
      norm[n] += w * w;
    }
  }

  for (let n = 0; n < outLen; n++) {
    if (norm[n] > 1e-10) {
      out[n] /= norm[n];
    }
  }

  return out;
}
