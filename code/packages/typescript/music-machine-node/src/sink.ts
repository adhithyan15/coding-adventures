/**
 * Audio device sink playback integration for Node.js environments (MUS02).
 */

import {
  AudioSink,
  AudioSinkError,
  NoopAudioSink,
  PcmFormat,
  PcmPlaybackBuffer,
  PlaybackReport,
} from "@coding-adventures/audio-device-sink";

import { parsePortableScore } from "./portable_parser.js";
import { renderPortableScoreToPcm, renderScoreToPcm } from "./render.js";
import { parseScore } from "./score_parser.js";
import { PortableScore, TextScore } from "./types.js";

export {
  AudioSink,
  AudioSinkError,
  NoopAudioSink,
  PcmFormat,
  PcmPlaybackBuffer,
  PlaybackReport,
};

/**
 * Render a TextScore and play it through an AudioSink (defaults to NoopAudioSink).
 */
export function playScore(
  score: TextScore,
  sink: AudioSink = new NoopAudioSink(),
  maxSampleCount?: number,
): PlaybackReport {
  const rendered = renderScoreToPcm(score, maxSampleCount);
  const sampleRate = rendered.pcmBuffer.pcmFormat.integerSampleRate();
  const format = new PcmFormat(sampleRate, 1, 16);
  const buffer = new PcmPlaybackBuffer(rendered.pcmBuffer.samples, format);
  return sink.playBlocking(buffer);
}

/**
 * Parse a beginner sheet music string, render it, and play it through an AudioSink.
 */
export function playScoreText(
  text: string,
  sink: AudioSink = new NoopAudioSink(),
  maxSampleCount?: number,
): PlaybackReport {
  const score = parseScore(text);
  return playScore(score, sink, maxSampleCount);
}

/**
 * Render a PortableScore and play it through an AudioSink.
 */
export function playPortableScore(
  score: PortableScore,
  sink: AudioSink = new NoopAudioSink(),
  maxSampleCount?: number,
): PlaybackReport {
  const rendered = renderPortableScoreToPcm(score, maxSampleCount);
  const sampleRate = rendered.pcmBuffer.pcmFormat.integerSampleRate();
  const format = new PcmFormat(sampleRate, 1, 16);
  const buffer = new PcmPlaybackBuffer(rendered.pcmBuffer.samples, format);
  return sink.playBlocking(buffer);
}

/**
 * Parse a portable score text, render it, and play it through an AudioSink.
 */
export function playPortableScoreText(
  text: string,
  sink: AudioSink = new NoopAudioSink(),
  maxSampleCount?: number,
): PlaybackReport {
  const score = parsePortableScore(text);
  return playPortableScore(score, sink, maxSampleCount);
}
