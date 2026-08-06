// HL05 chapter capability layer — loader and policy round-trip.
//
// This slice deliberately ships NO gates (those are HL-C03). What it must prove is
// narrower and more foundational: that the ledger and the policy load off real disk
// with the shapes the gates will later depend on, and that an unauthored track is
// distinguishable from an empty one. If that distinction collapses, the gap report
// silently loses the debt it exists to measure.

import { describe, it, expect } from "vitest";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import {
  loadTrackChapters,
  loadChapterPolicy,
  defaultCurriculumRoot,
  loadLessons,
} from "../src/loader.js";
import { handwrittenBookChapters } from "../src/book-cli.js";
import type { TrackChapters, ChapterPolicy } from "../src/types.js";

const policy = loadChapterPolicy();
const ledgers = loadTrackChapters();

describe("chapter policy", () => {
  it("loads with every tunable present", () => {
    expect(policy.version).toBe(1);
    expect(typeof policy.payoffRepresentativeness).toBe("number");
    expect(typeof policy.maxNewAtomsPerLesson).toBe("number");
    expect(typeof policy.maxNewAtomsPerChapter).toBe("number");
  });

  it("keeps representativeness a share, not a count", () => {
    expect(policy.payoffRepresentativeness).toBeGreaterThan(0);
    expect(policy.payoffRepresentativeness).toBeLessThanOrEqual(1);
  });

  it("keeps the chapter ramp budget at or above the lesson budget", () => {
    // A chapter that may introduce less than a single lesson would be incoherent.
    expect(policy.maxNewAtomsPerChapter).toBeGreaterThanOrEqual(policy.maxNewAtomsPerLesson);
  });

  it("records the measurement its thresholds were drawn from", () => {
    // The thresholds are only defensible if the distribution behind them is written
    // down. Without this, a later reader cannot tell a measured value from a guess.
    const raw = policy as ChapterPolicy & { provenance?: Record<string, unknown> };
    expect(raw.provenance).toBeDefined();
    expect(raw.provenance?.corpus).toBeTruthy();
    expect(raw.provenance?.rationale).toBeTruthy();
  });
});

describe("chapter capability ledgers", () => {
  it("loads at least the authored Spanish ledger", () => {
    expect(ledgers.length).toBeGreaterThanOrEqual(1);
    expect(ledgers.map((l) => l.language)).toContain("spanish");
  });

  it("returns ledgers in stable language order", () => {
    const languages = ledgers.map((l) => l.language);
    expect(languages).toEqual([...languages].sort());
  });

  it("gives every authored chapter a capability and a payoff", () => {
    for (const ledger of ledgers) {
      for (const chapter of ledger.chapters) {
        expect(Number.isInteger(chapter.chapter)).toBe(true);
        expect(chapter.title.trim()).not.toBe("");
        expect(chapter.label.trim()).not.toBe("");
        expect(chapter.canDo.trim()).not.toBe("");
        expect(chapter.payoff.lesson.trim()).not.toBe("");
        expect(chapter.payoff.summary.trim()).not.toBe("");
        expect(chapter.payoff.assesses.length).toBeGreaterThan(0);
      }
    }
  });

  it("states each canDo in the first person, as the reader's own claim", () => {
    for (const ledger of ledgers) {
      for (const chapter of ledger.chapters) {
        expect(chapter.canDo.startsWith("I can ")).toBe(true);
      }
    }
  });

  it("uses one entry per chapter number per track", () => {
    for (const ledger of ledgers) {
      const numbers = ledger.chapters.map((c) => c.chapter);
      expect(new Set(numbers).size).toBe(numbers.length);
    }
  });

  it("points every payoff at a lesson that exists in that same chapter", () => {
    const lessons = loadLessons();
    const byId = new Map(lessons.map((l) => [String(l.frontmatter.id), l]));
    for (const ledger of ledgers) {
      for (const chapter of ledger.chapters) {
        const lesson = byId.get(chapter.payoff.lesson);
        expect(lesson, `${ledger.language} ch${chapter.chapter} payoff`).toBeDefined();
        expect(lesson?.language).toBe(ledger.language);
        expect(Number(lesson?.frontmatter.chapter)).toBe(chapter.chapter);
      }
    }
  });

  it("only claims atoms the payoff lesson actually practises", () => {
    // The payoff's `assesses` is a claim about a real lesson. If it can drift from
    // that lesson's own declared practice set, the representativeness gate would be
    // measuring authored optimism rather than taught material.
    const lessons = loadLessons();
    const byId = new Map(lessons.map((l) => [String(l.frontmatter.id), l]));
    for (const ledger of ledgers) {
      for (const chapter of ledger.chapters) {
        const lesson = byId.get(chapter.payoff.lesson);
        const practised = new Set(
          (lesson?.frontmatter["practises.knowledge"] as string[] | undefined) ?? [],
        );
        for (const atom of chapter.payoff.assesses) {
          expect(practised.has(atom), `${chapter.payoff.lesson} practises ${atom}`).toBe(true);
        }
      }
    }
  });

  it("matches the titles and labels the book generator still owns", () => {
    // HL-C04 inverts this dependency so chapters.json becomes canonical. Until then
    // the two must agree, or the transition would silently rename printed chapters.
    const config = JSON.parse(
      readFileSync(join(defaultCurriculumRoot(), "core", "book-generation.json"), "utf8"),
    ) as { targets: { language: string; chapter: number; title: string; label: string }[] };
    for (const ledger of ledgers) {
      for (const chapter of ledger.chapters) {
        const target = config.targets.find(
          (t) => t.language === ledger.language && t.chapter === chapter.chapter,
        );
        if (!target) continue;
        expect(chapter.title).toBe(target.title);
        expect(chapter.label).toBe(target.label);
      }
    }
  });

  it("matches the titles and labels of the hand-written chapters too", () => {
    // The generator owns only part of each book. Early chapters were written by hand
    // before the manifest existed, so the check above found no target and skipped them
    // — which left their ledger titles verified by nothing at all. `handwritten[]`
    // closes that hole: every chapter a ledger claims is now checked against one of the
    // two lists, so HL-C04 cannot silently rename a printed chapter on the way through.
    const handwritten = handwrittenBookChapters();
    for (const ledger of ledgers) {
      for (const chapter of ledger.chapters) {
        const entry = handwritten.find(
          (h) => h.language === ledger.language && h.chapter === chapter.chapter,
        );
        if (!entry) continue;
        expect(chapter.title, `${ledger.language} ch${chapter.chapter} title`).toBe(entry.title);
        expect(chapter.label, `${ledger.language} ch${chapter.chapter} label`).toBe(entry.label);
      }
    }
  });

  it("leaves no ledger chapter unchecked by either list", () => {
    // The guard on the two tests above: without this, deleting an entry from either list
    // would turn a real assertion into a silent `continue` and nothing would notice.
    const config = JSON.parse(
      readFileSync(join(defaultCurriculumRoot(), "core", "book-generation.json"), "utf8"),
    ) as { targets: { language: string; chapter: number }[] };
    const handwritten = handwrittenBookChapters();
    for (const ledger of ledgers) {
      for (const chapter of ledger.chapters) {
        const covered =
          config.targets.some(
            (t) => t.language === ledger.language && t.chapter === chapter.chapter,
          ) ||
          handwritten.some(
            (h) => h.language === ledger.language && h.chapter === chapter.chapter,
          );
        expect(covered, `${ledger.language} ch${chapter.chapter} has no title/label source`).toBe(
          true,
        );
      }
    }
  });
});

describe("loadTrackChapters on a synthetic root", () => {
  function withRoot(build: (root: string) => void): TrackChapters[] {
    const root = mkdtempSync(join(tmpdir(), "hl-chapters-"));
    try {
      build(root);
      return loadTrackChapters(root);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  }

  it("skips tracks with no ledger rather than inventing one", () => {
    const loaded = withRoot((root) => {
      mkdirSync(join(root, "klingon"));
      // no chapters.json written
    });
    expect(loaded).toEqual([]);
  });

  it("distinguishes an unauthored track from an authored-but-empty one", () => {
    // This is the distinction the gap report depends on: "not yet written" and
    // "written, covering nothing" are different kinds of debt.
    const loaded = withRoot((root) => {
      mkdirSync(join(root, "absent"));
      mkdirSync(join(root, "empty"));
      writeFileSync(
        join(root, "empty", "chapters.json"),
        JSON.stringify({ version: 1, language: "empty", chapters: [] }),
      );
    });
    expect(loaded.map((l) => l.language)).toEqual(["empty"]);
    expect(loaded[0]?.chapters).toEqual([]);
  });

  it("ignores stray files that are not directories", () => {
    const loaded = withRoot((root) => {
      writeFileSync(join(root, "README.md"), "not a track");
    });
    expect(loaded).toEqual([]);
  });
});
