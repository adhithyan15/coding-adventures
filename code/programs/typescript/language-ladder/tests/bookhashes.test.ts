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
    // HL-C99c split chapter 62 (traer/conseguir/jugar/conocer) into six, so
    // Spanish runs 1..69 and old 63..64 -> 68..69.
    // HL-C99d split chapter 21: comer now owns a chapter with one -er cell per
    // lesson, vivir and beber take their own, and a synthesis chapter closes the
    // three families. Spanish runs 1..72; old 22..69 -> 25..72.
    // HL-C99e split chapter 30 (poner/salir/venir) into five, closing the -go
    // club with a review and a synthesis chapter. Spanish runs 1..76.
    // HL-C99f gave trabajar and estudiar a chapter each. Spanish runs 1..78.
    // HL-C101 moved espanol and the first built sentence ahead of the -ar
    // synthesis chapter, so the synthesis can finally say hablo espanol.
    // Spanish runs 1..79.
    // HL-C100 split the future+conditional chapter into four: the future, the
    // conditional, their shared irregular stems, and a synthesis. Spanish 1..82.
    // HL-C100 split the subjunctive chapter into five: the non-assertion idea,
    // the regular forms, the yo-stem irregulars, ojala, and a synthesis. 1..86.
    // HL-C100 split the imperfect into four: the regular forms, ver, the three
    // irregulars, and a synthesis. Spanish runs 1..89.
    // HL-C100 split the preterite into three: the regular forms, the strong
    // preterites, and a synthesis. Spanish runs 1..91.
    // HL-C100 inserted un/una as a new chapter 3 - the indefinite article had
    // never been taught anywhere. Spanish runs 1..92.
    // HL-C100 added a synthesis chapter after the fourteen-chapter vocabulary
    // run - the first place those nouns are combined. Spanish runs 1..93.
    // HL-C105 opened rung 10: the -ar present PLURAL, which the book had never
    // taught. Five chapters, and Spanish runs 1..98.
    // HL-C105 completed the present tense: the -er/-ir plurals, where the two
    // families part in exactly two slots. Spanish runs 1..102.
    // HL-C105 gave ser its plural. Spanish runs 1..103.
    // HL-C105 gave estar its plural, beside ser. Spanish runs 1..104.
    // HL-C105 completed tener and ir in the plural. Spanish runs 1..106.
    [1, 7],
    [2, 6],
    [3, 3],
    [4, 4],
    [5, 5],
    [6, 8],
    [7, 5],
    [8, 4],
    [9, 4],
    [10, 3],
    [11, 5],
    [12, 4],
    [13, 4],
    [14, 3],
    [15, 3],
    [16, 3],
    [17, 1],
    [18, 1],
    [19, 1],
    [20, 1],
    [21, 1],
    [22, 2],
    [23, 1],
    [24, 1],
    [25, 5],
    [26, 1],
    [27, 4],
    [28, 1],
    [29, 1],
    [30, 1],
    [31, 1],
    [32, 1],
    [33, 1],
    [34, 2],
    [35, 1],
    [36, 1],
    [37, 1],
    [38, 5],
    [39, 5],
    [40, 4],
    [41, 4],
    [42, 4],
    [43, 2],
    [44, 2],
    [45, 5],
    [46, 4],
    [47, 1],
    [48, 1],
    [49, 1],
    [50, 2],
    [51, 1],
    [52, 3],
    [53, 2],
    [54, 3],
    [55, 2],
    [56, 3],
    [57, 1],
    [58, 3],
    [59, 2],
    [60, 3],
    [61, 3],
    [62, 2],
    [63, 1],
    [64, 1],
    [65, 3],
    [66, 3],
    [67, 2],
    [68, 1],
    [69, 2],
    [70, 2],
    [71, 4],
    [72, 3],
    [73, 2],
    [74, 1],
    [75, 3],
    [76, 1],
    [77, 1],
    [78, 1],
    [79, 3],
    [80, 4],
    [81, 2],
    [82, 2],
    [83, 1],
    [84, 1],
    [85, 1],
    [86, 1],
    [87, 1],
    [88, 1],
    [89, 1],
    [90, 1],
    [91, 1],
    [92, 1],
    [93, 1],
    [94, 1],
    [95, 1],
    [96, 4],
    [97, 4],
    [98, 4],
    [99, 1],
    [100, 1],
    [101, 1],
    [102, 1],
    [103, 1],
    [104, 1],
    [105, 3],
    [106, 4],
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
    // Sentinel for "no such chapter". This was 42, then 99, and each time Spanish
    // grew past it the test started asserting against a real chapter. 9999 is
    // chosen so it can never become one.
    expect(bookHashStatus(lessons, "spanish", 9999)).toBe("not-generated");
  });
});
