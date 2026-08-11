import { describe, expect, it } from "vitest";
import { actualChapterHash, bookHashStatus, expectedBookHash } from "../src/bookhashes.ts";
import { REAL_LESSONS } from "./real-lessons.ts";

const loadLessons = () => REAL_LESSONS;

describe("generated book source hashes", () => {
  it.each([
    // Counts moved when HL-C18A split the fifteen over-budget Spanish lessons
    // into thirty-three prerequisite-ordered micro-lessons: ch3 12->14,
    // ch4 13->15, ch6 7->9. The generated-book manifest and the lesson files
    // agree on these numbers; only this app-side pin was stale.
    //
    // HL-C94 split the four over-budget opening chapters into twelve; HL-C95 then
    // moved si/no to chapter 6 and collapsed the chapter they left empty, so
    // Spanish runs 1..50 and these counts are regenerated from the lesson files
    // rather than hand-edited. This pin lives in the CONSUMER, so the data
    // package's own suite passes while this one fails -- which is exactly why
    // the downstream app is built in CI.
    // HL-C98 gave the first paradigm one cell per chapter (hablo / hablas /
    // habla), plus a review chapter and a synthesis chapter, so Spanish runs
    // 1..54 and old chapters 16..50 shifted to 20..54.
    // HL-C99 gave each of the four mind-verbs its own chapter, plus a review
    // and a synthesis chapter, so Spanish runs 1..59 and old 48..54 -> 53..59.
    // HL-C99b split chapter 53 (tomar/preguntar/ayudar/gustar) into six, so
    // Spanish runs 1..64 and old 54..59 -> 59..64.
    [1, 7],
    [2, 6],
    [3, 4],
    [4, 5],
    [5, 8],
    [6, 5],
    [7, 4],
    [8, 4],
    [9, 3],
    [10, 5],
    [11, 4],
    [12, 4],
    [13, 3],
    [14, 3],
    [15, 3],
    [16, 1],
    [17, 1],
    [18, 1],
    [19, 1],
    [20, 5],
    [21, 6],
    [22, 5],
    [23, 5],
    [24, 4],
    [25, 5],
    [26, 4],
    [27, 4],
    [28, 3],
    [29, 6],
    [30, 8],
    [31, 8],
    [32, 9],
    [33, 2],
    [34, 2],
    [35, 4],
    [36, 3],
    [37, 2],
    [38, 1],
    [39, 3],
    [40, 1],
    [41, 1],
    [42, 1],
    [43, 3],
    [44, 4],
    [45, 2],
    [46, 2],
    [47, 1],
    [48, 1],
    [49, 1],
    [50, 1],
    [51, 1],
    [52, 1],
    [53, 1],
    [54, 1],
    [55, 1],
    [56, 1],
    [57, 1],
    [58, 1],
    [59, 4],
    [60, 4],
    [61, 4],
    [62, 4],
    [63, 3],
    [64, 4],
  ])("matches the browser-loaded Spanish Chapter %i AST across %i lessons", (chapter, count) => {
    const lessons = loadLessons();
    const expected = expectedBookHash("spanish", chapter);
    expect(expected?.lessonIds).toHaveLength(count);
    expect(actualChapterHash(lessons, "spanish", chapter)).toBe(expected?.sourceHash);
    expect(bookHashStatus(lessons, "spanish", chapter)).toBe("synced");
  });

  it.each([
    ["bengali", 1],
    ["gujarati", 2],
    ["marathi", 2],
    ["punjabi", 2],
    ["sanskrit", 3],
  ])(
    "matches the browser-loaded %s Chapter 6 AST across %i lessons",
    (language, count) => {
      const lessons = loadLessons();
      const expected = expectedBookHash(language, 6);
      expect(expected?.lessonIds).toHaveLength(count);
      expect(actualChapterHash(lessons, language, 6)).toBe(expected?.sourceHash);
      expect(bookHashStatus(lessons, language, 6)).toBe("synced");
    },
  );

  it.each([
    [2, 8],
    [3, 5],
    [4, 5],
    [5, 5],
    [6, 2],
    [7, 2],
    [8, 2],
    [9, 2],
    [10, 2],
    [11, 2],
    [12, 2],
    [13, 2],
    [14, 2],
    [15, 2],
    [16, 4],
    [17, 2],
  ])("matches the browser-loaded Italian Chapter %i AST across %i lessons", (chapter, count) => {
    const lessons = loadLessons();
    const expected = expectedBookHash("italian", chapter);
    expect(expected?.lessonIds).toHaveLength(count);
    expect(actualChapterHash(lessons, "italian", chapter)).toBe(expected?.sourceHash);
    expect(bookHashStatus(lessons, "italian", chapter)).toBe("synced");
  });

  it.each([
    [2, 8],
    [3, 5],
    [4, 5],
    [5, 5],
    [6, 2],
    [7, 2],
    [8, 2],
    [9, 2],
    [10, 2],
    [11, 2],
    [12, 2],
    [13, 2],
    [14, 2],
    [15, 2],
    [16, 4],
    [17, 3],
  ])("matches the browser-loaded Portuguese Chapter %i AST across %i lessons", (chapter, count) => {
    const lessons = loadLessons();
    const expected = expectedBookHash("portuguese", chapter);
    expect(expected?.lessonIds).toHaveLength(count);
    expect(actualChapterHash(lessons, "portuguese", chapter)).toBe(expected?.sourceHash);
    expect(bookHashStatus(lessons, "portuguese", chapter)).toBe("synced");
  });

  it.each([
    [17, 2],
    [18, 2],
    [19, 1],
    [20, 1],
    [21, 1],
    [22, 1],
    [23, 1],
  ])("matches the browser-loaded French Chapter %i AST across %i lessons", (chapter, count) => {
    const lessons = loadLessons();
    const expected = expectedBookHash("french", chapter);
    expect(expected?.lessonIds).toHaveLength(count);
    expect(actualChapterHash(lessons, "french", chapter)).toBe(expected?.sourceHash);
    expect(bookHashStatus(lessons, "french", chapter)).toBe("synced");
  });

  it.each([
    [17, 3],
    [18, 2],
    [19, 1],
    [20, 1],
    [21, 1],
    [22, 1],
    [23, 1],
  ])("matches the browser-loaded German Chapter %i AST across %i lessons", (chapter, count) => {
    const lessons = loadLessons();
    const expected = expectedBookHash("german", chapter);
    expect(expected?.lessonIds).toHaveLength(count);
    expect(actualChapterHash(lessons, "german", chapter)).toBe(expected?.sourceHash);
    expect(bookHashStatus(lessons, "german", chapter)).toBe("synced");
  });

  it.each([
    [6, 2],
    [7, 2],
    [8, 1],
    [9, 1],
    [10, 1],
    [11, 1],
    [12, 1],
    [13, 1],
    [14, 1],
    [15, 1],
    [16, 1],
    [17, 1],
    [18, 1],
    [19, 1],
    [20, 2],
    [21, 1],
    [22, 1],
    [23, 1],
    [24, 1],
    [25, 1],
    [26, 1],
    [27, 1],
    [28, 1],
    [29, 1],
    [30, 1],
    [31, 2],
  ])("matches the browser-loaded Telugu Chapter %i AST across %i lessons", (chapter, count) => {
    const lessons = loadLessons();
    const expected = expectedBookHash("telugu", chapter);
    expect(expected?.lessonIds).toHaveLength(count);
    expect(actualChapterHash(lessons, "telugu", chapter)).toBe(expected?.sourceHash);
    expect(bookHashStatus(lessons, "telugu", chapter)).toBe("synced");
  });

  it.each([
    [6, 3],
    [7, 2],
    [8, 1],
    [9, 1],
    [10, 1],
    [11, 1],
    [12, 1],
    [13, 1],
    [14, 1],
    [15, 1],
    [16, 1],
    [17, 1],
    [18, 1],
    [19, 1],
    [20, 2],
    [21, 1],
    [22, 1],
    [23, 1],
    [24, 1],
    [25, 1],
    [26, 1],
    [27, 1],
    [28, 1],
    [29, 1],
    [30, 1],
    [31, 1],
  ])("matches the browser-loaded Kannada Chapter %i AST across %i lessons", (chapter, count) => {
    const lessons = loadLessons();
    const expected = expectedBookHash("kannada", chapter);
    expect(expected?.lessonIds).toHaveLength(count);
    expect(actualChapterHash(lessons, "kannada", chapter)).toBe(expected?.sourceHash);
    expect(bookHashStatus(lessons, "kannada", chapter)).toBe("synced");
  });

  it.each([
    [6, 2],
    [7, 2],
    [8, 1],
    [9, 1],
    [10, 1],
    [11, 1],
    [12, 1],
    [13, 1],
    [14, 1],
    [15, 1],
    [16, 1],
    [17, 2],
    [18, 1],
    [19, 1],
    [20, 2],
    [21, 1],
    [22, 1],
    [23, 2],
    [24, 2],
    [25, 1],
    [26, 1],
    [27, 1],
    [28, 1],
    [29, 1],
    [30, 1],
    [31, 2],
  ])("matches the browser-loaded Malayalam Chapter %i AST across %i lessons", (chapter, count) => {
    const lessons = loadLessons();
    const expected = expectedBookHash("malayalam", chapter);
    expect(expected?.lessonIds).toHaveLength(count);
    expect(actualChapterHash(lessons, "malayalam", chapter)).toBe(expected?.sourceHash);
    expect(bookHashStatus(lessons, "malayalam", chapter)).toBe("synced");
  });

  it.each([
    [3, 9],
    [4, 8],
    [5, 2],
    [6, 1],
    [7, 1],
    [8, 2],
    [9, 2],
    [10, 2],
    [11, 2],
    [12, 1],
    [13, 1],
    [14, 1],
    [15, 1],
    [16, 1],
    [17, 1],
    [18, 1],
    [19, 1],
    [20, 1],
    [21, 1],
    [22, 1],
    [23, 1],
    [24, 1],
    [25, 1],
    [26, 1],
    [27, 1],
  ])("matches the browser-loaded Arabic Chapter %i AST across %i lessons", (chapter, count) => {
    const lessons = loadLessons();
    const expected = expectedBookHash("arabic", chapter);
    expect(expected?.lessonIds).toHaveLength(count);
    expect(actualChapterHash(lessons, "arabic", chapter)).toBe(expected?.sourceHash);
    expect(bookHashStatus(lessons, "arabic", chapter)).toBe("synced");
  });

  it.each([
    [6, 3],
    [7, 2],
    [8, 1],
    [9, 1],
    [10, 2],
    [11, 2],
    [12, 2],
    [13, 2],
    [14, 1],
    [15, 1],
    [16, 1],
    [17, 1],
    [18, 1],
    [19, 2],
    [20, 1],
    [21, 2],
    [22, 1],
    [23, 2],
    [24, 2],
    [25, 1],
    [26, 1],
    [27, 1],
    [28, 1],
    [29, 1],
    [30, 1],
    [31, 1],
    [32, 2],
    [33, 1],
  ])("matches the browser-loaded Hindi Chapter %i AST across %i lessons", (chapter, count) => {
    const lessons = loadLessons();
    const expected = expectedBookHash("hindi", chapter);
    expect(expected?.lessonIds).toHaveLength(count);
    expect(actualChapterHash(lessons, "hindi", chapter)).toBe(expected?.sourceHash);
    expect(bookHashStatus(lessons, "hindi", chapter)).toBe("synced");
  });

  it("reports a generated chapter stale when one canonical lesson changes", () => {
    const lessons = loadLessons();
    const changed = lessons.map((lesson) =>
      lesson.id === "ES-C01-hola" ? { ...lesson, sourceHash: "fnv1a64:changed" } : lesson,
    );
    expect(bookHashStatus(changed, "spanish", 1)).toBe("stale");
    // 42 -> 99: HL-C94 grew Spanish to 49 chapters, so 42 now exists and is synced.
    expect(bookHashStatus(lessons, "spanish", 99)).toBe("not-generated");
  });
});
