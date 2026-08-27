// Stable consumer for script-owned glyph evidence.
//
// Glyph changes belong in `glyph-evidence/*.evidence.ts`, never in this file.
// Vite expands the eager glob at build time, while ESM module caching and this
// single context keep the script corpus and Language Ladder helpers loaded once.

import { describe, it } from "vitest";
import { SCRIPTS } from "@coding-adventures/script-ductus";
import { isSyllabary } from "../src/syllabary";
import { buildSyllableMatrix } from "../src/matrix";
import {
  assertValidGlyphEvidenceRanks,
  compareGlyphEvidence,
  type GlyphEvidenceModule,
  type LocatedGlyphEvidence,
} from "./glyph-evidence/types";

const modules = import.meta.glob<GlyphEvidenceModule>("./glyph-evidence/*.evidence.ts", {
  eager: true,
});

const context = Object.freeze({ SCRIPTS, isSyllabary, buildSyllableMatrix });
const evidence: LocatedGlyphEvidence[] = Object.entries(modules).flatMap(([modulePath, module]) =>
  module.default.map((entry) => ({ ...entry, modulePath })),
);

// Explicit numeric ranks preserve the pre-shard execution order without making
// the aggregator a hand-edited file registry. Same-rank additions are allowed:
// parallel agents can independently choose the next rank, and stable textual
// tie-breakers give their merged evidence one deterministic order.
const orderBySuite = new Map<string, number>();
for (const entry of evidence) {
  assertValidGlyphEvidenceRanks(entry);
  const orderForSuite = orderBySuite.get(entry.suite);
  if (orderForSuite !== undefined && orderForSuite !== entry.suiteOrder) {
    throw new Error(
      `glyph evidence suite '${entry.suite}' uses both ${orderForSuite} and ${entry.suiteOrder}`,
    );
  }
  orderBySuite.set(entry.suite, entry.suiteOrder);
}

evidence.sort(compareGlyphEvidence);

for (const [suite] of [...orderBySuite].sort(
  ([leftSuite, leftOrder], [rightSuite, rightOrder]) =>
    leftOrder - rightOrder ||
    (leftSuite < rightSuite ? -1 : leftSuite > rightSuite ? 1 : 0),
)) {
  describe(suite, () => {
    for (const entry of evidence.filter((candidate) => candidate.suite === suite)) {
      it(entry.name, () => entry.verify(context));
    }
  });
}
