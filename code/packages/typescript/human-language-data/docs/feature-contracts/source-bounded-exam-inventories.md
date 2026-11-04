# Source-bounded exam inventories (HL20)

An exam inventory is complete only when it accounts for all four content
dimensions: communicative functions, grammar, phonology/orthography, and
lexicon. A complete dimension must use one of two honest boundaries:

1. a finite external source that actually enumerates the dimension; or
2. when no such source exists, an explicit finite checked-in project-owned
   selection whose exact points, assessment purpose, external anchors, and
   source limitations are all reviewable.

The second case closes this repository's enumeration only. It does not turn the
selection into an external syllabus or prove lesson coverage, task performance,
standard setting, learner readiness, validation, certification, or attainment.
Each dimension carries its own `complete` or `partial` status, source, and a
plain-language boundary note. Completeness is derived; it is never inferred
from a file merely existing.

```ts
import {
  isExamInventoryComplete,
  loadExamInventory,
  measureExamCoverage,
} from "@coding-adventures/human-language-data";

const inventory = loadExamInventory("german", "A1");
isExamInventoryComplete(inventory); // false until every dimension has a finite honest boundary
measureExamCoverage(inventory, lessons); // its enumerated points still count
```

A partial inventory remains useful evidence and still generates named exam-point
gaps. It also remains an `exam-inventory` backlog item. Thus 100% coverage means
"all currently enumerated points are taught," never "the whole exam construct is
known." The planner derives complete and partial totals from the checked-in
dimension statuses rather than treating file presence as completion.

Spanish A1 shows what that distinction buys. It read 85/85 and 100% while it
enumerated grammar alone. Adding the PCIC functional inventory, the general and
specific notions, and the orthography inventory took it to **223/273 and 82%**,
with 50 points that no atom in the corpus corresponds to — clothing, the
alphabet, most punctuation, the affirmative imperative. No lesson was removed
between those two readings. The number fell because the target stopped
flattering the book, which is the whole point of measuring against a finite,
independently reviewable target list. Spanish A1 is still a partial inventory:
its pronunciation dimension has no A1-only boundary in the source (see HL20
§6a).

Kannada B2 demonstrates the project-owned case. Its external anchors bound
generic B2 performance, intermediate four-skill instruction, Kannada encoding,
and contemporary genre domains, but none is presented as a finite Kannada B2
grammar or word list. The checked-in inventory therefore names exactly 36
project-selected demands, nine per dimension, and records those source limits.
That makes the repository's enumeration complete while its strict lesson
coverage can remain 0/36 and every uncovered point remains planner-visible.

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
