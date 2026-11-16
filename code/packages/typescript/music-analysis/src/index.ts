/**
 * @coding-adventures/music-analysis (MUS08)
 *
 * Music Understanding, Audio Feature Extraction, Pitch Detection, and Sample-to-Score Transcription.
 */

export {
  buildHannWindow,
  checkedFrameBounds,
  computeSpectralFluxStep,
  parabolicInterpolation,
  rfftMagnitudes,
  validateAudioInputs,
  validateFraming,
  validatePcmSourceBounds,
} from "./dsp_utils.js";

export {
  rmsEnergy,
  rmsEnergyFramed,
  rms_energy,
  rms_energy_framed,
  spectralCentroid,
  spectralFlux,
  spectralFluxSequence,
  spectralRolloff,
  spectral_centroid,
  spectral_flux,
  spectral_flux_sequence,
  spectral_rolloff,
  zeroCrossingRate,
  zeroCrossingRateFramed,
  zero_crossing_rate,
  zero_crossing_rate_framed,
} from "./features.js";

export {
  PITCH_NAMES,
  frequencyToNote,
  frequencyToNoteA440,
  frequencyToNoteOpt,
  frequency_to_note,
  frequency_to_note_a440,
  frequency_to_note_opt,
} from "./frequency.js";

export {
  extractHarmonicProfile,
  extractHarmonicProfileSimple,
  extract_harmonic_profile,
  extract_harmonic_profile_simple,
} from "./harmonic.js";

export {
  detectOnsets,
  detectOnsetsSamples,
  detectOnsetsSimple,
  detect_onsets,
  detect_onsets_samples,
  detect_onsets_simple,
  estimateTempo,
  estimate_tempo,
} from "./onset.js";

export {
  autocorrelationPitch,
  autocorrelation_pitch,
  hpsPitch,
  hpsPitchSimple,
  hps_pitch,
  hps_pitch_simple,
  yinPitch,
  yinPitchSimple,
  yinPitchTrack,
  yin_pitch,
  yin_pitch_simple,
  yin_pitch_track,
} from "./pitch.js";

export {
  transcribePcmBuffer,
  transcribePcmToPortableScore,
  transcribeSamples,
  transcribe_pcm_buffer,
  transcribe_pcm_to_portable_score,
  transcribe_samples,
  type TranscriptionOptions,
} from "./transcription.js";

export {
  AnalysisError,
  BudgetExceededError,
  DEFAULT_A4_HZ,
  InvalidInputError,
  MAX_BPM,
  MAX_FFT_LEN,
  MAX_FRAMES,
  MAX_ONSETS,
  MAX_PARTIALS,
  MAX_SAMPLE_RATE_HZ,
  MAX_SAMPLES,
  MIN_BPM,
  MIN_SAMPLE_RATE_HZ,
  MusicAnalysisError,
  validateFiniteFloat,
  validateNonNegativeInt,
  validatePositiveFloat,
  validatePositiveInt,
  validateSamples,
  validateSpectrum,
  type NotePitch,
} from "./types.js";
