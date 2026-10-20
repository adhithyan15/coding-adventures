## Unreleased — Pre-A1 ATTAINED: the reinforcement tail (chapters 335-336)

### Added

- **Two `review` lessons, chapters 335 and 336**, which close the last open
  pre-A1 criterion for Spanish. With them, `runLevelGate` reports

  ```
  levels ATTAINED (HL09 §3.1): 1 track at pre-A1 (spanish); 23 track(s) touch a level they have not attained
  ```

  That line has read `none` since the gate was written. Spanish is the first
  track in the 23-track corpus to attain any CEFR level under HL09 §3.1, and it
  now stands at pre-A1 with A1 in progress.

  - **Chapter 335, *Review --- Six Words That Arrived Last*** —
    `ES-C335-repaso-seis-palabras`, sequence 5870. English in, Spanish out, with
    the article.
  - **Chapter 336, *Review --- The Same Six, In Use*** —
    `ES-C336-repaso-seis-en-uso`, sequence 5880. The same six inside the
    shortest real turn each belongs to: *una entrada, por favor*, *¿y el
    cambio?*, *tengo calor*, *¿qué edad tienes?*

  Neither lesson introduces an atom. Both practise exactly six, all owned by
  earlier lessons and all reachable through the prerequisite chain.

### Why these six, and why two lessons rather than one

The criterion was reported as **4 atoms at or below pre-A1 revisited fewer than
twice**, and the four were `ES-LEX-C309-KIN-19` (*la pareja*),
`ES-LEX-C309-KIN-20` (*el bebé*), `ES-LEX-C313-ASK-19` (*la entrada*) and
`ES-LEX-C313-ASK-20` (*el cambio*).

This is a **structural tail, not sloppiness**. `measureContinuity` only judges a
reinforcement window the track was long enough to contain, so the atoms of a
track's last chapter are never measured at all, and the last two atoms of any
run are the two the following lessons had no reason to pick up. Every
vocabulary tranche therefore ends in an invisible tail of about two atoms, and
the NEXT tranche is what makes the previous one's tail visible.

Two consequences shaped the fix.

**First**, no existing lesson could honestly close them. The words *pareja*,
*bebé*, *entrada* and *cambio* appear nowhere in the prose of any later lesson,
so there was no lesson already retrieving the material with only the record
missing. Wiring `practises.knowledge` onto a lesson that does not retrieve the
atom would make the criterion pass by making the record false, so all four were
closed by new review material instead.

**Second**, appending lessons moves the measurable horizon and exposes the tail
underneath. Two more lessons make position 749 — `ES-C334-edad`,
`ES-LEX-C334-BODY-35` — judgeable for the first time at R1, and it had zero
revisits. So the review lessons practise **six** atoms rather than four,
picking up `el calor` and `la edad` as well. After the change every one of the
six is revisited at least twice and none of them misses a window it is long
enough to be judged on.

Two lessons rather than one because two of the six had **zero** revisits, and
one lesson can only supply one.

### Changed

- **`renderCurriculumGapReport` names the attaining tracks.** The
  `levels ATTAINED` line printed `${count} tracks at ${level}`, and because the
  count had been zero for the whole life of the gate, the populated branch had
  never once been read. It showed: `1 tracks at pre-A1` is a broken plural and
  gives no way to tell which track. The line now reads
  `1 track at pre-A1 (spanish)`. `attainedByLevel[l]` and the number of tracks
  whose `attained` is `l` are the same figure by construction, so naming them
  adds information without weakening the count.

- **Four assertions in `level-gate.test.ts` that dereferenced a null
  `attained`.** Each is rewritten rather than deleted, and each keeps the claim
  the test is named for:

  - *separates TOUCHES from ATTAINED* — pinned `attained === null`, which was
    only ever a proxy for "the two numbers disagree". Now pins the
    disagreement, plus the floor actually reached.
  - *reports every track as overstating* — pinned `tracksWithAnyLevel === 0`.
    Overstating means touching HIGHER than attained, not having attained
    nothing, so the 23 stands and the zero becomes a 1, checked against the
    named track list so the summary cannot drift from `tracks`.
  - *names which criterion failed* — pinned the ABSENCE of a `vocabulary`
    blocker, which was true of pre-A1 and is false of A1, the rung Spanish now
    works on. Replaced with the closure itself (`attained >= pre-A1`) plus an
    explicit anti-vacuity guard, so the per-blocker loop below it can never pass
    over an empty array.
  - *fails an authored-but-unrealized level on a COUNT* — pinned that NO track
    had attained anything, a stronger claim than the sentence above it. Scoped
    to B1-and-above, which is the rung that test is actually about.

- **The etymology-waiver counterfactual compares whole verdicts, not
  shortfalls.** A blocker's shortfall is scoped to the level a track is in
  progress at, so two runs that stop at different levels report shortfalls for
  different rungs and `unwaived >= waived` stops meaning anything. That is
  Spanish today: with the waiver it stands at pre-A1 and carries 88
  under-reinforced **A1** atoms; with the etymons renamed it never leaves pre-A1
  and carries 27 there. The invariant is now stated at the level it holds at —
  the waiver can only ever let a track stand at the same rung or a higher one —
  and a track that changes rung under the rename counts as the strongest
  possible bite rather than a failure.
