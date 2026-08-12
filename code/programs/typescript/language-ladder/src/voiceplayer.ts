// ---------------------------------------------------------------------------
// voiceplayer.ts — running a voice script through the browser's own speech.
//
// The script (`voicescript.ts`) says what to do; this does it. Kept apart from
// the script builder because one is pure and exhaustively testable while this
// one is a thin, untestable shell around two browser APIs — and mixing them
// would make the testable half untestable.
//
// WHAT IT USES. `speechSynthesis`, which every current browser has and which
// needs no permission. Recognition — the learner's half of the loop — is NOT
// here. `SpeechRecognition` requires a microphone permission, is prefixed in
// most engines, and is absent in some; wiring it blind is how you ship a
// feature that works on the machine it was written on. A `respond` step
// therefore waits its authored budget and moves on, which is exactly what a
// cassette course did, and is genuinely useful while somebody is driving.
//
// WHY A CANCEL TOKEN rather than a boolean. `speechSynthesis.cancel()` fires
// pending `onend` handlers, so a naive player resumes the next step after being
// stopped. Each run gets a token; a handler that finds the token stale does
// nothing.
// ---------------------------------------------------------------------------

import type { VoiceStep } from "./voicescript.ts";

/** The two browser bits this needs, narrowed so a test can supply fakes. */
export interface SpeechPort {
  speak(text: string, onEnd: () => void): void;
  cancel(): void;
}

/** What the caller is told as the script runs, so the page can follow along. */
export interface VoiceEvents {
  onStep?(index: number, step: VoiceStep): void;
  onDone?(): void;
}

export interface VoiceHandle {
  stop(): void;
  /** True until the script finishes or is stopped. */
  running(): boolean;
}

/**
 * Play a script. Returns a handle; call `stop()` to abandon it.
 *
 * `wait` uses the supplied timer so a test can run a ten-minute lesson
 * instantly. Nothing here reads a clock directly.
 */
export function playVoiceScript(
  steps: readonly VoiceStep[],
  speech: SpeechPort,
  events: VoiceEvents = {},
  setTimer: (fn: () => void, ms: number) => unknown = setTimeout,
): VoiceHandle {
  const token = {};
  let current: typeof token | null = token;

  const advance = (index: number): void => {
    if (current !== token) return;
    if (index >= steps.length) {
      current = null;
      events.onDone?.();
      return;
    }
    const step = steps[index]!;
    events.onStep?.(index, step);
    switch (step.kind) {
      case "speak":
        speech.speak(step.text, () => advance(index + 1));
        return;
      case "wait":
      case "respond":
        setTimer(() => advance(index + 1), step.seconds * 1000);
        return;
    }
  };

  advance(0);

  return {
    stop(): void {
      if (current !== token) return;
      current = null;
      speech.cancel();
    },
    running: () => current === token,
  };
}

/** The real browser speech port, or null where the API is absent. */
export function browserSpeech(language: string): SpeechPort | null {
  const synth = typeof speechSynthesis === "undefined" ? null : speechSynthesis;
  if (!synth) return null;
  return {
    speak(text, onEnd) {
      const utterance = new SpeechSynthesisUtterance(text);
      // A BCP-47 tag the engine can act on. Wrong-language TTS is worse than
      // none: it mispronounces the very word the lesson is teaching.
      utterance.lang = speechTagFor(language);
      utterance.onend = () => onEnd();
      // An engine that errors (no voice for the tag, muted tab) must not hang
      // the script — treat it as a finished utterance and keep going.
      utterance.onerror = () => onEnd();
      synth.speak(utterance);
    },
    cancel() {
      synth.cancel();
    },
  };
}

/** Track name → BCP-47 tag. Unknown tracks get the browser default. */
export function speechTagFor(language: string): string {
  const tags: Record<string, string> = {
    spanish: "es-ES",
    french: "fr-FR",
    italian: "it-IT",
    portuguese: "pt-PT",
    german: "de-DE",
    russian: "ru-RU",
    arabic: "ar",
    persian: "fa-IR",
    urdu: "ur",
    hindi: "hi-IN",
    bengali: "bn",
    gujarati: "gu",
    marathi: "mr",
    punjabi: "pa",
    sanskrit: "sa",
    tamil: "ta",
    telugu: "te",
    kannada: "kn",
    malayalam: "ml",
    chinese: "zh-CN",
    japanese: "ja-JP",
    latin: "it-IT",
  };
  return tags[language] ?? "";
}
