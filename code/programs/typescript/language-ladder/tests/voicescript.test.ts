import { describe, expect, it } from "vitest";
import { MANUAL_CUE_ACTIONS } from "@coding-adventures/human-language-data/src/narration.ts";
import {
  BLOCK_GAP_SECONDS,
  DEFAULT_RESPONSE_SECONDS,
  type NarrationLesson,
  type NarrationSegment,
  buildVoiceScript,
  isHandsOnCue,
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

// Hands and eyes are not a voice. These pin the rule from the module header:
// a cue the generator marked `spoken: false` is SAID as a deferral — in the
// narration's own words — never asked, and is queued for the end of the lesson.
describe("hands-on cues, the way narration treats them", () => {
  /** A prompt segment exactly as narration-cli writes one into the JSON. */
  function cue(action: string, instruction: string, spoken?: boolean): NarrationSegment {
    return {
      kind: "prompt",
      action,
      instruction,
      ...(spoken === undefined ? {} : { spoken }),
      responseSeconds: 8,
    };
  }

  it("defers a WRITE cue: says it, leaves no answer gap, queues it", () => {
    const steps = buildVoiceScript(
      lesson([{ segments: [cue("WRITE", "hola, twice", false)] }]),
    );
    expect(steps).toEqual([
      { kind: "speak", text: "Once you have stopped driving — write: hola, twice." },
      { kind: "wait", seconds: BLOCK_GAP_SECONDS },
      {
        kind: "speak",
        text: "Saved for when you have stopped driving, one thing. Write: hola, twice.",
      },
    ]);
    // The whole point: nothing here asks a driver to do anything now.
    expect(respondCount(steps)).toBe(0);
    // Only the gap before the recap is silence — the eight seconds a WRITE
    // used to get as a "response" are gone.
    expect(scriptSilence(steps)).toBe(BLOCK_GAP_SECONDS);
  });

  it("still asks for a SAY cue, with its answer gap, and queues nothing", () => {
    const steps = buildVoiceScript(lesson([{ segments: [cue("SAY", "hola", true)] }]));
    expect(steps).toEqual([{ kind: "respond", instruction: "hola", seconds: 8 }]);
  });

  it("keeps the order of a block that mixes the two, and recaps every deferral once", () => {
    const steps = buildVoiceScript(
      lesson(
        [
          {
            title: "Guided Practice",
            segments: [
              cue("SAY", "ja", true),
              cue("TRACE", "జ three times", false),
              { kind: "speech", text: "Good." },
              cue("WRITE", "the whole word.", false),
            ],
          },
        ],
        "జ — ja",
      ),
    );
    expect(steps.map((s) => (s.kind === "speak" ? s.text : s.kind))).toEqual([
      "జ — ja",
      "wait",
      "Guided Practice",
      "respond",
      "Once you have stopped driving — trace: జ three times.",
      "Good.",
      // An author's own full stop is kept, not doubled.
      "Once you have stopped driving — write: the whole word.",
      "wait",
      "Saved for when you have stopped driving, 2 things. Trace: జ three times. Write: the whole word.",
    ]);
  });

  it("speaks a qualifier with its verb, joined the way the narration joins it", () => {
    const steps = buildVoiceScript(
      lesson([{ segments: [{ ...cue("WRITE", "cansado", false), qualifier: "(m.)" }] }]),
    );
    expect(steps[0]).toEqual({
      kind: "speak",
      text: "Once you have stopped driving — write (m.): cansado.",
    });
  });

  it("defers a multi-word action when any of its words needs a hand", () => {
    // No `spoken` flag, so the module asks the generator's set itself.
    expect(isHandsOnCue(cue("WRITE OUT", "the alphabet"))).toBe(true);
    expect(isHandsOnCue(cue("trace over", "the dotted line"))).toBe(true);
    expect(isHandsOnCue(cue("RETURN TO", "the opener"))).toBe(false);
    expect(isHandsOnCue(cue("READ ALOUD", "the line"))).toBe(
      MANUAL_CUE_ACTIONS.has("READ") || MANUAL_CUE_ACTIONS.has("ALOUD"),
    );
    const steps = buildVoiceScript(
      lesson([{ segments: [cue("WRITE OUT", "the alphabet"), cue("RETURN TO", "the opener")] }]),
    );
    expect(steps[0]).toEqual({
      kind: "speak",
      text: "Once you have stopped driving — write out: the alphabet.",
    });
    expect(steps[1]).toEqual({ kind: "respond", instruction: "the opener", seconds: 8 });
  });

  it("follows the generator's set, so a verb added there is deferred here too", () => {
    // Iterating the imported set rather than a list written in this test is
    // the point: when human-language-data grows MANUAL_CUE_ACTIONS, this test
    // covers the new verbs without being touched.
    expect(MANUAL_CUE_ACTIONS.size).toBeGreaterThan(0);
    for (const action of MANUAL_CUE_ACTIONS) {
      expect(isHandsOnCue(cue(action, "it"))).toBe(true);
    }
    expect(isHandsOnCue(cue("SAY", "it"))).toBe(false);
    // A prompt with no verb at all is a plain spoken prompt, as before.
    expect(isHandsOnCue({ kind: "prompt", instruction: "Say it." })).toBe(false);
  });

  it("trusts the generator's verdict over its own reading of the verb", () => {
    // The flag is what the narration plain text was rendered from, so voice
    // mode and the narration can never disagree about the same cue.
    expect(isHandsOnCue(cue("WRITE", "it", true))).toBe(false);
    expect(isHandsOnCue(cue("SAY", "it", false))).toBe(true);
  });

  it("opens with the lesson's notice, straight after the title", () => {
    const text = "Before we start: this one needs your hands, so it is not a driving lesson.";
    const steps = buildVoiceScript({
      id: "L",
      title: "س — seen",
      notice: { text },
      blocks: [{ segments: [{ kind: "speech", text: "Hello." }] }],
    });
    expect(steps.slice(0, 2)).toEqual([
      { kind: "speak", text: "س — seen" },
      { kind: "speak", text },
    ]);
    // A drivable lesson has `notice: null`, and says nothing extra.
    expect(
      buildVoiceScript({ id: "L", title: "hola", notice: null, blocks: [] }),
    ).toEqual([{ kind: "speak", text: "hola" }]);
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

describe("hands-on cues in the real narration", () => {
  it("never leaves an answer gap after an Arabic chapter-one WRITE or TRACE", async () => {
    // Arabic chapter one teaches letter shapes, so it has real `spoken: false`
    // cues beside ordinary SAY cues — the mix this change is about.
    const chapter = (await import(
      "../../../../learning/human-languages/arabic/narration/ch01.json"
    )) as unknown as { default: { lessons: NarrationLesson[] } };
    let manualSeen = 0;
    for (const source of chapter.default.lessons) {
      const segments = source.blocks.flatMap((block) => block.segments);
      const manual = segments.filter((s) => s.kind === "prompt" && s.spoken === false);
      const asked = new Set(
        segments
          .filter((s) => (s.kind === "prompt" && s.spoken !== false) || s.kind === "activity")
          .map((s) => (s.instruction ?? s.prompt ?? "").trim()),
      );
      const steps = buildVoiceScript(source);
      for (const segment of manual) {
        manualSeen += 1;
        const instruction = (segment.instruction ?? "").trim();
        // Said as a deferral…
        expect(
          steps.some(
            (s) =>
              s.kind === "speak" &&
              s.text.startsWith("Once you have stopped driving — ") &&
              s.text.includes(instruction),
          ),
        ).toBe(true);
        // …and never asked, unless a spoken cue happens to share its words.
        if (!asked.has(instruction)) {
          expect(steps.some((s) => s.kind === "respond" && s.instruction === instruction)).toBe(
            false,
          );
        }
      }
      if (manual.length > 0) {
        const last = steps[steps.length - 1];
        expect(last?.kind === "speak" && last.text.startsWith("Saved for when")).toBe(true);
      }
      if (source.notice?.text) {
        expect(steps[1]).toEqual({ kind: "speak", text: source.notice.text });
      }
    }
    expect(manualSeen).toBeGreaterThan(0);
  });
});
