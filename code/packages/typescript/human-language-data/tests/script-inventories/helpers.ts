// The validator owns glyph-closure semantics. Inventory tests own only their
// exact evidence, so they share this measurement without sharing an edit surface.
import { expect } from "vitest";
import type { loadEverything } from "../../src/loader.js";
import { validate } from "../../src/validate.js";
type Corpus = ReturnType<typeof loadEverything>;
type GlyphInputs = Pick<Corpus, "taxonomy" | "lessons" | "scripts">;
export type ScriptEvidenceContext = GlyphInputs & {
  affected: Map<string, number>;
  missingByScript: Map<string, Set<string>>;
};
export interface ScriptInventoryEvidenceModule {
  scriptInventoryEvidence: {
    name: string;
    assert(context: ScriptEvidenceContext): void;
  };
}
export function measureGlyphGaps({ taxonomy, lessons, scripts }: GlyphInputs): {
  affected: Map<string, number>;
  missingByScript: Map<string, Set<string>>;
} {
  const gaps = validate({ taxonomy, lessons, scripts }).filter(
    (issue) => issue.level === "warning" && issue.code === "uncovered-glyphs",
  );
  const affected = new Map<string, number>();
  const missingByScript = new Map<string, Set<string>>();
  for (const issue of gaps) {
    const match = issue.message.match(/characters not yet in ([^:]+): (.*)$/);
    expect(match, issue.message).not.toBeNull();
    const [, file, characters] = match!;
    const missing = missingByScript.get(file!) ?? new Set<string>();
    for (const character of characters!.split(" ")) {
      missing.add(character);
      affected.set(character, (affected.get(character) ?? 0) + 1);
    }
    missingByScript.set(file!, missing);
  }
  return { affected, missingByScript };
}
