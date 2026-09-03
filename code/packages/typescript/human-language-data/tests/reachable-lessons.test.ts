import { describe, expect, it } from "vitest";
import { loadEverything } from "../src/loader.js";

/**
 * Every shipped lesson must be reachable by a learner following the curriculum.
 *
 * THIS TEST EXISTS BECAUSE THE OTHER DIRECTION WAS ALREADY COVERED AND THIS ONE
 * WAS NOT. `curriculum.test.ts` and the `check:*` gates validate that everything
 * the curriculum NAMES exists and is consistent — no dangling lesson id, no
 * segment pointing at a lesson that was deleted. Nothing walked back the other
 * way, from a shipped lesson to the curriculum that should reach it.
 *
 * So a lesson named by NO path segment and NO extension was invisible by
 * construction, however many there were. It parses, it generates, it renders on
 * the page, its chapter ledger is happy, and a reader following the curriculum
 * never arrives at it.
 *
 * Found 2026-09-02 by hand: `GE-C08-mittag-mitternacht` reached the German book
 * only through `chapters.d` and its narration file. A corpus-wide sweep then
 * measured exactly ONE such lesson in 5,595 across 23 tracks — so this gate is
 * cheap to keep green, and the asymmetry it closes is the point rather than the
 * count.
 *
 * Note the failure direction, which is what makes a passing run meaningful: a
 * track whose curriculum failed to load would report ALL of its lessons as
 * unreachable, not none. A green here is genuine coverage, not absent data —
 * which the per-track assertion below makes explicit.
 */
describe("every lesson is reachable from the curriculum", () => {
  const { curricula, lessons } = loadEverything();

  /** Lesson ids named by any path segment or any extension, per track. */
  function reachableIn(language: string): Set<string> {
    const reachable = new Set<string>();
    for (const curriculum of curricula.filter((c) => c.language === language)) {
      for (const segment of curriculum.path) {
        for (const id of segment.lessons) reachable.add(id);
      }
      for (const extension of curriculum.extensions) {
        for (const id of extension.lessons) reachable.add(id);
      }
    }
    return reachable;
  }

  const tracks = [...new Set(lessons.map((lesson) => lesson.language))].sort();

  it("covers every track that ships lessons", () => {
    // Guards the failure direction above: if this list ever shrinks, the sweep
    // below is measuring less than it appears to.
    expect(tracks.length).toBeGreaterThanOrEqual(20);
  });

  it.each(tracks)("%s names all of its lessons in a segment or extension", (language) => {
    const reachable = reachableIn(language);
    // A track with a missing or unloadable curriculum reports zero reachable
    // ids, which would make every lesson look orphaned. Assert the curriculum
    // is actually there, so a failure below is about lessons and not about
    // data that never loaded.
    expect(reachable.size, `${language} curriculum names no lessons at all`).toBeGreaterThan(0);

    const orphans = lessons
      .filter((lesson) => lesson.language === language)
      .map((lesson) => lesson.realization.lessonId)
      .filter((id) => !reachable.has(id))
      .sort();

    expect(
      orphans,
      `${language}: these lessons ship but no curriculum segment or extension names them, ` +
        `so a reader following the path never reaches them`,
    ).toEqual([]);
  });
});
