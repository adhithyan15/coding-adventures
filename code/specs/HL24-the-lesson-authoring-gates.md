# HL24 — The lesson authoring gates

**Status:** reference, derived from the code rather than authored ahead of it.
**Audience:** anyone writing lesson markdown under `code/learning/human-languages/<track>/lessons/`.

---

## 0. Why this document exists

Every gate below is enforced. None of them was written down in one place, so each authoring
tranche has rediscovered them one CI failure at a time — and the expensive ones are not the
gates that fail loudly but the **exact-equality pins with no headroom**, where a single new
sentence turns a green suite red in a file the author never opened.

This is an inventory, not a policy. It states what the code checks **today** and where. Where a
threshold is a ceiling that may fall but never rise, it says so, because that distinction is the
difference between "you have budget" and "you have none".

Nothing here replaces reading `lessons.md`. This is the mechanical half; `lessons.md` is the half
that explains why the mechanical half exists.

---

## 1. Prose content gates

### 1.1 Banned words — a CEILING, and it is at zero headroom for new prose

`tests/banned-words.test.ts`. HL10 §7.4 bans four terms from **learner-facing prose**:

| term | matched as |
|---|---|
| `simply` | `/\bsimply\b/gi` |
| `just` | `/\bjust\b/gi` |
| `obviously` | `/\bobviously\b/gi` |
| `as you know` | `/\bas you know\b/gi` |

They are the register of a course that keeps telling the reader a thing is easy, which is the one
thing a gentle ramp must never do — if it were easy the lesson would not exist.

**`just` fires on every sense**, including *"the old part is just carrying something in"* and
*"just as in English"*. There is no adversative or temporal exemption. Rephrase.

Scope is block markdown only: frontmatter, `hl-*` directives and fenced code are excluded, so
`just` inside an etymology gloss or an activity payload does not count.

The pin is a **ceiling measured from the corpus**, not zero — a check that fails the whole corpus
on the day it lands gets deleted. It may ratchet down and **must never be raised to accommodate
new prose**.

### 1.2 Rule statements — a ceiling at 30, corpus-wide

`src/info-dump.ts`, pinned in `tests/info-dump.test.ts`. Four patterns, applied per line:

```js
{ name: "is-used-for",       pattern: /\b(?:is|are) used (?:for|to|when|with|in)\b/i },
{ name: "always-never",      pattern: /\b(?:always|never) (?:takes|uses|has|is|comes|goes|means|ends|begins|appears)\b/i },
{ name: "there-are-n-kinds", pattern: /\bthere are (?:\d+|two|three|…|twelve) (?:kinds|types|ways|forms|categories|classes|groups)\b/i },
{ name: "the-rule-is",       pattern: /\bthe rule (?:is|for|here)\b/i },
```

One finding per line, not one per pattern.

**Phrasings that trip it:** *"the subjunctive is used for doubt"*, *"`ser` is used with
professions"*, *"this ending always takes an accent"*, *"`gustar` never means to like"*, *"there
are three ways to say this"*, *"the rule is that stress falls on the penultimate"*.

**Phrasings that do not:** show the instance instead. *"`intrāre` surfaced as `entrar`"* states a
fact about one word; *"Latin short i always becomes Spanish e"* states a rule. Both teach the same
thing and only one is counted, which is the point — the ceiling exists to push authors toward
instances.

`chapter-policy.json` also declares `maxRuleStatementsPerLesson: 1`.

### 1.3 Paradigm tables

Also `src/info-dump.ts`. A run of **3+ consecutive table rows** where **3+ are person rows** is
recorded as `partial-paradigm-table`, and at `FULL_GRID_ROWS` as `full-paradigm-grid`. Both count
against the same findings ledger.

Practical consequence: **a vocabulary lesson should contain no markdown tables at all.** Bulleted
contrast lists carry the same content and are invisible to this detector.

### 1.4 "The course" and cross-volume claims

`tests/standalone-book.test.ts` — *"a reader holding the PDF is never told it already learned
something that lives in another volume"*. Offending strings include **`the course`**. A reader
holding one volume cannot check a claim about "the course", so the prose must not make one.

Say *"you have met it already"*, not *"the course taught you this"*.

### 1.5 Chapter cross-references

`tests/chapter-references.test.ts`. Two assertions: such references **never appear in Spanish**,
the track whose chapters actually move, and **do not grow** in the tracks that still carry them.

So `back in chapter 380` is a hard failure in a Spanish lesson. Link to the lesson instead — the
link survives renumbering and the prose sentence does not.

### 1.6 Block titles

`classifyBlock` accepts a fixed vocabulary of `##` headings; an ad-hoc title is rejected. The seven
used by every recent Spanish vocabulary lesson are:

```
Warm-up · You'll want to know first · Sounds you'll need · The word, taken apart
What you've built · Guided Practice · Wrap-up Recall
```

**Apostrophes in headings must be ASCII `'` (U+0027).** A curly `’` is a different string and
does not match.

### 1.7 Voice cues

Narration recognises `[PAUSE Ns]` and `[PAUSE Ns each]`. Anything else — `[PAUSE]`, `[pause 2s]`,
`[PAUSE 2 s]` — is not a cue. The rule for prose generally is **move the prose, not the
detector**: if narration cannot speak a construct, rewrite the sentence.

---

## 2. Frontmatter gates

| field | rule | where |
|---|---|---|
| `concept_tag` | canonical taxonomy id, or namespaced `/^[A-Z]{2}-[A-Z0-9-]+$/` | `validate.ts`, `constants.ts` |
| `etymology_hook` | > 120 chars is a **warning**, not an error; recent tranches run 600-800 | `validate.ts` |
| `introduces.knowledge` | ≤ 3 atoms (`maxNewAtomsPerLesson`) | `chapter-policy.json` |
| `sounds` | **nothing validates this today** — see §5 | — |
| `spine_node` | must match the segment that contains the lesson | `curriculum.ts` |
| `prerequisites` | must appear earlier in realization order | `curriculum-prerequisite-order` |
| `practises.knowledge` | **must declare every atom any block asserts it assesses** | `validateCurriculum` |

That last one is easy to break by editing: changing a block's `assesses=[…]` without updating
`practises.knowledge` fails with `block 'X' assesses 'Y' without declaring it in
practises.knowledge`.

---

## 3. Structural and duplicate gates

- **Duplicate headwords.** No gate enforces this directly; the level gate counts *distinct*
  lower-cased headwords, so a duplicate is silently absorbed and shows up only as a count that
  is one lower than the author expected. **Keep a corpus-wide duplicate-headword check in your
  own pre-commit sweep** — see `lessons.md`, "Anything you author is a candidate".
- **Shard ordinals are positional.** Element *i* is named `(i+1)*10`, zero-padded to four. Inserting
  mid-list renumbers everything after it; `--shard` performs the rename and removes the old files.
  Never hand-pick an ordinal.
- **Monolith tracks.** `marwadi` has no `curriculum.d/`. `check:shards` never reads it, so a
  ledger edit that is wrong there passes that gate and fails `curriculum-shards` instead.
- **Every node needs a realization entry in every track** (`spine map omits '<id>'`), so a new
  spine node costs 23 ledger entries whether or not anyone realizes it.

---

## 4. Exact-equality pins that a content change will move

These are the expensive ones. Each is `toEqual`/`toBe` against a committed number, so any content
change that moves them fails a test in a file you did not edit.

| pin | file |
|---|---|
| level histogram + lesson count per track | `tests/levels.test.ts`, `core/level-snapshots/` |
| `summary.totalNodes`, nodes per strand | `tests/strands.test.ts` |
| exam coverage `covered` / `percent` / `unmapped` | `tests/exam-inventory.test.ts` |
| the exact set of `probe: null` point ids | `tests/exam-inventory.test.ts` |
| `spanish.extras.length` (namespaced verb count) | `tests/verbs.test.ts` |
| uncovered-point total in the plan report | `tests/plan-cli.test.ts` |
| rule statements, lessons with findings | `tests/info-dump.test.ts` |
| banned-word totals | `tests/banned-words.test.ts` |

**Update the pin and say why in the comment beside it.** Every one of these files already carries
prose explaining its previous movements; a bare number change is not in keeping and hides the
reason from the next reader.

---

## 5. Gaps — checks that do not exist

Stated so nobody assumes coverage that is not there.

- **`sounds:` tags are read by nothing.** The vocabulary drifts freely; a typo or an invented tag
  ships silently. Match neighbouring lessons by convention. (PR #13215 proposes rejecting unknown
  tags.)
- **`maxNewIdiomsPerLesson`, `maxNewSensesPerLesson`, `maxNewCultureClaimsPerLesson`** are declared
  in `chapter-policy.json` and **no code reads them**. They constrain nothing today.
- **Effective duration** — `duration.max_seconds` is authored, not computed from content. Nothing
  cross-checks the declaration against the prose. The 300-second ceiling is an authoring
  discipline: **if a lesson runs over, cut it; never re-declare it.**

---

## 6. Content safety — the gap with the worst consequences and no gate at all

Every check in §1-§4 is mechanical. **None of them can tell you that a drill you are about to
write will make a beginner say something obscene aloud in a classroom.** That failure is not
theoretical, it is not caught anywhere, and it is worth more attention than any pin in §4.

### 6.1 The rule

**Screen what the DRILL PRODUCES, not what the headword means.** A headword can be entirely
innocent while the forms a lesson generates around it are not. The generative moves in this
curriculum's lesson shape are the dangerous ones:

- inflecting for gender (`-o` → `-a`) to show agreement;
- pluralising to show number;
- pairing with a common verb to make a phrase;
- drilling a minimal pair by reading its neighbours aloud in sequence.

Each of those is a normal teaching move. Each can manufacture a form the author never typed.

### 6.2 The worked example: `pollo` / `polvo`

`HL23` §12.3 mandates that `pollo`/`polvo` be taught **as a minimal pair**, because both are
exam-required and the confusability screen disambiguates rather than drops. The obvious way to
teach it — walk the learner through `pollo`, `polla`, `polvo`, `polvos` — is **hazardous**:

- **`polla`** is vulgar slang for *penis* in peninsular Spanish;
- **`echar un polvo`** means *to have sex*.

The DLE marks all of it explicitly, and the exact wording is worth keeping so nobody has to take
this on trust:

| entry | sense | DLE |
|---|---|---|
| `polla` | 2 | *f.* **malson.** pene |
| `polvo` | 5 | *m.* coloq. coito. **Echar un polvo** |
| `polvo` | 6 | heroin (*jerg.*) |
| `pollo` | 8 | escupitajo |

Note the last row: **`pollo` itself carries a coarse sense** ("gob of spit"), so the headword is
not innocent either — only unmarked in the sense the lesson teaches.

This is not a modern coincidence that a dictionary might miss. Corominas derives the obscene
sense from `polla` "girl" and refers to it in as many words as *"la ac. obscena"* — read verbatim
in the `POLLO` and `POLVO` entries, volume Mi–Ri (see §7).

**The constraint, for whoever authors chapter 403:**

- teach the contrast as **`pollo` vs `polvo` only**;
- **do not inflect** either into `polla` / `pollas`;
- **do not put `echar` anywhere near `polvo`**, in prose, in a `[YOU SAY:]` cue, or in an example;
- **do not build a gender-contrast drill on this pair.** If the chapter needs other forms of the
  bird, **`gallina` and `pollitos` are the safe substitutes**; if it needs a feminine counterpart
  to show the *o/a* pattern, use a different word entirely;
- **leave a comment in the lesson source saying why the forms are missing**, so a later editor
  does not helpfully complete the paradigm.

That last point is the reusable half. A safety omission that is not explained looks exactly like
an oversight, and the next person will fix it.

### 6.3 Why no gate will catch this

A banned-word list cannot work here: `pollo`, `polvo`, `polla` and `echar` are all ordinary words,
and `polla` also means *young hen* and, regionally, *bet*. The hazard is **compositional** — it
lives in the combination and the register, not in any token. Detecting it mechanically means
modelling what a learner will produce and how it lands in each variety, which is not a lint.

So this stays an **authoring discipline with a written record**, which is why it is in a spec
rather than in a test. The practical instruction: when a lesson teaches a minimal pair or an
inflection pattern, **write out the forms the drill will actually generate and look at that list**
before writing the prose. It takes a minute and it is the only check there is.

### 6.4 The second case, found only because the first one prompted a sweep: `verdura`

`verdura` carries a DLE sense 4, **"f. obscenidad"**, and `verde` carries sense 12, *"Dicho de un
cuento, de una comedia, de un chiste: Indecentes, eróticos."*

This is a **milder** hazard than §6.2 and the difference is worth stating, because it changes what
the author has to do. `polla` is *accidentally producible* — a normal inflection drill emits it.
`verdura`'s obscene sense is not something a beginner will stumble into by inflecting. The
requirement is therefore narrower but real: **do not gloss `verde` as "just the colour."** A
lesson that says the word means only one thing has told the learner something false, and this is
the register they will meet it in.

### 6.5 The sweep, and what it did and did not establish

After §6.2, an explicit scan for the DLE register marks **`malson.`, `vulg.`, `obscen.` and
`despect.`** was run across the other twenty-eight tranche-B words. It returned **zero**.

- **`recto` is clean.** It was checked specifically because Spanish `recto` has an anatomical
  sense; that sense is clinical and unmarked, and *todo recto* is ordinary language.
- **Two are flagged unverified rather than clean**, and the distinction is the point: **`costo`**
  (checked on suspicion of a hashish sense — the DLE lists none, but the check could not confirm
  the absence) and the phrase **"hacer un tren"**. Neither is cleared; both are simply not
  established either way. **Do not treat an unconfirmed check as a pass.**

**One standing note, outside this tranche's thirty but adjacent to two of its chapters:**
**`concha` is DLE-marked `malson.` across Argentina, Bolivia, Chile, Guatemala, Paraguay, Peru and
Uruguay.** Any beach, seaside or shopping chapter reaching for a plausible nearby noun can walk
into it. It is recorded here rather than in a tranche note because the next author to write a
beach lesson in any track is the person who needs it.

The honest summary of the sweep: **one confirmed hazard means the check had never been run, not
that there was exactly one.** The sweep found a second case immediately.

---

## 7. Sources, and the attribution boundaries that go with them

Corominas's *Diccionario crítico etimológico castellano e hispánico* is readable in full text on
archive.org, which removes the constraint that shaped this programme's first two tranches: the DLE
could always be checked directly, while Corominas could only be reached by proxy — and `HL23` §12
and §13 both record cases where a Wiktionary citation of Corominas turned out to misreport him.

**Volumes read, and what was actually read in each:**

| volume | archive.org identifier | entries read verbatim |
|---|---|---|
| C–F | `corominas-diccionario-critico-etimologico-castellano-c-f` | `CONCERTAR`, `COSTAR` |
| G–Ma | `diccionario-critico-etimologico-castellano-g-ma-corominas-joan-pdf` | `GORRA` |
| Mi–Ri | `diccionario-critico-etimologico-castellano-mi-ri-corominas-joan-pdf` | `PLAYA`, `POLLO`, `POLVO`, `PAILA` |
| Rj–X | — | partial OCR (~222k chars), yielded nothing |
| A–CA | — | **never loaded** |

### 7.1 Words that may NOT carry a Corominas citation

This list matters more than the one above. A citation is a claim about where a fact came from, and
attaching Corominas's name to a fact reached some other way is the precise error this programme
keeps catching in other people's sources. **Do not repeat it in our own.**

- **`talla`** — the borrowing from Catalan, `tajar` as the native reflex, and the stature sense
  from French *taille* at *Academia* 1817 are all **subagent-sourced**. The T volume was never
  opened. State the facts without the attribution.
- **`tren`** — the 1705 Sobrino and *Autoridades* 1739 datings are **subagent-sourced**, same
  condition.
- **`camisia`** — the `CAMISA` headword was **not read**; volume A–CA never loaded. What *was*
  read is the **`CERVEZA`** entry, where Corominas lists `camisia` among *"otros celtismos."*
  Cite it as that and claim nothing further.
- **`helar` / `helado`** — the search returned empty. **No Corominas reading exists.** Attribute
  nothing to him.
- **`paella`** — it is not its own headword. The *patella* / *paella* discussion sits under
  **`PAILA`**, and that is where it must be cited.

### 7.2 Two calibrations that are about not over-swinging

- **`wifi`.** "Wireless Fidelity" is a backronym, and the honest verdict is **disputed, not
  false** — a lesson that flatly calls it a myth is as wrong as one repeating it as fact.
  **Put no duration in the prose.** "About a year" traces to one account through a subagent and
  was never independently verified. The wording to use: *the name came first as a Hi-Fi
  sound-alike; "Wireless Fidelity" was attached afterward as marketing by the Alliance, and later
  dropped.*
- **`gorro` / `gorra`.** Corominas gives *"de origen incierto"* and hedges the `gorre` guess with
  *quizá* / *acaso*. **Teach no etymology at all** — repeating a hedged guess as settled is exactly
  the error being avoided. But **`gorra` is primary and `gorro` is derived from it, and that is a
  real fact worth keeping.** Substitute the shape contrast — a `gorra` has a peak, a `gorro` is
  round and brimless, and they are **not** a gender pair of one object — plus the idioms *de
  gorra*, *pasar la gorra*, *gorrón*, and *estar hasta el gorro*.

---

## 8. Running the gates

```
code/scripts/verify-human-languages.sh --fast    # everything except the 22-book XeLaTeX compile
code/scripts/verify-human-languages.sh           # everything
```

Generate before you check — the drift gates compare committed artifacts to freshly generated ones:

```
npm run generate:books generate:narration generate:modality generate:progress
npm run generate:gentle-snapshots generate:level-snapshots generate:assessment-artifacts
```

`dist/` is gitignored, so `npm run build` first or you are checking a stale CLI. `npx tsc` is a
stub here; use `node node_modules/typescript/bin/tsc`.

`language-ladder` reads the spine and is **not** rebuilt by a curriculum-only PR's change
detection. Run its `BUILD` by hand when you touch `core/spine.json`.
