# HL15 — The completion plan: a backlog that is a function, not a file

**Status:** specification, 2026-08-17
**Supersedes as the ordering authority:** the hand-maintained prioritization
sections in `code/learning/human-languages/BACKLOG.md`, which stay as the
narrative record of what was learned and why.

**Goal (owner, restated 2026-08-12 and unchanged since):** *"The goal is not
whether something touches some level. The goal is can someone pass that level of
exam with just reading the book and slowly following its gentle ramp."*

**Second goal (owner, 2026-08-17):** *"I want to also introduce the script as
well for many of these languages in a very gentle way."* This is not a
sub-clause of the first. A reader who cannot decode the writing system cannot
sit a reading paper at all, and the script ramp is the one ramp in this project
that is **finite and ends** — so it is ordered above the vocabulary grind rather
than behind it. See §4.2.

---

## 1. Why this spec exists

The backlog has 148 hand-written entries and a prioritization header that was
already three days stale when it was read on 2026-08-17: it ordered work against
a frame that HL-C183/HL-C184 had replaced. That is not a discipline failure. It
is what happens when the ordering lives in prose that nothing recomputes.

The corpus, meanwhile, already knows the answer. `level-gate.ts` measures every
track against four criteria and reports the shortfall in each criterion's own
units. `script-closure.ts` counts, per track, exactly how many glyphs a reader is
asked to decode that no lesson taught. `exam-inventory.ts` measures coverage
against an external, finite published list. Every number needed to order the work
is computed on every run.

So the backlog should be **derived from those numbers, not typed beside them**:

> **The work queue is a pure function of the measured deficit.**
> `BACKLOG.md` records what was *learned*. `plan-cli` computes what is *next*.

This is the same argument `exam-inventory.ts` already makes for probes over
annotations, one level up. An annotation goes stale silently and in the
flattering direction. So does a hand-ordered backlog.

## 2. What "done" means, per track

A track is **done** when it has *attained* C2 under the HL09 §3.1 gate — all four
criteria hold at C2 and at every level below it — **and** every exam point on the
external inventory for each level is covered.

The four criteria are already implemented (`level-gate.ts`); this spec adds the
fifth requirement and the ordering over all five.

| # | criterion | measured by | source of truth |
|---|---|---|---|
| 1 | every spine node at the level is realized | `level-gate.ts` | `core/spine.json` |
| 2 | cumulative vocabulary target met | `level-gate.ts` | `LEVEL_VOCABULARY` |
| 3 | no lesson over the new-atom budget | `ramp.ts` | HL14 |
| 4 | every atom revisited at least twice | `continuity.ts` | HL09 §3.1 |
| 5 | **every exam point covered** | `exam-inventory.ts` | the awarding body |
| 6 | **script closure: no untaught glyph is load-bearing** | `script-closure.ts` | HL11 |

Criteria 5 and 6 are the two this spec adds to the definition of done. Criterion 5
is the only one that comes from outside this repository, which is why it is the
one that settles the owner's question. Criterion 6 is the one the second goal
names.

**Nothing here measures a learner.** HL-C182's warning stands and is restated so
it cannot be lost: a track that satisfies all six is a *corpus* claim. The honest
test needs a human sitting a past paper, and that item is on the queue (§6).

## 3. The item families

Seven, and every one of them is enumerable from a report the package already
produces. No work item is ever authored by hand; a *finding* is, and findings go
in `BACKLOG.md`.

| kind | one item is | outstanding is counted in | tranche |
|---|---|---|---|
| `exam-inventory` | write the external point list for one (track, level) | inventories | 1 |
| `script-closure` | teach the glyphs one track shows but never taught | glyphs | 10 |
| `vocabulary` | raise one track's headword count at one level | headwords | 35 |
| `exam-point` | cover one named point from the inventory | points | 5 |
| `reinforcement` | revisit atoms that appear once and never again | atoms | 25 |
| `atom-budget` | split lessons that introduce more than the budget | lessons | 10 |
| `spine-nodes` | realize the level's unrealized spine nodes | nodes | 3 |

Tranche sizes are **empirical, not aesthetic**. 35 headwords is the size
HL-C198's six merged tranches actually sustained at one PR each; 10 glyphs is a
chapter's worth of drizzle under HL11. They are recorded in one constant so a
future measurement can move them, and moving one re-shapes the queue without
anybody editing a list.

## 4. The ordering

Three keys, applied in order. All three are mechanical; none is a judgement call
made at queue-build time.

### 4.1 Level rank first — the floor is universal

Every pre-A1 item in every track outranks every A1 item in any track.

This is the gentle ramp expressed as an ordering. A track that climbs to A2 while
its pre-A1 vocabulary sits at 40% has not built a ramp, it has built a cliff with
some A2 lessons on top — which is precisely the state HL-C183 found and named. All
22 tracks are currently `inProgressAt: pre-A1`, so today this key sorts nothing.
It exists to stop the next climb from starting early, and it will bind the moment
one track clears the floor.

### 4.2 Kind priority second

```
1. exam-inventory   you cannot aim at a target you have not written down
2. script-closure   decoding is a precondition for reading, and it ENDS
3. vocabulary       the dominant remaining mass
4. exam-point       named gaps against the external list
5. reinforcement    retention — what separates a corpus claim from a learner one
6. atom-budget      a ramp that got steeper is a regression, not a backlog item
7. spine-nodes      functional coverage, the coarsest of the six
```

**Why `exam-inventory` is first and cheapest.** Until the list exists, every other
number for that (track, level) is a proxy for something nobody is graded on. It is
also finite: 22 tracks × 6 CEFR levels = 132 inventories, of which **one** exists
(Spanish A1). Research and JSON authoring, no content.

**Why `script-closure` outranks `vocabulary`.** Two reasons, and the second is the
one that decides it. First, the owner asked for it directly. Second, it is the only
family with a **terminal state**: Tamil has 247 glyphs and then it is done forever,
whereas vocabulary runs to 16,000 per track. Finite work that unblocks infinite work
goes first. A reader who cannot decode a glyph cannot read a word built from it, so
every vocabulary tranche authored into an unclosed script is authored onto sand.

**Why `atom-budget` is near the bottom despite being cheap.** It is a *regression*
signal, not a deficit. A lesson over budget means content already landed too steeply.
HL-C167's rule applies — revert the content, do not re-seat the number — so these
are handled where they are found rather than queued as fresh work.

### 4.3 Cheapest first, then alphabetical

Within one level and one kind, the track with the fewest outstanding tranches goes
first, ties broken by language name so the queue is deterministic and two runs on
one commit agree.

Cheapest-first is also the owner's standing alternation rule expressed
mechanically: it rotates work onto whichever track is furthest behind its
siblings, so no track is left to rot, without anybody maintaining a rota.

## 5. The size of the thing

Enumerating every item to C2 would produce roughly **11,000 work items**, and a
file holding them would be wrong within one merge. So `plan-cli` emits:

- the **head** — the next N items, fully enumerated, ready to pick up; and
- the **projection** — per family and per track, how many items remain to a
  chosen ceiling, counted rather than listed.

The projection is what makes the scale honest without pretending the tail is
planned. At the corpus's measured density the tail is years of work, and §6 of
HL-C184 already accepted that.

**The enumeration rule is the artifact. The queue head is what you act on.**

## 6. What this plan does not measure, recorded so it is not discovered late

Carried forward from HL-C182 and HL-C184 Phase 3, unchanged:

- **retention over time** — nothing here re-tests a reader a month later;
- **production under time pressure** — an exam is timed and this corpus is not;
- **listening at natural speed** — narration exists; natural-speed audio does not;
- **the papers themselves** — 704 of 704 activities are one shape (`kind: "text"`),
  and a candidate must clear reading, writing, listening and oral *independently*.

The last one is a genuine gap in this spec's own criterion 5: an inventory covers
what a candidate must *hold*, not the *shapes* they must perform. Criterion 5 will
pass on a track that would still fail the paper. That is recorded here as a known
limit and queued as its own family when the task-shape work (HL-C130) lands, rather
than left to be rediscovered.

**And the terminal item, which no report can close:** a human sits a real past
paper. Until that happens, every claim in this file is about a corpus.
