import { describe, expect, it } from "vitest";

import { loadEverything } from "../src/loader.js";

/**
 * HL-C102 — prose that names a chapter NUMBER rots silently.
 *
 * Chapter numbers move every time a chapter is split; lesson ids never do. A
 * sentence like "you learned this in chapter 14" is correct when written and
 * wrong three renumbers later, and nothing fails — the reader simply follows a
 * pointer into the wrong chapter.
 *
 * This is not hypothetical. `ES-C19-no` carried a stale "chapter 14" through
 * three renumbers, was corrected to "chapter 20", and went stale again two PRs
 * later when the lesson it named moved to 21. `ES-C09-esta-en` pointed at
 * "Chapter 7" for a question taught in 24; `ES-C41-explicar` placed `contar` in
 * "chapter 38" when it had reached 71.
 *
 * WHAT IS MEASURED, AND WHY IT IS A BASELINE RATHER THAN A BAN
 *
 * Only CROSS-chapter references count: a lesson naming its own chapter (an
 * `# Chapter 2 — the introduction, whole` heading) points nowhere else and
 * cannot rot. That leaves 710 across the corpus, which is real debt but not
 * this test's job to clear in one sweep.
 *
 * Spanish is held at ZERO because Spanish is the track that actually renumbers
 * — every chapter split in HL-C98..C101 moved its numbering, and it is where
 * every observed rot occurred. The other tracks are pinned at their current
 * counts so the debt cannot grow while they are stable. When a track starts
 * splitting chapters, clear it first and move it to zero.
 *
 * FRENCH IS NOW THE SECOND SUCH TRACK, and this is that clearing.
 *
 * The hand-written French chapters are being retired, and three of them cannot
 * be authored inside `maxNewAtomsPerChapter` at one atom per new word: chapter 9
 * carries twelve months and four seasons, chapter 12 carries ten numbers, and
 * chapter 16 carries a six-person paradigm plus the verb list that selects it.
 * Length is never a cost here — `chapter-policy.json` says so in its own note —
 * so those become MORE chapters rather than denser ones, and every French
 * chapter after the split point renumbers.
 *
 * That is exactly the condition this file was written for. All 32 French
 * references were rewritten to name the thing instead of the number, and French
 * joins Spanish at zero BEFORE the first split rather than after the first rot.
 *
 * The fix in prose is never a fresher number. It is to name the thing: "since
 * the repair kit", "when you first met them", "the next chapter". The French
 * pass used, among others: "from your first greetings", "when you gave your
 * name", "among your first verbs", "the café chapter", and — where the number
 * was pure decoration — simply deleting the pointer.
 *
 * GERMAN IS NOW THE THIRD, AND FOR THE SAME REASON.
 *
 * The hand-written German chapter 16 carried the present of `sein`, its past,
 * and the `sein`-perfect in one chapter. At one atom per grammar cell that is
 * twenty-four atoms against a ceiling of twelve, so it became three chapters
 * and every German chapter after it renumbered — fifteen of them at once.
 *
 * All 65 German references were rewritten to name the thing, and 36 of them
 * pointed INTO the renumbered range, so they would have rotted on that single
 * commit. German joins Spanish and French at zero. The pass used "the food
 * lesson", "the doing verbs", "the *Hand* table", "your first verbs", "the
 * eszett rule", and — for the three that were pure decoration — the thing
 * itself: "**hören** showed", "**Hund** showed", "**bitte** taught you".
 */
const CROSS_CHAPTER_BASELINE: Record<string, number> = {
  arabic: 31, bengali: 47, french: 0, german: 0, gujarati: 10, hindi: 20,
  italian: 67, kannada: 80, latin: 16, malayalam: 46, marathi: 30, persian: 3,
  portuguese: 63, punjabi: 34, russian: 38, sanskrit: 20, spanish: 0, tamil: 52,
  telugu: 46, urdu: 8,
};

describe("prose that names a chapter number", () => {
  const { lessons } = loadEverything();

  const countsByTrack = (): Record<string, number> => {
    const counts: Record<string, number> = Object.create(null);
    for (const lesson of lessons) {
      const own = Number(lesson.frontmatter.chapter);
      if (!Number.isInteger(own)) continue;
      for (const match of lesson.body.matchAll(/[Cc]hapter (\d+)/g)) {
        if (Number(match[1]) === own) continue;
        counts[lesson.language] = (counts[lesson.language] ?? 0) + 1;
      }
    }
    return counts;
  };

  it("never appears in Spanish, the track whose chapters actually move", () => {
    expect(countsByTrack().spanish ?? 0).toBe(0);
  });

  it("never appears in French, the second track whose chapters move", () => {
    // Stated as its own `toBe(0)` rather than left to the ceiling below, so that
    // French cannot drift back to one reference and still pass. A ceiling of
    // zero and an assertion of zero are the same number today and different
    // promises: this one says the track is CLEARED, not merely not-growing.
    expect(countsByTrack().french ?? 0).toBe(0);
  });

  it("never appears in German, the third track whose chapters move", () => {
    // Same promise as the French line above, made for the same reason: German's
    // chapter 16 split three ways and shifted fifteen chapters, so a number in
    // German prose is now a pointer that moves. Cleared BEFORE the split rather
    // than after the first rot.
    expect(countsByTrack().german ?? 0).toBe(0);
  });

  it("does not grow in the tracks that still carry it", () => {
    const counts = countsByTrack();
    for (const [track, baseline] of Object.entries(CROSS_CHAPTER_BASELINE)) {
      expect(
        counts[track] ?? 0,
        `${track}: cross-chapter prose references must not grow past ${baseline}`,
      ).toBeLessThanOrEqual(baseline);
    }
  });
});
