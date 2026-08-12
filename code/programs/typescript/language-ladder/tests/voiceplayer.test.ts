import { describe, expect, it } from "vitest";
import type { VoiceStep } from "../src/voicescript.ts";
import { type SpeechPort, playVoiceScript, speechTagFor } from "../src/voiceplayer.ts";

/** A speech port that records what it was asked to say and finishes instantly. */
function fakeSpeech(): SpeechPort & { said: string[]; cancels: number } {
  const said: string[] = [];
  let cancels = 0;
  return {
    said,
    get cancels() {
      return cancels;
    },
    speak(text, onEnd) {
      said.push(text);
      onEnd();
    },
    cancel() {
      cancels += 1;
    },
  };
}

/** A timer that fires immediately, so a ten-minute lesson runs in no time. */
const instant = (fn: () => void) => fn();

describe("playing a script", () => {
  const script: VoiceStep[] = [
    { kind: "speak", text: "hola" },
    { kind: "wait", seconds: 2 },
    { kind: "respond", instruction: "Say it.", seconds: 8 },
    { kind: "speak", text: "Good." },
  ];

  it("speaks every utterance, in order, and reports done", () => {
    const speech = fakeSpeech();
    let done = false;
    playVoiceScript(script, speech, { onDone: () => void (done = true) }, instant);
    expect(speech.said).toEqual(["hola", "Good."]);
    expect(done).toBe(true);
  });

  it("reports each step so the page can follow along", () => {
    const seen: Array<[number, string]> = [];
    playVoiceScript(
      script,
      fakeSpeech(),
      { onStep: (i, step) => void seen.push([i, step.kind]) },
      instant,
    );
    expect(seen).toEqual([
      [0, "speak"],
      [1, "wait"],
      [2, "respond"],
      [3, "speak"],
    ]);
  });

  it("finishes an empty script without speaking", () => {
    const speech = fakeSpeech();
    let done = false;
    const handle = playVoiceScript([], speech, { onDone: () => void (done = true) }, instant);
    expect(speech.said).toEqual([]);
    expect(done).toBe(true);
    expect(handle.running()).toBe(false);
  });

  it("stops for good — a late onEnd must not resume it", () => {
    // The real hazard: speechSynthesis.cancel() fires pending onend handlers,
    // so a naive player advances one more step after being stopped.
    const said: string[] = [];
    let resume: (() => void) | null = null;
    const speech: SpeechPort = {
      speak(text, onEnd) {
        said.push(text);
        resume = onEnd;
      },
      cancel() {
        // Exactly what the browser does: run the pending handler.
        resume?.();
      },
    };
    const handle = playVoiceScript(script, speech, {}, instant);
    expect(said).toEqual(["hola"]);
    handle.stop();
    expect(handle.running()).toBe(false);
    expect(said).toEqual(["hola"]);
  });

  it("stopping twice is harmless", () => {
    const speech = fakeSpeech();
    const handle = playVoiceScript([{ kind: "wait", seconds: 5 }], speech, {}, () => {});
    handle.stop();
    handle.stop();
    expect(speech.cancels).toBe(1);
  });

  it("waits the authored budget rather than a guess", () => {
    const delays: number[] = [];
    playVoiceScript(script, fakeSpeech(), {}, (fn, ms) => {
      delays.push(ms);
      fn();
    });
    expect(delays).toEqual([2000, 8000]);
  });
});

describe("speech tags", () => {
  it("gives an engine something it can pronounce", () => {
    expect(speechTagFor("spanish")).toBe("es-ES");
    expect(speechTagFor("japanese")).toBe("ja-JP");
    // Latin has no TTS voice anywhere; Italian is the closest living phonology
    // and is what a Latin teacher would reach for.
    expect(speechTagFor("latin")).toBe("it-IT");
  });

  it("falls back to the browser default rather than guessing wrong", () => {
    expect(speechTagFor("klingon")).toBe("");
    expect(speechTagFor("")).toBe("");
  });
});
