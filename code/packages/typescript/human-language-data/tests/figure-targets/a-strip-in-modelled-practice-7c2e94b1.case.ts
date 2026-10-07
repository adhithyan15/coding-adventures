// A writing lesson with no Writing or Script block prints its strip in the
// first practice block that SHOWS the learner a model, and only there.
//
// The writing stage (`<!-- hl-writing-stage: … -->`, HL19) is what decides,
// not the block title: "Guided Practice" heads a copy beside the model in
// ZH-W16-han-guided and a dictation with no model at all in
// ZH-R17-writing-five. See "A lesson with no Writing or Script block" in
// src/figure-targets.ts and the matching HL06 section.
import { describe, expect, it } from "vitest";
import { resolvedFigureTargets } from "../../src/figure-cli.js";
import {
  MODELLED_WRITING_STAGES,
  filmstripCandidates,
  filmstripImageMarkdown,
  letterBlockIndex,
  stripBlockIndex,
  withFilmstripImages,
  writingLetterOf,
} from "../../src/figure-targets.js";
import { defaultCurriculumRoot, loadLessons } from "../../src/loader.js";
import type { ParsedLesson } from "../../src/parse.js";
import { lesson } from "./fixture.js";

/** A Warm-up / Guided Practice / Wrap-up lesson whose practice block has `stage`. */
function practice(id: string, stage: string | undefined, headword = "汉"): ParsedLesson {
  const built = lesson(id, {
    language: "chinese",
    headword,
    blocks: ["Warm-up", "Guided Practice", "Wrap-up Recall"],
  });
  if (stage !== undefined) (built.blocks[1] as { writingStage?: string }).writingStage = stage;
  return built;
}

describe("a strip in modelled practice", () => {
  it("names exactly the three stages that show a model", () => {
    expect([...MODELLED_WRITING_STAGES].sort()).toEqual(["delayed-copy", "guided-copy", "observe-trace"]);
  });

  for (const stage of ["observe-trace", "guided-copy", "delayed-copy"]) {
    it(`puts the strip at the top of a ${stage} block`, () => {
      const copy = practice("ZH-W1", stage);
      expect(writingLetterOf(copy)).toBe("汉");
      const targets = filmstripCandidates([copy]);
      expect(targets).toHaveLength(1);
      const [placed] = withFilmstripImages([copy], targets);
      expect(placed!.blocks[1]!.markdown.startsWith(filmstripImageMarkdown(targets[0]!))).toBe(true);
      expect(placed!.blocks.filter((block) => block.markdown.includes("filmstrip")).length).toBe(1);
    });
  }

  for (const stage of [
    "dictation-transcription",
    "controlled-composition",
    "connected-composition",
    "timed-assessment-production",
  ]) {
    it(`never prints one where the learner writes without a model (${stage})`, () => {
      const blind = practice("ZH-R1", stage);
      expect(stripBlockIndex(blind)).toBe(-1);
      expect(writingLetterOf(blind)).toBeUndefined();
      expect(filmstripCandidates([blind])).toEqual([]);
    });
  }

  it("never prints one in a practice block that declares no stage, whatever its title", () => {
    // PA-W09-date-select's "Guided Practice — decide before writing".
    const selector = practice("PA-W9", undefined, "ਕ · ਖ");
    selector.language = "punjabi";
    expect(stripBlockIndex(selector)).toBe(-1);
    expect(filmstripCandidates([selector])).toEqual([]);
  });

  it("chooses the first modelled block, past an earlier dictation", () => {
    const mixed = practice("ZH-W2", "guided-copy");
    mixed.blocks.splice(1, 0, { ...mixed.blocks[1]!, title: "Guided Practice — Spaced Return", writingStage: "dictation-transcription" } as never);
    expect(stripBlockIndex(mixed)).toBe(2);
  });

  it("is only a fallback: a Writing or Script block still wins", () => {
    const both = lesson("ZH-W3", {
      language: "chinese",
      headword: "汉",
      blocks: ["Guided Practice", "Script — the shape", "Wrap-up Recall"],
    });
    (both.blocks[0] as { writingStage?: string }).writingStage = "guided-copy";
    expect(stripBlockIndex(both)).toBe(1);
    const [placed] = withFilmstripImages([both], filmstripCandidates([both]));
    expect(placed!.blocks[0]!.markdown).not.toContain("filmstrip");
    expect(placed!.blocks[1]!.markdown).toContain("filmstrip");
  });

  it("does not make a copy lesson a letter lesson for letter anchoring", () => {
    // letter-anchoring.ts counts lessons with a Writing or Script block only.
    const copy = practice("ZH-W4", "guided-copy");
    expect(letterBlockIndex(copy)).toBe(-1);
    expect(stripBlockIndex(copy)).toBe(1);
  });
});

describe("modelled-practice strips in the real corpus", () => {
  const root = defaultCurriculumRoot();
  const lessons = loadLessons(root);
  const byId = new Map(lessons.map((entry) => [entry.realization.lessonId, entry]));
  const drawn = new Set(
    resolvedFigureTargets(root, lessons)
      .filter((target) => target.kind === "script-filmstrip")
      .map((target) => target.lessonId),
  );

  it("draws exactly the copy lessons whose headword is cited", () => {
    // Measured when the fallback landed: 64 writing lessons on switched-on
    // tracks had no Writing or Script block, 30 had a fully cited headword,
    // and these 25 have a guided- or delayed-copy block to put the strip in.
    const viaPractice = [...drawn]
      // (A DECLARED target on a word lesson, FA-C03-chist, has neither block
      // either; it is placed by the block that introduces its letter.)
      .filter((id) => letterBlockIndex(byId.get(id)!) === -1 && stripBlockIndex(byId.get(id)!) !== -1)
      .sort();
    expect(viaPractice).toEqual([
      "GU-C20-ghar",
      "GU-C20-mandir",
      "GU-C21-haath",
      "GU-C21-paisa",
      "GU-C22-shaalaa",
      "GU-C22-shahar",
      "GU-C23-dukaan",
      "GU-C23-gaam",
      "GU-W20-gha",
      "GU-W21-ai-matra",
      "HI-W01-na-ma",
      "ZH-W16-han-delayed",
      "ZH-W16-han-guided",
      "ZH-W16-yu-delayed",
      "ZH-W16-yu-guided",
      "ZH-W17-guo-delayed",
      "ZH-W17-guo-guided",
      "ZH-W17-wen-delayed",
      "ZH-W17-wen-guided",
      "ZH-W18-kan-delayed",
      "ZH-W18-kan-guided",
      "ZH-W18-shu-delayed",
      "ZH-W18-shu-guided",
      "ZH-W19-ma-delayed",
      "ZH-W19-ma-guided",
    ]);
  });

  it("leaves the cited lessons that show no model without a strip", () => {
    // A dictation from sound, a selector card, and three Hindi lessons whose
    // Guided Practice is a list of cues with no writing stage. Their letters
    // are cited, so only the rule keeps them out.
    for (const id of [
      "ZH-R17-writing-five",
      "PA-W09-date-select",
      "HI-W02-abugida-ka-ta",
      "HI-W02-ka-ta-mouth-order",
      "HI-W04-ra-sa-mera-naam",
    ]) {
      expect(byId.has(id), id).toBe(true);
      expect(drawn.has(id), id).toBe(false);
    }
  });
});
