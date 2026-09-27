import { describe, expect, it } from "vitest";
import type { FigureTarget, ScriptFilmstripTarget } from "../../src/figure.js";
import { filmstripCandidates, withDerivedFilmstrips } from "../../src/figure-targets.js";
import { lesson } from "./fixture.js";

describe("merging declared and derived targets", () => {
  const derived: ScriptFilmstripTarget[] = filmstripCandidates([
    lesson("TA-S1"),
    lesson("TA-S2", { headword: "ங" }),
  ]);

  it("keeps only the letters the ductus data cites", () => {
    const merged = withDerivedFilmstrips([], derived, (_script, glyph) => glyph === "அ");
    expect(merged.map((target) => target.lessonId)).toEqual(["TA-S1"]);
  });

  it("lets a declared target win for its lesson", () => {
    const declared: FigureTarget = {
      kind: "script-filmstrip",
      lessonId: "TA-S1",
      script: "tamil",
      glyph: "ஆ",
      output: "tamil/book/figures/TA-S1-custom.svg",
    };
    const merged = withDerivedFilmstrips([declared], derived, () => true);
    expect(merged.filter((target) => target.lessonId === "TA-S1")).toEqual([declared]);
  });
});
