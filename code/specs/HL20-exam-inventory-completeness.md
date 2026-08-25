# HL20 — Source-bounded exam inventory completeness

**Status:** specification, 2026-08-20

**Extends:** HL15, HL16, and HL18

**Durable work item:** [#12230](https://github.com/adhithyan15/coding-adventures/issues/12230)

## 1. The failure this closes

An exam-inventory file used to count as complete merely because it parsed. That
made a narrow but useful source dangerous. Adding an official word list for one
level could suppress the whole `exam-inventory/<language>/<level>` backlog item,
even though grammar, sound/spelling, and communicative functions were still
unknown. A 100% point-coverage result could then mean only “the book teaches all
words we happened to list” while looking like “the book covers the exam.”

This is a flattering failure and therefore a release blocker. Presence is not
completeness. Coverage of a partial list is not coverage of the construct.

## 2. The four required content dimensions

Every `<track, certifiable level>` inventory has exactly these dimensions:

1. `communicative-functions` — purposes, situations, discourse acts, register,
   and pragmatic choices the candidate must handle;
2. `grammar` — morphology, syntax, and productive/receptive structural control;
3. `phonology-orthography` — sound contrasts, decoding, spelling, punctuation,
   and script conventions; and
4. `lexicon` — words, multiword units, domains, and receptive/productive bounds.

Task format is intentionally not a fifth dimension. HL18 owns the sourced
reading, listening, writing, and speaking performance shapes. HL16 owns pass
rules, writing progression, mocks, and human validation. All three artifacts are
required for an exam-readiness claim; none substitutes for another.

## 3. A dimension is a sourced boundary, not a checkbox

Each dimension declares:

```json
{
  "status": "complete | partial",
  "source": "stable awarding-body URL or checked-in project source",
  "note": "what the source enumerates and what remains outside it"
}
```

`complete` means the cited source or source set gives a closed enough boundary
to enumerate the entire dimension in project-owned words. The note explains the
reviewer's reasoning. `partial` means the evidence is useful but does not close
that boundary. Unknowns are partial, never silently promoted by editorial
confidence.

The strict loader rejects a missing or extra dimension, an unknown status, empty
provenance, an empty boundary note, mismatched language/level identity, and
malformed point metadata. These checks cannot prove that a citation was
interpreted correctly; review must still inspect the primary source. They do
make every completeness claim explicit, local, and falsifiable.

## 4. Derived inventory completeness

Inventory completeness is derived mechanically:

> **complete inventory = every required dimension has status `complete`**

There is no independent file-level boolean to drift away from the dimensions.
One partial dimension makes the entire inventory partial. A valid partial file
remains loadable and its enumerated points remain measurable. It must not count
as a completed inventory or remove its stable completion-plan item.

This yields two simultaneous, non-contradictory statements:

- “Spanish A1 covers 223/273 points currently enumerated”; and
- “Spanish A1’s overall inventory is partial.”

The first drives immediate lesson work. The second preserves the research work
needed to know whether those 273 points cover the whole target.

Spanish A1 is also the worked example of why the second statement matters. When
this spec was written the file enumerated grammar alone and read 85/85, 100%.
Enumerating three more dimensions from their own PCIC inventories took it to
223/273, 82%, with 50 points that no atom in the corpus corresponds to. Nothing
about the book changed between those two measurements. A 100% that covered one
of four dimensions was precisely the flattering failure §1 describes, and the
lower number is the more complete one.

## 5. Planner behavior and dependency order

For each certifiable track/level pair, the completion plan distinguishes:

- **absent** — write the source-backed inventory;
- **partial** — complete its missing source dimensions while continuing to use
  all valid point probes; and
- **complete** — suppress the inventory-research item, while retaining any
  uncovered `exam-point` work.

The finite source work remains upstream of large curriculum rewrites. A language
may safely receive vocabulary or grammar micro-lessons while another inventory
is being researched, but no track may claim a level complete until its content
inventory, HL18 task shapes, HL16 assessment evidence, writing ramp, and learner
validation all close.

## 6. Migration baseline

The three existing A1 inventories remain valuable and measurable, but their old
presence claims were too broad. At migration:

- Spanish A1 has a source-closed grammar dimension and three partial dimensions;
- French A1 has four partial dimensions; and
- German A1 has four partial dimensions.

The honest corpus baseline is therefore **0 complete and 3 partial inventories
out of 138 track × certifiable-level targets**. No measured exam point is lost.
The backlog grows by three items because the old count hid unfinished research.

## 6a. Progress against that baseline

Spanish A1 has since closed `communicative-functions` and `lexicon` from the
PCIC functional inventory and the general/specific notions respectively, and has
enumerated the orthography half of `phonology-orthography`. It remains
**partial**, on one specific and stated ground: the PCIC publishes its
pronunciation-and-prosody inventory as a single undivided A1–A2 band with no
per-item level marking, while its grammar, functions, notions and orthography
inventories all split A1 from A2 by column. There is therefore no A1-only
pronunciation boundary to restate, and drawing one by editorial judgement is
what §3 forbids. Closing it needs a second source that separates A1 pronunciation
from A2, or an explicit project-owned decision to treat the whole A1–A2
pronunciation band as A1.

This is the intended shape of progress under §4: a dimension flips only when its
own source closes, the file stays partial while any dimension is open, and the
measured point coverage moves independently of both.

## 7. Acceptance tests

The gate is complete when tests prove that:

1. a missing, extra, unsourced, or unbounded dimension fails strict loading;
2. one partial dimension makes the inventory partial;
3. partial inventories still produce measured exam-point coverage;
4. partial inventories remain in the `exam-inventory` queue with “complete the
   partial” wording;
5. only all-complete inventories suppress that queue item; and
6. the real-corpus report names complete and partial counts separately.
