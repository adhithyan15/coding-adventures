// A lesson announced as drivable must not ask a driver to look or to use a hand.
//
// ---------------------------------------------------------------------------
// What this file holds
// ---------------------------------------------------------------------------
//
// `src/drivable-instructions.ts` finds the steps a narrator would read out
// plainly that need eyes or hands — "[PAUSE 3s] Read **नमस्ते**.", "Write
// **か** once from memory", "[YOU RECALL: say *āmi*, then read **কেমন**]".
// `src/modality.ts` turns any such step in a lesson's preamble or kept
// sections into the `eyes-or-hands-step` reason, which makes the CORE `sight`,
// so the lesson stops being announced as "you can do this one in the car".
//
// Before that rule, the same detectors lived in a test that scanned only
// lessons that are `voice` through and through (`drivable`). A lesson with a
// detachable letters or writing section is never `drivable`, so its core was
// never read, while the narration announced that core as a car lesson. When
// this was measured there were 166 such steps in the cores of 132
// core-drivable lessons in fifteen tracks. 78 lessons were rewritten to defer
// the step in a cue ([YOU READ: …], [YOU WRITE: …]) or say it for the ear;
// the other 54 are now honestly not drivable at the core.
//
// The file has four parts:
//
//   1. the detector, case by case: what the new prose objects and the
//      pointing and gesture rules fire on, and what they deliberately do not;
//   2. the aggregator `eyesOrHandsSteps`: every family reaches it, deferred
//      cues never do, and its vocabulary screen cannot hide a step;
//   3. the classifier and the narration, on synthetic lessons: the rule decides
//      the core, a deferred cue keeps a lesson drivable, a step inside a
//      detachable section changes nothing, and the stop guard is spoken where
//      the step is;
//   4. the corpus gate: ZERO kept sections of a core-drivable lesson carry such
//      a step, judged both by the aggregator and by every separate check, with
//      a synthetic demonstration on real lessons so the zero cannot be vacuous.

import { describe, expect, it } from "vitest";
import { loadEverything, loadModalityManifest } from "../src/loader.js";
import {
  drivableWritingInstructions,
  eyesOrHandsSteps,
  gestureSpokenCues,
  mayAskForEyesOrHands,
  pointingOrGestureProse,
  pointingRecallCues,
  proseAsksToPointOrGesture,
  proseAsksToReadOrHandleCards,
  proseReadsThePage,
  readingOrCardProse,
  readingSpokenCues,
} from "../src/drivable-instructions.js";
import {
  deriveLessonModality,
  eyesOrHandsStepsIn,
  isDetachableBlock,
} from "../src/modality.js";
import { narrateLesson, renderLessonNarrationText, STOP_GUARD } from "../src/narration.js";
import { parseLesson } from "../src/parse.js";

// ---------------------------------------------------------------------------
// 1. The detector
// ---------------------------------------------------------------------------

describe("proseReadsThePage: the objects that put a reading step on the page without script", () => {
  // `ahead` is the text just after a prose "read", as the step scan hands it over.
  it.each([
    [" these shuffled numerals before looking back", "a determiner, then numerals"],
    [" the visible row once.", "a row, and a reading pass"],
    [" the middle two columns down", "columns"],
    [" each closed practice bank once:", "a bank"],
    [" your repaired word as *sa-LU*.", "your own writing"],
    [" your own column back and say each amount", "your own column"],
    [" what you wrote once.", "your own writing, as a clause"],
    [" the question aloud from the page. (**…**)", "a source on the page"],
    [" the answer once from your own handwriting.", "your handwriting"],
    [" the three lines once and score it", "the + two words, then once"],
    [" it, and say which plain letter it is built on", "a pronoun, then ', and say'"],
    [" each one and say what it means", "each one, then 'and say'"],
    [" it one small piece at a time.", "piece by piece"],
    [" both, then answer.", "both, then answer"],
    [" it once without stopping.", "it once"],
    [" all five aloud. Which one is Sanskrit?", "a count, then aloud"],
  ])("fires on %j (%s)", (ahead) => {
    expect(proseReadsThePage(ahead)).toBe(true);
  });

  it.each([
    [" it literally and it says", "interpretation"],
    [" it as a sum now rather than a word:", "read X as Y"],
    [" once. Say each aloud", "a bare once describes ('taught once and read once')"],
    [" them for now.", "no manner, no page"],
    [" back together at a distant window", "bare 'back' is a participle's adverb"],
    [" it aloud in the order you might guess", "describes a misreading"],
    [" the frame as three beats:", "'frame' is not a page noun"],
    [" the German for its meaning", "a language named, not a thing printed"],
    [" the whole line", "a line of dialogue is heard as readily as read"],
    [" a notice like this and act on it", "advice for the street"],
    [" the door before you lean on it", "a door is not a page"],
  ])("does not fire on %j (%s)", (ahead) => {
    expect(proseReadsThePage(ahead)).toBe(false);
  });
});

describe("proseAsksToPointOrGesture", () => {
  it.each([
    "Point to **ش · ک · ر** and say *sh · k · r*.",
    "Give its meaning and point to the final verb in **મારું નામ મીરા છે**.",
    "Page shut. Point at things around you and land each sentence on *hai*.",
    "Say **namaskāra** with a small bow: “hello” or “greetings.”",
    "Then greet her with palms together.",
  ])("fires on %j", (span) => {
    expect(proseAsksToPointOrGesture(span)).toBe(true);
  });

  it.each([
    // A recall's answer naming what a prefix does, not a movement.
    "(Ask, open the answering clause, point at what it means.)",
    // A custom described, not a gesture asked for.
    "*Vaṇakkam* is said with pressed palms or a small head-bow: the gesture is the word.",
    // A picture asked for, not a movement.
    "Imagine meeting someone with your palms together and a small bow.",
    "The French for to point out is *signaler*.",
    // Inside a quotation: the material being said.
    'Say "point to the door, please" in Spanish.',
  ])("does not fire on %j", (span) => {
    expect(proseAsksToPointOrGesture(span)).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// 2. The aggregator
// ---------------------------------------------------------------------------

describe("eyesOrHandsSteps: every family reaches the classifier", () => {
  it.each([
    ["a bare reading step", "[PAUSE 3s] Read **नमस्ते**. What does it mean?"],
    ["a bare writing step", "[PAUSE 15s] Write **か** once from memory."],
    ["a chained writing step", "Say, read, and write each answer."],
    ["covering the page", "Look, cover, and wait five seconds."],
    ["handling cards", "Then take six meaning cards and say each word."],
    ["pointing in prose", "Point to **ش · ک · ر** and say *sh · k · r*."],
    ["a gesture as a manner", "Say **namaskāra** with a small bow."],
    ["a recall that reads", "- [YOU RECALL: say *āmi*, then read **কেমন**]"],
    ["a recall that writes", "- [YOU RECALL: write **ば** — **R1**]"],
    ["a recall that points", "- [YOU RECALL: point to the sign in **でんわ** with the mark]"],
    ["a spoken cue that reads", "- [YOU SAY: *chha, sāt* — then read **સાત** sign by sign]"],
    ["a spoken cue that gestures", "- [YOU SAY: ದ then ಧ, with a hand in front of your mouth]"],
    ["a nested manual cue", "- [YOU HEAR: *añcŭ*; YOU SHOW: 5]"],
  ])("finds %s", (_label, markdown) => {
    expect(eyesOrHandsSteps(markdown)).toHaveLength(1);
  });

  it.each([
    ["a deferred reading", "[PAUSE 3s] [YOU READ: **नमस्ते**] What does it mean?"],
    ["a deferred writing", "[PAUSE 15s] [YOU WRITE: **か** once from memory]"],
    ["a deferred feeling", "- [YOU FEEL: the breath, with a hand in front of your mouth]"],
    ["a gloss", "- [YOU RECALL: say the Japanese for to read, then say it again]"],
    ["an italic meaning", "- [YOU RECALL: read *reception* on a sign — **R1**]"],
    ["interpretation", "Read it literally and it says *it is two hours*."],
    ["material to be said", '- [YOU SAY: "Leo un libro" — I read a book]'],
    ["plain speech", "[PAUSE 2s] Say *hola* twice, then answer the question."],
  ])("ignores %s", (_label, markdown) => {
    expect(eyesOrHandsSteps(markdown)).toEqual([]);
  });

  it("returns each part once, quoted as authored, in body order", () => {
    const markdown = [
      "Read **नमस्ते**. Then write it.",
      "",
      "- [YOU SAY: hola]",
      "- [YOU RECALL: say *āmi*, then read **কেমন**]",
    ].join("\n");
    expect(eyesOrHandsSteps(markdown)).toEqual([
      "Read **नमस्ते**. Then write it.",
      "[YOU RECALL: say *āmi*, then read **কেমন**]",
    ]);
  });

  it("modality reads bodies, not headings", () => {
    expect(eyesOrHandsStepsIn("# Practice — hear, say, read, and write water\n\nSay *pāṇī*.")).toEqual(
      [],
    );
    expect(eyesOrHandsStepsIn("## Wrap-up\n\n[PAUSE 3s] Read **नमस्ते**.")).toEqual([
      "Read **नमस्ते**.",
    ]);
  });
});

describe("the vocabulary screen is a superset of every check", () => {
  // Each anchor the checks start from, at the start of a word. If one of these
  // stopped passing the screen, a whole family of steps would become invisible
  // to the classifier while every other test stayed green.
  it.each([
    "Write it", "rewrite it", "draw it", "copy it", "circle it", "underline it", "sketch it",
    "jot it", "Read it", "the cards", "cover it", "Uncover it", "hide it", "point to it",
    "clapping", "tap twice", "touch it", "raise a finger", "hold up", "show 3", "gesturing",
    "wave goodbye", "nod", "shake your head", "a small bow", "palms together", "a hand",
    "x; YOU SHOW: 5",
  ])("lets %j through", (text) => {
    expect(mayAskForEyesOrHands(text)).toBe(true);
  });

  it("screens out text that no check can start from", () => {
    expect(mayAskForEyesOrHands("[PAUSE 2s] Say *hola* twice, then answer the question.")).toBe(false);
    // A word-start screen, not a substring one: "already" does not open "read".
    expect(mayAskForEyesOrHands("You already know this one.")).toBe(false);
    // "you" the pronoun is not a nested cue.
    expect(mayAskForEyesOrHands("you say it")).toBe(false);
  });
});

describe("the new patterns stay linear", () => {
  // About 50,000 characters each; only the answers are asserted, because
  // timing bounds flake on a loaded runner.
  it.each([
    ["many readings of a pronoun with no manner", `${"then read it ".repeat(4_000)}`, false],
    ["many readings of 'the' with no manner", `${"then read the the ".repeat(3_000)}`, false],
    ["a long clause before a page source", `read ${"a".repeat(50_000)} from the page`, false],
    ["many page sources too late", `${"read x y z ".repeat(5_000)}`, false],
    ["many readings, the last one with a manner", `${"then read it ".repeat(4_000)}then read it once`, true],
  ])("reading: %s", (_label, content, expected) => {
    expect(content.length).toBeGreaterThan(40_000);
    expect(proseAsksToReadOrHandleCards(content)).toBe(expected);
  });

  it.each([
    ["many pointings at a proposition", `${"and point at what ".repeat(3_000)}`, false],
    ["many points out", `${"then point out ".repeat(4_000)}`, false],
    ["many spoken clauses with no manner", `${"Say it. ".repeat(7_000)}`, false],
    ["many spoken clauses, the last with a bow", `${"Say it. ".repeat(7_000)}Say it with a small bow.`, true],
  ])("pointing and gesture: %s", (_label, content, expected) => {
    expect(content.length).toBeGreaterThan(40_000);
    expect(proseAsksToPointOrGesture(content)).toBe(expected);
  });
});

// ---------------------------------------------------------------------------
// 3. The classifier and the narration
// ---------------------------------------------------------------------------

/** A Hindi lesson with a detachable letters section and the given wrap-up. */
function lessonWithWrapUp(wrapUp: string, extra = ""): ReturnType<typeof parseLesson> {
  return parseLesson(
    [
      "---",
      "schema_version: 2",
      "id: HI-C01-test",
      "chapter: 1",
      "type: word",
      "headword: नमस्ते",
      "gloss: hello",
      "concept_tag: GREETING-HELLO",
      "skills: [listening, speaking, reading]",
      "---",
      "",
      "# नमस्ते — hello",
      "",
      "## Warm-up",
      "",
      "[PAUSE 2s] Say *namaste* out loud.",
      "",
      "## The letters in this word",
      "",
      "Read **न**, then **म**: the letters are new, so this section is set aside in the car.",
      "",
      extra,
      "## Wrap-up Recall",
      "",
      wrapUp,
      "",
    ].join("\n"),
    "hindi",
  );
}

describe("the classifier reads the core for steps", () => {
  it("a reading step in the wrap-up makes the core sight, with the step quoted", () => {
    const entry = deriveLessonModality(
      lessonWithWrapUp("[PAUSE 3s] Read **नमस्ते**. What does it literally mean?"),
    );
    expect(entry.modality).toBe("sight");
    expect(entry.coreModality).toBe("sight");
    expect(entry.coreReasons).toEqual(["eyes-or-hands-step"]);
    expect(entry.reasons).toContain("eyes-or-hands-step");
    expect(entry.coreEyesOrHandsSteps).toEqual([
      "Read **नमस्ते**. What does it literally mean?",
    ]);
    const wrapUp = entry.blocks.find((block) => block.title === "Wrap-up Recall");
    expect(wrapUp?.modality).toBe("sight");
    expect(wrapUp?.reasons).toEqual(["eyes-or-hands-step"]);
  });

  it("the same step deferred as a READ cue keeps the lesson a car lesson", () => {
    const entry = deriveLessonModality(
      lessonWithWrapUp("[PAUSE 3s] [YOU READ: **नमस्ते**] What does it literally mean?"),
    );
    expect(entry.modality).toBe("sight");
    expect(entry.coreModality).toBe("voice");
    expect(entry.coreReasons).toEqual(["no-visual-dependency"]);
    expect(entry.coreEyesOrHandsSteps).toEqual([]);
  });

  it("a step inside a detachable section changes nothing: the section's type already decides it", () => {
    const entry = deriveLessonModality(lessonWithWrapUp("[PAUSE 3s] What does it mean?"));
    // The letters section says "Read **न**", and is set aside whole.
    const letters = entry.blocks.find((block) => block.detachable);
    expect(letters?.eyesOrHandsSteps).toEqual([]);
    expect(letters?.reasons).toEqual(["script-block"]);
    expect(entry.reasons).not.toContain("eyes-or-hands-step");
    expect(entry.coreModality).toBe("voice");
  });

  it("a recall that reads is a step; split into RECALL and READ, it is not", () => {
    const spoken = deriveLessonModality(
      lessonWithWrapUp("- [YOU RECALL: say *āmi*, then read **कैसे**]"),
    );
    expect(spoken.coreModality).toBe("sight");
    const split = deriveLessonModality(
      lessonWithWrapUp("- [YOU RECALL: say *āmi*]\n- [YOU READ: **कैसे**]"),
    );
    expect(split.coreModality).toBe("voice");
  });

  it("the narration stops announcing the lesson as drivable and guards the section with the step", () => {
    const narration = narrateLesson(lessonWithWrapUp("[PAUSE 3s] Read **नमस्ते**. What does it mean?"));
    expect(narration.coreModality).toBe("sight");
    expect(narration.notice?.text).not.toContain("you can do this one in the car");
    expect(narration.notice?.waitUntilStopped).toContain("Wrap-up Recall");
    expect(narration.notice?.needs.join(" ")).toContain("read, point, write or gesture");

    const text = renderLessonNarrationText(narration);
    const title = text.indexOf("Wrap-up Recall.");
    const guard = text.indexOf(STOP_GUARD, title);
    const step = text.indexOf("Read नमस्ते");
    expect(title).toBeGreaterThan(-1);
    expect(guard).toBeGreaterThan(title);
    expect(step).toBeGreaterThan(guard);
  });
});

// ---------------------------------------------------------------------------
// 4. The corpus
// ---------------------------------------------------------------------------

const { lessons } = loadEverything();
const manifest = loadModalityManifest();
const coreDrivableIds = new Set(
  manifest.lessons.filter((row) => row.coreDrivable).map((row) => row.id),
);
const coreDrivableLessons = lessons.filter((lesson) =>
  coreDrivableIds.has(lesson.realization.lessonId),
);
/** Every fourth core-drivable lesson in id order: see the cross-check cases. */
const crossCheckSample = [...coreDrivableLessons]
  .sort((left, right) => left.realization.lessonId.localeCompare(right.realization.lessonId))
  .filter((_lesson, index) => index % 4 === 0);

/** The preamble and every kept section of a lesson: what the car hears plainly. */
function keptSections(lesson: (typeof lessons)[number]): string[] {
  return [lesson.preamble, ...lesson.blocks.filter((block) => !isDetachableBlock(block)).map((b) => b.markdown)];
}

/** Every separate check in drivable-instructions.ts, each walking the text on its own. */
function everySeparateCheck(markdown: string): string[] {
  return [
    ...drivableWritingInstructions(markdown),
    ...pointingRecallCues(markdown),
    ...readingSpokenCues(markdown),
    ...gestureSpokenCues(markdown),
    ...readingOrCardProse(markdown),
    ...pointingOrGestureProse(markdown),
  ];
}

describe("no core-drivable lesson asks a driver to look or use a hand", () => {
  it("scans a real corpus, not an empty one", () => {
    // Anti-vacuity for the scan itself: if the manifest stopped loading or ids
    // stopped matching, every check below would pass over nothing.
    expect(coreDrivableLessons.length).toBeGreaterThan(lessons.length / 2);
  });

  it("the committed manifest's core-drivable lessons carry no eyes-or-hands step in a kept section", () => {
    // The gate the security review asked for. Read through the manifest, so a
    // stale `core/lesson-modality` is caught as well as a wrong rule.
    const problems: string[] = [];
    for (const lesson of coreDrivableLessons) {
      for (const section of keptSections(lesson)) {
        for (const step of eyesOrHandsStepsIn(section)) {
          problems.push(
            `${lesson.realization.lessonId}: announced as drivable, but a kept section asks for ` +
              `eyes or hands. Defer the step in a cue the narration hedges ([YOU READ: …], ` +
              `[YOU WRITE: …]) or say it for the ear:\n       ${step.slice(0, 160)}`,
          );
        }
      }
    }
    expect(problems, problems.join("\n")).toEqual([]);
  });

  // The gate above trusts the aggregator, and the aggregator trusts its
  // vocabulary screen. The next two cases judge the same sections by each
  // check on its own walk, with no screen, split by what the screen decided:
  // a word missing from the screen fails the first, a family dropped from the
  // aggregator fails the second.
  //
  // They read a fixed quarter of the core-drivable lessons, not all of them.
  // Six separate walks over every kept section cost about thirteen seconds
  // here, and a case that slow under full-suite load in CI is the timeout this
  // suite's config warns about. The sample is no weaker for the defects these
  // cases exist to catch: a screen word or an aggregator family is used by
  // thousands of sections across every track (1,023 lessons carry the reason
  // today), so dropping one shows up in any quarter. Which quarter is fixed —
  // every fourth lesson in id order — so a failure reproduces.
  it("the vocabulary screen held back no step: sections it skips are clean by every separate check", () => {
    const problems: string[] = [];
    for (const lesson of crossCheckSample) {
      for (const section of keptSections(lesson)) {
        if (mayAskForEyesOrHands(section)) continue;
        for (const step of everySeparateCheck(section)) {
          problems.push(`${lesson.realization.lessonId}: ${step.slice(0, 160)}`);
        }
      }
    }
    expect(problems, problems.join("\n")).toEqual([]);
  });

  it("the aggregator dropped no family: sections the screen passes are clean by every separate check", () => {
    const problems: string[] = [];
    for (const lesson of crossCheckSample) {
      for (const section of keptSections(lesson)) {
        if (!mayAskForEyesOrHands(section)) continue;
        for (const step of everySeparateCheck(section)) {
          problems.push(`${lesson.realization.lessonId}: ${step.slice(0, 160)}`);
        }
      }
    }
    expect(problems, problems.join("\n")).toEqual([]);
  });

  it("the aggregator finds exactly what the separate checks find, wherever a step exists", () => {
    // Positives, corpus-wide: every kept section with a step (these sit in
    // lessons that are now honestly not drivable).
    let compared = 0;
    const problems: string[] = [];
    for (const lesson of lessons) {
      for (const section of keptSections(lesson)) {
        const found = eyesOrHandsSteps(section);
        if (found.length === 0) continue;
        compared += 1;
        const separate = new Set(everySeparateCheck(section).map((step) => step.trim()));
        const together = new Set(found);
        const same = separate.size === together.size && [...together].every((step) => separate.has(step));
        if (!same) problems.push(`${lesson.realization.lessonId}: ${[...together].join(" | ").slice(0, 160)}`);
      }
    }
    expect(compared, "kept sections with a step").toBeGreaterThan(0);
    expect(problems, problems.join("\n")).toEqual([]);
  });

  it("the rule is not vacuous: every deferred READ in a core-drivable lesson, spoken again, takes the lesson out of the car", () => {
    // The zero above is the goal, so it cannot also be the proof that the rule
    // works. The proof is synthetic, on real lessons: undo the fix. Every kept
    // section that defers a reading step as `[YOU READ: **…**]` gets it back as
    // the bare sentence it replaced ("Read **…**."), and the classifier must
    // then move that lesson's core to sight. The set grows as debt is paid,
    // because every reading fix adds a READ cue.
    const misses: string[] = [];
    let demonstrated = 0;
    for (const lesson of coreDrivableLessons) {
      const index = lesson.blocks.findIndex(
        (block) => !isDetachableBlock(block) && block.markdown.includes("[YOU READ: **"),
      );
      if (index === -1) continue;
      const block = lesson.blocks[index]!;
      const at = block.markdown.indexOf("[YOU READ: **");
      const close = block.markdown.indexOf("]", at);
      const object = block.markdown.slice(at + "[YOU READ: ".length, close);
      const undone = {
        ...lesson,
        blocks: lesson.blocks.map((candidate, position) =>
          position === index
            ? {
                ...candidate,
                markdown: `${candidate.markdown.slice(0, at)}Read ${object}.${candidate.markdown.slice(close + 1)}`,
              }
            : candidate,
        ),
      };
      demonstrated += 1;
      const entry = deriveLessonModality(undone);
      if (entry.coreModality === "voice") misses.push(`${lesson.realization.lessonId}: Read ${object}.`);
    }
    expect(demonstrated, "core-drivable lessons with a deferred READ cue").toBeGreaterThan(0);
    expect(misses, misses.join("\n")).toEqual([]);
  });
});
