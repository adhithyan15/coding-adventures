## HL-C302 — the exam inventory's null probes ARE the vocabulary work queue, and a tranche that leaves them null under-reports itself

The Hindi pre-A1 vocabulary round-four tranche started from `npm run plan`,
which named `vocabulary/hindi/pre-A1` — *teaches 155 distinct headwords at or
below pre-A1, against 300* — and nothing else about which words to pick. The
plan cannot say which words, because the vocabulary item is a COUNT.

The words were chosen from a different artifact entirely: the 127 uncovered
points in `core/exam-inventory-hindi-a1.json`, every one of which carries a
`note` saying, in prose, exactly which word is missing and which mock item needs
it. *"`shahar` is a required field in BOTH mocks' writing papers."* *"Mock 2's
listening profile has Imran playing cricket."* *"`aaj` is untaught while `kal`
is."* Those notes are an authoring brief, and they were written before anyone
had to use them as one.

**Choosing by exam point rather than by topic changed the answer.** A round of
five-word topic chapters chosen for coherence closed roughly two points per
chapter. Choosing for exam demand closed 18 points across 7 chapters as first
measured, and one chapter — `aaj`, `bhi`, `bahut`, `kab`, `kyon` — closed five
points with five words, because the inventory had enumerated each of them as its
own demand. A health-vocabulary chapter was designed, measured at ONE point, and
cut for that chapter instead. That trade is only visible if the inventory is
consulted while choosing, not after authoring.

WHAT SHIPPED, AND THE HAZARD THAT CHANGED IT. The tranche shipped as **33 words
in chapters 75-81**, closing **16** points, 176/282 -> 192/282. It was authored
against chapters 68-74; #14113 merged into those seven numbers first, and both
tranches had been chosen from the SAME uncovered list, so both taught `kab` and
`kyon` and both closed HI-A1-Q-07 and HI-A1-Q-08. That is the sharp edge of this
entry's own advice: **an uncovered-point list is a shared queue, and two authors
reading it in parallel will pick the same words.** Two of the five words in the
`aaj`/`bhi`/`bahut`/`kab`/`kyon` chapter were cut for exactly that reason. Claim
the points you intend to close before authoring, or expect to cut.

### The trap: the probe does not fill itself

`measureExamCoverage` recomputes coverage on every run, which is the whole
argument for a probe over an annotation. But a point whose `probe` is `null`
stays uncovered FOREVER, however much the corpus learns, because null means "no
atom in the corpus corresponds to this" and nothing re-derives that claim.

So the last step of a vocabulary tranche is editing the inventory: replacing
`probe: null` with the atoms the new lessons introduce, and rewriting the `note`
to say what is still missing. Skip it and the tranche teaches 35 words and
reports **zero** movement — 155/282 before, 155/282 after — which reads exactly
like a tranche that taught nothing. That was measured on this branch: coverage
sat at 155/282 with all 35 lessons authored, green, and merged into the book,
and moved only when the probes were filled.

The note rewrite is not optional politeness. `HI-A1-LEX-56` claimed "`desh` is
not taught either"; leaving that sentence in place beside a filled probe is a
file that contradicts itself, and the next author reads the note, not the array.

### The other trap: appending five-word chapters costs reinforcement

Measured on the first draft of this tranche: 35 lessons drove Hindi's pre-A1
thin-atom count 54 -> 68. The +14 was not diffuse. Inside a five-lesson chapter
each word is retrieved by the lessons that follow it, so words 1, 2 and 3
accumulate revisits and words **4 and 5 have nowhere left to go** -- one later
practise each, against the two the gate asks for. Twelve of the fourteen were
exactly the fourth and fifth atom of the first six chapters.

The fix is not a pin reseat and not more drills inside the chapter, because the
shortfall is at the boundary. Every chapter-OPENING lesson declares the two
preceding words -- reaching across into the previous chapter -- and retrieves
both by name in its Warm-up and Guided Practice; the second lesson of each
chapter picks up the one still short. That is four frontmatter lines per lesson
-- `prerequisites`, `requires.knowledge`, `practises.knowledge` and
`reviews_of` -- plus one retrieval bullet, and a reworded Warm-up in each
chapter opener. Twelve lessons, opening the tranche's second through seventh
chapters: 68 -> 56.

The last three came from the same rule applied one step further back, in the
two lessons that open the tranche's first chapter. They reach past the A1
name-field runway to `ghas` and `kuan`, the final two words of the PREVIOUS
vocabulary run, and pick up the runway's own payoff atom on the way. All three
had been sitting at the end of the corpus with their windows unjudged and became
judgeable the moment the tranche was appended after them. 56 -> 53, below where
the tranche started.

Two general points fall out. **Appending makes previously-unjudged tail atoms
judgeable**, so a tranche inherits the previous tranche's tail whether it wants
to or not -- budget for it. And **the boundary between chapters is where
reinforcement leaks**, so a tranche of N five-word chapters should expect to
spend its retrieval budget at the openings, not spread evenly.

### Two smaller things worth carrying forward

- **The taught-glyph filter is a hard constraint on word choice, and it is
  cheap to check.** Hindi teaches 47 Devanagari glyphs and shows 58. Every
  candidate headword was spelled from the 47, and so was every example
  sentence — which ruled out `डॉक्टर` (needs the candra-o sign), `थोड़ा`, and
  `पढ़ना`, and forced `बहुत बड़ा` in place of `बहुत अच्छा`. Four such slips got
  through first drafting and were caught by re-running the glyph walk over the
  new files only. Do that walk before the prose is polished, not after.
- **Bringing a canonical verb below its spine node is a two-line ledger edit,
  not a fight.** `VERB-GO`, `VERB-PLAY`, `VERB-BUY` and `VERB-DRINK` are owned
  by A1 and A2 nodes. Teaching them at pre-A1 means deleting the concept from
  the owning node's `omits` and adding it to its `relocates`. The validator
  computes both from the lessons, so it tells you the exact expected value.
