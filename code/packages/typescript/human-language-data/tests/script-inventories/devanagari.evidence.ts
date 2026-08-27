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
  },
};
