// A Writing or Script block whose writing stage shows the learner NO model
// (a dictation, a composition, a timed task) never takes the strip.
//
// The book prints a strip at the TOP of its block, under the heading and above
// the task. In a dictation that is the answer, drawn large, above the cue:
// ES-W00-hola-dictation printed "How hola is written, letter by letter" right
// above "Hear: OH-la. Write the Spanish greeting from that sound alone", and
// SA-S02-dictation printed न's numbered strokes a few lines under "no stroke
// order in front of you". See "A Writing block that is a dictation" in
// src/figure-targets.ts and the matching HL06 section.
import { describe, expect, it } from "vitest";
import { resolvedFigureTargets } from "../../src/figure-cli.js";
import {
  filmstripBlockIndex,
  filmstripCandidates,
  letterBlockIndex,
  stripBlockIndex,
  withFilmstripImages,
  writingLetterOf,
} from "../../src/figure-targets.js";
import { writtenLettersOf } from "../../src/letter-anchoring.js";
import { defaultCurriculumRoot, loadLessons } from "../../src/loader.js";
import type { ParsedLesson } from "../../src/parse.js";
import { lesson } from "./fixture.js";

const NO_MODEL_STAGES = [
  "dictation-transcription",
  "controlled-composition",
  "connected-composition",
  "timed-assessment-production",
];

/** A lesson whose blocks are `titles`, with `stages[i]` on block i when given. */
function staged(id: string, titles: string[], stages: Record<number, string>, headword = "ब"): ParsedLesson {
  const built = lesson(id, { language: "marathi", headword, blocks: titles });
  for (const [index, stage] of Object.entries(stages)) {
    (built.blocks[Number(index)] as { writingStage?: string }).writingStage = stage;
  }
  return built;
}

describe("no strip above a dictation", () => {
  for (const stage of NO_MODEL_STAGES) {
    it(`skips a Writing block whose stage is ${stage}`, () => {
      const blind = staged("MR-W1", ["Warm-up", "Writing — task", "Wrap-up Recall"], { 1: stage });
      expect(stripBlockIndex(blind)).toBe(-1);
      expect(writingLetterOf(blind)).toBeUndefined();
      expect(filmstripCandidates([blind])).toEqual([]);
    });
  }

  it("moves to the Script block that shows the model the dictation covers", () => {
    // MR-W03-ba: Script prints ब and its strokes; "Writing — heard cue" says
    // "Cover the model. [YOU HEAR: ba]".
    const marathi = staged(
      "MR-W2",
      ["Warm-up", "Script", "Writing — heard cue", "Wrap-up Recall"],
      { 2: "dictation-transcription" },
    );
    expect(stripBlockIndex(marathi)).toBe(1);
    const [placed] = withFilmstripImages([marathi], filmstripCandidates([marathi]));
    expect(placed!.blocks[1]!.markdown).toContain("filmstrip");
    expect(placed!.blocks[2]!.markdown).not.toContain("filmstrip");
  });

  it("skips a Script block that is itself a dictation", () => {
    const both = staged("MR-W3", ["Script — recall", "Guided Practice"], {
      0: "dictation-transcription",
      1: "guided-copy",
    });
    expect(stripBlockIndex(both)).toBe(1);
  });

  it("keeps a Writing block that declares a modelled stage, or none", () => {
    for (const stage of ["observe-trace", "guided-copy", "delayed-copy"]) {
      expect(stripBlockIndex(staged("MR-W4", ["Warm-up", "Writing — copy"], { 1: stage }))).toBe(1);
    }
    expect(stripBlockIndex(staged("MR-W5", ["Warm-up", "Writing: ब"], {}))).toBe(1);
  });

  it("still counts a single-letter dictation as a letter lesson for letter anchoring", () => {
    // Anchoring asks whether the lesson writes the letter, not whether it
    // prints a strip, so it is unchanged by this rule.
    const dictation = staged("SA-S1", ["Warm-up", "Writing — dictation"], { 1: "dictation-transcription" }, "न");
    expect(letterBlockIndex(dictation)).toBe(1);
    expect(stripBlockIndex(dictation)).toBe(-1);
    expect(writtenLettersOf(dictation)).toEqual(["न"]);
  });
});

describe("no-model Writing blocks in the real corpus", () => {
  const root = defaultCurriculumRoot();
  const lessons = loadLessons(root);
  const byId = new Map(lessons.map((entry) => [entry.realization.lessonId, entry]));
  const drawn = new Set(
    resolvedFigureTargets(root, lessons)
      .filter((target) => target.kind === "script-filmstrip")
      .map((target) => target.lessonId),
  );

  it("prints no strip in a block that shows no model", () => {
    const blind = [...drawn].filter((id) => {
      const entry = byId.get(id)!;
      const stage = entry.blocks[filmstripBlockIndex(entry)]?.writingStage;
      return stage !== undefined && NO_MODEL_STAGES.includes(stage);
    });
    expect(blind).toEqual([]);
  });

  it("drops the 33 dictation and composition lessons that printed one", () => {
    // Measured when the rule landed: every one has a fully cited headword, so
    // only the rule keeps it out. Their earlier copy lessons keep their strips.
    for (const id of [
      "AR-W04-arbaa-sutur",
      "ES-W00-hola-dictation",
      "ES-W01-frase-propia",
      "ES-W02-cuatro-lineas-ayer",
      "FA-W00-alef-dictation",
      "FR-W01-salut-dictation",
      "FR-W04-quatre-lignes",
      "GE-W01-hallo-dictation",
      "GE-W04-vier-zeilen",
      "GU-R03-doorway-three-r1",
      "GU-R04-doorway-nine-r2",
      "GU-R04-first-four-r1",
      "GU-R05-second-four-r1",
      "GU-R06-first-four-r1",
      "GU-R07-second-four-r1",
      "GU-R13-doorway-nine-r3",
      "GU-R15-chha-r4",
      "GU-R15-ka-r4",
      "GU-R15-nna-r4",
      "GU-R15-sha-r4",
      "GU-R15-u-matra-r4",
      "GU-R19-doorway-nine-r4",
      "GU-W01-haa-dictation",
      "IT-W01-ciao-dictation",
      "KA-S01-dictation",
      "LA-W04-quattuor-versus",
      "ML-W01-na-ma-dictation",
      "PT-W01-ola-dictation",
      "SA-S02-dictation",
      "SA-W03-mama-dictation",
      "SA-W03-mama-nama-dictation",
      "SA-W05-vocalic-r-dictation",
      "TE-S01-dictation",
    ]) {
      expect(byId.has(id), id).toBe(true);
      expect(drawn.has(id), id).toBe(false);
    }
    for (const id of ["ES-W00-hola-delayed-copy", "SA-S02-letter-na", "GU-W01-haa-delayed-copy"]) {
      expect(drawn.has(id), id).toBe(true);
    }
  });

  it("moves the Marathi heard-cue lessons' strip up to their Script block", () => {
    for (const id of ["MR-W03-ba", "MR-W03-lla", "MR-W03-va", "MR-W03-ya"]) {
      const entry = byId.get(id)!;
      expect(drawn.has(id), id).toBe(true);
      expect(entry.blocks[filmstripBlockIndex(entry)]!.title, id).toBe("Script");
    }
  });
});

describe("a lesson that prints no filmstrip", () => {
  // The converse of a-strip-lesson-never-disclaims-its-stroke-order: the
  // prose written for strip lessons ("follow the numbered strip", "The strip
  // shows where to start") must not be left on a lesson whose strip has gone.
  const FIGURE_PROSE = /numbered strip|\bthe strip (?:shows|names|writes|gives|draws)\b/i;
  const root = defaultCurriculumRoot();
  const lessons = loadLessons(root);
  const drawn = new Set(
    resolvedFigureTargets(root, lessons)
      .filter((target) => target.kind === "script-filmstrip")
      .map((target) => target.lessonId),
  );

  it("never tells the learner to follow a strip", () => {
    const orphaned = lessons
      .filter((entry) => !drawn.has(entry.realization.lessonId) && FIGURE_PROSE.test(entry.body))
      .map((entry) => entry.realization.lessonId);
    expect(orphaned).toEqual([]);
  });

  it("is a live check: strip lessons do use that wording", () => {
    expect(lessons.some((entry) => drawn.has(entry.realization.lessonId) && FIGURE_PROSE.test(entry.body))).toBe(true);
  });
});
