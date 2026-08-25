## Unreleased — A1: the two writing stages, and the six over-budget lessons split

Two of the six A1 gate criteria for Spanish close here. After this change,
`plan-cli --ceiling C2` lists neither `writing-stage` nor `atom-budget` for
spanish, and the pre-A1 attainment is unchanged:

```
levels ATTAINED (HL09 §3.1): 1 track at pre-A1 (spanish); 23 track(s) touch a level they have not attained
```

### Added — the A1 writing runway (chapters 337-339)

HL19's ladder is cumulative, and pre-A1 proved its first four rungs in the
`ES-W00-hola-*` runway: observe-trace, guided-copy, delayed-copy,
dictation-transcription. A1 adds two more, and both are proved here.

- **Chapter 337, *Writing --- Your Own Line*** — `ES-W01-frase-propia`,
  sequence 5890, `SPINE-TIME-OF-DAY`. Stage **controlled-composition**. The
  learner is handed five known words and a situation (four in the afternoon,
  meeting a stranger) and writes a greeting and a name. Every earlier writing
  step supplied a model to reach for — a shape to trace, a word to copy, a
  sound to transcribe. This one supplies none, and the choosing is the whole
  exercise. It introduces the single atom `ES-WRITING-CONTROLLED-COMPOSITION`,
  which names that difference.
- **Chapter 338, *Writing --- Four Lines About You*** —
  `ES-W01-ficha-personal`, sequence 5900. Stage **controlled-composition**
  again, on the shape the DELE A1 paper actually opens with: a
  personal-information form. Four frames, one blank each, then the courtesy
  word.
- **Chapter 339, *Writing --- With A Clock*** — `ES-W01-con-reloj`, sequence
  5910. Stage **timed-assessment-production**. The same four lines under the
  exam's own published conditions.

**Stage order is checked, not assumed.** `measureWritingStages` walks a track's
evidence in sequence order and refuses credit for a stage whose prerequisites
have not already been evidenced — controlled-composition therefore has to land
before timed-assessment-production, which is why 337 and 338 come first.

**The level comes from the spine node, not the chapter number.** All three hang
on `SPINE-TIME-OF-DAY`, which `core/spine.d/0080-SPINE-TIME-OF-DAY.json` stages
at A1. Proving these stages on a pre-A1 node would have been evidence at the
wrong rung and would not have moved the blocker.

**No invented exam numbers.** The timing (25 minutes, two tasks), the word count
(15-25 for the personal-information task), the forbidden aids (dictionary,
phone) and the two scoring criteria all come from
`spanish/task-shapes/a1.json`, which is already sourced to the Instituto
Cervantes guide, specification and model paper. Chapter 339 gives its single
task three minutes — a share of the paper's clock, stated as a share rather
than presented as the exam's own figure for one task, which Cervantes does not
publish.

**Every word composed was already taught.** The supply lists are closed sets
drawn from `hola`, `buenos días`, `buenas tardes`, `buenas noches`, `me llamo`,
`tengo … años`, `soy de …`, `vivo en …` and `gracias`. Nothing on the learner's
page is new Spanish; the step is that they decided where each old word went.

### Changed — six over-budget lessons split (never trimmed)

`chapter-policy.json` sets `maxNewAtomsPerLesson: 3`. Six Spanish lessons at or
below A1 introduced **four** atoms each. Every one of them split along a seam
that was already in the file: each had two atom-introducing body blocks, and the
split simply gave the second block its own lesson. **No atom was deleted.** The
totals below are unchanged before and after; only their distribution moved.

| was | atoms | becomes | atoms |
|---|---|---|---|
| `ES-C42-te` | 4 | `ES-C42-te` + `ES-C42-te-thee` (seq 1222) | 3 + 1 |
| `ES-C45-nos` | 4 | `ES-C45-nos` + `ES-C45-nosotros` (seq 1227) | 2 + 2 |
| `ES-C60-mal` | 4 | `ES-C60-mal` + `ES-C60-apocope` (seq 2377) | 3 + 1 |
| `ES-C65-ahora-hoy` | 4 | `ES-C65-ahora-hoy` + `ES-C65-hoy` (seq 2497) | 2 + 2 |
| `ES-C65-vi-di` | 4 | `ES-C65-vi-di` + `ES-C65-di` (seq 2512) | 2 + 2 |
| `ES-C67-primero` | 4 | `ES-C67-primero` + `ES-C67-primer-libro` (seq 2547) | 2 + 2 |

Each new lesson stays in its sibling's chapter, so no chapter was renumbered and
no chapter's introduced-atom set changed — the HL05 payoff and representativeness
findings are exactly what they were.

Each split earns its own keep rather than being half a lesson:

- **`ES-C42-te-thee`** takes the *tú*/*te* — *thou*/*thee* etymology, one
  ancient pair inherited down two different lines. `ES-C42-te` keeps the
  grammar and the sentence people actually need.
- **`ES-C45-nosotros`** takes *nosotros* = *nos* + *otros* and the *-mos*
  ending, which is the same ancient word worn onto the back of the verb. That
  is a different discovery from "*nos* means us", and it was crowding a lesson
  whose job was the object pronoun.
- **`ES-C60-apocope`** takes the generalisation across *mucho*/*muy* and
  *malo*/*mal*. A rule stated over two examples is not a fact about *mal*; it
  is its own idea, and it is what chapter 262 later leans on.
- **`ES-C65-hoy`** takes *hoy* and *hodie*. One word per lesson, which is what
  the rest of this stretch already does.
- **`ES-C65-di`** takes *dar* — an *-ar* verb that changes families for the
  preterite alone, and a verb the learner had never met.
- **`ES-C67-primer-libro`** takes where an ordinal stands and what happens to
  *primero* and *tercero* when they get there — the third sighting of the
  apocope habit, now resting on the lesson that named it.

**One sequence shift was needed.** `ES-C45-nos` sat at 1226 with 1225 and 1227
both occupied, so sequences 1227-1240 were shifted by +1 (to 1228-1241, a range
that was free) to open the slot immediately after it. Relative reading order is
unchanged, which is what every position-based measurement in `continuity.ts`
actually reads.

**Prerequisite chains were re-threaded, not bypassed.** Each split lesson takes
its sibling as a prerequisite, and the lesson that used to follow the sibling
now takes the split lesson instead. Every atom that moved is still reachable
through the declared chain from every lesson that requires it.

### Measured effect

- `atom-budget` at A1: **6 lessons over budget → 0**.
- `writing-stage` at A1: **2 stages unproved → 0**.
- `reinforcement` at A1: **88 → 81**. Not a target of this change; the split
  lessons practise their siblings' atoms and the three writing lessons retrieve
  a wide spread of early vocabulary, so seven atoms picked up the second revisit
  they were short of.
- `vocabulary` at A1: 378 → 379 headwords. Unchanged in substance — *hoy* now
  heads its own lesson.
- pre-A1 attainment: **unchanged**.
