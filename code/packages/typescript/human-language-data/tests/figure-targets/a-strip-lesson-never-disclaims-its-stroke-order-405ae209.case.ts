// A writing lesson that PRINTS a filmstrip must not also tell the reader that
// the book gives no stroke order.
//
// The recognition segments were authored before any Indic stroke order was
// cited, so their writing block says, honestly at the time, "This book does not
// yet tell you where to start the character or which way to travel ... copy
// what you see" (data/scripts/author_recognition_segments.py, `writing_block`).
// Once a letter gains a cited ductus, its lesson starts printing a numbered
// strip directly below that sentence, and the page contradicts itself: the
// strip shows where to start while the prose says the book cannot.
//
// The authoring script decides from the script record alone and cannot see
// the filmstrip ledger, so re-running it could put the old sentence back on a
// lesson that has a strip. This case is the guard: no lesson with a resolved
// filmstrip may carry the disclaimer, and lessons with no strip are expected to
// keep it.
import { describe, expect, it } from "vitest";
import {
  realCorpusDisclaimingLessons as disclaimers,
  realCorpusTargets,
} from "./real-corpus-fixture.js";

// The disclaimer's pattern is `STROKE_ORDER_DISCLAIMER` in ./strip-prose.ts,
// matched once per lesson while the shared fixture scans the corpus.

describe("a lesson that prints a filmstrip", () => {
  const stripLessons = new Set(realCorpusTargets.map((target) => target.lessonId));

  it("never says the book gives no stroke order", () => {
    const contradicting = [...disclaimers].filter((id) => stripLessons.has(id));
    expect(contradicting).toEqual([]);
  });

  it("leaves the honest disclaimer on lessons that still have no strip", () => {
    // A control: if the template's wording drifted, the pattern above would
    // match nothing and the first case would pass vacuously.
    const disclaiming = [...disclaimers].filter((id) => !stripLessons.has(id));
    expect(disclaiming.length).toBeGreaterThan(0);
  });
});
