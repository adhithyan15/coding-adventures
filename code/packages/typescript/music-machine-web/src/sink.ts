/**
 * Web Audio sink playback integration for browser environments (MUS06).
 */

import {
  createAudioBufferFromPcm,
  playPcmBuffer,
  type AudioContextLike,
  type PlaybackReport,
  type WebAudioSinkOptions,
} from "@coding-adventures/web-audio-sink";

import { parsePortableScore } from "./portable_parser.js";
import { renderPortableScoreToPcm, renderScoreToPcm } from "./render.js";
import { parseScore } from "./score_parser.js";
import { PortableScore, TextScore } from "./types.js";

export {
  createAudioBufferFromPcm,
  playPcmBuffer,
  type AudioContextLike,
  type PlaybackReport,
  type WebAudioSinkOptions,
};

/**
 * Render a TextScore and play it through Web Audio.
 */
export async function playScore(
  score: TextScore,
  options: WebAudioSinkOptions = {},
  maxSampleCount?: number,
): Promise<PlaybackReport> {
  const rendered = renderScoreToPcm(score, maxSampleCount);
  return playPcmBuffer(
    {
      samples: rendered.pcmBuffer.samples,
      format: {
        sampleRateHz: rendered.pcmBuffer.pcmFormat.integerSampleRate(),
        channelCount: 1,
        bitDepth: 16,
      },
    },
    options,
  );
}

/**
 * Parse a beginner sheet music string, render it, and play it through Web Audio.
 */
export async function playScoreText(
  text: string,
  options: WebAudioSinkOptions = {},
  maxSampleCount?: number,
): Promise<PlaybackReport> {
  const score = parseScore(text);
  return playScore(score, options, maxSampleCount);
}

/**
 * Render a PortableScore and play it through Web Audio.
 */
export async function playPortableScore(
  score: PortableScore,
  options: WebAudioSinkOptions = {},
  maxSampleCount?: number,
): Promise<PlaybackReport> {
  const rendered = renderPortableScoreToPcm(score, maxSampleCount);
  return playPcmBuffer(
    {
      samples: rendered.pcmBuffer.samples,
      format: {
        sampleRateHz: rendered.pcmBuffer.pcmFormat.integerSampleRate(),
        channelCount: 1,
        bitDepth: 16,
      },
    },
    options,
  );
}

/**
 * Parse a portable score text, render it, and play it through Web Audio.
 */
export async function playPortableScoreText(
  text: string,
  options: WebAudioSinkOptions = {},
  maxSampleCount?: number,
): Promise<PlaybackReport> {
  const score = parsePortableScore(text);
  return playPortableScore(score, options, maxSampleCount);
}
