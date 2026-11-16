import { rfftScalar } from "@coding-adventures/dsp-fft";
import { MAX_FFT_LEN, MAX_FRAMES, MAX_SAMPLES } from "./constants.js";
import {
  EmptySignalError,
  FftError,
  InvalidParamError,
  SignalTooShortError,
  validateFloats,
} from "./errors.js";
import { WindowType } from "./types.js";
import { buildWindow } from "./windows.js";

/**
 * Compute the short-time Fourier transform (STFT) of signal using strict-mode framing.
 *
 * Returns flattened row-major [numFrames, nFft / 2 + 1, 2] Float64Array holding
 * interleaved-complex spectrogram [re, im].
 *
 * numFrames = 1 + Math.floor((signal.length - nFft) / hopLength).
 */
export function stft(
  signal: ArrayLike<number>,
  nFft: number,
  hopLength: number,
  window: WindowType | string = WindowType.HANN
): Float64Array {
  const n = signal.length;
  if (n === 0) {
    throw new EmptySignalError("STFT signal must be non-empty");
  }
  if (!Number.isSafeInteger(nFft) || nFft <= 0) {
    throw new InvalidParamError(`nFft must be a positive integer; got ${nFft}`);
  }
  if (!Number.isSafeInteger(hopLength) || hopLength <= 0) {
    throw new InvalidParamError(
      `hopLength must be a positive integer; got ${hopLength}`
    );
  }
  if (nFft > MAX_FFT_LEN) {
    throw new InvalidParamError(
      `nFft ${nFft} exceeds MAX_FFT_LEN ${MAX_FFT_LEN}`
    );
  }
  if (n > MAX_SAMPLES) {
    throw new InvalidParamError(
      `signal length ${n} exceeds MAX_SAMPLES ${MAX_SAMPLES}`
    );
  }
  if (n < nFft) {
    throw new SignalTooShortError(
      `signal length ${n} < nFft ${nFft} (strict-mode framing)`
    );
  }

  const numFrames = 1 + Math.floor((n - nFft) / hopLength);
  if (numFrames > MAX_FRAMES) {
    throw new InvalidParamError(
      `numFrames ${numFrames} exceeds MAX_FRAMES ${MAX_FRAMES}`
    );
  }

  const bins = Math.floor(nFft / 2) + 1;
  const frameFloats = bins * 2;
  const outLen = numFrames * frameFloats;

  if (outLen > MAX_SAMPLES || !Number.isSafeInteger(outLen)) {
    throw new InvalidParamError(
      `output length ${outLen} exceeds MAX_SAMPLES ${MAX_SAMPLES}`
    );
  }

  validateFloats(signal, "signal");

  const win = buildWindow(window, nFft);
  const out = new Float64Array(outLen);
  const frameBuf = new Float64Array(nFft);

  for (let m = 0; m < numFrames; m++) {
    const frameStart = m * hopLength;
    for (let k = 0; k < nFft; k++) {
      frameBuf[k] = Number(signal[frameStart + k]) * win[k];
    }
    let spec: Float64Array;
    try {
      spec = rfftScalar(frameBuf);
    } catch (err) {
      throw new FftError(`FFT failure in frame ${m}: ${String(err)}`);
    }
    out.set(spec, m * frameFloats);
  }

  return out;
}
