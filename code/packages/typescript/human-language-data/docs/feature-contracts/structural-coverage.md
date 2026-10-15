# Structural coverage, versus touching a level (HL09 §3.1)

```ts
import { runLevelGate } from "@coding-adventures/human-language-data";

const gate = runLevelGate({ lessons, levels, curricula, spine, ramp, continuity });
gate.tracks.find((t) => t.language === "spanish");
// { touches: "A2", attained: null, inProgressAt: "pre-A1", vocabulary: 113,
//   blockers: [{ criterion: "vocabulary", shortfall: 256,
//               detail: "teaches 44 distinct headwords at or below pre-A1, against 300" }, …] }
```

`touches` is the highest level any lesson **sits at** — one lesson pointing at one A2
node is enough to move it. `attained` is the highest level where every gate criterion
holds, here and below: spine nodes realized, cumulative vocabulary met, no lesson over
the atom budget, every atom revisited twice, and every required cumulative writing
stage proved. Every criterion is scoped **at or below
the level** — Spanish teaches 113 headwords in total but only **44** at or below
pre-A1, and it is the 44 the gate judges.

That gap between the two numbers is what let "Spanish reaches A2" stand on fourteen
present-tense lessons.

**`attained` is a property of the corpus, not of a reader.** Read the criteria again:
spine nodes realized, headwords taught, verb headwords taught, lessons over budget,
atoms revisited. Every one counts something about the *book*; not one of them requires
a person to answer a question. The field keeps its name for compatibility, but the
report prints **`levels with STRUCTURAL COVERAGE COMPLETE … performance UNVERIFIED`**
and says underneath that no track has had a single exam item scored against it. A
`mock-performance` criterion is sketched in HL09 §3.2 and deliberately not implemented,
pending the first scored mock.
