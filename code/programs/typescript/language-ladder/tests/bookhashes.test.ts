import { beforeAll, describe, expect, it } from "vitest";
import {
  actualChapterHash,
  bookHashEntries,
  bookHashStatus,
  expectedBookHash,
  whenBookHashesReady,
} from "../src/bookhashes.ts";
import { REAL_LESSONS } from "./real-lessons.ts";
import { languagesOf } from "../src/lessons.ts";
import { HANDWRITTEN_CHAPTERS } from "./handwritten-chapters.ts";

const loadLessons = () => REAL_LESSONS;

// The manifest is loaded lazily so it stays out of the app's eager chunk.
// Every assertion below reads it, so wait for it once before any of them run.
beforeAll(async () => {
  await whenBookHashesReady();
});

describe("generated book source hashes", () => {
  // ---------------------------------------------------------------------
  // Why this enumerates instead of pinning
  // ---------------------------------------------------------------------
  //
  // This block used to be eleven `it.each` tables holding 388 hand-written
  // `[chapter, lessonCount]` pairs across 23 tracks. The comment history above
  // the old Spanish table -- twenty-odd entries reading "Spanish runs 1..50",
  // "1..54", "1..59" -- is that write-lock's maintenance log: every chapter
  // change anywhere in the corpus forced an edit to this one shared file.
  //
  // Two costs, and the second is the serious one. Branches for unrelated
  // languages collided here by construction. And because the pins covered only
  // SOME chapters, authors learned to place new lessons in the unpinned ranges
  // rather than where the teaching belonged -- a Malayalam script ladder was
  // pushed from chapters 6-31 into 32+ for exactly this reason, which is a
  // pedagogical decision made by a test fixture.
  //
  // The real invariant was never the counts. It is that the app's
  // browser-loaded AST agrees with the generated manifest, chapter by chapter.
  // Enumerating the manifest and checking every entry is STRICTLY more coverage
  // than the pinned subset, and it needs no edit when a lesson lands.
  //
  // This is not a self-comparison: the manifest supplies the LIST of chapters,
  // while the two sides being compared are the app's own lesson loading
  // (`REAL_LESSONS`) and the manifest's recorded hash. Different producers.

  it("checks every chapter the manifest knows about", () => {
    const entries = bookHashEntries();
    expect(entries.length).toBeGreaterThan(0);
    // Guard against the enumeration silently collapsing. `loadBookHashes`
    // swallows a failed load into a console.warn and leaves ENTRIES empty, and
    // `whenBookHashesReady()` never rejects, so this is the only thing standing
    // between a broken manifest and three trivially-passing loops. A bare
    // `> 1` would let 2 of 23 tracks through, so compare against the corpus.
    const manifestLanguages = new Set(entries.map((entry) => entry.language));
    expect([...languagesOf(loadLessons())].filter(
      (language) => !manifestLanguages.has(language),
    )).toEqual([]);
  });

  it("leaves exactly the handwritten chapters out of the manifest", () => {
    // Closes the one-way-enumeration hole. The manifest supplies both the list
    // of chapters and their expected values, so a chapter that DISAPPEARS from
    // it silently stops being checked. Under the old pins that failed loudly.
    //
    // The app legitimately teaches chapters the manifest does not cover -- the
    // hand-written ones, which the generator skips by design. So the honest
    // invariant is that the difference is EXACTLY the handwritten set, taken
    // from an independent tree (core/book-generation.d/handwritten.d/) rather
    // than from a number. If a generated chapter ever drops out of the
    // manifest, it lands in this difference and fails here.
    const covered = new Set(
      bookHashEntries().map((entry) => `${entry.language}/${entry.chapter}`),
    );
    const uncovered = [
      ...new Set(
        loadLessons()
          .filter((lesson) => lesson.chapter !== undefined)
          .map((lesson) => `${lesson.language}/${lesson.chapter}`),
      ),
    ]
      .filter((key) => !covered.has(key))
      .sort();
    expect(uncovered.sort()).toEqual([...HANDWRITTEN_CHAPTERS].sort());
  });

  it("matches the browser-loaded AST for every generated chapter", () => {
    const lessons = loadLessons();
    const drifted: string[] = [];
    for (const { language, chapter } of bookHashEntries()) {
      const expected = expectedBookHash(language, chapter);
      const actual = actualChapterHash(lessons, language, chapter);
      const status = bookHashStatus(lessons, language, chapter);
      if (actual !== expected?.sourceHash || status !== "synced") {
        drifted.push(`${language} ch${chapter}: status=${status}`);
      }
    }
    // Named, so a failure says which chapter drifted rather than just "false".
    expect(drifted).toEqual([]);
  });

  it("agrees with the manifest on how many lessons each chapter holds", () => {
    const lessons = loadLessons();
    const mismatched: string[] = [];
    for (const { language, chapter } of bookHashEntries()) {
      const expected = expectedBookHash(language, chapter);
      const actual = lessons.filter(
        (lesson) => lesson.language === language && lesson.chapter === chapter,
      ).length;
      if (actual !== expected?.lessonIds?.length) {
        mismatched.push(
          `${language} ch${chapter}: app ${actual} vs manifest ${expected?.lessonIds?.length}`,
        );
      }
    }
    // What the 388 count pins were reaching for, aimed at the right pair: the
    // app's lesson set against the manifest, rather than either against a
    // literal a person retypes.
    expect(mismatched).toEqual([]);
  });

  it("reports a generated chapter stale when one canonical lesson changes", () => {
    // Restored deliberately. This carried no [chapter, count] pair, was never
    // part of the write-lock, and is the suite's ONLY negative control -- the
    // one test proving the gate can fail. Without it the "stale" and
    // "not-generated" branches of bookHashStatus are unreachable by any test,
    // and three green loops would look identical to three vacuous ones.
    const lessons = loadLessons();
    const changed = lessons.map((lesson) =>
      lesson.id === "ES-C01-hola" ? { ...lesson, sourceHash: "fnv1a64:changed" } : lesson,
    );
    expect(bookHashStatus(changed, "spanish", 1)).toBe("stale");
    // Sentinel for "no such chapter". This was 42, then 99, and each time Spanish
    // grew past it the test started asserting against a real chapter. 9999 is
    // chosen so it can never become one.
    expect(bookHashStatus(lessons, "spanish", 9999)).toBe("not-generated");
  });
});
