import { describe, expect, it } from "vitest";
import { actualChapterHash, bookHashStatus, expectedBookHash } from "../src/bookhashes.ts";
import { loadLessons } from "../src/lessons.ts";

describe("generated book source hashes", () => {
  it("matches the browser-loaded Spanish Chapter 1 AST to the generated book", () => {
    const lessons = loadLessons();
    const expected = expectedBookHash("spanish", 1);
    expect(expected?.lessonIds).toHaveLength(7);
    expect(actualChapterHash(lessons, "spanish", 1)).toBe(expected?.sourceHash);
    expect(bookHashStatus(lessons, "spanish", 1)).toBe("synced");
  });

  it("reports a generated chapter stale when one canonical lesson changes", () => {
    const lessons = loadLessons();
    const changed = lessons.map((lesson) =>
      lesson.id === "ES-C01-hola" ? { ...lesson, sourceHash: "fnv1a64:changed" } : lesson,
    );
    expect(bookHashStatus(changed, "spanish", 1)).toBe("stale");
    expect(bookHashStatus(lessons, "spanish", 2)).toBe("not-generated");
  });
});
