import { describe, expect, it } from "vitest";
import {
  BLOCK_GAP_SECONDS,
  DEFAULT_RESPONSE_SECONDS,
  type NarrationLesson,
  buildVoiceScript,
  respondCount,
  scriptSilence,
} from "../src/voicescript.ts";

function lesson(blocks: NarrationLesson["blocks"], title?: string): NarrationLesson {
  return { id: "L", title, blocks };
}

describe("building a spoken script", () => {
  it("speaks the lesson title, then each block's title", () => {
    const steps = buildVoiceScript(
      lesson([{ title: "Warm-up", segments: [{ kind: "speech", text: "Hello." }] }], "hola — hello"),
    );
    expect(steps[0]).toEqual({ kind: "speak", text: "hola — hello" });
    // A gap separates the title from the block, so they do not run together.
    expect(steps[1]).toEqual({ kind: "wait", seconds: BLOCK_GAP_SECONDS });
    expect(steps[2]).toEqual({ kind: "speak", text: "Warm-up" });
    expect(steps[3]).toEqual({ kind: "speak", text: "Hello." });
  });

  it("keeps authored pauses, which are the thinking time", () => {
    const steps = buildVoiceScript(lesson([{ segments: [{ kind: "pause", seconds: 3 }] }]));
    expect(steps).toContainEqual({ kind: "wait", seconds: 3 });
    // A pause with no usable duration is dropped rather than becoming a zero.
    expect(
      buildVoiceScript(lesson([{ segments: [{ kind: "pause", seconds: 0 }] }])).some(
        (s) => s.kind === "wait" && s.seconds === 0,
      ),
    ).toBe(false);
  });

  it("turns a prompt into a chance to speak, with the authored budget", () => {
    const steps = buildVoiceScript(
      lesson([
        {
          segments: [
            { kind: "prompt", instruction: '"tú" — for a close friend', responseSeconds: 8 },
          ],
        },
      ]),
    );
    expect(steps).toContainEqual({
      kind: "respond",
      instruction: '"tú" — for a close friend',
      seconds: 8,
    });
  });

  it("falls back to a default budget when the corpus did not give one", () => {
    const steps = buildVoiceScript(
      lesson([{ segments: [{ kind: "prompt", instruction: "Say it." }] }]),
    );
    expect(steps).toContainEqual({
      kind: "respond",
      instruction: "Say it.",
      seconds: DEFAULT_RESPONSE_SECONDS,
    });
  });

  it("carries an activity's accepted answers, answer included", () => {
    const steps = buildVoiceScript(
      lesson([
        {
          segments: [
            {
              kind: "activity",
              prompt: "Say a day in Spanish.",
              answer: "lunes",
              accepted: ["el lunes"],
              responseSeconds: 8,
            },
          ],
        },
      ]),
    );
    const respond = steps.find((s) => s.kind === "respond");
    expect(respond).toEqual({
      kind: "respond",
      instruction: "Say a day in Spanish.",
      seconds: 8,
      accepted: ["lunes", "el lunes"],
    });
  });

  it("omits accepted entirely when there is nothing to accept", () => {
    const steps = buildVoiceScript(
      lesson([{ segments: [{ kind: "activity", prompt: "Say something." }] }]),
    );
    expect(steps.find((s) => s.kind === "respond")).toEqual({
      kind: "respond",
      instruction: "Say something.",
      seconds: DEFAULT_RESPONSE_SECONDS,
    });
  });

  it("speaks a table as its pre-flattened rows, because a table cannot be read aloud", () => {
    const steps = buildVoiceScript(
      lesson([
        {
          segments: [
            { kind: "table", utterances: ["you say: tú. you are signalling: closeness.", "  "] },
          ],
        },
      ]),
    );
    expect(steps).toContainEqual({
      kind: "speak",
      text: "you say: tú. you are signalling: closeness.",
    });
    // A blank row is dropped rather than spoken as silence.
    expect(steps.filter((s) => s.kind === "speak")).toHaveLength(1);
  });

  it("repeats the block it sits in, the number of extra times asked", () => {
    const steps = buildVoiceScript(
      lesson([
        {
          segments: [
            { kind: "speech", text: "Say it." },
            { kind: "pause", seconds: 2 },
            { kind: "repeat", times: 3 },
          ],
        },
      ]),
    );
    // Once through, then two more: three utterances and three pauses.
    expect(steps.filter((s) => s.kind === "speak" && s.text === "Say it.")).toHaveLength(3);
    expect(steps.filter((s) => s.kind === "wait" && s.seconds === 2)).toHaveLength(3);
  });

  it("does not repeat the previous block, only its own", () => {
    const steps = buildVoiceScript(
      lesson([
        { segments: [{ kind: "speech", text: "First block." }] },
        { segments: [{ kind: "speech", text: "Second." }, { kind: "repeat", times: 2 }] },
      ]),
    );
    expect(steps.filter((s) => s.kind === "speak" && s.text === "First block.")).toHaveLength(1);
    expect(steps.filter((s) => s.kind === "speak" && s.text === "Second.")).toHaveLength(2);
  });

  it("treats repeat x1 and repeat x0 as nothing to do", () => {
    for (const times of [0, 1]) {
      const steps = buildVoiceScript(
        lesson([{ segments: [{ kind: "speech", text: "Once." }, { kind: "repeat", times }] }]),
      );
      expect(steps.filter((s) => s.kind === "speak" && s.text === "Once.")).toHaveLength(1);
    }
  });

  it("skips an unknown segment kind rather than speaking it", () => {
    const steps = buildVoiceScript(
      lesson([{ segments: [{ kind: "hologram" } as never, { kind: "speech", text: "Fine." }] }]),
    );
    expect(steps).toEqual([{ kind: "speak", text: "Fine." }]);
  });

  it("survives an empty or malformed lesson", () => {
    expect(buildVoiceScript({ id: "L", blocks: [] })).toEqual([]);
    expect(buildVoiceScript({ id: "L" } as NarrationLesson)).toEqual([]);
    expect(buildVoiceScript(lesson([{ segments: [] }]))).toEqual([]);
    // Whitespace-only speech is not worth an utterance.
    expect(buildVoiceScript(lesson([{ segments: [{ kind: "speech", text: "   " }] }]))).toEqual([]);
  });
});

describe("what a script costs", () => {
  it("adds up every second the learner is not being spoken to", () => {
    const steps = buildVoiceScript(
      lesson([
        {
          segments: [
            { kind: "pause", seconds: 2 },
            { kind: "prompt", instruction: "Say it.", responseSeconds: 8 },
            { kind: "speech", text: "Good." },
          ],
        },
      ]),
    );
    expect(scriptSilence(steps)).toBe(10);
    expect(respondCount(steps)).toBe(1);
  });
});

// The fixtures above prove the rules. This proves the rules meet the corpus:
// a script built from real generated narration, not from a shape I invented.
describe("against the real narration", () => {
  it("builds a runnable script for chapter one of Spanish", async () => {
    const chapter = (await import(
      "../../../../learning/human-languages/spanish/narration/ch01.json"
    )) as unknown as { default: { lessons: NarrationLesson[] } };
    const lessons = chapter.default.lessons;
    expect(lessons.length).toBeGreaterThan(0);

    for (const source of lessons) {
      const steps = buildVoiceScript(source);
      expect(steps.length).toBeGreaterThan(0);
      // Nothing unspeakable got through: no empty utterance, no zero wait.
      for (const step of steps) {
        if (step.kind === "speak") expect(step.text.trim()).not.toBe("");
        if (step.kind === "wait") expect(step.seconds).toBeGreaterThan(0);
        if (step.kind === "respond") {
          expect(step.instruction.trim()).not.toBe("");
          expect(step.seconds).toBeGreaterThan(0);
        }
      }
    }

    // The first lesson should be a real, sittable micro-lesson: it speaks, it
    // pauses for thought, and it asks the learner to say something.
    const first = buildVoiceScript(lessons[0]!);
    expect(first.some((s) => s.kind === "speak")).toBe(true);
    expect(first.some((s) => s.kind === "wait")).toBe(true);
    expect(respondCount(first)).toBeGreaterThan(0);
    // And it fits inside the authored five-minute budget with room to speak.
    expect(scriptSilence(first)).toBeLessThan(180);
  });
});
