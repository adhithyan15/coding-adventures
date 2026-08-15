# HL14 — The vocabulary ramp: one new word per lesson

**Status:** specification, 2026-08-15
**Rule (owner, 2026-08-15):** *"One new word per lesson but n number of existing
words can be re-emphasized and used?"*
**Scale accepted (owner, same day):** *"That is totally fine with me. Again, not
worried about lesson counts. Just worried about a very gentle ramp."*

---

## 1. The rule

> **A lesson introduces exactly ONE new lexical item.**
> **It may reuse any number of items already taught, and should.**

The cap is on what is **new**. Reuse is free, unlimited, and encouraged — it is
the mechanism by which the ramp stays gentle without the corpus staying small.

Two corollaries follow immediately, and both were previously open questions:

- **A lesson's headword names one item.** A lesson listing the twelve months
  becomes twelve lessons.
- **Lesson count is not a budget.** It is an output. See §5.

## 2. Why this is the right form of "gentle"

The project has circled this for weeks under the heading *gentle ramp*, and the
phrase was ambiguous in a way that cost real work. It was read as *small
chapters* and *a compact book*, and neither was ever meant.

| | |
|---|---|
| **What must stay small** | new material **per lesson** |
| **What must grow without limit** | the **number of lessons** |

Reading it the other way produced a corpus that closed all 33 spine nodes while
failing the pre-A1 vocabulary floor — 211 words against 300 — because chapters
were sized to close a node rather than to carry vocabulary. HL-C183 records that
failure; this spec is the rule that prevents its recurrence.

## 3. What the rule settles

**Counting becomes unambiguous.** Before the rule, Spanish's vocabulary was
211 (headwords), ~285 (lexical items) or 361 (whitespace tokens) — a ±35% band,
with no mechanical rule to choose between them, because `la cabeza` is one noun
in two tokens while `negro, blanco` is two adjectives in two tokens. **After the
rule, headword count IS word count.**

**A proposed schema field is cancelled.** `teaches_items` was designed to record
how many items a lesson teaches. Under this rule it is always 1. **It must not be
built.**

**The reinforcement ratio gets easier.** `R1` asks whether an atom is revisited
within three lessons. A drizzled script letter was alone in that window *by
construction*, which blocked Sanskrit's letter ledger (HL-C167). If every lesson
reuses prior material by design, those windows fill by construction too.
**HL-C167 must be re-measured after the first tranche before any work is done on
it** — the tension may not survive this rule.

## 4. The migration, and why it is not one regex

Measured 2026-08-15 across all 22 tracks:

| | |
|---|---:|
| `type: word` lessons | 1,407 |
| teaching more than one item | **400 (28%)** |
| extra lessons if every one splits | **+718** (upper bound) |

718 is an upper bound because **a multi-token headword is not always a list**:

```
la cabeza              one noun with its article      → do NOT split
السلام عليكم           one greeting                   → do NOT split
negro, blanco          two adjectives                 → split
يناير فبراير مارس …    twelve nouns                   → split
```

A comma is a reliable list separator in the Latin-script tracks. The Indic and
Arabic month lists are **space-separated**, and space is also what joins
`la cabeza`. No single rule covers both, so **the split is a reviewed pass per
track, never a corpus-wide regex.** Spanish goes first as the reference track,
per HL13's two-reference-track method.

## 5. The scale this implies, stated before it is discovered

At one new word per lesson, a track reaches C2's 16,000-word target
(`LEVEL_VOCABULARY`, `level-gate.ts`) in **~16,000 lessons**. That is *fewer*
than the corpus's current trajectory implied, because 1.0 words/lesson is denser
than the 0.58 it has been running at.

**Across 22 tracks: ~352,000 lessons.**

The owner has seen and accepted this figure. It is recorded here so that no later
session treats it as a surprise, an error, or a reason to compress — and so that
nobody proposes relaxing the ramp to reduce it. Per HL12 §3.1: **no rule in this
curriculum may be relaxed to save pages, and none may be relaxed to save
lessons.**

## 6. What this does not claim

A track at 16,000 words is a statement about **the corpus**, not about a reader.
It does not measure retention, production under time pressure, listening at
natural speed, or exam task formats. See HL-C182. The honest test of whether
someone can sit a C2 paper is a person sitting one.

## 7. Order of work

1. This spec (specs before implementation, per CLAUDE.md §8).
2. Split the 400 multi-item lessons, per track, reviewed — Spanish first.
3. **Re-measure everything**: vocabulary per track, R1, the atom budget. The
   211/300 figure will move, and HL-C167 may resolve itself.
4. Author against the remaining deficit, one new word at a time, alternating
   Spanish with the Indic six so no track is left to rot.

## 8. Provenance

| claim | source |
|---|---|
| the rule, and the accepted scale | owner, 2026-08-15, quoted in the header |
| 1,407 / 400 / +718 migration figures | corpus scan, same date, backlog HL-C186 |
| Spanish 211 words vs a 300-word pre-A1 floor | `report --format text`, backlog HL-C183 |
| 16,000-word C2 target | `LEVEL_VOCABULARY`, `level-gate.ts:42` |
| 0.58 words per lesson, current density | 267 tokens / 463 lessons, backlog HL-C183 |
| R1 may resolve under this rule | reasoning, unverified — §3 requires re-measurement |
