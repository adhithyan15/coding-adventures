export {
  createAudioBufferFromPcm,
  pcmSamplesToFloat32,
  playPcmBuffer,
  validatePcmBuffer,
} from "./web-audio-sink.js";

export type {
  AudioBufferLike,
  AudioBufferSourceNodeLike,
  AudioContextLike,
  GainNodeLike,
  PcmFormat,
  PcmPlaybackBuffer,
  PlaybackReport,
  WebAudioSinkOptions,
} from "./web-audio-sink.js";
