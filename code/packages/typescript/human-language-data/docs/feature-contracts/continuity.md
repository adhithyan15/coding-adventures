# Continuity — does the course have a memory of itself? (HL09)

The ramp budgets measure how big each **step** is. This measures whether the steps
hold together.

```ts
import { measureContinuity, loadEverything } from "@coding-adventures/human-language-data";

const { lessons } = loadEverything();
const c = measureContinuity(lessons);

c.summary.lessonsWithoutSequence;   // 515 — no declared reading order at all
c.summary.atomsNeverRevisited;      // 745 of 1599 (47%) taught once, never again
c.summary.missedByWindow;           // { R1: 778, R2: 1159, R3: 711, R4: 137 }
c.summary.forwardReferences;        // 504 uses of material a later lesson teaches
```

Order comes first: 515 lessons carry no `sequence`, so their order exists only in
hand-typed LaTeX (French: **64 of 73**). A ramp whose order is unknown cannot be
verified, so every other number is provisional until that reaches zero.

Spanish is the worked example. Declaring the real order for 50 of its lessons cut
its forward prerequisites from **31 to 5** — 26 of the 31 were never real, just
artifacts of an alphabetical fallback sorting `beber` before `comer`.

Reinforcement reads `practises.knowledge`, **never `reviews_of`** — which 144 of
Spanish's 146 lessons set and which cannot close a window, because it names lesson
ids while atoms live in another namespace.

Forward references carry their own evidence: a word is reported only when a *later
lesson's headword* teaches it. That reproduces what a human reviewer found by
reading — `ES-C07-beber` says *"Como pan y bebo agua"* while `pan` and `agua` are
chapter 26.
