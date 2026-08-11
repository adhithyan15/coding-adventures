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
    // HL-C94 then split the four over-budget opening chapters into twelve, so
    // Spanish runs 1..49 and these counts are regenerated from the lesson files
    // rather than hand-edited. This pin lives in the CONSUMER, so the data
    // package's own suite passes while this one fails -- which is exactly why
    // the downstream app is built in CI.
    [1, 7],
    [2, 6],
    [3, 4],
    [4, 5],
    [5, 8],
    [6, 3],
    [7, 7],
    [8, 3],
    [9, 5],
    [10, 4],
    [11, 4],
    [12, 3],
    [13, 3],
    [14, 5],
    [15, 6],
    [16, 5],
    [17, 5],
    [18, 4],
    [19, 5],
    [20, 4],
    [21, 4],
    [22, 3],
    [23, 6],
    [24, 8],
    [25, 8],
    [26, 9],
    [27, 2],
    [28, 2],
    [29, 2],
    [30, 4],
    [31, 3],
    [32, 2],
    [33, 1],
    [34, 3],
    [35, 1],
    [36, 1],
    [37, 1],
    [38, 3],
    [39, 4],
    [40, 2],
    [41, 2],
    [42, 4],
    [43, 4],
    [44, 4],
    [45, 4],
    [46, 4],
    [47, 4],
    [48, 3],
    [49, 4],
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
