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
//      a synthetic demonstration on real lessons so the zero cannot be vacuous;
//   5. what the body walk cannot see — scored activity prompts (kept in HTML
//      comments) and headings — case by case, in the narration, and as a gate
//      over the committed narration: no prompt or heading SPOKEN in a drivable
//      part of a core-drivable lesson asks to read, write, type or point.

import { readdirSync, readFileSync, existsSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot, loadEverything, loadModalityManifest } from "../src/loader.js";
import {
  acceptsOnlyScript,
  activityNeedsEyesOrHands,
  activityPromptNeedsEyesOrHands,
  asksToType,
  drivableWritingInstructions,
  eyesOrHandsSteps,
  gestureSpokenCues,
  handVerbsInPrompt,
  headingStepIndex,
  mayAskForEyesOrHands,
  pointingOrGestureProse,
  gradesScriptGlyphs,
  pointingRecallCues,
  promptAsksForAWrittenAnswer,
  promptChoosesAmongShownGlyphs,
  promptReadsTheShownScript,
  promptTypesInAScript,
  proseAsksToPointOrGesture,
  proseAsksToReadOrHandleCards,
  proseAsksToWrite,
  proseReadsThePage,
  readingOrCardProse,
  readingSpokenCues,
} from "../src/drivable-instructions.js";
import { headingAsksForEyesOrHands } from "../src/for-the-ear.js";
import type { LessonNarration, NarrationActivity } from "../src/narration.js";
import type { LessonActivity } from "../src/types.js";
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

    // The guard is spoken before the section's title, then the title, then the step.
    const lines = renderLessonNarrationText(narration).split("\n");
    const title = lines.indexOf("Wrap-up Recall.");
    const step = lines.findIndex((line) => line.startsWith("Read नमस्ते"));
    expect(title).toBeGreaterThan(0);
    expect(lines[title - 1]).toBe(STOP_GUARD);
    expect(step).toBeGreaterThan(title);
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

// ---------------------------------------------------------------------------
// 5. What the body walk cannot see: activity prompts and headings
// ---------------------------------------------------------------------------

describe("activityPromptNeedsEyesOrHands: a scored prompt a driver cannot answer aloud", () => {
  it.each([
    ["Write son in Chinese characters.", "eyes-or-hands-step"],
    ["Read 汉字 without pinyin, then give its meaning.", "eyes-or-hands-step"],
    ["From sound alone, write the taught farewell, then say it once.", "eyes-or-hands-step"],
    ["Complete all four scored checks, then write the two newest words.", "eyes-or-hands-step"],
    ["Type 'a car' in hiragana.", "typing-in-a-script"],
    ["You have just been introduced to someone. Type the Mandarin greeting in characters.", "typing-in-a-script"],
    ["Type the Mandarin character for person.", "typing-in-a-script"],
    ["Read this shuffled pair and give each meaning: આજ — રાત.", "reading-the-shown-script"],
    ["Read and distinguish 中文 and 中国 without pinyin.", "reading-the-shown-script"],
    [
      "In ਪੜ੍ਹਨਾ, identify the small ha tucked under the preceding letter and then read the whole known word aloud.",
      "reading-the-shown-script",
    ],
    // The security review's regressions: real prompts, each spoken as a live
    // question by the first version, because the prose checks under-fire by design.
    ["From bachchā, write the taught word.", "hand-verb"],
    ["After hiding the model, write kāĩ and give its meaning.", "hand-verb"],
    ["After all four checks pass, write home, person, and family members.", "hand-verb"],
    ["Write is that so, without pinyin.", "hand-verb"],
    ["Hear dhanyavad; give its meaning and write it in Devanagari.", "hand-verb"],
    ["From the heard morae i | chi | do, write once in hiragana.", "hand-verb"],
    // Any emphasis is the bare word.
    ["*Write* the word for water.", "hand-verb"],
    ["_write_ it once.", "hand-verb"],
    ["*Type* 'a car' in hiragana.", "typing-in-a-script"],
    ["Read **this** pair: 中文 — 中国.", "reading-the-shown-script"],
    // The second security review: questions with no hand verb whose answer is
    // WRITTEN, each spoken live in a kept section of a core-drivable lesson.
    ["Refuse an offer and then leave, in two written lines.", "written-answer"],
    ["Refuse the offer, then leave the door open — two written lines.", "written-answer"],
    ["Someone says das karo. Refuse in one word, in Devanagari.", "written-answer"],
    ["Record the retrieval string: e, five, mouth, please say it, hair, eye.", "hand-verb"],
    ["Which already-taught Gurmukhi consonant is na: ਨ or ਸ?", "choice-among-shown-glyphs"],
    // …and the excuses it found too loose (each also has a control below).
    ["Do it again and copy the sign in Devanagari.", "hand-verb"],
    ["Do the same and write it in hiragana.", "hand-verb"],
    ["Do please write 水 in characters.", "hand-verb"],
    ["If you can now write it in hiragana, do so.", "hand-verb"],
    ["Copy the tone marks onto the vowels.", "hand-verb"],
    ["Copy the tone mark onto the vowel.", "hand-verb"],
    ["Copy the stress mark onto the right vowel.", "hand-verb"],
    ["Copy the pitch accent marks.", "hand-verb"],
    ["Follow this: 'You trace the letter's outline with a finger.'", "hand-verb"],
    ["Remember what it means to write it: do it in Devanagari.", "hand-verb"],
  ])("defers %j (%s)", (prompt, reason) => {
    expect(activityPromptNeedsEyesOrHands(prompt)).toContain(reason);
  });

  it.each([
    // A typed WORD is said: the renderer words it "Say …" instead.
    "Type the Japanese for 'a stick, a pole'.",
    "Type the Persian word for 'tea'.",
    "Ask how someone is respectfully aloud, then type the romanized words you said.",
    "How many marked tones does Mandarin have? Type the number as a word.",
    // "Read in …" is a participle; Latin-script material is not the shown script.
    "Read in the falling fourth tone, 好 stops meaning 'good'. Type its meaning with that tone, in English.",
    "Read se compran libros as if se still meant themselves. What does it say?",
    // No step link before "read".
    "How is the character 日 read inside 日本語?",
    "Which one points at a day we both already know about: el dia or un dia?",
    "Give the word for friend, the old root meaning, and its borrowing type.",
    // The fail-safe check's excused shapes, each from the corpus.
    "Say the Japanese for 'to write'.",
    "Say the Urdu verb meaning 'to write'.",
    "What did the Sanskrit root behind likhna mean before it meant to write?",
    "Which phrase best describes tuve: a letter-for-letter copy of tenuī, or a historically reshaped strong form?",
    "You hear thāro. Does it point to my or your?",
    "Does **یہ** (*yih*) point at what is near you or what is far from you?",
    // Descriptions, not requests: not the base form.
    "Which two of hablé, hablaste, habló carry written accents?",
    "Which pointing word goes with ahi?",
    // Controls for the second review's shapes: "written" describing, not asking
    // for, a form; a script named with no preposition; a singular "character";
    // a choice that is romanized, so it can be heard; a sound copied by mouth.
    "Name the mark that is written left of its letter.",
    "You hear pāṇī with no written cue. What does it mean?",
    "Which hiragana sign becomes が when the dakuten is added?",
    "What is the relationship between the number of strokes in this character and what it means?",
    "Which is right, 中文 (Zhōngwén) or 中国 (Zhōngguó)?",
    "Copy the rhythm of the line.",
    "Copy the pitch accent of the word.",
    "What does one speaker write after midday?",
  ])("asks %j", (prompt) => {
    expect(activityPromptNeedsEyesOrHands(prompt)).toEqual([]);
  });

  it("stays linear on adversarial prompts", () => {
    // Only answers are asserted: timing bounds flake on a loaded runner.
    const typed = `${"then type the word ".repeat(2_500)}in the end`;
    const read = `中 ${"read in ".repeat(6_000)}`;
    expect(typed.length + read.length).toBeGreaterThan(80_000);
    expect(promptTypesInAScript(typed)).toBe(false);
    expect(promptReadsTheShownScript(read)).toBe(false);
    expect(asksToType(typed)).toBe(true);
  });
});

describe("handVerbsInPrompt: the fail-safe check, and each excused shape against its control", () => {
  // Every excuse is paired with a control that differs only in what the excuse
  // keys on, so an excuse that stopped applying — or started applying to the
  // control — fails here, not silently in the corpus.
  it.each([
    ["Say the Japanese for 'to write'.", [], "a quoted gloss"],
    ["Say the word for 'write'.", ["write"], "a bare quoted verb is not a gloss"],
    ["Say 'I write letters' in Hindi.", [], "a quoted gloss with a pronoun"],
    ["Say 'then write it down' in Hindi.", ["write"], "a quote that opens with a connective is an instruction"],
    ["Hear 'to, then write it' and answer.", ["write"], "a quoted span with punctuation inside is no gloss"],
    ["… before it meant to write?", [], "an infinitive gloss"],
    ["Remember to write it down.", ["write"], "'to' with no 'mean' before it"],
    ["What is the point of the second line?", [], "a noun after a determiner, with 'of'"],
    ["Point at the second line.", ["point"], "the same word as an imperative"],
    ["Which phrase is a letter-for-letter copy of tenuī?", [], "'copy of' is a noun"],
    ["Copy the word once.", ["copy"], "copy with an object"],
    ["Copy the rhythm of the line.", [], "copying a sound is done with the mouth"],
    ["Does it point to my or your?", [], "a third-person question"],
    ["Do you point to it?", ["point"], "the learner as subject"],
    ["Can I write it in pinyin?", ["write"], "the learner as subject, 'I'"],
    ["Write is that so, without pinyin.", ["write"], "no copula excuse in a prompt"],
    ["Look at the sign, then tap each mora and clap once.", ["look at", "tap", "clap"], "every verb, in order"],
    ["Circle, underline or tick the answer; then check off the list and fill in the gap.", ["circle", "underline", "tick", "check off", "fill in"], "page verbs"],
    ["Select A or B, then cross out the other and highlight the rest.", ["select", "cross out", "highlight"], "screen verbs"],
    ["Rewrite it, re-write it, jot it, sketch it, trace it, draw it.", ["rewrite", "re-write", "jot", "sketch", "trace", "draw"], "writing verbs"],
    ["You'll see tapēte; the writer wrote it in written form.", [], "whole words only: no 'tap' in tapēte, no base form"],
    // The second security review's tightenings, each against its control.
    ["Follow this: 'You trace the letter's outline.'", ["trace"], "a quote after a colon quotes an instruction"],
    ["Hear 'you trace' and give its meaning.", [], "the same quoted gloss, not after a colon"],
    ["Say 'to write's' in Hindi.", ["write"], "an apostrophe inside a word is not a closing quote"],
    ["Remember what it means to write it: do it.", ["write"], "'means to write' with an object is not a gloss"],
    ["Which verb means to write?", [], "'means to write' ending its sentence is a gloss"],
    ["Do it again and copy the sign.", ["copy"], "an imperative 'Do', ending in '.'"],
    ["Do the same and write it.", ["write"], "an imperative 'Do', ending in '.'"],
    ["Do please write it?", ["write"], "'please' is never a subject"],
    ["If you can now write it, do so.", ["write"], "the auxiliary does not open the sentence"],
    ["Hear it. Does it point to my or your?", [], "the auxiliary opens the second sentence, which ends in '?'"],
    ["What does one speaker write after midday?", [], "a question word, then the auxiliary"],
    ["Copy the tone marks.", ["copy"], "a tone MARK is written"],
    ["Copy the stress mark onto the right vowel.", ["copy"], "a stress mark is written"],
    ["Copy the pitch accent marks.", ["copy"], "pitch accent marks are written"],
    ["Copy the tone of the line.", [], "a tone is copied by mouth"],
    ["Record the string, note down the word and transcribe it.", ["record", "note down", "transcribe"], "the second review's verbs"],
    ["Which word is a record of the visit?", [], "'a record' is a noun"],
  ])("%j → %j (%s)", (prompt, verbs) => {
    expect(handVerbsInPrompt(prompt)).toEqual(verbs);
  });

  it("stays linear on adversarial prompts", () => {
    // Every excuse reads a fixed window, so a prompt made of near-excuses costs
    // the same per verb as one verb does. Only answers are asserted.
    const verbs = "'to write' the point of ".repeat(5_000);
    const questions = "does it point ".repeat(8_000);
    expect(verbs.length + questions.length).toBeGreaterThan(200_000);
    expect(handVerbsInPrompt(verbs)).toEqual([]);
    // Without a "?" none of those is a question any more, so every one counts…
    expect(handVerbsInPrompt(questions)).toHaveLength(8_000);
    // …and as questions, every one is excused.
    expect(handVerbsInPrompt("Does it point? ".repeat(8_000))).toEqual([]);
    expect(handVerbsInPrompt(`${"x ".repeat(100_000)}write`)).toEqual(["write"]);
    // The sentence-end lookup is one pass however many verbs there are.
    // Only the first "Does" opens its sentence, so only its verb is excused, by a
    // "?" some 280,000 characters on.
    expect(handVerbsInPrompt(`${"Does it point ".repeat(20_000)}?`)).toHaveLength(19_999);
    expect(handVerbsInPrompt(`Does it point${" x".repeat(100_000)}.`)).toEqual(["point"]);
  });
});

describe("a prompt whose answer is written: the form it asks for, and what it accepts", () => {
  it.each([
    ["Refuse an offer and then leave, in two written lines.", true],
    ["Refuse the offer, then leave the door open — two written lines.", true],
    ["Answer in writing.", true],
    ["Give the written form of the word.", true],
    ["Someone says das karo. Refuse in one word, in Devanagari.", true],
    ["Give son in Chinese characters.", true],
    ["Put it into hiragana.", true],
    ["Answer using the Arabic script.", true],
    ["Give it in Gujarati script.", true],
    // A spoken fact naming a script is deferred too, on purpose (see the source).
    ["In Devanagari, ক is not a letter at all. What is it?", true],
    // Controls: a description of writing, a language (not a script), a single
    // character, no preposition.
    ["Which two of hablé, hablaste, habló carry written accents?", false],
    ["Name the mark that is written left of its letter.", false],
    ["Say hello in Gujarati.", false],
    ["Both greetings end in the same character. Which one?", false],
    ["Which hiragana sign becomes ga?", false],
  ])("%j asks for a written answer: %s", (prompt, expected) => {
    expect(promptAsksForAWrittenAnswer(prompt)).toBe(expected);
  });

  it.each([
    ["Which already-taught Gurmukhi consonant is na: ਨ or ਸ?", true],
    ["Which is the word: 中文 / 中国?", true],
    ["Pick one: আজ vs রাত.", true],
    ["Which is it, **ਨ** or **ਸ**?", true],
    // Controls: romanized, so it can be heard; a choice between Latin words; a
    // single shown word.
    ["Which is right, 中文 (Zhōngwén) or 中国 (Zhōngguó)?", false],
    ["Does یہ (yih) point at what is near you or what is far from you?", false],
    ["Which English word is the cousin of دودھ (dūdh) — dough, or doughty?", false],
  ])("%j offers a choice among shown glyphs: %s", (prompt, expected) => {
    expect(promptChoosesAmongShownGlyphs(prompt)).toBe(expected);
  });

  it("defers a question that accepts only script, whatever its prompt says", () => {
    const woman = {
      prompt: "Which taught character is the Level 1 word female or woman?",
      assesses: ["ZH-LEX-NU-01"],
      acceptedResponses: ["女", "女 nǚ"],
    };
    expect(activityPromptNeedsEyesOrHands(woman.prompt)).toEqual([]);
    expect(activityNeedsEyesOrHands(woman)).toEqual(["accepts-only-script"]);
    // Control: one romanized answer is enough to make it sayable.
    expect(activityNeedsEyesOrHands({ ...woman, acceptedResponses: ["女", "nǚ"] })).toEqual([]);
    // Digits, punctuation and the Japanese length bar are no script; nor is an empty list.
    expect(acceptsOnlyScript(["3", "three"])).toBe(false);
    expect(acceptsOnlyScript(["ー"])).toBe(false);
    expect(acceptsOnlyScript([])).toBe(false);
    expect(acceptsOnlyScript(["え・五・口・いってください・かみ・め"])).toBe(true);
  });

  it("defers a question that grades a script atom with a script answer, and asks one that grades knowledge about a script", () => {
    const refuse = {
      prompt: "Someone says das karo. Refuse in one word.",
      assesses: ["MW-LEX-KONI-01", "MW-SCRIPT-KONI-01"],
      acceptedResponses: ["कोनी", "कोनी।", "koni"],
    };
    expect(gradesScriptGlyphs(refuse)).toBe(true);
    expect(activityNeedsEyesOrHands(refuse)).toEqual(["grades-script-glyphs"]);
    // Controls: the same answer without the script atom, and a script atom whose
    // canonical answer is said ("ka kha ga gha").
    expect(activityNeedsEyesOrHands({ ...refuse, assesses: ["MW-LEX-KONI-01"] })).toEqual([]);
    expect(
      activityNeedsEyesOrHands({
        prompt: "Say the four velar letters in order.",
        assesses: ["MR-SCRIPT-VELAR-ROW-01"],
        acceptedResponses: ["ka kha ga gha"],
      }),
    ).toEqual([]);
  });

  it("stays linear on adversarial prompts", () => {
    // Only answers are asserted: timing bounds flake on a loaded runner.
    const run = "ਨ".repeat(100_000);
    expect(promptChoosesAmongShownGlyphs(`${run} x`)).toBe(false);
    expect(promptChoosesAmongShownGlyphs(`${run} or ਸ`)).toBe(true);
    // Runs of combining marks: a variable-width lookbehind walked back over the
    // whole run at every position (quadratic; minutes at this size).
    expect(promptChoosesAmongShownGlyphs("\u0301".repeat(100_000))).toBe(false);
    expect(promptChoosesAmongShownGlyphs(`ਨ${"\u0A3E".repeat(100_000)} or ਸ`)).toBe(true);
    expect(promptChoosesAmongShownGlyphs(`a${"\u0301".repeat(100_000)} or b`)).toBe(false);
    const prepositions = "in the long ".repeat(10_000);
    expect(promptAsksForAWrittenAnswer(`${prepositions}word`)).toBe(false);
    expect(promptAsksForAWrittenAnswer(`${"in ".repeat(10_000)}kana`)).toBe(true);
  });
});

describe("an emphasised verb is the bare verb, for every detector", () => {
  // The narration strips `*`, `_` and `**` alike, so a driver hears "*Read* the
  // line" exactly as "Read the line". The detectors used to see only `**bold**`.
  it("prose", () => {
    expect(proseAsksToWrite("Say it. *Write* it once.")).toBe(true);
    expect(proseAsksToWrite("_write_ the word once.")).toBe(true);
    expect(proseAsksToWrite("_Write_ is Old English.")).toBe(false);
    expect(proseAsksToReadOrHandleCards("*Read* the printed line once.")).toBe(true);
    expect(proseAsksToReadOrHandleCards("_Read_ the printed line once.")).toBe(true);
    expect(proseAsksToReadOrHandleCards("**Read** the printed line once.")).toBe(true);
  });

  it("headings", () => {
    expect(headingStepIndex("Guided Practice — *read*, copy")).toBe(15);
    expect(headingStepIndex("Guided Practice: _write_ it")).toBe(15);
    expect(headingStepIndex("_Write_ is Old English *wrītan*")).toBe(-1);
  });
});

describe("headingStepIndex: a heading that asks for an eye or a hand", () => {
  it.each([
    ["Guided Practice — read, copy, recall", 15],
    ["Read two, say one", 0],
    ["Sounds you'll need: Read the cedilla", 18],
    ["Practice — hear, say, read, and write water", 20],
    ["(el mensaje del vecino) — read it, then answer", 23],
    ["Guided Practice — point and write", 15],
    // The second security review: a bare "point" ending a list item, spoken as a
    // lesson's opening line.
    ["Retrieval 6 — greet, repair, point", 27],
    ["Practice — point, then say it", 8],
    ["Guided Practice — copy the tone marks", 15],
  ])("finds the step in %j at %d", (heading, index) => {
    expect(headingStepIndex(heading)).toBe(index);
  });

  it.each([
    "Guided Practice",
    "You'll want to know: tapēte, tapētis",
    "Guided Practice: point every verb at somebody",
    "**Write** is Old English *wrītan*",
    "Hear *kànshū* — to read a book",
    "Retrieval 1 — look, see, or good-looking?",
    // Controls for the bare "point": a noun, and "point" with any other object.
    "Grammar Lens: spacing, and where the point goes",
    "Grammar Lens: no new grammar — that's the point",
    "Why it's said this way: one verified claim, one still just a single data point",
    "Guided Practice — copy the tone of the line",
  ])("finds none in %j", (heading) => {
    expect(headingStepIndex(heading)).toBe(-1);
  });

  it("stays linear on a long label", () => {
    const heading = `${"Guided Practice — reading, ".repeat(2_000)}done`;
    expect(headingStepIndex(heading)).toBe(-1);
  });
});

/** A drivable Hindi lesson: one kept section with the given title, and an optional scored activity in it. */
function drivableLesson(options: {
  title?: string;
  sectionTitle?: string;
  gloss?: string;
  prompt?: string;
  accepted?: string[];
  setAsideSection?: boolean;
}): ReturnType<typeof parseLesson> {
  const activity =
    options.prompt === undefined
      ? ""
      : `<!-- hl-activity: ${JSON.stringify({
          id: "HI-C01-test-check",
          kind: "text",
          assesses: ["HI-LEX-NAMASTE"],
          prompt: options.prompt,
          answer: "नमस्ते",
          accepted: options.accepted ?? ["namaste"],
          feedback: { correct: "Yes.", incorrect: "नमस्ते." },
          response_seconds: 12,
        })} -->\n`;
  return parseLesson(
    [
      "---",
      "schema_version: 2",
      "id: HI-C01-test",
      "chapter: 1",
      "type: word",
      "headword: नमस्ते",
      `gloss: ${options.gloss ?? "hello"}`,
      "concept_tag: GREETING-HELLO",
      "skills: [listening, speaking]",
      "---",
      "",
      `# ${options.title ?? "नमस्ते — hello"}`,
      "",
      "## Warm-up",
      "",
      "[PAUSE 2s] Say *namaste* out loud.",
      "",
      ...(options.setAsideSection
        ? ["## The letters in this word", "", "**न**, then **म**: the letters are new.", ""]
        : []),
      `## ${options.sectionTitle ?? "Wrap-up Recall"}`,
      "<!-- hl-knowledge: introduces=[]; assesses=[HI-LEX-NAMASTE] -->",
      activity,
      "[PAUSE 3s] What does it mean? (Hello.)",
      "",
    ].join("\n"),
    "hindi",
  );
}

/**
 * The gate, as a function of one committed narration: every prompt and heading a
 * driver would hear with no stop guard in front of it — the opening line, each
 * title of a section the notice does not set aside, each scored question asked
 * there — that still asks to read, write, type or point. Only for a lesson whose
 * core is `voice`: that is the lesson announced as one "you can do in the car".
 */
function spokenEyesOrHands(narration: LessonNarration): string[] {
  if (narration.coreModality !== "voice") return [];
  const problems: string[] = [];
  const opening = (renderLessonNarrationText(narration).split("\n")[0] ?? "").replace(/\.$/u, "");
  const context = { headword: narration.headword, gloss: narration.gloss };
  if (headingAsksForEyesOrHands(opening, context)) problems.push(`opening: ${opening}`);
  const setAside = new Set(narration.notice?.waitUntilStopped ?? []);
  for (const block of narration.blocks) {
    if (setAside.has(block.title)) continue;
    if (block.title !== "" && headingAsksForEyesOrHands(block.title)) problems.push(`heading: ${block.title}`);
    for (const segment of block.segments) {
      if (segment.kind !== "activity" || !segment.spoken) continue;
      if (activityNeedsEyesOrHands(segment).length > 0 || asksToType(segment.prompt)) {
        problems.push(`question: ${segment.prompt}`);
      }
    }
  }
  return problems;
}

describe("the narration defers a question that needs a pen, and says the rest for the ear", () => {
  it("defers 'Write …' in a lesson that stays drivable, with no answer gap", () => {
    const narration = narrateLesson(drivableLesson({ prompt: "Write hello in Devanagari." }));
    expect(narration.coreModality).toBe("voice");
    expect(narration.notice).toBeNull();
    const activity = narration.blocks.flatMap((block) => block.segments).find((s) => s.kind === "activity");
    expect(activity).toMatchObject({ kind: "activity", scored: true, spoken: false, prompt: "Write hello in Devanagari." });
    const text = renderLessonNarrationText(narration);
    expect(text).toContain("[once you have stopped driving — question: Write hello in Devanagari.]");
    expect(text).not.toContain("[question — say your answer");
    expect(spokenEyesOrHands(narration)).toEqual([]);
  });

  it("in a lesson with a part set aside, names the section and guards it like a deferred cue", () => {
    const narration = narrateLesson(
      drivableLesson({ prompt: "Type 'hello' in Devanagari script.", setAsideSection: true }),
    );
    expect(narration.coreModality).toBe("voice");
    expect(narration.notice?.waitUntilStopped).toEqual(["The letters in this word", "Wrap-up Recall"]);
    const lines = renderLessonNarrationText(narration).split("\n");
    expect(lines[lines.indexOf("Wrap-up Recall.") - 1]).toBe(STOP_GUARD);
  });

  it("defers a question whose every accepted answer is script, though its prompt asks for nothing", () => {
    const prompt = "Which greeting did you learn first?";
    const deferred = narrateLesson(drivableLesson({ prompt, accepted: ["नमस्ते जी"] }));
    const activity = deferred.blocks.flatMap((block) => block.segments).find((s) => s.kind === "activity");
    expect(activity).toMatchObject({ spoken: false, prompt });
    expect(renderLessonNarrationText(deferred)).toContain(`[once you have stopped driving — question: ${prompt}]`);
    // Control: the same lesson accepting "namaste" asks it.
    const asked = narrateLesson(drivableLesson({ prompt }));
    expect(asked.blocks.flatMap((block) => block.segments).find((s) => s.kind === "activity")).toMatchObject({
      spoken: true,
    });
  });

  it("opens with a review title's label when its list ends in a bare 'point'", () => {
    const narration = narrateLesson(drivableLesson({ title: "Retrieval 6 — greet, repair, point" }));
    expect(narration.title).toBe("Retrieval 6");
    expect(renderLessonNarrationText(narration).split("\n")[0]).toBe("Retrieval 6 — hello.");
    // Control: a headword–gloss title keeps its gloss, even when the gloss is "point".
    const sasu = narrateLesson(drivableLesson({ title: "नमस्ते — point", gloss: "point" }));
    expect(sasu.title).toBe("नमस्ते — point");
  });

  it("asks a typed word aloud, as 'Say …'", () => {
    const narration = narrateLesson(drivableLesson({ prompt: "Type the Hindi for 'hello', in rōmaji." }));
    const activity = narration.blocks.flatMap((block) => block.segments).find((s) => s.kind === "activity");
    expect(activity).toMatchObject({ spoken: true, prompt: "Say the Hindi for 'hello'." });
    const text = renderLessonNarrationText(narration);
    expect(text).toContain("[question — say your answer, then pause 12 seconds]\nSay the Hindi for 'hello'.");
    expect(text).not.toMatch(/\btype\b/i);
  });

  it("says a kept section's title as its label, and a lesson title without its asking", () => {
    const narration = narrateLesson(
      drivableLesson({
        title: "Practice — hear it, answer it, write it",
        sectionTitle: "Guided Practice — read, copy, recall",
      }),
    );
    expect(narration.title).toBe("Practice");
    expect(narration.blocks.map((block) => block.title)).toEqual(["Warm-up", "Guided Practice"]);
    const text = renderLessonNarrationText(narration);
    // "Practice" has no dash, so the gloss joins it, as for any such title.
    expect(text.split("\n")[0]).toBe("Practice — hello.");
    expect(text).not.toMatch(/read, copy|write it/);
    expect(spokenEyesOrHands(narration)).toEqual([]);
  });

  it("keeps a lesson's titles as authored when its notice already says it is not a driving lesson", () => {
    // The same lesson, authored `sight`: its notice opens "not a driving lesson"
    // before any title, so nothing in it is announced as drivable.
    const authored = drivableLesson({
      title: "Practice — hear it, answer it, write it",
      sectionTitle: "Guided Practice — read, copy, recall",
    });
    const sight = narrateLesson({
      ...authored,
      frontmatter: { ...authored.frontmatter, modality: "sight", modality_reason: "test" },
    });
    expect(sight.coreModality).toBe("sight");
    expect(sight.title).toBe("Practice — hear it, answer it, write it");
    expect(sight.blocks.at(-1)?.title).toBe("Guided Practice — read, copy, recall");
  });

  it("puts the stop guard before a set-aside section's title, which may itself ask", () => {
    const narration = narrateLesson(
      parseLesson(
        [
          "---",
          "schema_version: 2",
          "id: TA-C09-test",
          "chapter: 9",
          "type: word",
          "headword: மன்னிக்கவும்",
          "gloss: please forgive",
          "concept_tag: APOLOGY",
          "skills: [listening, speaking, reading]",
          "---",
          "",
          "# மன்னிக்கவும் — please forgive",
          "",
          "## Warm-up",
          "",
          "Say *maṉṉikkavum*.",
          "",
          "## Script you'll notice: Read ன்னி",
          "",
          "**ன்** + **னி** becomes **ன்னி**.",
          "",
        ].join("\n"),
        "tamil",
      ),
    );
    expect(narration.coreModality).toBe("voice");
    const lines = renderLessonNarrationText(narration).split("\n");
    const title = lines.indexOf("Script you'll notice: Read ன்னி.");
    expect(title).toBeGreaterThan(0);
    expect(lines[title - 1]).toBe(STOP_GUARD);
  });
});

/** One lesson of a committed `chNN.json`, as `narration-cli.ts` serializes it. */
interface CommittedLesson {
  id: string;
  title: string;
  headword: string;
  romanization: string;
  gloss: string;
  script: string;
  modality: LessonNarration["modality"];
  notice: LessonNarration["notice"];
  blocks: LessonNarration["blocks"];
}

// The committed narration, every chapter of every track: what ships, read the way a
// voice agent reads it. `check:narration` holds these files equal to the generator's
// output, so a gate here is a gate on the generator over the real corpus. The JSON
// does not carry the core modality, so it is read from the committed manifest —
// the same flag the gate in part 4 reads.
const narrationRoot = defaultCurriculumRoot();
const committedNarration: LessonNarration[] = readdirSync(narrationRoot).flatMap((track) => {
  const directory = join(narrationRoot, track, "narration");
  if (!existsSync(directory)) return [];
  return readdirSync(directory)
    .filter((name) => name.endsWith(".json"))
    .flatMap((name) => {
      const chapter = JSON.parse(readFileSync(join(directory, name), "utf8")) as {
        lessons: CommittedLesson[];
      };
      return chapter.lessons.map(
        (lesson): LessonNarration => ({
          lessonId: lesson.id,
          language: track,
          chapter: null,
          sequence: null,
          title: lesson.title,
          headword: lesson.headword,
          romanization: lesson.romanization,
          gloss: lesson.gloss,
          script: lesson.script,
          modality: lesson.modality,
          coreModality: coreDrivableIds.has(lesson.id) ? "voice" : lesson.modality,
          derivedModality: lesson.modality,
          modalityReasons: [],
          sourceHash: "",
          notice: lesson.notice,
          blocks: lesson.blocks,
          findings: [],
        }),
      );
    });
});

describe("no prompt or heading spoken in a drivable part of a core-drivable lesson asks for an eye or a hand", () => {
  it("reads the committed narration of a real corpus", () => {
    const coreDrivable = committedNarration.filter((narration) => narration.coreModality === "voice");
    expect(coreDrivable.length).toBeGreaterThan(committedNarration.length / 2);
    // And those lessons do ask questions, so the question half of the gate reads something.
    expect(
      coreDrivable.some((narration) =>
        narration.blocks.some((block) => block.segments.some((segment) => segment.kind === "activity")),
      ),
    ).toBe(true);
  });

  it("the committed narration of every core-drivable lesson speaks none, before any stop guard", () => {
    const problems = committedNarration.flatMap((narration) =>
      spokenEyesOrHands(narration).map((problem) => `${narration.lessonId}: ${problem.slice(0, 160)}`),
    );
    expect(problems, problems.join("\n")).toEqual([]);
  });

  it("every question the narration does ask carries a verdict, and every deferred one is said as a deferral", () => {
    const problems: string[] = [];
    for (const narration of committedNarration) {
      const text = renderLessonNarrationText(narration);
      for (const segment of narration.blocks.flatMap((block) => block.segments)) {
        if (segment.kind !== "activity") continue;
        if (typeof segment.spoken !== "boolean") problems.push(`${narration.lessonId}: ${segment.id} has no verdict`);
        if (!segment.spoken && !text.includes(`[once you have stopped driving — question: ${segment.prompt}]`)) {
          problems.push(`${narration.lessonId}: ${segment.id} is deferred in the JSON but not in the text`);
        }
      }
    }
    expect(problems, problems.join("\n")).toEqual([]);
  });

  // The zero above is the goal, so it cannot also be the proof that the gate can
  // fire. The proof is synthetic, on real lessons: put into a kept section of a
  // fixed sample of real core-drivable lessons each shape the renderer must handle,
  // narrate, and require (a) the renderer's answer to pass the gate and (b) the
  // answer the renderer used to give — the authored words, asked plainly — to fail
  // it. None of this depends on the corpus still holding such a prompt or heading.
  const sample = coreDrivableLessons
    .filter((lesson) => lesson.blocks.some((block) => !isDetachableBlock(block) && block.title !== ""))
    .sort((left, right) => left.realization.lessonId.localeCompare(right.realization.lessonId))
    .filter((_lesson, index) => index % 250 === 0);

  /** The last kept, titled section of a lesson: where the shapes are planted. */
  function plantingSite(lesson: (typeof lessons)[number]): number {
    let site = -1;
    lesson.blocks.forEach((block, index) => {
      if (!isDetachableBlock(block) && block.title !== "") site = index;
    });
    return site;
  }

  function withActivity(lesson: (typeof lessons)[number], prompt: string): (typeof lessons)[number] {
    const site = plantingSite(lesson);
    const activity: LessonActivity = {
      id: "planted-check",
      kind: "text",
      assesses: ["PLANTED-01"],
      prompt,
      answer: "x",
      accepted: [],
      feedback: { correct: "Yes.", incorrect: "No." },
      responseSeconds: 10,
    };
    return {
      ...lesson,
      blocks: lesson.blocks.map((block, index) =>
        index === site ? { ...block, activities: [...(block.activities ?? []), activity] } : block,
      ),
    };
  }

  it("samples real lessons", () => {
    expect(sample.length).toBeGreaterThan(20);
  });

  it.each([
    "Write the word for water in its own script.",
    "Type 'water' in hiragana.",
    "Read this pair aloud and give each meaning: 水 — 火.",
    "Refuse in one word, in Devanagari.",
    "Give the greeting in two written lines.",
    "Which sign is water: 水 or 火?",
  ])("a planted %j is deferred, and asking it plainly would fail the gate", (prompt) => {
    const misses: string[] = [];
    let demonstrated = 0;
    for (const lesson of sample) {
      const narration = narrateLesson(withActivity(lesson, prompt));
      const planted = narration.blocks.flatMap((block) => block.segments).find(
        (segment) => segment.kind === "activity" && segment.id === "planted-check",
      );
      if (planted?.kind !== "activity" || planted.spoken) {
        misses.push(`${lesson.realization.lessonId}: asked`);
        continue;
      }
      if (narration.coreModality !== "voice") continue;
      if (spokenEyesOrHands(narration).length > 0) misses.push(`${lesson.realization.lessonId}: gate fired`);
      // What the renderer used to say: the lesson as it was, with the question
      // asked plainly in that section (and so no guard added for it).
      const before = narrateLesson(lesson);
      const site = plantingSite(lesson);
      const siteBlock = before.blocks.find((block) => block.index === site);
      if (siteBlock === undefined || (before.notice?.waitUntilStopped ?? []).includes(siteBlock.title)) continue;
      demonstrated += 1;
      const asked: LessonNarration = {
        ...before,
        blocks: before.blocks.map((block) =>
          block.index === site ? { ...block, segments: [...block.segments, { ...planted, spoken: true }] } : block,
        ),
      };
      if (spokenEyesOrHands(asked).length === 0) misses.push(`${lesson.realization.lessonId}: gate blind`);
    }
    expect(demonstrated).toBeGreaterThan(10);
    expect(misses, misses.join("\n")).toEqual([]);
  });

  it("a planted 'Type the word for …' is asked as 'Say …', and left as 'Type' would fail the gate", () => {
    const misses: string[] = [];
    for (const lesson of sample) {
      const narration = narrateLesson(withActivity(lesson, "Type the word for 'water'."));
      if (narration.coreModality !== "voice") continue;
      const planted = narration.blocks.flatMap((block) => block.segments).find(
        (segment) => segment.kind === "activity" && segment.id === "planted-check",
      );
      if (planted?.kind !== "activity" || !planted.spoken || planted.prompt !== "Say the word for 'water'.") {
        misses.push(`${lesson.realization.lessonId}: not said for the ear`);
      }
      if (spokenEyesOrHands(narration).length > 0) misses.push(`${lesson.realization.lessonId}: gate fired`);
      const typed: LessonNarration = {
        ...narration,
        blocks: narration.blocks.map((block) => ({
          ...block,
          segments: block.segments.map((segment) =>
            segment.kind === "activity" && segment.id === "planted-check"
              ? { ...segment, prompt: "Type the word for 'water'." }
              : segment,
          ),
        })),
      };
      if (spokenEyesOrHands(typed).length === 0) misses.push(`${lesson.realization.lessonId}: gate blind`);
    }
    expect(misses, misses.join("\n")).toEqual([]);
  });

  it("a planted heading that asks is said as its label, and said whole would fail the gate", () => {
    const misses: string[] = [];
    let demonstrated = 0;
    for (const lesson of sample) {
      const site = plantingSite(lesson);
      const authored = `Guided Practice — read, copy, recall`;
      const planted = {
        ...lesson,
        blocks: lesson.blocks.map((block, index) => (index === site ? { ...block, title: authored } : block)),
      };
      const narration = narrateLesson(planted);
      if (narration.coreModality !== "voice") continue;
      const block = narration.blocks.find((candidate) => candidate.index === site);
      // A section the notice sets aside keeps its whole title behind the guard.
      if (block === undefined || (narration.notice?.waitUntilStopped ?? []).includes(block.title)) continue;
      demonstrated += 1;
      if (block.title !== "Guided Practice") misses.push(`${lesson.realization.lessonId}: said "${block.title}"`);
      if (spokenEyesOrHands(narration).length > 0) misses.push(`${lesson.realization.lessonId}: gate fired`);
      const whole: LessonNarration = {
        ...narration,
        blocks: narration.blocks.map((candidate) =>
          candidate.index === site ? { ...candidate, title: authored } : candidate,
        ),
      };
      if (spokenEyesOrHands(whole).length === 0) misses.push(`${lesson.realization.lessonId}: gate blind`);
    }
    expect(demonstrated).toBeGreaterThan(10);
    expect(misses, misses.join("\n")).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// 6. An independent gate: a plain word scan, with no detector in it
// ---------------------------------------------------------------------------
//
// Every gate above judges the committed narration with the SAME detectors the
// generator used to write it (`activityPromptNeedsEyesOrHands`,
// `headingAsksForEyesOrHands`). A gate that calls the detector it gates can only
// agree with it: when the prose checks under-fired on "From bachchā, write the
// taught word.", the generator asked it and the gate passed it, for the same
// reason. Found by a security review, with about fifty such questions live.
//
// So this gate shares nothing with the generator but the committed output. It is
// a plain, case-insensitive, whole-word scan for the hand verbs over every
// question asked (`spoken !== false`) and every section title said with no stop
// guard before it, in every core-drivable lesson. The verb list is written out
// here on purpose rather than imported: an edit that drops a verb from the
// generator must not drop it from the gate in the same keystroke.
//
// A second security review found what a verb scan cannot see: questions whose
// ANSWER is written though no hand verb is named ("Refuse in one word, in
// Devanagari.", "… in two written lines."), questions whose accepted answers
// are all script ("Which taught character is … woman?" — 女), a choice between
// shown glyphs ("… is na: ਨ or ਸ?"), and a review lesson's opening line
// ("Retrieval 6 — greet, repair, point."). So the scan reads three more things,
// each with its own literal copy of what it looks for:
//
//   - written answer forms ("in writing", "written lines", "in Devanagari", "in
//     characters", …) in questions and titles alike;
//   - for each question, what it accepts and what it assesses, straight from the
//     committed JSON: a question is a hit when every accepted answer holds a
//     letter of a non-Latin script, or when it assesses a `-SCRIPT-` atom and its
//     canonical answer does, or when it offers a choice between two runs of
//     unromanized script;
//   - the opening line, when the lesson's title is not a headword title.
//
// What the scan does NOT read, and why:
//   - "read" and "type". Both are everywhere in questions a driver can answer
//     ("How is 日 read inside 日本語?", "its borrowing type"), so a plain scan
//     would be nothing but allowlist. They have their own shape checks above,
//     and spoken questions are separately held to `asksToType` returning false.
//   - the opening line of a HEADWORD title: one whose label (before its first
//     " — ") holds the lesson's headword as a whole word. Its tail is the word's
//     meaning, and about a hundred meanings name a hand verb ("かく (kaku) — to
//     write", "صنبور (ṣunbūr) — a tap", "el punto — the point, from the prick of
//     a needle"); a word scan cannot tell the course's subject from a request,
//     and the heading check in part 5 judges that line. Every other opening line
//     — a review lesson's "Retrieval 6 — …", a lesson named for its task — is
//     scanned like a section title.
//
// The allowlist is exact: lesson id and the text as spoken. A new question or
// title that names a hand verb fails this test until a person has read it and
// either reworded it, let the generator defer it, or added it here with a reason.
// An entry that no longer matches anything fails too, so the list cannot rot.

/** The hand verbs, as whole words. A deliberate copy: see the header above. */
const PLAIN_HAND_VERB =
  /(?<![\p{L}\p{M}\p{N}])(?:re-?write|write|jot|copy|trace|draw|sketch|record|note down|transcribe|circle|underline|tick|check off|cross out|highlight|fill in|fill out|select|point|look at|tap|clap)(?![\p{L}\p{M}\p{N}])/iu;

/** Written answer forms, and a script named as an answer's form. A deliberate copy too. */
const PLAIN_WRITTEN_FORM =
  /(?<![\p{L}\p{M}])(?:in writing|on paper|written (?:lines?|form|answers?|sentences?|responses?|reply|version)|(?:in|into|using|as) (?:the )?(?:\S+ )?(?:devanagari|gurmukhi|shahmukhi|nastaliq|naskh|perso-arabic|cyrillic|hangul|kana|hiragana|katakana|kanji|hanzi|characters|script))(?![\p{L}\p{M}])/iu;

/**
 * A letter of a script other than Latin, written as a set difference (the
 * generator writes it as a lookahead; the two must agree on the corpus without
 * sharing a source). Common and Inherited are left out: digits, punctuation, the
 * length bar ー and combining marks are no script anyone has to read.
 */
const PLAIN_SCRIPT_LETTER = new RegExp(
  String.raw`[\p{L}--[\p{Script=Latin}\p{Script=Common}\p{Script=Inherited}]]`,
  "v",
);

/** Two unromanized script runs offered as a choice: "ਨ or ਸ", "中文 / 中国". */
const PLAIN_GLYPH_CHOICE = new RegExp(
  String.raw`[\p{L}--[\p{Script=Latin}\p{Script=Common}\p{Script=Inherited}]]\p{M}*['’"”]?(?:,? or | ?\/ ?| vs\.? | versus )['‘"“]?[\p{L}--[\p{Script=Latin}\p{Script=Common}\p{Script=Inherited}]]`,
  "v",
);

/**
 * Spoken text that names a hand verb and is not a request, keyed by lesson id.
 * Each entry says why. Grouped by why, so a reviewer can check the reasons once.
 */
const SPOKEN_HAND_VERB_ALLOWLIST: ReadonlyMap<string, readonly string[]> = new Map([
  // A question that asks for a verb's MEANING: the verb is the answer's gloss,
  // inside quotes or after "meaning/meant to". Nothing is to be written.
  ["JA-C27-kaku", ["Say the Japanese for 'to write'."]],
  ["JA-C87-sasu", ["Say the Japanese for 'to point'."]],
  ["JA-C88-utsusu", ["Say the Japanese for 'to copy'."]],
  ["MW-C57-likhno", ["Say the Marwadi for 'to write'."]],
  ["UR-C07-likhna", ["Say the Urdu verb meaning 'to write'."]],
  ["PA-C08-likhna", ["What did the Sanskrit root behind likhna mean before it meant to write?"]],
  // A grammar question whose subject is the WORD: "does it point" asks what a
  // demonstrative refers to, and the answer is said ("my" / "near").
  ["MW-C05-hear-tharo", ["You hear thāro. Does it point to my or your?"]],
  ["UR-C17-yih", ["Does یہ (yih) point at what is near you or what is far from you?"]],
  // "copy" as a noun, in an etymology question answered aloud.
  [
    "ES-C15-tener-preterite",
    ["Which phrase best describes tuve: a letter-for-letter copy of tenuī, or a historically reshaped strong form?"],
  ],
  // Section titles that use the word as a noun ("the point", "a circle"): a
  // label, introducing an explanation that is spoken.
  ["AR-C04-maa-salama", ["You'll want to know — The circle closes"]],
  ["HI-C89-kar-raha-hun", ["Grammar Lens: the stem is bare, and that is the point"]],
  ["KA-C31-shubha-madhyahna", ["Why it's said this way: one verified claim, one still just a single data point"]],
  ["ES-C07-beber", ["Grammar Lens: no new grammar — that's the point"]],
  ["ES-C424-parentesis", ["Grammar Lens: spacing, and where the point goes"]],
  ["ES-C467-producir", ["Grammar Lens: nothing new here, which is the point"]],
  ["ES-C471-demasiado", ["Grammar Lens: the word that says it went past the point"]],
  // Section titles whose verb has a third-person subject — sources, suffixes, a
  // chapter, a track — describing, not asking.
  ["KA-C29-shubhodaya", ["Why it's said this way: sources conflict, and the careful ones point to Hindi's own story"]],
  ["ES-C466-visible", ["Grammar Lens: -able and -ible point two ways"]],
  ["PT-C28-second-pass-finished-or-ongoing", ["What you've built — the goodbyes draw the same line"]],
  ["TE-C37-noru", ["The word, taken apart - the one place this chapter cannot draw a clean line"]],
  ["FA-C09-nan", ["The word, taken apart — a root this track cannot trace"]],
  // Section titles that NAME writing as the topic of a spoken explanation:
  // spelling variants, a nasal sign, a Latin verb quoted with its meaning.
  ["HI-C93-ke-binaa", ["Grammar Lens: two ways to write it"]],
  ["HI-C91-kitaben", ["Grammar Lens: the nasal you already write"]],
  ["LA-C38-scribo", ["You'll want to know: scrībō — \"I write\""]],
  // "point every verb at somebody" is grammatical: give each reflexive verb a
  // person. The section's drills are said aloud; no hand points at anything.
  ["GE-R37-mich-dich-sich", ["Guided Practice: point every verb at somebody"]],
  // Section titles that NAME a written form or a record as the topic of a spoken
  // explanation; nothing is to be written.
  ["PT-C28-indicador", ["You'll want to know: The written form"]],
  ["ES-R465-repaso-apunte", ["Grammar Lens: an impression is not a record"]],
  // Opening lines of review lessons (not headword titles) that describe what the
  // lesson revisits: the letters "write" a word as a spelling does, and a dative
  // frame has "somebody to point at" grammatically. Said, not asked.
  [
    "AR-R23-please-sorry-thank-you",
    [
      "Please, sorry, thank you — the courtesy kit retrieved whole — yes, please, sorry and thank you, the two letters that write the last of them, and the fact that three of the four bend to the person you are speaking to.",
    ],
  ],
  [
    "BN-R25-the-one-it-happens-to",
    [
      "The one it happens to — one ending, one question word written identically, and a dative frame that finally has somebody to point at.",
    ],
  ],
]);

/** A letter or a mark: what may not touch a whole word on either side. */
const WORD_CHAR = /[\p{L}\p{M}]/u;

/**
 * Is a lesson's title a headword title: its label, before any " — ", holds the
 * headword's first word as a whole word? A plain `indexOf` walk with a fixed
 * neighbour test: compiling a Unicode regex per lesson cost 25 s over the corpus.
 */
function isHeadwordTitle(title: string, headword: string): boolean {
  const first = headword.split(/[\s·・\/,—]+/u).find((word) => word !== "") ?? "";
  if (first === "") return false;
  const dash = title.indexOf(" — ");
  const label = dash === -1 ? title : title.slice(0, dash);
  for (let at = label.indexOf(first); at !== -1; at = label.indexOf(first, at + 1)) {
    const before = label.slice(0, at).at(-1) ?? "";
    const after = label.slice(at + first.length)[0] ?? "";
    if (!WORD_CHAR.test(before) && !WORD_CHAR.test(after)) return true;
  }
  return false;
}

/** One text the plain scan reads; a question carries what it accepts and assesses. */
interface PlainScanTarget {
  kind: "question" | "heading" | "opening";
  text: string;
  accepted?: readonly string[];
  assesses?: readonly string[];
}

/** Every text a driver hears in a drivable part of a core-drivable lesson that the plain scan reads. */
function plainScanTargets(narration: LessonNarration): PlainScanTarget[] {
  if (narration.coreModality !== "voice") return [];
  const setAside = new Set(narration.notice?.waitUntilStopped ?? []);
  const targets: PlainScanTarget[] = [];
  if (!isHeadwordTitle(narration.title, narration.headword)) {
    const opening = renderLessonNarrationText(narration).split("\n")[0] ?? "";
    if (opening !== "") targets.push({ kind: "opening", text: opening });
  }
  for (const block of narration.blocks) {
    if (setAside.has(block.title)) continue;
    if (block.title !== "") targets.push({ kind: "heading", text: block.title });
    for (const segment of block.segments) {
      if (segment.kind === "activity" && segment.spoken !== false) {
        targets.push({
          kind: "question",
          text: segment.prompt,
          accepted: segment.acceptedResponses,
          assesses: segment.assesses,
        });
      }
    }
  }
  return targets;
}

/** Why a target is a hit, or null. Each test is a literal of this file; none calls the generator. */
function plainHitReason(target: PlainScanTarget): string | null {
  if (PLAIN_HAND_VERB.test(target.text)) return "hand verb";
  if (PLAIN_WRITTEN_FORM.test(target.text)) return "written form";
  if (target.kind !== "question") return null;
  if (PLAIN_GLYPH_CHOICE.test(target.text)) return "choice among shown glyphs";
  const accepted = target.accepted ?? [];
  if (accepted.length > 0 && accepted.every((answer) => PLAIN_SCRIPT_LETTER.test(answer))) return "accepts only script";
  const canonical = accepted[0] ?? "";
  if ((target.assesses ?? []).some((atom) => atom.includes("-SCRIPT-")) && PLAIN_SCRIPT_LETTER.test(canonical)) {
    return "grades a script atom in script";
  }
  return null;
}

/** The plain scan over one narration: every target that is a hit and not allowlisted for its lesson. */
function plainHandVerbHits(narration: LessonNarration, used?: Set<string>): string[] {
  const allowed = SPOKEN_HAND_VERB_ALLOWLIST.get(narration.lessonId) ?? [];
  const hits: string[] = [];
  for (const target of plainScanTargets(narration)) {
    const reason = plainHitReason(target);
    if (reason === null) continue;
    if (allowed.includes(target.text)) {
      used?.add(`${narration.lessonId}\u0000${target.text}`);
      continue;
    }
    hits.push(`${narration.lessonId}: ${target.kind} (${reason}): ${target.text}`);
  }
  return hits;
}

describe("a plain word scan, independent of every detector, finds no hand verb spoken at the wheel", () => {
  it("reads thousands of questions, titles and opening lines", () => {
    const targets = committedNarration.flatMap(plainScanTargets);
    expect(targets.filter((target) => target.kind === "question").length).toBeGreaterThan(1_000);
    expect(targets.filter((target) => target.kind === "heading").length).toBeGreaterThan(1_000);
    // Review lessons and lessons named for a task: hundreds, not the whole corpus,
    // because a headword title's tail is its meaning (see the header).
    const openings = targets.filter((target) => target.kind === "opening").length;
    expect(openings).toBeGreaterThan(100);
    expect(openings).toBeLessThan(targets.filter((target) => target.kind === "heading").length);
    // And the questions carry what they accept, so the structural checks read something.
    expect(
      targets.filter((target) => target.kind === "question" && (target.accepted ?? []).length > 0).length,
    ).toBeGreaterThan(1_000);
  });

  it("tells a headword title from any other", () => {
    expect(isHeadwordTitle("かく (kaku) — to write", "かく")).toBe(true);
    expect(isHeadwordTitle("hic — the near point of a three-way system", "hic, haec, hoc")).toBe(true);
    expect(isHeadwordTitle("शनिवार and रविवार — Saturday and Sunday", "शनिवार रविवार")).toBe(true);
    expect(isHeadwordTitle("Retrieval 6 — greet, repair, point", "こんにちは・よくわかりません・て・おなか")).toBe(false);
    expect(isHeadwordTitle("Retrieval 7 — from あ to せなか", "あ・ど・もう・みみ・せなか")).toBe(false);
    // Whole words only: "a" is not found inside "Guided practice".
    expect(isHeadwordTitle("Guided practice — tap it", "a torneira")).toBe(false);
  });

  it("the committed narration names none outside the allowlist, and every allowlist entry is still used", () => {
    const used = new Set<string>();
    const hits = committedNarration.flatMap((narration) => plainHandVerbHits(narration, used));
    expect(hits, hits.join("\n")).toEqual([]);
    const stale = [...SPOKEN_HAND_VERB_ALLOWLIST].flatMap(([lessonId, texts]) =>
      texts.filter((text) => !used.has(`${lessonId}\u0000${text}`)).map((text) => `${lessonId}: ${text}`),
    );
    expect(stale, `stale allowlist entries — remove them:\n${stale.join("\n")}`).toEqual([]);
  });

  it("the security reviews' questions are deferred in the committed narration", () => {
    const expected = new Map([
      ["MW-C09-bachcha-dictation", "From bachchā, write the taught word."],
      ["MW-C05-kain-delayed", "After hiding the model, write kāĩ and give its meaning."],
      ["ZH-C12-practice-family-payoff", "After all four checks pass, write home, person, and family members."],
      ["ZH-C19-shima-write", "Write is that so, without pinyin."],
      ["MR-R18-script-warmup-check", "Hear dhanyavad; give its meaning and write it in Devanagari."],
      ["JA-C09-ichido-dictation", "From the heard morae i | chi | do, write once in hiragana."],
      // The second review: written answer forms, a script-only answer, a choice
      // between shown glyphs.
      ["MW-C39-refusal-four-close", "Refuse an offer and then leave, in two written lines."],
      ["MW-R39-refusal-second-pass-close", "Refuse the offer, then leave the door open — two written lines."],
      ["MW-R39-refuse-price-turn", "Someone says das karo. Refuse in one word, in Devanagari."],
      ["JA-R12-foundation-02-card", "Record the retrieval string: e, five, mouth, please say it, hair, eye."],
      ["PA-R28-head-na-r4-script", "Which already-taught Gurmukhi consonant is na: ਨ or ਸ?"],
      ["ZH-C12-nu-meaning", "Which taught character is the Level 1 word female or woman?"],
    ]);
    const found = new Map<string, { spoken: boolean; prompt: string }>();
    for (const narration of committedNarration) {
      for (const segment of narration.blocks.flatMap((block) => block.segments)) {
        if (segment.kind === "activity" && expected.has(segment.id)) found.set(segment.id, segment);
      }
    }
    for (const [id, prompt] of expected) {
      expect(found.get(id), id).toMatchObject({ spoken: false, prompt });
    }
  });

  it("the review lesson whose title ended in a bare 'point' opens with its label", () => {
    const retrieval = committedNarration.find((narration) => narration.lessonId === "JA-R12-repair-01");
    expect(retrieval?.coreModality).toBe("voice");
    expect(retrieval?.title).toBe("Retrieval 6");
    const opening = renderLessonNarrationText(retrieval as LessonNarration).split("\n")[0];
    expect(opening).toBe("Retrieval 6 — distant greeting and repair retrieval.");
    // The authored title, spoken whole, is what the scan exists to catch.
    const whole = { ...(retrieval as LessonNarration), title: "Retrieval 6 — greet, repair, point" };
    expect(plainHandVerbHits(whole)).toEqual(["JA-R12-repair-01: opening (hand verb): Retrieval 6 — greet, repair, point."]);
  });

  // Non-vacuity, without depending on the corpus still holding a bad question:
  // plant one into a kept section of real core-drivable narrations, asked, and
  // require the scan to find it — and not to excuse it by the lesson's allowlist.
  it("finds a planted question or title in a kept section of real lessons", () => {
    const sample = committedNarration
      .filter((narration) => narration.coreModality === "voice")
      .filter((narration) => plainScanTargets(narration).some((target) => target.kind === "heading"))
      .filter((_narration, index) => index % 300 === 0);
    expect(sample.length).toBeGreaterThan(5);
    const blind: string[] = [];
    for (const narration of sample) {
      const setAside = new Set(narration.notice?.waitUntilStopped ?? []);
      const site = narration.blocks.findIndex((block) => block.title !== "" && !setAside.has(block.title));
      const planted = (block: LessonNarration["blocks"][number]): LessonNarration => ({
        ...narration,
        blocks: narration.blocks.map((candidate, index) => (index === site ? block : candidate)),
      });
      const original = narration.blocks[site];
      if (original === undefined) continue;
      const question: NarrationActivity = {
        kind: "activity",
        scored: true,
        spoken: true,
        id: "planted",
        prompt: "From bachchā, write the taught word.",
        assesses: [],
        responseSeconds: 10,
        acceptedResponses: [],
        feedback: { correct: "Yes.", incorrect: "No." },
      };
      const withQuestion = planted({ ...original, segments: [...original.segments, question] });
      const withTitle = planted({ ...original, title: `${original.title} — copy it out` });
      if (plainHandVerbHits(withQuestion).length === 0) blind.push(`${narration.lessonId}: question`);
      if (plainHandVerbHits(withTitle).length === 0) blind.push(`${narration.lessonId}: heading`);
      // The second review's shapes, each planted alone into a clean question:
      // none names a hand verb, so only the new checks can find them.
      const clean = { ...question, prompt: "Which one did you learn first?", acceptedResponses: ["namaste"] };
      const shapes: [string, NarrationActivity][] = [
        ["written form", { ...clean, prompt: "Refuse in one word, in Devanagari." }],
        ["written lines", { ...clean, prompt: "Refuse, then leave, in two written lines." }],
        ["glyph choice", { ...clean, prompt: "Which sign is na: ਨ or ਸ?" }],
        ["script-only answers", { ...clean, acceptedResponses: ["女", "女 nǚ"] }],
        ["script atom", { ...clean, assesses: ["MW-SCRIPT-KONI-01"], acceptedResponses: ["कोनी", "koni"] }],
      ];
      for (const [label, shape] of shapes) {
        const withShape = planted({ ...original, segments: [...original.segments, shape] });
        if (plainHandVerbHits(withShape).length === plainHandVerbHits(narration).length) {
          blind.push(`${narration.lessonId}: ${label}`);
        }
      }
      // The clean question itself is no hit: each shape above is what the scan found.
      const withClean = planted({ ...original, segments: [...original.segments, clean] });
      if (plainHandVerbHits(withClean).length !== plainHandVerbHits(narration).length) {
        blind.push(`${narration.lessonId}: counted the clean control`);
      }
      // An opening line that is not a headword title is scanned; a headword title's is not.
      const review = { ...narration, title: "Retrieval 9 — greet, then point", headword: "x-not-in-title" };
      if (!plainHandVerbHits(review).some((hit) => hit.includes(": opening "))) blind.push(`${narration.lessonId}: opening`);
      const word = { ...narration, title: "かく (kaku) — to write", headword: "かく", gloss: "to write" };
      if (plainHandVerbHits(word).some((hit) => hit.includes(": opening "))) {
        blind.push(`${narration.lessonId}: scanned a headword title`);
      }
      // And the same question, deferred, is not a hit: the scan reads what is ASKED.
      const deferred = planted({ ...original, segments: [...original.segments, { ...question, spoken: false }] });
      if (plainHandVerbHits(deferred).length !== plainHandVerbHits(narration).length) {
        blind.push(`${narration.lessonId}: counted a deferred question`);
      }
    }
    expect(blind, blind.join("\n")).toEqual([]);
  });
});
