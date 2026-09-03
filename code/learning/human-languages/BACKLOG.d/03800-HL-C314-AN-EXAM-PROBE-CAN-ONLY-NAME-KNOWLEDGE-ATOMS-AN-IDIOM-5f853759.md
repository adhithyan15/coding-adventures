## HL-C314 — an exam probe can only name knowledge atoms; an idiom or a culture claim reports as a typo

Found while writing `core/exam-inventory-french-a2.json`, the eleventh inventory
and the second above A1.

### The trap

A lesson declares new material in **three separate namespaces**:

    introduces:
      knowledge: [FR-PRAGMATICS-AIMER-BIEN-11]
    introduces_idioms: [FR-IDIOM-CA-MARCHE-AGREEMENT-01]
    introduces_senses: [FR-SENSE-MARCHER-WORK-01]
    introduces_culture_claims: [FR-CULTURE-OUI-OIL-OC-01]

`measureExamCoverage` resolves a probe against `trackIntroducedAtoms`, which is
`ramp.ts`'s `introducedAtoms` — **`introduces.knowledge` plus the block-level
`hl-knowledge` directives, and nothing else.** Idioms, senses and culture claims
are invisible to it.

So a probe naming a real, committed, correctly spelled idiom id behaves exactly
like a probe naming a typo: the atom "is not introduced", and the whole point
reports **uncovered**. It is fail-safe and silent — it under-reports the corpus
and sends an author to write something that already exists.

This is the same failure the French A1 file's own `probes only atoms that EXIST`
test was written to catch, and it caught this one on the first run: A2-F-11
probed `FR-IDIOM-CA-MARCHE-AGREEMENT-01`. Every inventory needs that test.

### What to do

- **Probe knowledge atoms only.** When the point's evidence lives in an idiom, a
  sense or a culture claim, probe the nearest knowledge atom and CITE the other
  unit in the point's `note`. Both French A2 register points do this, and the
  file's `probeSemantics` string says so outright so the next author does not
  rediscover it.
- Some real content is therefore **unprobeable by construction**. Chapter 18's
  `FR-CULTURE-OUI-OIL-OC-01` (the langue d'oil / langue d'oc split that named
  half of France) is committed, taught, and cannot close any point. That is a
  fact about the measurement, not about the corpus, and it argues for typing a
  claim as `knowledge` when a syllabus is likely to ask for it.

### The other finding from the same pass

Writing an inventory **audits the one next to it**. Enumerating A2's modal-verb
point made it obvious that French A1's `A1-V-11` — vouloir, pouvoir and devoir
in the singular — had been reading as a content gap while chapter 33 had given
each of the three its own lesson with *je / tu / il* printed since it was
written. French A1 moved 31/74 -> 32/74 with nothing authored.

**Write the level above before believing the level below's uncovered list.**
