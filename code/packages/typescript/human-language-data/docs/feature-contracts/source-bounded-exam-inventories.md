# Source-bounded exam inventories (HL20)

An exam inventory is complete only when it accounts for all four content
dimensions: communicative functions, grammar, phonology/orthography, and
lexicon. Each dimension carries its own `complete` or `partial` status, source,
and a plain-language boundary note. Completeness is derived; it is never inferred
from a file merely existing.

```ts
import {
  isExamInventoryComplete,
  loadExamInventory,
  measureExamCoverage,
} from "@coding-adventures/human-language-data";

const inventory = loadExamInventory("german", "A1");
isExamInventoryComplete(inventory); // false until every dimension is source-closed
measureExamCoverage(inventory, lessons); // its enumerated points still count
```

A partial inventory remains useful evidence and still generates named exam-point
gaps. It also remains an `exam-inventory` backlog item. Thus 100% coverage means
"all currently enumerated points are taught," never "the whole exam construct is
known." The initial baseline is deliberately honest: zero complete and three
partial inventories across 138 track/level targets.

Spanish A1 shows what that distinction buys. It read 85/85 and 100% while it
enumerated grammar alone. Adding the PCIC functional inventory, the general and
specific notions, and the orthography inventory took it to **223/273 and 82%**,
with 50 points that no atom in the corpus corresponds to — clothing, the
alphabet, most punctuation, the affirmative imperative. No lesson was removed
between those two readings. The number fell because the target stopped
flattering the book, which is the whole point of measuring against an external
list. Spanish A1 is still a partial inventory: its pronunciation dimension has no
A1-only boundary in the source (see HL20 §6a).

Committed inventory assertions follow the same ownership boundary as their
curriculum. Generic probe semantics and hostile-file guards stay in
`tests/exam-inventory.test.ts`; language-specific coverage pins, unmapped-point
audits, and source notes live in `tests/exam-inventories/<language>.test.ts`
(with older already-independent tracks retaining their
`tests/exam-inventory-<language>.test.ts` owners). A content tranche edits only
its language-owned test. The owned suites use `loadTrackLessons` rather than
loading all 23 tracks, so parallel ownership does not multiply the full-corpus
cost tracked by #12732.

Hindi, Malayalam, and Marathi A1 inventories are also shard-native. Stable document fields
and the ordered identity manifest live in each
`core/exam-inventory-<language>-a1.d/_meta.json`; every complete exam point lives
in one canonical `NNNN-<point-id>.json` owner. `loadExamInventory` preserves the
same public `ExamInventory` contract while rejecting missing, unexpected,
reordered, unsafe, noncanonical, nested, linked, or resurrected aggregate data.
Ordinary lesson work edits only the points whose probes or notes changed.
