# Tamil Roadmap

Same shape as the other tracks: deep one-word lessons in themed chapters.
Slug-identified; order lives in the book and
[`session-map.md`](./session-map.md). See
[`HL00`](../../../specs/HL00-human-language-curriculum-framework.md).

Grounding: English + the **Dravidian family** (Kannada, Telugu, Malayalam) +
Sanskrit/Hindi, with Tamil as the native-Dravidian root the sisters trace back
to. Tamil script is taught **inline**, inside the word lessons, never as a
gated reading course. Grammar is introduced piece by piece, on the first word
that needs it.

## Authored

- **Ch. 1 — Greetings**: vaṇakkam → naṉṟi → ām → illai → sari → practice.
  Tamil script introduced through the words (inherent *a*, puḷḷi, vowel signs,
  independent vowels, the three-way *n*/*l*/*r* distinction), and the native
  vs. Sanskrit split shown through the greetings themselves.
- **Ch. 2 — Introducing Yourself**: peyar → eṉ → **eṉ peyar** ("my name is,"
  zero copula) → nī/nīṅgaḷ → eṉṉa → **uṅgaḷ peyar eṉṉa?** → magiḻcci → practice.
  Every atom native Dravidian and traced (*peyar* ≠ *name*); the **zero copula**
  (no word for "is"); respect-by-plural like French *vous*.

- **Ch. 3 — How Are You**: eppaḍi (how; the native *e-* questions) → nīṅgaḷ
  eppaḍi irukkiṟīrgaḷ? (the verb *iru* "to be" — the copula returns for states) →
  nāṉ (I ← Proto-Dravidian, unrelated to *me*) → nalam (well ← *nal-* "good," the
  root of *naṉṟi*) → paravāyillai (you're welcome; the *iru*/*illai* pair) →
  practice.
- **Ch. 4 — Farewells**: pō/vā (go/come) → pōy varugiṟēṉ ("I'll go and come
  back" — the Dravidian promise-of-return goodbye) → nāḷai pārkkalām (see you
  tomorrow) → mīṇḍum sandippōm (we'll meet again; native *mīṇḍum* + Sanskrit
  *sandi*) → practice.
- **Ch. 5 — First Verbs**: pēsu (to speak; stem + tense + person) → nāṉ tamiḻ
  pēsugiṟēṉ (I speak Tamil; the retroflex *ḻ*; no gender in the 1st person) →
  vāḻ (to live/flourish) → vēlai sey (to work; noun + *sey*, the twin of Hindi's
  *karnā*) → practice.

- **Ch. 6 — Case endings, and the sentence with no subject**: **-உக்கு** (*-ukku*,
  "to/for") — the first case ending, taught as the entry point to
  **agglutination**: Tamil **adds** a suffix that carries **one** meaning, keeps
  its shape and leaves the **seam visible** (*peyar* + *ukku*), where a Latin
  ending like *-īs* **fuses** case *and* number *and* declension into one
  indivisible lump. Includes the irregular *nāṉ* → **எனக்கு** *enakku* ("to me") →
  **எனக்குத் தமிழ் தெரியும்** (*enakku tamiḻ teriyum*, "I know Tamil") — literally
  "**to-me Tamil is-known**," with **no nominative "I"** — the person sits in the
  dative instead (a **dative subject**), while the theme *tamiḻ* stays unmarked. The
  **dative-subject** rule: knowing, liking, wanting and being cold *happen to*
  you rather than being done *by* you, so the experiencer takes the dative
  (English keeps one fossil of it in "**methinks**"). Closes on the four-sister
  table — *-ukku / -ku / -ge / -ikku* are visibly the **same suffix**, the
  Dravidian family showing its bones the way *blanc/bianco/branco* did for
  Romance. **Authored.**

### Writing the letters *(authored)* — the "break it apart and write it" strand

The **first handwriting track for any Dravidian language**. Until now
`data/scripts/` held Arabic, Chinese, Cyrillic, Devanagari, Gujarati and Hebrew —
nothing for Tamil, Telugu, Kannada or Malayalam — so four tracks had vocabulary
through Chapter 6 and no way to learn to read it. `tamil.json` is new here.

Lessons follow dependency order, taking the letters the Chapter 1 words
actually need. The original four topics are now eight sub-five-minute steps:

- **`TA-W01-curves-va-ka`** — **why Tamil is round**. Opens on the question the whole script answers: *why
  is Tamil round?* The usual account is **palm leaves**: incised with a stylus,
  where a straight stroke along the grain can **split the leaf**, so strokes bend
  into curves. The lesson gives that as the standard explanation *rather than a
  settled fact* — earliest Tamil-Brahmi is angular, the rounding arrived later
  via Vaṭṭeḻuttu, and Devanagari used the same leaves without going round. The
  durable point is that **the tool leaves fingerprints on the letters**.
- **`TA-W01-abugida-va-ka`** — **வ, க** and the **abugida** principle (க is *ka*, not *k*), plus the fact that **one letter க
  spells k, g and h**, decided by position — which is why Tamil needs 18
  consonant letters where Devanagari needs 33.
- **`TA-W02-ma-retroflex-na`** — **ம, ண**, and the **retroflex**: ண is said with the tongue
  curled back, a sound English lacks and cannot hear at first. Introduces the
  physical tongue position before comparing the full set.
- **`TA-W02-three-ns`** — the three-n map: **ந** dental · **ன** alveolar · **ண**
  retroflex, without yet drawing the other two.
- **`TA-W03-pulli-vanakkam`** — the **puḷḷi** ் ("the dot"), which removes the inherent vowel —
  and the sharp divergence from Devanagari: Tamil does **not fuse** the bare
  consonant into a conjunct. Both letters keep their shape and the dot stays
  visible, which is why Tamil's whole character set is ~247 where Devanagari has
  hundreds of ligatures.
- **`TA-W03-write-vanakkam`** — assembles **வணக்கம்**, the first whole written
  word, and holds its doubled consonant.
- **`TA-W04-vowel-signs-nandri`** — **ந, ன, ற**, completing the letter bodies
  needed for the second whole word.
- **`TA-W04-i-sign-write-nandri`** — introduces the first vowel sign **ி**,
  assembles **நன்றி**, and shows why the three n's earn their keep:
  **ன் + ற** is said together as *ndr* — one instance of the general rule that a
  **nasal voices the stop after it** (ந்த *nd* · ண்ட *ṇḍ* · ன்ற *ndr*), so each
  n produces its own cluster and the spelling tells you which. It is also why
  *naṉṟi* is so often written *nandri* in English.

Next in this strand: the remaining vowel signs (**ை** is written *before* the
consonant and pronounced *after*, the same trap as Devanagari's ि), then the
letters the lessons already quote but do not yet teach — **ப, ள, ு** from
*puḷḷi*'s own name, and ங, ட, த, ர among others. W03 carries a **standing
read-now-draw-later note** for the whole track rather than an enumerated list,
since the list grows with every example.

## Chapter 32 — The Core Verbs (authored)

Six lessons, one verb each, and the track's first realisation of the shared
canonical `VERB-*` concepts: இரு (`VERB-BE`), போ (`VERB-GO`), வா (`VERB-COME`),
சாப்பிடு (`VERB-EAT`), பார் (`VERB-SEE`), தெரி (`VERB-KNOW`). The through-line is
agglutination — stem + tense + person — introduced on இரு and then pressed once
per lesson: the tense bead (போ), a stem with two shapes (வா), a noun plus a light
verb (சாப்பிடு), the strong/weak split (பார்), and the person-less verb whose
knower sits in the dative (தெரி). It is the first Tamil content above A1.

## Chapters 33–34 — the eight shared verbs (authored)

Eight more lessons on `SPINE-SAY-WHAT-I-DO`, in two chapters of four, taking
Tamil from 6 to **14 of the canonical core 40**. Chapter 33 is the mind — நினை
(`VERB-THINK`), புரி (`VERB-UNDERSTAND`), படி (`VERB-READ`), எழுது
(`VERB-WRITE`) — and presses the strong/weak split until it is a reflex, while
finally stating the positional-voicing rule (why this course writes *paḍi*, not
*paṭi*) and giving **ழ** an honest lesson of its own. Chapter 34 is between
people — எடு (`VERB-TAKE`), கேள் (`VERB-ASK`), உதவு (`VERB-HELP`), பிடி
(`VERB-LIKE-LOVE`) — and states the track's **diglossia** position outright on
*kēḷ* (literary *kēḷ* · standard spoken *kēṭkiṟēṉ*, which is what these lessons
teach · colloquial *kēkka*). Its payoff, **எனக்குப் பிடிக்கும்**, completes a
set of three dative-experiencer verbs with *teriyum* and *purigiṟadu*.

Still open in this strand: the past and future in their own right, negation
(*-illai* on a verb), and the infinitive/habitual pair the spine node still
records as omitted. `VERB-HEAR` also stays omitted even though கேள் covers
hearing, because one lesson realises one concept — a separate hearing lesson
would have to earn its own place.

## Chapters 35–38 — the first nouns, and a level-gate probe (authored)

Fifteen everyday nouns in four chapters, attached to **pre-A1** spine nodes
through five new `TA-EXT-03*-LANGUAGE-SPECIFIC` extensions so that they count
where the gate measures. Chapter 35 (`SPINE-MEET-GREET`) is arriving at a house;
36 (`SPINE-POLITE-REQUEST-REPAIR`) is what you are offered and how you say the
mistake was yours; 37 (`SPINE-EXCHANGE-NAMES`) is the native place and the
friend, ending on **இவர் என் நண்பர்**; 38 (`SPINE-CHECK-WELLBEING` and
`SPINE-TAKE-LEAVE`) asks after a body and takes leave.

The measurement it was written to take: pre-A1 headwords **33 → 48**, the
vocabulary shortfall **267 → 252**, and pre-A1 atoms revisited fewer than twice
**29 → 0**. Fifteen lessons bought fifteen headwords, because the gate counts
one headword per lesson; the remaining 252 are therefore roughly 252 more
lessons at pre-A1, which is HL09 §3's own arithmetic seen from the other side.

Still open at pre-A1 for this track: the vocabulary gap above, and
`TA-W01-abugida-va-ka`, which introduces four atoms against a budget of three
and is the last `atom-budget` blocker.

## Chapter 65 — the pre-A1 verb floor (authored)

The **verb-vocabulary** criterion turned out to be closable by five lessons and
nothing else could close it. HL09 §3.1 asks for five distinct verb headwords at
or below pre-A1; the track was teaching four, while carrying twenty-three verb
lessons in total — the other nineteen all realise `SPINE-SAY-WHAT-I-DO`,
`SPINE-NAME-EVERYDAY-ACTIONS`, `SPINE-SAY-WHAT-I-WANT` or `SPINE-SAY-WHAT-I-LIKE`,
every one of them A1 or A2. Teaching a twenty-fourth verb at A1 would have moved
the number by zero.

Chapter 65 puts five on `SPINE-MEET-GREET`: குடி, நட, நிறுத்து, திற, மூடு, in
opposing pairs, with `TA-W21-read-kudi` returning to the first of them once it
is familiar by ear. **verb-vocabulary 4/5 → 9/5, criterion cleared;
vocabulary 155/300 → 160/300.**

The reusable finding, for whoever writes the next tranche in any Dravidian
track: **check which spine node a criterion is counting before authoring against
it.** The concept tags here are namespaced `TA-VERB-*` on purpose — canonical
`VERB-DRINK`, `VERB-WALK`, `VERB-OPEN` and `VERB-CLOSE` are owned by
`SPINE-NAME-EVERYDAY-ACTIONS`, so claiming them would have relocated all five
lessons to A1 and left pre-A1 exactly where it started.

Still open at pre-A1 after this chapter: **vocabulary** (140 short of 300),
**reinforcement** (50 atoms revisited fewer than twice), and the single
`atom-budget` lesson above.

## Chapter 66 — which way, and the two endings behind it (authored)

Six directions on `SPINE-MEET-GREET`: மேலே, கீழே, உள்ளே, வெளியே, வலது, இடது,
each arriving beside the direction that answers it. **vocabulary
160/300 → 166/300.**

The chapter is really about two endings the reader already owns without having
been told they were endings. Four of the six are a bare place-word plus **-ஏ**,
the same ending inside **இங்கே** and **அங்கே**; the other two are a word plus
**-து**, the same ending inside **இது** and **அது** — which is why வலது and
இடது go in front of a noun (**வலது கை**) while the other four go in front of a
verb (**உள்ளே வா**, **வெளியே போ**). Every verb and noun they attach to was
taught earlier, so six new words buy a dozen usable instructions.

`TA-W22-read-mele` is the strand's second consecutive no-new-letter lesson: it
reads **மேலே**, whose whole content is the **ே** sign ridden twice, and it sits
third so the word is retrieved inside its R1 window.

The reusable finding: **a new tranche can be authored with zero closure cost
once a track's glyph inventory is closed.** Every glyph chapter 66 prints is
taught by chapter 39, so the chapter adds no script-closure violation at all —
which is only true because the inventory was closed first, and is a reason to
close it early in the other Indic tracks.

## Chapters 67-73 - the pre-A1 vocabulary tranche (authored)

**Seven chapters, five headwords each: 166/300 -> 201/300 at pre-A1.** The
completion plan (HL15) measures vocabulary as **92% of all remaining work to
C2** across the corpus; this is one tranche against Tamil's share of it.

| chapter | node | five words |
|---|---|---|
| 67 What You Wear | SPINE-COURTESY-THANK | sattai, vetti, pudavai, seruppu, toppi |
| 68 At the Shop | SPINE-RESPOND-BASIC | kadai, vilai, panam, vaangu, pai |
| 69 Getting There | SPINE-TAKE-LEAVE | vandi, perundu, rayil, saalai, nilaiyam |
| 70 The Room You Are In | SPINE-EXCHANGE-NAMES | mesai, kattil, suvar, tarai, kurai |
| 71 Through the Day | SPINE-MEET-GREET | ezhu, tungu, odu, kazhuvu, vilaiyaadu |
| 72 How It Feels | SPINE-CHECK-WELLBEING | kopam, payam, varuttam, siri, azhu |
| 73 Today, Yesterday, and the Year | SPINE-POLITE-REQUEST-REPAIR | inru, netru, munbu, maadam, aandu |

**A pre-A1 node on every lesson is the whole trick.** Level is read from the
spine node, not from the chapter number, so a tranche of excellent words on A1
nodes moves the pre-A1 number by exactly zero. All seven pre-A1 nodes are in
rotation here, one per chapter, and every node is freely reusable.

**The two-back rollback.** Each lesson's Guided Practice ends with a
`[YOU RECALL: ...]` line naming the two items before it, and the frontmatter
declares those two atoms as `requires`/`practises`. That is not bookkeeping: it
is what keeps every atom revisited twice, and it is why 42 new lessons left the
reinforcement finding at **80, unchanged**, where a naive append would have
pushed it to 98.

**Reading is interleaved, not blocked.** Every chapter carries one script lesson
in third position — `TA-W23`-`TA-W28` read a word from that same chapter, and
`TA-S126-letter-oo` teaches the single letter **O**, the last independent vowel
the track had never taught. It lands one lesson before **odu** needs it, which is
the only place in the tranche a new glyph was spent.

**Two things this cost, and what was done about them instead of around them:**

* **varam was cut.** It was the natural "week" between month and day, and
  chapter 10 already prints it - as the *Sanskrit* word Tamil's own weekday
  names deliberately do **not** build on. Teaching it here would have both
  contradicted chapter 10 and pushed Tamil's forward-reference count from 7 to 8.
  **munbu** ("before, earlier") took the slot instead: unshown anywhere in the
  track, and the opposite of **piraku**, which the reader already has.
* **Nothing else was reseated.** `forwardReferences` stayed at 7,
  `scriptClosureViolations` and `neverTaughtGlyphs` at 0, and the atom budget at 1.
  The only pinned number that moved is the one this work exists to move.

## Planned

| Chapter | Theme |
|---|---|
| 7 | The rest of the case suffixes — accusative *-ai*, locative *-il* — now that Ch. 6 has established how stacking works |
| 8+ | Tense (past/future), numbers, family, food — always with the Dravidian-cognate thread |

Note: Tamil marks "you" by **register** (*nī* familiar / *nīṅgaḷ* respectful,
also plural) — like the Romance/Germanic tracks, and worth teaching beside
them.
