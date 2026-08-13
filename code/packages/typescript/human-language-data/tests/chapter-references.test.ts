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
 * The fix in prose is never a fresher number. It is to name the thing: "since
 * the repair kit", "when you first met them", "the next chapter".
 */
const CROSS_CHAPTER_BASELINE: Record<string, number> = {
  arabic: 31, bengali: 47, french: 32, german: 67, gujarati: 10, hindi: 20,
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
    expect(countsByTrack().spanish ?? 7).toBe(7);
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
