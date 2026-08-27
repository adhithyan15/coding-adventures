// Stable consumer for script-owned glyph evidence.
//
// Glyph changes belong in `glyph-evidence/*.evidence.ts`, never in this file.
// Vite expands the eager glob at build time, while ESM module caching and this
// single context keep the script corpus and Language Ladder helpers loaded once.

import { describe, it } from "vitest";
import { SCRIPTS } from "@coding-adventures/script-ductus";
import { isSyllabary } from "../src/syllabary";
import { buildSyllableMatrix } from "../src/matrix";
import type { GlyphEvidence, GlyphEvidenceModule } from "./glyph-evidence/types";

interface LocatedEvidence extends GlyphEvidence {
  readonly modulePath: string;
}

const modules = import.meta.glob<GlyphEvidenceModule>("./glyph-evidence/*.evidence.ts", {
  eager: true,
});

const context = Object.freeze({ SCRIPTS, isSyllabary, buildSyllableMatrix });
const evidence: LocatedEvidence[] = Object.entries(modules).flatMap(([modulePath, module]) =>
  module.default.map((entry) => ({ ...entry, modulePath })),
);

// Fail closed before registering tests. Explicit numeric ranks preserve the
// pre-shard execution order without making the aggregator a hand-edited file
// registry; stable textual tie-breakers make even malformed duplicate ranks
// deterministic enough to report the same error on every platform.
const suiteByOrder = new Map<number, string>();
const orderBySuite = new Map<string, number>();
const caseKeys = new Set<string>();
for (const entry of evidence) {
  const suiteAtOrder = suiteByOrder.get(entry.suiteOrder);
  if (suiteAtOrder !== undefined && suiteAtOrder !== entry.suite) {
    throw new Error(
      `glyph evidence suite order ${entry.suiteOrder} is shared by '${suiteAtOrder}' and '${entry.suite}'`,
    );
  }
  const orderForSuite = orderBySuite.get(entry.suite);
  if (orderForSuite !== undefined && orderForSuite !== entry.suiteOrder) {
    throw new Error(
      `glyph evidence suite '${entry.suite}' uses both ${orderForSuite} and ${entry.suiteOrder}`,
    );
  }
  suiteByOrder.set(entry.suiteOrder, entry.suite);
  orderBySuite.set(entry.suite, entry.suiteOrder);

  const caseKey = `${entry.suiteOrder}\u0000${entry.caseOrder}`;
  if (caseKeys.has(caseKey)) {
    throw new Error(
      `glyph evidence suite '${entry.suite}' has duplicate case order ${entry.caseOrder}`,
    );
  }
  caseKeys.add(caseKey);
}

evidence.sort(
  (left, right) =>
    left.suiteOrder - right.suiteOrder ||
    left.caseOrder - right.caseOrder ||
    (left.modulePath < right.modulePath ? -1 : left.modulePath > right.modulePath ? 1 : 0) ||
    (left.name < right.name ? -1 : left.name > right.name ? 1 : 0),
);

for (const [suiteOrder, suite] of [...suiteByOrder].sort(([left], [right]) => left - right)) {
  describe(suite, () => {
    for (const entry of evidence.filter((candidate) => candidate.suiteOrder === suiteOrder)) {
      it(entry.name, () => entry.verify(context));
    }
  });
}
