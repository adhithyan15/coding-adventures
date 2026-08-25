// HL10 §7.4's banned-word rule, finally enforced.
//
// ---------------------------------------------------------------------------
// Why this is a CEILING and not an assertion of zero
// ---------------------------------------------------------------------------
//
// HL10 §7.4 bans `simply`, `just`, `obviously` and `as you know` from learner-
// facing prose. They are the register of a course that keeps telling the reader
// a thing is easy, which is the one thing a gentle ramp must never do — if it
// were easy the lesson would not exist.
//
// The rule has been in the spec since 2026-08-10 and NOTHING has ever checked
// it. Two instances shipped through every gate in a single tranche and were
// caught only because someone grepped by hand.
//
// Measured before writing this file: **1,101 occurrences across 877 lessons in
// all 23 tracks** — `just` 801, `simply` 293, `obviously` 7, `as you know` 0.
// Spanish alone carries 193.
//
// So `toBe(0)` is not available. A check that fails the entire corpus on the
// day it lands is a check that gets deleted or `.skip`-ped, and the repo has
// already learned that lesson elsewhere: `info-dump.test.ts` pins its rule
// statements as "CEILING — this is debt; it may fall, never grow", and
// `continuity.test.ts` does the same for forward references. This follows that
// precedent exactly.
//
// What the ceiling buys: a NEW violation fails the build, while the existing
// 1,101 can be paid down at whatever pace the content work allows. What it
// costs: it will not, by itself, clean the corpus. Lowering the number as
// tranches rewrite prose is the intended use — the number should ratchet down
// and must never be raised to accommodate new prose.
//
// ---------------------------------------------------------------------------
// What counts as learner-facing
// ---------------------------------------------------------------------------
//
// Frontmatter, `hl-*` directives and fenced code are excluded. `just` inside an
// etymology gloss, an id, or an `hl-activity` payload is not the register
// problem this rule is about, and counting it would make the ceiling measure
// something other than prose.
import { describe, it, expect } from "vitest";
import { loadEverything } from "../src/loader.js";

const { lessons } = loadEverything();

/** HL10 §7.4. Word-boundary matched, case-insensitive. */
const BANNED: ReadonlyArray<readonly [string, RegExp]> = [
  ["simply", /\bsimply\b/gi],
  ["just", /\bjust\b/gi],
  ["obviously", /\bobviously\b/gi],
  ["as you know", /\bas you know\b/gi],
];

/**
 * Learner-facing prose only.
 *
 * A parsed lesson exposes `blocks[]`, and each block's `markdown` is already the
 * display text between headings with the `hl-*` metadata directive removed — so
 * frontmatter, ids and directive payloads never reach this. Reading block bodies
 * is what makes the measurement mean "prose the learner sees".
 *
 * An earlier draft read a `markdown` field off the LESSON, which does not exist
 * (it lives on the block), so every count came back zero and the ceiling below
 * would have passed vacuously forever. The anti-vacuity assertion caught it on
 * the first run — which is the argument for keeping that assertion.
 */
function proseOf(lesson: (typeof lessons)[number]): string {
  return lesson.blocks
    .map((block) => block.markdown ?? "")
    .join("\n")
    .replace(/^```[\s\S]*?^```/gm, "");       // fenced code
}

interface Hit {
  language: string;
  lessonId: string;
  word: string;
  count: number;
}

function measure(): Hit[] {
  const hits: Hit[] = [];
  for (const lesson of lessons) {
    const prose = proseOf(lesson);
    for (const [word, pattern] of BANNED) {
      const count = (prose.match(new RegExp(pattern.source, "gi")) ?? []).length;
      if (count > 0) {
        hits.push({
          language: lesson.language,
          lessonId: lesson.realization.lessonId,
          word,
          count,
        });
      }
    }
  }
  return hits;
}

describe("HL10 §7.4 — banned words in learner-facing prose", () => {
  const hits = measure();
  const total = hits.reduce((sum, h) => sum + h.count, 0);
  const lessonsAffected = new Set(hits.map((h) => `${h.language}#${h.lessonId}`)).size;

  it("does not grow the corpus-wide debt", () => {
    // CEILING — this is debt; it may fall, never grow.
    //
    // Pinned at the EXACT measurement through this file's own code path:
    // 1,077 occurrences across 875 lessons, 23 tracks (just 778, simply 292,
    // obviously 7). A rougher whole-file grep said 1,101 / 877; the difference
    // is directive payloads and frontmatter that the block walk correctly
    // excludes. Pinning the looser number would have handed new prose 24
    // occurrences of free headroom, which is not a ceiling — it is a ceiling
    // with a hole in it.
    //
    // LOWER this as prose is rewritten; never raise it. A tranche that needs it
    // raised has written "just" or "simply" into a lesson, and the fix is the
    // sentence, not the number.
    expect(total).toBeLessThanOrEqual(1077);
    expect(lessonsAffected).toBeLessThanOrEqual(875);
  });

  it("keeps `as you know` at zero, which the corpus has never violated", () => {
    // The one member of the set with no existing debt, so it gets the assertion
    // the other three cannot have. It is also the worst of the four: it tells a
    // reader who does not know that they should have.
    expect(hits.filter((h) => h.word === "as you know")).toEqual([]);
  });

  it("reports the worst offenders, so paying the debt down has a starting point", () => {
    const byWord = new Map<string, number>();
    for (const h of hits) byWord.set(h.word, (byWord.get(h.word) ?? 0) + h.count);
    // Not an assertion about content — a guard that the measurement still sees
    // the corpus. If a refactor broke `markdown` or `proseOf`, every count would
    // silently drop to zero and the ceiling above would pass vacuously.
    expect(total).toBeGreaterThan(0);
    expect(byWord.get("just") ?? 0).toBeGreaterThan(0);
  });
});
