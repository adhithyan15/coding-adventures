# adj-facts-stdlib — the graded, byte-provenanced *facts* standard library (K → med)

A curriculum of **recallable facts**, climbing from **what every kindergartener
learns → medical school**, as importable, grounded ADJ libraries. The sibling of
[`adj-formula-stdlib`](../adj-formula-stdlib/) (graded *formulas*) and the
[medical recall domains](../mycin-2026/recall/) (single-hop clinical recall): where
those hold computation and clinical edges, this holds the **elementary facts a
learner memorizes first** — shapes, colors, counting, the calendar, the planets —
and grows upward toward the science facts a clinician recalls.

## Why facts are first-class

The ADJ language natively supports facts (`relate`, `dictionary`) and now native
tabular data ([`table`](../../ADJ-TABLES.md)). A fact library is *imported* and
*recalled* — the model performs no lookup from memory; the engine resolves a
binding query against the grounded rows and returns the answer **with its
citation**, on the CPU, with zero answer-time model calls. Every shipped fact is
byte-provenanced from a citable source (see
[feedback: nothing human-authored]) — nothing is asserted "from memory."

## Layout (by grade level)

| level | what | examples |
|-------|------|----------|
| `kindergarten/` | the first facts a child learns | shapes → sides (this PR) |
| … | grade-school, middle, high-school, undergrad … | *(grown one small library per rotation)* |
| → medical school | the science facts a clinician recalls | (already: `mycin-2026/recall/`, 63 domains) |

## Consuming a fact library

```adj
import "kindergarten/shapes.adj"
? polygon_sides(hexagon, $Sides)     % 6, cited to the source
```

Because a `table` row lowers to a relation whose value is a number, a recalled
fact **composes into a formula**: e.g. a looked-up side count feeds a
perimeter/area formula from `adj-formula-stdlib` — the concrete K-math bridge from
*recall* to *compute*.
