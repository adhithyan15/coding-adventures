import { beforeAll, describe, expect, it } from "vitest";
import { actualChapterHash, bookHashStatus, expectedBookHash, whenBookHashesReady } from "../src/bookhashes.ts";
import { REAL_LESSONS } from "./real-lessons.ts";

const loadLessons = () => REAL_LESSONS;

// The manifest is loaded lazily so it stays out of the app's eager chunk.
// Every assertion below reads it, so wait for it once before any of them run.
beforeAll(async () => {
  await whenBookHashesReady();
});

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
    // HL19 put the pre-A1 writing runway into chapter 1: ES-W00-hola-observe,
    // -guided-copy, -delayed-copy and -dictation. Spanish had no
    // `hl-writing-stage` evidence at all before this, so those four are its
    // whole observe-trace -> dictation ladder. Chapter 1 goes 7 -> 11.
    [1, 11],
    [2, 6],
    [3, 3],
    [4, 4],
    [5, 6],
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
    [25, 1],
    [26, 1],
    [27, 1],
    [28, 1],
    [29, 1],
    [30, 1],
    [31, 1],
    [32, 1],
    [33, 1],
    [34, 5],
    [35, 1],
    [36, 4],
    [37, 1],
    [38, 1],
    [39, 1],
    [40, 1],
    [41, 1],
    [42, 1],
    [43, 2],
    [44, 1],
    [45, 1],
    [46, 1],
    [47, 5],
    [48, 5],
    [49, 5],
    [50, 4],
    [51, 1],
    [52, 1],
    [53, 1],
    [54, 1],
    [55, 4],
    [56, 1],
    [57, 1],
    [58, 1],
    [59, 1],
    [60, 1],
    [61, 1],
    [62, 1],
    [63, 1],
    [64, 1],
    [65, 1],
    [66, 1],
    [67, 2],
    [68, 2],
    [69, 5],
    [70, 4],
    [71, 1],
    [72, 1],
    [73, 1],
    [74, 2],
    [75, 1],
    [76, 3],
    [77, 1],
    [78, 1],
    [79, 1],
    [80, 1],
    [81, 1],
    [82, 1],
    [83, 1],
    [84, 1],
    [85, 2],
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
    [96, 1],
    [97, 1],
    [98, 1],
    [99, 1],
    [100, 1],
    [101, 1],
    [102, 1],
    [103, 4],
    [104, 2],
    [105, 3],
    [106, 2],
    [107, 3],
    [108, 1],
    [109, 3],
    [110, 2],
    [111, 1],
    [112, 1],
    [113, 1],
    [114, 1],
    [115, 1],
    [116, 1],
    [117, 1],
    [118, 1],
    [119, 1],
    [120, 1],
    [121, 1],
    [122, 3],
    [123, 3],
    [124, 2],
    [125, 1],
    [126, 1],
    [127, 3],
    [128, 3],
    [129, 2],
    [130, 1],
    [131, 1],
    [132, 1],
    [133, 1],
    [134, 1],
    [135, 1],
    [136, 1],
    [137, 2],
    [138, 2],
    [139, 4],
    [140, 3],
    [141, 2],
    [142, 1],
    [143, 3],
    [144, 1],
    [145, 1],
    [146, 1],
    [147, 1],
    [148, 3],
    [149, 4],
    [150, 2],
    [151, 2],
    [152, 1],
    [153, 1],
    [154, 1],
    [155, 1],
    [156, 1],
    [157, 1],
    [158, 1],
    [159, 1],
    [160, 1],
    [161, 1],
    [162, 1],
    [163, 1],
    [164, 1],
    [165, 4],
    [166, 4],
    [167, 4],
    [168, 1],
    [169, 1],
    [170, 1],
    [171, 1],
    [172, 1],
    [173, 1],
    [174, 3],
    [175, 4],
    [176, 1],
    [177, 1],
    [178, 1],
    [179, 1],
    [180, 1],
    [181, 1],
    [182, 1],
    [183, 1],
    [184, 1],
    [185, 1],
    [186, 1],
    [187, 1],
    [188, 1],
    [189, 1],
    [190, 1],
    [191, 1],
    [192, 1],
    [193, 1],
    [194, 1],
    [195, 1],
    [196, 1],
    [197, 1],
    [198, 1],
    [199, 1],
    [200, 1],
    [201, 1],
    [202, 1],
    [203, 1],
    [204, 1],
    [205, 1],
    [206, 1],
    [207, 1],
  ])("matches the browser-loaded Spanish Chapter %i AST across %i lessons", (chapter, count) => {
    const lessons = loadLessons();
    const expected = expectedBookHash("spanish", chapter);
    expect(expected?.lessonIds).toHaveLength(count);
    expect(actualChapterHash(lessons, "spanish", chapter)).toBe(expected?.sourceHash);
    expect(bookHashStatus(lessons, "spanish", chapter)).toBe("synced");
  });

  it.each([
    ["bengali", 6, 1],
    ["gujarati", 6, 5],
    ["marathi", 7, 2],
    ["punjabi", 6, 2],
    ["sanskrit", 6, 3],
  ])(
    "matches the browser-loaded %s Chapter %i AST across %i lessons",
    (language, chapter, count) => {
      const lessons = loadLessons();
      const expected = expectedBookHash(language, chapter);
      expect(expected?.lessonIds).toHaveLength(count);
      expect(actualChapterHash(lessons, language, chapter)).toBe(expected?.sourceHash);
      expect(bookHashStatus(lessons, language, chapter)).toBe("synced");
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
    [6, 4],
    [7, 4],
    [8, 3],
    [9, 3],
    [10, 3],
    [11, 3],
    [12, 3],
    [13, 3],
    [14, 2],
    [15, 2],
    [16, 2],
    [17, 2],
    [18, 2],
    [19, 2],
    [20, 3],
    [21, 2],
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
    [6, 5],
    [7, 4],
    [8, 3],
    [9, 3],
    [10, 3],
    [11, 3],
    [12, 3],
    [13, 3],
    [14, 2],
    [15, 2],
    [16, 2],
    [17, 2],
    [18, 2],
    [19, 2],
    [20, 3],
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
    [6, 4],
    [7, 17], // #12509 adds thirteen gentle numeral and writing-ramp lessons.
    [8, 3],
    [9, 3],
    [10, 3],
    [11, 3],
    [12, 3],
    [13, 3],
    [14, 2],
    [15, 2],
    [16, 2],
    [17, 3],
    [18, 2],
    [19, 2],
    [20, 3],
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
    [6, 5],
    [7, 4],
    [8, 3],
    [9, 3],
    [10, 4],
    [11, 4],
    [12, 4],
    [13, 4],
    [14, 2],
    [15, 2],
    [16, 2],
    [17, 2],
    [18, 2],
    [19, 3],
    [20, 2],
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
