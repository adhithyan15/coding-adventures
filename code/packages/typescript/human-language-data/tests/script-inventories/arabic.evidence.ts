// Exact real-corpus evidence owned by the Arabic inventory.
// See HL24: unrelated script authors must not share an executable edit surface.

import { expect } from "vitest";
import type { ScriptEvidenceContext } from "./helpers.js";

export const scriptInventoryEvidence = {
  name: "Arabic",
  assert({
    taxonomy,
    lessons,
    scripts,
    affected,
    missingByScript,
  }: ScriptEvidenceContext): void {
    expect(missingByScript.get("arabic.json")?.has("ٓ") ?? false).toBe(false);
  },
};
