## French gets an A2 exam inventory, and it audits the A1 one on the way in

`core/exam-inventory-french-a2.json` enumerates **104 DELF A2 points** across
thirteen categories, restated in our own words from the CECRL A2 descriptors and
the structure the DELF A2 syllabus and the Alliance française référentiels
publish. **16 are covered** by the corpus as it stands. It is the eleventh
inventory in the repo and only the second above A1, after German's.

### Built to be comparable with the A1 file, on purpose

Same categories in the same order, plus two: **Actes de parole** at the front,
because A2 is where the exam stops asking for words and starts asking for a
paragraph that does something, and **Lexique** (renamed from *Lexique de base*,
because at A2 it is not basic). A test pins that correspondence, so a point can
be seen to move from the A1 column to the A2 one rather than the two files
drifting into different shapes.

### The gap is function-shaped and past-tense-shaped

Fifteen of the sixteen **Actes de parole** are uncovered. So is every point in
*Le nom*, *Les déterminants*, *L'adjectif*, *Les prépositions*, *L'adverbe*,
*La phrase* and *La négation*. The **passé composé** and the **imparfait**
between them own six A2 verb points and none of them can be probed, because the
two French chapters that carry those tenses are **still hand-written** — the
same finding chapter 1 produced, arriving again one level up.

Where the corpus is strong is exactly where the retirement work has already
been: sixteen everyday verbs typed across chapters 24–27, the three question
forms and the register that chooses between them, five different kinds of
irregular present stem, and the tu/vous system.

### A lexical point here means a domain, not a word list

Goethe publishes a finite ~1,300-item A2 word list, which is what lets the German
A2 file close its lexicon against a source. There is no published French
equivalent. So the fifteen lexical points are enumerated at the level of the
**domain**, and every covered one carries a note naming the exact set the corpus
holds and what is missing from it — *"eight food and drink words across chapters
28 and 30; ordering, quantities and the shop itself are not taught."* That note
is the authoring brief; a bare `covered: true` would have hidden it.

### Two findings, one of which corrects the A1 file

**A probe can only name knowledge atoms.** A lesson declares new material in
three namespaces — `introduces.knowledge`, `introduces_idioms`,
`introduces_senses`, `introduces_culture_claims` — and `measureExamCoverage`
reads only the first. A probe naming a real, committed, correctly spelled *idiom*
id behaves exactly like a probe naming a typo: silently uncovered. `A2-F-11`
probed `FR-IDIOM-CA-MARCHE-AGREEMENT-01` and the new file's own *probes only
atoms that EXIST* test caught it on the first run. Recorded as HL-C314.

**Writing A2 audited A1.** Enumerating the modal-verb point made it obvious that
`A1-V-11` — *vouloir*, *pouvoir* and *devoir* in the singular — had been reading
as a content gap while chapter 33 had given each of the three its own lesson with
*je / tu / il* printed since it was written. Nothing was authored; French A1
moves **31/74 → 32/74**. An uncovered list that names work already done is worse
than no list, so: write the level above before believing the level below.

### Measured

- French exam inventories: **1 -> 2**. Corpus written inventories: **10 -> 11**.
- French A2 coverage: **16/104**, of which *Lexique* is 7 and *Actes de parole* 1.
- French A1 coverage: **31/74 -> 32/74**, with nothing authored.
- Every probed atom resolves against a real lesson; the whole package suite and
  every `check:*` gate are green.

