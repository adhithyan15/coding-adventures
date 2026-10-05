// Exact real-corpus evidence owned by the Devanagari inventory.
// See HL24: unrelated script authors must not share an executable edit surface.

import { expect } from "vitest";
import { validate } from "../../src/validate.js";
import type { ScriptEvidenceContext } from "./helpers.js";

export const scriptInventoryEvidence = {
  name: "Devanagari",
  assert({
    taxonomy,
    lessons,
    scripts,
    affected,
    missingByScript,
  }: ScriptEvidenceContext): void {
    const gaps = validate({ taxonomy, lessons, scripts }).filter(
      (issue) =>
        issue.level === "warning" &&
        issue.code === "uncovered-glyphs" &&
        issue.message.includes("devanagari.json"),
    );
    expect(gaps).toEqual([]);
    const missing = new Set(
      gaps.flatMap((issue) =>
        issue.message
          .split("characters not yet in devanagari.json: ")[1]!
          .split(" "),
      ),
    );
    expect(missing).toEqual(new Set());
    expect(scripts.devanagari!.complete).toBe(true);
    // These eleven once lifted the pen after every run of their Commons
    // animation or panel diagram (3, 3, 2, 2, 3, 3, 2, 2, 2, 3 and 6 lifts),
    // a count at most 15% of HP Labs India's native writers use. They now
    // lift only where those writers do. Each note must cite the source's
    // movement count, the native stroke count and its share; each variation
    // must say the data gives counts only.
    const nativeLifts: ReadonlyArray<
      readonly [string, number, string, string, string, string]
    > = [
      ["क", 1, "four", "two", "74", "61 of 82"],
      ["य", 1, "four", "two", "89", "74 of 83"],
      ["र", 1, "three", "two", "96", "81 of 84"],
      ["प", 1, "three", "two", "98", "80 of 82"],
      ["ध", 1, "four", "two", "81", "64 of 79"],
      ["ल", 1, "four", "two", "54", "45 of 84"],
      ["द", 1, "three", "two", "95", "77 of 81"],
      ["ठ", 1, "three", "two", "89", "73 of 82"],
      ["घ", 1, "three", "two", "83", "68 of 82"],
      ["ष", 2, "four", "three", "83", "69 of 83"],
      ["औ", 5, "seven", "six", "60", "49 of 82"],
    ];
    for (const [
      glyph,
      lifts,
      movements,
      strokes,
      share,
      samples,
    ] of nativeLifts) {
      const letter = scripts.devanagari!.letters.find(
        (entry) => entry.glyph === glyph,
      )!;
      expect(letter.penLifts, glyph).toBe(lifts);
      expect(letter.strokeOrderNote, glyph).toContain(
        `The source shows ${movements} movements; native writers draw ${glyph} as ${strokes} pen-down strokes (${share}% of HP Labs India's native-writer samples, ${samples}).`,
      );
      expect(letter.strokeOrderSource?.citation, glyph).toMatch(
        /^(Opiaterein|Saurmandal), ‘De/,
      );
      for (const phrase of [
        "they are not a count of native pen lifts",
        "hpl-dvng-iso-char, https://lipitk.sourceforge.net/datasets/dvngchardata.htm",
        "MIT-licensed LipiTk 4.0 Devanagari recognizer",
        "Those counts say how many strokes writers use, not where they break",
      ]) {
        expect(letter.strokeOrderSource?.variation, glyph).toContain(phrase);
      }
      expect(
        (letter.strokeOrder ?? []).filter((step) => step.startsWith("lift")),
        glyph,
      ).toHaveLength(lifts);
      expect(
        (letter.strokeOrder ?? []).filter((step) =>
          step.startsWith("without lifting"),
        ).length,
        glyph,
      ).toBeGreaterThan(0);
      expect(missingByScript.get("devanagari.json")?.has(glyph) ?? false).toBe(
        false,
      );
      expect(affected.get(glyph) ?? 0).toBe(0);
    }
  },
};
