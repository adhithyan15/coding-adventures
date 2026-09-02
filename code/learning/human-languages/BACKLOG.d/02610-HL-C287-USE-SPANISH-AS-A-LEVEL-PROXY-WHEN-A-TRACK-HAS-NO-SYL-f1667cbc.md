## HL-C287 - Use Spanish as a level proxy when a track has no syllabus: the Hindi A1 inventory, and the method the other tracks should copy

Hindi is the first track outside Spanish, French and German to get an
`exam-inventory`, and the first Indic track to have its exam gap MEASURED
rather than proxied. Recording the method so the other eighteen tracks inherit
the search that FAILED as well as the one that worked.

### 1. No Hindi awarding body publishes a content syllabus. Checked, not assumed

`core/exam-levels.json` maps Hindi A1 to the **Dakshina Bharat Hindi Prachar
Sabha** "Prathmic" rung, `basis: editorial`. That is the strongest external
anchor any Indic track has, so it was the obvious place to look. It does not
contain a syllabus.

- `dbhpscentral.org/Syllabus.html` and `dbhpscentral.org/examination.html`
  publish the **names** of the examinations (Parichaya, Prathamic, Madhyama,
  Rashtrabhasha, Praveshika, Rashtrabhasha Visharad, Rashtrabhasha Praveen)
  and the prescribed readers. No grammar inventory, no word list, no can-do
  descriptors, **no A1/A2 column**.
- **Kendriya Hindi Sansthan** (Agra) and the **Central Hindi Training
  Institute** publish prospectuses and course names, not level-split content.
- There is **no Council of Europe Reference Level Description for Hindi**.

The Spanish pattern - restate a published, finite, level-split inventory in our
own words - is **not available** for Hindi, and will not be available for any
other Indic track, none of which even has a named external ladder. **Do not
spend another agent-hour looking for a South Asian PCIC.**

### 2. The wrong conclusion, and the right one

The first draft of this file concluded from section 1 that the point set had to
come from the **CEFR Companion Volume's A1 descriptors** plus the project's own
checked-in A1 mocks. That produced **172 points and a coverage of 67%**.

It was too generous, and the reason generalises: **descriptors are deliberately
abstract where a syllabus enumerates.** "Can ask and answer questions about
personal details" is one descriptor; the DELE inventory spends a dozen points
on what that actually requires. A target list built from descriptors is a
target list drawn short, and a coverage number measured against a short ruler
flatters the corpus. That is worse than no number.

**The right move is the owner's: when a track has no syllabus of its own, use a
language that does as a PROXY FOR LEVEL.** `core/exam-inventory-es-a1.json`
restates the Instituto Cervantes inventory behind DELE - a real awarding body,
published, finite, split into A1 and A2 columns - and holds **273 points**.
Walking those 273 against the Hindi draft raised it to **282 points and 55%**.

### 3. How to walk the proxy

Ask of each proxy point **what it demands of a LEARNER**, not what Spanish
grammar it names. Then find the Hindi thing that carries the same load.

- "Direct object: people take the preposition `a`, things do not" is not about
  `a`. It is *animate objects are marked*, and Hindi marks them with **`ko`**.
- "Reflexive `se`" is *the possessive changes when the owner is the subject*,
  which in Hindi is **`apna`** - a word BOTH A1 mocks use and neither the
  descriptors nor a mock-reading had surfaced.
- "The definite article" does not transfer as morphology, but the demand
  underneath does: *how is definiteness carried?* Hindi's answer is that it
  has no article, and **nothing in the corpus tells the learner that.**
- "Propose a toast" is not a Hindi move. The demand is *a set-phrase well-wish*,
  and the `shubh-` formulas answer it.

Some genuinely do not transfer - Spanish written accents, inverted question
marks, the dialogue dash. **Name those with a reason rather than dropping them
silently.** Nine of the 273 are recorded that way.

And **do not let the proxy shrink the track.** 22 Hindi points answer no
Spanish demand at all: the Devanagari script points, the postposition and
oblique-case system, the nuqta, the register split, the `karna` compound verbs,
the abstract courtesy nouns. A proxy is a scaffold, not a template to translate.

### 4. Make the walk PROVABLE, not asserted

Two checks in the generator, both fatal:

1. **Every probe atom exists** in the set the track actually introduces. A
   probe naming a nonexistent atom reports "not covered" forever and is
   indistinguishable from real debt. 357 atoms across 286 lessons were the
   authority.
2. **Every one of the 273 proxy points is either cited by some point's
   `derivedFrom` or listed in `NON_TRANSFERRING` with a reason.** This one
   caught two real omissions on its first run (`bastante`, and the quantity
   adverbs) that a by-eye pass had missed. Without it "I walked all 273" is a
   claim; with it, it is a build failure when untrue.

Each point carries **`derivedFrom`** - the proxy point ids it answers, or
`hindi-specific`. It is a data-only field; no code reads it, and adding it
needed no change to `exam-inventory.ts`, which matters when a dozen track
agents are editing in parallel.

### 5. What the proxy found that nothing else had

Whole demands the descriptor-derived draft never enumerated, every one of them
tested by the checked-in mocks: **`aaj`** (untaught while `kal` is taught),
**`bhi`**, **`bahut`**, **`aur`/`ya`/`lekin`/`ki`/`jab`/`jo`** (the joining
category scores **0 of 6**), the **ordinals**, **`apna`**, **`ko`**,
**possession with `mere pas`**, and the lexical domains of **transport,
payment, free time, media, written correspondence, documents and educational
institutions**.

### 6. Two cheap, real fixes this surfaced - worth their own PRs

- **Eight Hindi writing lessons teach something and declare an EMPTY
  `introduces` list**, so they contribute no atom and cannot be probed at all.
  `HI-W06-name-sentence-stop` teaches the danda; `HI-W06-two-sentence-card` and
  `HI-W06-two-sentence-no-model` teach the 30-40 word message to a named
  reader, which is **60 of the A1 writing paper's 100 points**. The teaching
  exists and the knowledge contract does not. One frontmatter field per lesson
  makes two inventory points measurable with **no new content**.
- **The nuqta is taught nowhere and used everywhere.** The corpus's own
  headwords carry it constantly - shukriya, khushi, zarur, safed, darvaza, mez,
  sabzi, zyada, safar, tohfa, bukhar, fasal, khushbu, mulaqat - and no lesson
  explains the mark. Only an external target list surfaces this, because every
  corpus-internal metric counts those lessons as taught.

### 7. Conventions the other tracks should copy

1. Look for a real syllabus first, and **write down that you looked** and what
   you found. Section 1 is the deliverable when the answer is "nothing".
2. **No syllabus? Use the Spanish DELE-derived inventory as a level proxy.**
   Do not build from CEFR descriptors alone; they draw the ruler short.
3. State in `about` what KIND of claim the file is, in the first sentence.
   Attribute nothing to a body that did not publish it - this file says
   NOTHING MAY BE ATTRIBUTED TO DBHPS *and* ATTRIBUTE NOTHING TO DELE ABOUT
   HINDI, because both are true and only one is obvious.
4. `partial` on every dimension until a source genuinely closes it.
5. Validate every probe id mechanically, and make the proxy walk **total** with
   a build-failing check.
6. `probe: null` plus a note naming the missing piece, citing the mock item
   that demands it wherever one does. Never an invented id.
7. Mine your track's own mocks for the gap notes. A note that cites a numbered
   mock item is checkable in one step by someone who does not speak the
   language, which is the difference between a finding and an opinion.
