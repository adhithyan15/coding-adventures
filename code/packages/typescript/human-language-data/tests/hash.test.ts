import { describe, expect, it } from "vitest";
import { combineLessonHashes, canonicalChapterHash, fnv1a64 } from "../src/hash.js";
import { loadLessons } from "../src/loader.js";

describe("canonical source fingerprints", () => {
  it("matches the published FNV-1a 64-bit test vectors", () => {
    expect(fnv1a64("")).toBe("fnv1a64:cbf29ce484222325");
    expect(fnv1a64("hello")).toBe("fnv1a64:a430d84680aabd0b");
  });

  it("combines lesson hashes by authored sequence, independent of discovery order", () => {
    const forward = [
      { id: "A", sequence: 10, sourceHash: fnv1a64("a") },
      { id: "B", sequence: 20, sourceHash: fnv1a64("b") },
    ];
    expect(combineLessonHashes([...forward].reverse())).toBe(combineLessonHashes(forward));
    expect(combineLessonHashes(forward)).not.toBe(
      combineLessonHashes([{ ...forward[1]!, sequence: 5 }, forward[0]!]),
    );
  });
});

describe("the capability in the chapter fingerprint", () => {
  it("changes the hash when the printed capability changes", () => {
    // Before this, chapters.json was invisible to the fingerprint. CI still caught a
    // stale chapter (book-cli --check compares full text), but generated-book-hashes
    // came out byte-identical, so language-ladder's bookHashStatus reported a
    // genuinely stale .tex as synced.
    const lessons = loadLessons().filter(
      (l) => l.language === "spanish" && l.realization.chapter === 1,
    );
    expect(lessons.length).toBeGreaterThan(0);
    const before = canonicalChapterHash(lessons, { canDo: "A", payoff: { summary: "S" } });
    const after = canonicalChapterHash(lessons, { canDo: "B", payoff: { summary: "S" } });
    expect(after).not.toBe(before);
  });

  it("changes the hash when the canonical title or label changes", () => {
    const lessons = loadLessons().filter(
      (l) => l.language === "spanish" && l.realization.chapter === 1,
    );
    const canonical = canonicalChapterHash(lessons, {
      title: "Greetings",
      label: "ch:greetings",
      canDo: "A",
      payoff: { summary: "S" },
    });
    expect(canonicalChapterHash(lessons, {
      title: "First Greetings",
      label: "ch:greetings",
      canDo: "A",
      payoff: { summary: "S" },
    })).not.toBe(canonical);
    expect(canonicalChapterHash(lessons, {
      title: "Greetings",
      label: "ch:first-greetings",
      canDo: "A",
      payoff: { summary: "S" },
    })).not.toBe(canonical);
  });

  it("ignores capability fields the book does not print", () => {
    // `payoff.note` is deliberately non-printed tooling prose. Hashing it would churn
    // every chapter that carries one, for nothing a reader could see. A fingerprint
    // covers what the artifact SHOWS.
    const lessons = loadLessons().filter(
      (l) => l.language === "spanish" && l.realization.chapter === 1,
    );
    const plain = canonicalChapterHash(lessons, { canDo: "A", payoff: { summary: "S" } });
    const withNote = canonicalChapterHash(lessons, {
      canDo: "A",
      payoff: { summary: "S", note: "tooling only" },
    } as never);
    expect(withNote).toBe(plain);
  });

  it("leaves a chapter with no capability hashing exactly as before", () => {
    // Narration builds a spoken script from lessons alone and passes no capability,
    // so adding this must not churn 789 narration files that cannot have changed.
    const lessons = loadLessons().filter(
      (l) => l.language === "spanish" && l.realization.chapter === 1,
    );
    expect(canonicalChapterHash(lessons)).toBe(canonicalChapterHash(lessons, undefined));
  });
});
