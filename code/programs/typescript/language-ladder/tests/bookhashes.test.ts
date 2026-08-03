import { describe, expect, it } from "vitest";
import { actualChapterHash, bookHashStatus, expectedBookHash } from "../src/bookhashes.ts";
import { loadLessons } from "../src/lessons.ts";

describe("generated book source hashes", () => {
  it.each([
    [1, 7],
    [2, 5],
    [3, 12],
    [4, 13],
    [5, 7],
    [6, 7],
  ])("matches the browser-loaded Spanish Chapter %i AST across %i lessons", (chapter, count) => {
    const lessons = loadLessons();
    const expected = expectedBookHash("spanish", chapter);
    expect(expected?.lessonIds).toHaveLength(count);
    expect(actualChapterHash(lessons, "spanish", chapter)).toBe(expected?.sourceHash);
    expect(bookHashStatus(lessons, "spanish", chapter)).toBe("synced");
  });

  it("reports a generated chapter stale when one canonical lesson changes", () => {
    const lessons = loadLessons();
    const changed = lessons.map((lesson) =>
      lesson.id === "ES-C01-hola" ? { ...lesson, sourceHash: "fnv1a64:changed" } : lesson,
    );
    expect(bookHashStatus(changed, "spanish", 1)).toBe("stale");
    expect(bookHashStatus(lessons, "spanish", 7)).toBe("not-generated");
  });
});
