# Changelog

## Unreleased — Chapters 51-57: 35 more words on the pre-A1 nodes

Thirty-five lessons, one new word each, appended after chapter 50 and chained
from `TA-C50-welcome`. The pre-A1 vocabulary count moves 84 → 119 of 300, a rise
of exactly one per lesson, and Tamil goes from the furthest-behind track on that
measure to level with the leaders.

Seven chapters, one pre-A1 spine node each, all seven nodes used a second time:

| Ch | Node | Words |
|---|---|---|
| 51 | MEET-GREET | ஆகாயம் சூரியன் நிலா விண்மீன் மேகம் |
| 52 | EXCHANGE-NAMES | மரம் கிளை தளிர் வேர் விதை |
| 53 | POLITE-REQUEST-REPAIR | நதி மலை வயல் பாதை கிராமம் |
| 54 | COURTESY-THANK | பாய் கூடை கத்தி கிண்ணம் பெட்டி |
| 55 | CHECK-WELLBEING | கழுத்து முதுகு உதடு நகம் எலும்பு |
| 56 | RESPOND-BASIC | சிறிது நிறைய குறைவு தேவையில்லை ஆகட்டும் |
| 57 | TAKE-LEAVE | காற்று மணல் சேறு புகை கனல் |

R1 — the tight reinforcement window — improved 0.2746 → 0.2723 with its
numerator held at 1127: the tranche added 35 atoms and missed the window with
none of them. Forward references held at 16, rule statements at 30, script
closure violations at 35 and cross-chapter number references at their baseline.

**Nine candidate words were discarded rather than reworded.** வானம் ('sky') is
already glossed inside `TA-C20-vaanilai`, ஞாயிறு ('sun') is committed as the
name of a weekday in `TA-C10-vaara-kizhamai`, and ஆறு ('river') is the same
string the book already counts with — so ஆகாயம், சூரியன் and நதி took their
places. இலை ('leaf') was caught only by checking ROMANIZATIONS as well as
script: `TA-C50-betel-leaf` spells out "*-ilai* is *ilai*, 'a leaf'" in prose,
so teaching it here would make that page point forward; தளிர் replaced it.
வேண்டாம் sits inside `TA-C39-vendum` and போதும் is already chapter 47's
headword, so தேவையில்லை and ஆகட்டும் took the last two reply slots.

**One discard no string check could have made.** அதிகம் ('a lot') passed every
substring and romanization test, because the corpus does not contain the word —
it contains *ati-*, glossed as "excessive, very" inside `TA-C26-kaalai`'s
account of அதிகாலை. Teaching அதிகம் would have re-taught that morpheme and
pointed the earlier page forward. நிறைய replaced it.

**Two were discarded by the taught-glyph filter.** கொஞ்சம் ('a little') and
ஓலை ('palm leaf') need ொ and ஓ, neither of which any Tamil writing lesson
teaches. Every headword in the tranche is spelled from the forty glyphs the
track's own writing lessons already cover, so the tranche adds zero script
closure violations and launders nothing through an exposure-exempt headword.
கொஞ்சம் still earns its mention in `TA-C56-a-little`, romanized.

Every cousin citation is checked against a string already committed in this
repo rather than recalled: the Malayalam and Telugu words quoted here are the
headwords of `ML-C53`..`ML-C59` and `TE-C53`..`TE-C59`. Citations needing a
letter the Tamil track has never shown are romanized instead, so the tranche
adds no new foreign glyphs to any lesson.

`TA-C50-welcome` was reworded before anything was appended (HL-C201). It said
the welcome "brings this book back to where it started", which was true while
it was the last lesson and false the moment chapter 51 existed. It now says the
word sets the reader back down where the book began — a claim about the reader,
not about where the book stops.

The book renders in `scriptSet: tamil-comparisons`, copied field for field from
the chapter-50 target: 403 pages, zero missing characters, zero overfull and
zero underfull boxes.

## Unreleased — Chapters 44-50: 35 words on the pre-A1 nodes

Thirty-five lessons, one new word each, appended after chapter 43 and chained
from `TA-C43-family`. The pre-A1 vocabulary count moves 49 → 84 of 300, a rise
of exactly one per lesson, and Tamil stops being the furthest-behind track on
the measure that decides when pre-A1 is actually attained.

Seven chapters, one pre-A1 spine node each, all seven nodes used:

| Ch | Node | Words |
|---|---|---|
| 44 | POLITE-REQUEST-REPAIR | பழம் துணி விளக்கு உப்பு புத்தகம் |
| 45 | CHECK-WELLBEING | கண் காது மூக்கு பல் வயிறு |
| 46 | EXCHANGE-NAMES | ஆசிரியர் மாணவன் மருத்துவர் விவசாயி விருந்தாளி |
| 47 | RESPOND-BASIC | உண்மை போதும் நிச்சயமாக ஒருவேளை அப்படியே |
| 48 | TAKE-LEAVE | இப்போது நேரம் பயணம் பிறகு புறப்படு |
| 49 | COURTESY-THANK | அன்பு மரியாதை சந்தோஷம் வாழ்த்து மிக்க நன்றி |
| 50 | MEET-GREET | வாசல் கோலம் மலர் வெற்றிலை வரவேற்பு |

Two of those nodes had never carried a Tamil lesson before: SPINE-RESPOND-BASIC
and SPINE-COURTESY-THANK were declared in the ledger but no lesson had ever sat
on them, so chapters 47 and 49 realise them for the first time.

Reuse is the point, not padding. Each lesson introduces one word and practises
the one before it, so every new word is retrieved again within a lesson and
again at the chapter payoff. R1 — the tight reinforcement window — improved
0.2949 → 0.2922 with its numerator held at 1106: the tranche added 35 atoms and
missed the window with none of them.

Eight candidate words were discarded rather than reworded. கால் ('leg') is
already spelled out inside `TA-C26-kaalai` and `TA-C35-naarkaali`, உதவி ('help')
inside `TA-C34-utavu`, and இனிப்பு ('sweet') inside `TA-C25-iniya-iravu` —
teaching any of them here would make those earlier pages point forward. Three
more were caught only by checking inside multi-word headwords, which a scan of
prose alone misses: மாலை ('garland') is the same string `TA-C27-maalai` already
teaches for 'evening', நாளை sits inside the headword *நாளை பார்க்கலாம்*, and
தலை and கை sit inside *தலை கை*. Forward references held at 500.

The book renders in `scriptSet: tamil-comparisons`, copied field for field from
the chapter-43 target, so every Tamil, Telugu, Kannada and Malayalam run is
wrapped by its own script command: 355 pages, zero missing characters.

## Unreleased — Chapter 40: Pointing, and Asking

Six words and the pattern behind them: இது அது இங்கே அங்கே யார் எங்கே

Until now the reader could NAME things and not point at them. With these they
can: *this one*, *that one*, *here*, *there*, and the two questions that matter
first — *who?* and *where?* Everything already in the book becomes a sentence
they can use.

The seventh lesson is the reason these six are one chapter. They are not six
words, they are **i- / a- / e-** — three beginnings on the same ending, and changing
the front walks the meaning from near to far to a question. A reader who sees
that once does not have to be taught the third member of the next family they
meet; they will work it out.

The whole chapter is **voice**: nothing in it needs eyes, so it is learnable end
to end at the wheel.

## Unreleased — the script drizzle begins

Nine lessons, one letter each: வ, ண, ன, ந, ற, க, ம, the puḷḷi and the i-sign.

Tamil already had 24 writing lessons, and every one of them was word-shaped —
*write வணக்கம்*, *read peyar* — showing four to sixteen letters at once. These
are the missing layer underneath: a letter met on its own, with its parts, its
pen path and its pen-lift count, immediately before the word lesson that uses
it. Closure violations 50 → 42.

The four n-letters ண, ன, ந and ற arrive consecutively on purpose. They share a
flat top bar, `tamil.json` already recorded that they are best learned as a
family, and splitting them across the book to chase payoff would trade a reading
ramp for a writing confusion.

## Unreleased — 46 words a reader can now say

Added `romanization` to 46 lessons that had none, so their headwords become
HL11 *exposure* — something the reader is shown and can use — rather than script
they are stuck on. Each is recovered from the pronunciation the lesson already
gives in its own prose, then checked against the headword's script so a wrong
grab cannot pass. Nothing is transliterated: a mechanical romanization of this
script disagrees with its own authors often enough to teach mispronunciations.

## Chapter 39 — asking for what you want, and the first chapter added to finish the script

`TA-W19` measured the writing strand out of room after itself: no slot followed
it that satisfied the twelve-atom chapter budget, the three-speaking-lessons
cadence and the rule that a chapter must not open on a pen lesson. Three
letters — **ஏ**, **ஐ**, **ஒ** — were still used inside words the learner reads
and never taught. Extending the track was chosen over relaxing any of those
constraints, and this is the first of the three chapters that does it.

The search that informed the decision also claimed there was no slot anywhere
earlier in the track. That claim was wrong — it mis-assigned end-of-chapter
positions to the following chapter and so never tested them. Corrected, exactly
one slot exists: chapter 35, after **நாற்காலி**, with gaps of 3 and 3 either
side and the chapter landing on its twelve-atom ceiling. One slot cannot hold
three letters at one letter per lesson, so the extension was needed regardless,
but the record should say one rather than none.

### What the chapter teaches

| lesson | seq | what it adds |
|---|---|---|
| `TA-C39-vendum` | 1170 | **வேண்டும்**, built with no subject; **வேண்டாம்** as its own negative rather than **இல்லை** |
| `TA-C39-evvalavu` | 1180 | **எவ்வளவு** for quantity and price, against **எத்தனை** for things counted one by one |
| `TA-C39-oru` | 1190 | **ஒரு** in front of a noun where **ஒன்று** cannot stand — the payoff, ordering one tea |
| `TA-W20-read-onru` | 1195 | **ஒ**, spelling **ஒன்று** and **ஒரு** |

### What it was worth, measured

The chapter reaches back further than it teaches forward. Naming **தெரியும்**,
**புரிகிறது** and **பிடிக்கும்** in one clause — and declaring all six of their
atoms rather than only mentioning the words — closes three reinforcement
windows open since chapters 32-33 and keeps two more from opening. A fourth
chapter-34 closure, `TA-SCRIPT-EE-SIGN-01`, is earned elsewhere in the lesson,
by the **ே** of **பேசு** in its letters section. Crediting chapter 6's dative and
chapter 7's numbers reaches those particular atoms at R4 distance for the first
time — `TA-GRAMMAR-DATIVE-SUBJECT-02` at 90 lessons. Twelve Tamil atoms already
had an R4-range revisit before this chapter; these three did not.

Ten atoms leave a window in total, and R3 comes out exactly level: the seven
atoms that enter it by the track simply getting longer are matched by seven the
chapter genuinely reinforces. Twelve glyphs remain — **ஏ**, **ஐ** and the ten
Tamil digits — which chapters 40 and 41 are planned to close.

## Chapters 35–38 — fifteen everyday nouns, authored as a level-gate probe

This tranche exists to answer a measurement question as much as a teaching one:
**does authoring track-local pre-A1 vocabulary actually move
`levelGate.tracks[tamil]`?** It does, and the size of the move is the finding.

### What the gate did

| figure | before | after |
|---|---|---|
| distinct headwords at or below pre-A1 | 33 | **48** |
| `vocabulary` shortfall against the 300 target | 267 | **252** |
| pre-A1 atoms revisited fewer than twice | 29 | **0** |
| track-wide distinct headwords | 67 | **82** |
| atoms never revisited (of atoms taught) | 48 of 126 | **37 of 156** |

Fifteen new lessons moved the pre-A1 vocabulary count by **exactly fifteen**,
because `vocabularyOf` counts **distinct headword strings, one per lesson** —
not the words a lesson teaches. `TA-C12-kudumbam` teaches six kinship terms and
counts as one; `TA-C15-thanneer-arisi` teaches three and counts as one. Closing
a 267-headword gap therefore means roughly **267 more lessons at pre-A1**, which
is the same arithmetic HL09 §3 already published (~150 lessons for the first 300
words, at ~2 words per lesson) arriving from the other direction.

The **reinforcement** criterion behaved very differently: it went from 29 to
**zero**, because a payoff can rescue many atoms at once. Reinforcement debt is
cheap to clear by authoring; vocabulary debt is not.

The `atom-budget` blocker is untouched at 1 — `TA-W01-abugida-va-ka` introduces
four atoms against a budget of three, and predates this work.

### The chapters

| chapter | node | lessons |
|---|---|---|
| 35 — Arriving at a House | `SPINE-MEET-GREET` | வீடு, கதவு, அறை, நாற்காலி |
| 36 — What You Are Offered | `SPINE-POLITE-REQUEST-REPAIR` | தேநீர், பால், உணவு, தவறு |
| 37 — Your Town, Your Friend | `SPINE-EXCHANGE-NAMES` | ஊர், நண்பன், இவர், இவர் என் நண்பர் |
| 38 — Keeping Well, and Leaving | `SPINE-CHECK-WELLBEING`, `SPINE-TAKE-LEAVE` | உடம்பு, சுகம், விடை |

Sequences 750–890, all schema v2, two new atoms each — 8, 8, 8 and 6 per
chapter, all inside the 12-atom chapter budget. Attached to pre-A1 spine nodes
through five new `TA-EXT-03*-LANGUAGE-SPECIFIC` extensions on path segments
`TA-PATH-031`–`035`, which is what makes them count at pre-A1 at all.

### Rescue, at two cadences

Every lesson practises the preceding one to three lessons (R1), and each chapter
payoff reaches several chapters back. Between them the tranche cleared **all 29**
pre-A1 atoms that were revisited fewer than twice — the eight Chapter-1 writing
atoms (**வ**, **க**, **ண**, **ற**, the puḷḷi, the no-conjunct rule, the **ி**
sign, nasal-triggered voicing), the dative pair from Chapter 6, the register
atoms from Chapters 8–9 and 19, the kinship atoms from Chapter 12, the body
parts from Chapter 13, and the water/rice atoms from Chapter 15. Each rescue is
a real revisit: **நாற்காலி** genuinely uses **நான்கு**, **தவறு** genuinely
re-reads **மன்னிக்கவும்**, **உடம்பு** genuinely recalls **தலை** and **கை**.

### What the words are, and what they are not

Several of the obvious "first nouns" were **already taught** and are not
repeated here: தண்ணீர் and சாதம் (Chapter 15), பெயர் (Chapter 2), and அப்பா,
அம்மா, அண்ணன், தம்பி, அக்கா, தங்கை (Chapter 12 — Tamil's age-graded kinship is
already in the course). Chapter 37 builds on that rather than restating it:
**நண்பன் / நண்பி / நண்பர்** splits by sex and respect where the sibling words
split by age, and the payoff names the contrast.

The etymology was checked against the Dravidian Etymological Dictionary, the
Madras Tamil Lexicon and Wiktionary before authoring, and four claims changed as
a result:

- **வீடு** "house" and **வீடு** "release" are **one DEDR entry**, headed by
  **விடு** "to let go" — and **விடை**, Chapter 38's payoff, is in that entry
  too. The chapter arc walks in through the house and out through the leave, on
  one root, because the dictionary says so.
- **தேநீர்**'s **தே** came through **Malay *teh***, not straight from Chinese;
  and **Portuguese *chá*** is the *exception* to the maritime-*te* pattern
  (Macau), not an example of the overland *cha* one.
- **கதவு** has **no Telugu cognate** in DEDR, and **அறை** has **no Kannada**
  one. Neither is claimed.
- **சாப்பிடு** is *not* safely *sāppu* "food" + **இடு**, as this course said in
  Chapter 32: no Tamil lexicon records a plain *sāppu* meaning "food," and the
  Dravidian dictionary has **no entry for the verb at all**. `TA-C36-unavu`
  says so plainly rather than repeating the earlier account.

One genuine English link is claimed and it is a new one: **mulligatawny** is
**மிளகுத்தண்ணீர்**, "pepper water," carrying the **தண்ணீர்** taught in
Chapter 15. *Catamaran*, *curry*, *mango* and *pariah* are Tamil's other gifts
to English and none of them comes from a root in this tranche, so none is
claimed.

### Book, narration, drivability

Chapters 35–38 are generated to `book/chapters/`, added to `book.tex`, and the
178-page book compiles with XeLaTeX with **zero** `Missing character` warnings.
All fifteen lessons derive `coreModality: voice` — every one is drivable once
the detachable *"The letters in this word"* section is set aside — and every
table is two or three columns, so the narration exporter linearises all of them
rather than refusing any.

## Chapters 33–34 — the eight shared verbs, and a Dravidian seat at the join

Chapter 32 put Tamil on the canonical `VERB-*` concepts with six verbs. Before
this tranche Tamil covered **6 of the core 40**, and eight of the concepts it
omitted — `VERB-THINK`, `VERB-UNDERSTAND`, `VERB-READ`, `VERB-WRITE`,
`VERB-TAKE`, `VERB-ASK`, `VERB-HELP`, `VERB-LIKE-LOVE` — were taught by exactly
**three** tracks in this repository: Spanish (21 core verbs), Latin (16) and
Portuguese (15). All three are Indo-European. Every one of these eight lessons
therefore widens a **three-way join to a four-way one**, and Tamil is the first
**Dravidian** contributor to any of them.

Tamil now covers **14 of the core 40 (35%)**, third behind Spanish and Latin.

### Two chapters of four, not one of eight

| lesson | word | concept |
|---|---|---|
| `TA-C33-ninai` | நினை (*ninai*) | `VERB-THINK` |
| `TA-C33-puri` | புரி (*puri*) | `VERB-UNDERSTAND` |
| `TA-C33-padi` | படி (*paḍi*) | `VERB-READ` |
| `TA-C33-ezhutu` | எழுது (*eḻudu*) | `VERB-WRITE` |
| `TA-C34-edu` | எடு (*eḍu*) | `VERB-TAKE` |
| `TA-C34-kel` | கேள் (*kēḷ*) | `VERB-ASK` |
| `TA-C34-utavu` | உதவு (*udavu*) | `VERB-HELP` |
| `TA-C34-pidi` | பிடி (*piḍi*) | `VERB-LIKE-LOVE` |

Sequences 670–740, all schema v2, all on `SPINE-SAY-WHAT-I-DO`. Chapter 33
introduces **9** atoms and Chapter 34 **10**, both under the 12-atom chapter
budget; no lesson introduces more than three, and every one is under five
minutes as computed (285–299 effective seconds).

### What each chapter is for

**Chapter 33 — the mind, and the two facts the track had been assuming.**
Agglutination is Tamil's gentle payoff, and this is where it earns its keep: two
strong verbs (*ninai*, *paḍi*) and two weak ones (*puri*, *eḻudu*), each
announcing its camp with a held or a softened consonant, so the learner sorts a
new verb by ear rather than by rule. Along the way the chapter finally says two
things out loud:

- **Tamil marks no voicing**, so ட spells *ṭ* and *ḍ* by position — which is why
  this course writes *paḍi*, *eḍu*, *sāppiḍu* and *pōgiṟēṉ*. That is Chapter 1's
  one-letter-three-sounds rule for க, applied where it had been silently in use.
- **ழ**, the retroflex glide in *tamiḻ* itself, gets an honest treatment: how to
  make it (curl back, do **not** touch), the fact that many fluent speakers merge
  it with ள, and the family evidence — Kannada and Telugu each had a letter for
  it and let it go.

*Puri* is the quiet structural win: **எனக்குத் தமிழ் புரிகிறது** fills the person
slot with *-adu*, "it," which pushes the understander into the dative. So the
dative subject is a pattern, not *teri*'s eccentricity.

**Chapter 34 — between people, ending on the inversion.** *Eḍu* sits one vowel
from the **இடு** inside *sāppiḍu* and the lesson says plainly that this is a
memory hook and **not** an etymology, because a tidy pair of opposites is exactly
the resemblance that fools people. *Kēḷ* is where the track states its
**diglossia** position outright — literary *kēḷ*, standard spoken *kēṭkiṟēṉ*
(the register these lessons teach), colloquial *kēkka* — and where the stem-shift
habit *vā/varu-* taught returns as *kēḷ/kēṭ-*. *Udavu* is a native word where
Telugu, Kannada, Malayalam and Hindi all take Sanskrit *sahāya*, the exact mirror
of Chapter 8, where all four sisters borrowed *daya* together; the point drawn is
that borrowing is decided word by word.

The payoff is **எனக்குத் தமிழ் பிடிக்கும்** — "to me, Tamil catches." It
completes a set of three dative-experiencer verbs (*teriyum*, *purigiṟadu*,
*piḍikkum*) and lands the same inversion Spanish *me gusta*, Italian *mi piace*
and Hindi *mujhe pasand hai* use, reached from an unrelated family. Beside it,
**விரும்பு** takes an ordinary subject, so the learner has to choose between
taste and desire.

### English cousins: none, and the lessons say so

Tamil gave English *catamaran*, *mango*, *curry* and *pariah*, but **none of
these eight verbs is the source of an English word**. `TA-C33-ninai` states that
once, for the whole tranche, rather than inventing links. The evidence used
instead is internal (Tamil's own verb→noun endings: *puridal*, *kēḷvi*, *udavi*)
and Dravidian (Kannada *nene*/*nenapu*, Malayalam *eḻutuka*, Kannada *hiḍi*).
One etymology is left **unsettled on purpose**: whether *paḍi* is inherited or
Sanskrit **पठ्**. Malayalam's aspirated *paṭhikkuka* points one way; Kannada and
Telugu use different words entirely; the lesson refuses to decide and teaches the
certain part instead — an aspirate cannot survive into Tamil.

### Reinforcement, at two cadences

Every lesson practises atoms from the one to three lessons before it, across the
chapter seam, which closes R1/R2 with no extra lessons. Both payoffs reach much
further back. Fifteen atoms that **no later lesson had ever revisited** are
rescued here, from Chapters 1, 6, 7, 8, 15 and 32:

| rescued atom | from | where it lands |
|---|---|---|
| `TA-SOUND-ABUGIDA-VA-KA-04` | Ch. 1 | ட is *ṭ* or *ḍ* by position (`padi`) |
| `TA-SOUND-MA-RETROFLEX-NA-03` | Ch. 1 | ட is made where ண is (`padi`) |
| `TA-SCRIPT-PULLI-VANAKKAM-02` | Ch. 1 | எழுத்து keeps two த's apart (`ezhutu`) |
| `TA-GRAMMAR-DATIVE-SUBJECT-02` | Ch. 6 | *enakku purigiṟadu* / *piḍikkum* |
| `TA-GRAMMAR-DATIVE-SUBJECT-FAMILY-01` | Ch. 6 | four sisters' datives (`puri`) |
| `TA-ETYMON-NUMBERS-6-10-FAMILY-01` | Ch. 7 | *ēḻu* against *ēḷu* (`ezhutu`) |
| `TA-ETYMON-NUMBERS-6-10-FAMILY-02` | Ch. 7 | *piḍi* → *hiḍi* (`pidi`) |
| `TA-ETYMON-TAYAVUSEYTU-02` | Ch. 8 | *daya* borrowed, *udavi* not |
| `TA-SCRIPT-PLEASE-REGISTER-02` | Ch. 8 | the puḷḷi seam in கேட்கிறேன் |
| `TA-LEX-THANNEER-ARISI-02` | Ch. 15 | *nāṉ arisi eḍukkiṟēṉ* |
| `TA-ETYMON-SAAPPIDU-02` | Ch. 32 | இடு beside எடு |
| `TA-GRAMMAR-VAA-02` | Ch. 32 | *vā/varu-* beside *kēḷ/kēṭ-* |
| `TA-GRAMMAR-PAAR-02` | Ch. 32 | strong/weak, pressed four times |
| `TA-LEX-TERI-01`, `TA-GRAMMAR-TERI-02` | Ch. 32 | the dative-subject set of three |

Measured over the track, atoms no later lesson revisits fall from **57 of 107
(53%)** to **48 of 126 (38%)**; corpus-wide the never-revisited count falls from
767 to 758.

### Drivability and the obsolete workaround

All eight lessons use the canonical `## The letters in this word` heading. That
types as a `script` block, which is **detachable**, so each lesson's
`coreModality` stays `voice` and the driving edition is unharmed — seven are
`sight` at full modality and one (`TA-C34-pidi`) is `voice` outright. An earlier
Tamil tranche routed letter notes into `## Sounds you'll need` to dodge a `sight`
label; that trade no longer exists and the workaround is not repeated here. No
lesson trips a sight cue, and no table exceeds three columns.


## Chapter 32 — the core verbs, and Tamil's first canonical `VERB-*` lessons

Tamil taught eighty-three lessons across thirty-one chapters and **four verbs**
— பேசு, வாழ், செய், போ — every one of them under a Tamil-only tag
(`TA-VERB-PESU`, `TA-VERB-VAZH`, …). A namespaced tag joins nothing across
languages, so on the cross-language measurement the track covered **zero** of
the forty canonical core verbs.

Chapter 32 adds six, one per lesson, in a prerequisite chain:

| lesson | word | concept |
|---|---|---|
| `TA-C32-iru` | இரு (*iru*) | `VERB-BE` |
| `TA-C32-po` | போ (*pō*) | `VERB-GO` |
| `TA-C32-vaa` | வா (*vā*) | `VERB-COME` |
| `TA-C32-saappidu` | சாப்பிடு (*sāppiḍu*) | `VERB-EAT` |
| `TA-C32-paar` | பார் (*pār*) | `VERB-SEE` |
| `TA-C32-teri` | தெரி (*teri*) | `VERB-KNOW` |

Sequences 610–660, all schema v2, all on `SPINE-SAY-WHAT-I-DO`. Tamil now
covers 6 of the core 40.

**The chapter is built around one thing: Tamil is agglutinative.** A verb is
beads on a string — stem, then tense, then person — so **இருக்கிறேன்** is
literally *be + present + I*, and the last bead already means "I," which is why
*nāṉ* can be dropped. That is a genuinely different machine from the European
verb, and it is the best hook the language offers a beginner.

One idea per lesson, each on the word that needs it:

- **இரு** — the three slots, named and assembled.
- **போ** — the middle bead alone carries tense: *pōgiṟēṉ / pōṉēṉ / pōvēṉ*. Its
  Kannada cousin **ಹೋಗು** (*hōgu*) is the same *p → h* softening Chapter 7
  already taught with *pattu* / *hattu*.
- **வா** — the form you call a verb by is not always the form the beads attach
  to: *vā!* as a command, but *varu-* under the suffixes. The farewell **போய்
  வருகிறேன்** from Chapter 4 has been carrying that form all along.
- **சாப்பிடு** — a verb Tamil assembled rather than inherited: சாப்பு (food) +
  இடு (to put), with இடு named as a **light verb**. Said honestly: the origin of
  சாப்பு itself is unsettled, and the inherited Dravidian eat-root did **not**
  die — it lives next door as Telugu *tinu*, Kannada *tinnu*, Malayalam
  *tinnuka*, and inside Tamil as தின் (narrowed to chewing) and literary உண்.
- **பார்** — the **strong/weak** split, on the one difference a learner has
  already heard four times: *pārkkiṟēṉ* doubles its *k* where *pōgiṟēṉ* keeps
  one, and strong verbs take a doubled *t* in the past (*pārttēṉ*). Two camps,
  learned with the verb. The family-wide see-root காண் is kept in view beside
  everyday பார்.
- **தெரி** — the closing symmetry. **தெரியும்** has no person bead at all, so
  the knower cannot climb into the last slot and rides in the dative instead:
  **எனக்குத் தமிழ் தெரியும்**, the sentence Chapter 6 gave whole and this lesson
  finally opens. Telugu builds it identically (**నాకు తెలుసు**).

Dravidian discipline held throughout: no Indo-European cognates are invented for
Tamil words, every cousin cited is a Dravidian sister with its form supplied, and
where a root is unsettled the lesson says so.

Drivability held deliberately: all six derive `voice`. No script blocks, no
sight cues, and the two tables are three wide, so the corpus's drivable count
rises by six with no new sight lessons. The letters each word needs are taught
inline in its *"Sounds you'll need"* block — every one of them was already met in
an earlier chapter, so nothing new had to be gated behind a reading section.

Wiring: `curriculum.json` gains `TA-PATH-028` on `SPINE-SAY-WHAT-I-DO` (the
track's first content above A1) with the six lessons attached as the required
`TA-EXT-028-CORE-VERBS` extension, and that node's `omits` ledger drops the six
concepts now realised. `chapters.json` gains a Chapter 32 entry whose payoff,
`TA-C32-teri`, assesses 7 of the chapter's 12 atoms (0.58, above the 0.5 floor).
`core/book-generation.json` gains the Chapter 32 target; the generated
`book/chapters/ch32-core-verbs.tex` is `\input` from `book.tex`, and the book
compiles under XeLaTeX with zero `Missing character` reports.

## ம's stroke order corrected against the cited ductus

Two places in the repo described how **ம** is written, and they disagreed about
the one thing a static picture cannot show: **where the hand leaves the paper.**

- `tamil.json` listed three steps — *left vertical, bottom horizontal, right
  arch* — with no citation. Rendered as a numbered list under *"Write it — stroke
  order"*, that reads as three strokes and **two pen lifts.**
- Language Ladder's `strokes.ts` carries an authored pen path for ம: **one
  unbroken stroke** of five joined segments — **zero lifts** — cited to
  Radhakrishnan's *Tamil Script Learners Manual* (Appendix I, Frame 1, UT Austin)
  and checked mechanically against the font outline.

They were **not** in conflict about the ink. The prose was a coarser, three-way
naming of the same left → bottom → right motion, and its order matched. They
conflicted about **pen lifts** — and only the ductus had evidence. So the cited
source wins:

- `strokeOrder` is now the five cited movements, each worded **"without
  lifting"**, ending "*and only now lift*".
- `strokeOrderNote` states the claim in the heading the app renders: *one
  unbroken stroke — five movements, no pen lift*, with the citation.
- New `penLifts: 0` and `strokeOrderSource` fields record the verified claim and
  its provenance as data rather than as prose.
- The letter's `notes` now say outright that the five movements are parts of one
  pen-down run, not five strokes.

The **other ten** Tamil letters have no authored pen path, so nothing verifies
where their pen lifts. Their steps are unchanged — inventing lifts would repeat
the mistake in the other direction — and the file's script-level `notes` now
tell the next author to read them as part order only. Backlog item **HL-C19**
tracks verifying all 190 prose stroke orders across the nine scripts.
## HL05 chapter capability ledger — 27 of 31 chapters

- Added [`chapters.json`](./chapters.json), the track's authored chapter
  capability ledger: a first-person `canDo`, the shared spine nodes the chapter
  realises, and a payoff (lesson, kind, one-line summary, and the knowledge
  atoms it exercises) for every chapter that can honestly carry one.
- `title` and `label` are reproduced exactly from `core/book-generation.json`
  for Chapters 6–31, so the HL-C04 title inversion cannot silently rename a
  printed chapter. Chapter 1 has no target there and takes its printed name
  from `book/chapters/ch01-greetings.tex`, the file it already prints from.
- `canDo` is written for this track's actual reader — someone fluent and
  literate in Tamil who never studied the grammar formally. The claims are
  about grammatical and etymological *control* (choosing the register a moment
  calls for, keeping இரவு and இருள் apart, reporting a hedged etymology with
  its hedge intact), not about decoding the script.
- **Chapters 2, 3, 4 and 5 are deliberately absent.** Every lesson in them is
  still schema v1 with no `practises.knowledge`, so no payoff could name a real
  atom without inventing one. An absent entry is honest, measurable debt; a
  stubbed one would destroy the signal the HL05 gap report exists to produce.
- No Tamil chapter has a terminal `practice` or `practice-mix` lesson under
  schema v2, so every payoff is the chapter's last lesson by `sequence`. Eight
  of those chapters end on a non-conversational lesson, and their `payoff.kind`
  says so: Chapter 1 ends on an inline `writing` lesson and is recorded as a
  `task` (write நன்றி from its three pieces), as are the four chapters whose
  closing work is weighing etymological evidence rather than speaking.
- Four payoffs assess less than half the atoms their chapter introduces and
  will fail HL-C03's representativeness gate at the configured 0.5 threshold:
  Chapter 1 (5/17), Chapter 7 (3/8), Chapter 6 (2/5), and Chapter 20 (2/5).
  Each is a chapter whose final lesson is a narrow etymology or family
  comparison rather than a consolidation; the fix is a real terminal practice
  lesson, not a wider `assesses` list.

## Warning-free 31-chapter edition

- The forced 117-page XeLaTeX build now reports zero missing glyphs,
  overfull or underfull boxes, duplicate labels, Hyperref warnings, LaTeX
  warnings, or font warnings.
- Every Tamil and comparison-script font maps bold and italic teaching
  contexts to the vendored static files, while PDF-safe script commands keep
  the full Tamil text in bookmarks without font-command warnings.
- Unique practice labels remove duplicate destinations, short running heads
  keep navigation readable, and open-right chapter versos remain print-friendly
  while rendering truly empty.
- Five canonical micro-lessons use shorter headings, a clearer two-column
  weekday table, and a scannable wrap-up checklist. The generated chapters and
  independently verified source hashes therefore stay synchronized with
  Language Ladder instead of receiving book-only patches.

## Canonical Chapters 6–31 publication

- Fifty-one lessons now use schema v2 with explicit shared-spine placement,
  prerequisite-safe sequences, typed knowledge boundaries, sub-five-minute
  budgets, skills, modes, strands, register, and variety.
- Eight dependency-ordered writing companions keep Tamil script inside the
  first spoken chapter; forty-three later lessons generate twenty-six chapters
  that extend the downloadable book through Chapter 31.
- A reusable multi-script generator set selects the appropriate vendored font
  for Tamil, Telugu, Kannada, Malayalam, Devanagari, and Arabic comparisons.
- Canonical lesson ids and source hashes are independently reproduced by
  Language Ladder, keeping book and app content synchronized. The 117-page
  forced XeLaTeX build has zero missing glyphs and intact PDF metadata.
- The expanded book's remaining layout, duplicate-label, bookmark, font, and
  blank-verso cleanup is recorded in `HL-B33`; roadmap/session-map
  reconciliation remains in `HL-M07`.

## Sub-five-minute lesson remediation — 42 violations to zero

- Corrects twenty-two declared budgets whose lesson bodies already compute
  below five minutes.
- Splits twenty genuinely long lessons into prerequisite-ordered pairs, adding
  twenty focused companions rather than deleting script, etymology, grammar,
  register, family-comparison, or source-evidence depth.
- Expands the writing strand from four long topics to eight gentle steps:
  curves → abugida, retroflex **ṇ** → the three-n map, visible puḷḷi → whole-word
  **வணக்கம்**, and letter bodies → the **ி** sign and whole-word **நன்றி**.
- Leaves every affected step below 300 effective seconds and keeps all
  prerequisite references resolvable for the shared app/book corpus.

## Writing track W01–W04 — the first handwriting lessons for any Dravidian language

Four tracks — Tamil, Telugu, Kannada, Malayalam — had reached Chapter 6 with
vocabulary and **no way to learn to read it**, because `data/scripts/` had no
letter data for any Dravidian script. This adds **`tamil.json`** and the first
four writing lessons.

### The data

`tamil.json` ships **11 letters and 4 marks** — deliberately not the full
247-character grid. It covers exactly what Chapter 1's words need, is marked
`complete: false`, and carries `strokeOrderNote: "conventional"` throughout,
because Tamil handwriting varies by region and school and the descriptions name
**the pieces a learner can see** rather than prescribing one correct hand.

Every glyph was verified against its Unicode name, and every mark's `example`
was verified by actually concatenating base + mark and comparing to the claimed
result — so `ற` + `ி` really does produce `றி`.

**Also wires `gujarati.json` into the app**, which had existed since the Gujarati
track was authored and was never added to `SCRIPTS` — so the app rendered five
scripts while six had data. It now renders seven.

### The lessons — in book order, one piece at a time

- **`TA-W01` — வ, க.** Opens on the question the script answers: **why is Tamil
  round?** The usual account is **palm leaves**: incised with a stylus, where a
  straight stroke *along the grain* can split the leaf, so strokes bend into
  curves. Given as **the standard explanation, not a settled fact** — earliest
  Tamil-Brahmi is angular, the rounding came later via Vaṭṭeḻuttu, and Devanagari
  was written on the same leaves without going round. What survives the hedge is
  the general point: **the tool leaves fingerprints on the letters.** (Compare
  Latin's straight strokes, which suit a wax tablet and a chisel.) The lesson
  also notes that straight strokes *do* exist in Tamil — most of these letters
  have a flat bar across the top. Then the **abugida** principle,
  and that **one letter க spells *k*, *g* and *h***, decided by position — the
  reason Tamil needs 18 consonants where Devanagari needs 33.
- **`TA-W02` — ம, ண.** The **retroflex**: tongue curled back to the roof of the
  mouth, a sound English has no letter for and a beginner genuinely cannot hear
  at first — said plainly, because the alternative is a learner assuming they're
  failing. Sets up the three-n table (**ந** dental · **ன** alveolar · **ண**
  retroflex) without yet drawing the other two.
- **`TA-W03` — the puḷḷi ்**, "the dot", which removes the inherent vowel. The
  lesson's real content is a **divergence from Devanagari**: Tamil does **not**
  fuse the bare consonant into a conjunct — both letters keep their full shape
  and the dot stays visible. The consequence is arithmetic: ~247 characters
  total, against Devanagari's hundreds of ligature shapes. Tamil traded a smaller
  alphabet for slightly longer words. **Assembles வணக்கம்**, the first word of
  the course, and flags the doubled **க்க** (*vaṇak-kam*).
- **`TA-W04` — ந, ன, ற and the vowel sign ி.** Completes the three n's and adds
  the third thing a consonant can do (keep its vowel / lose it / **replace** it).
  **Assembles நன்றி** — and the payoff is that **ன் + ற** is pronounced together
  as *ndr*. That is presented as **one instance of a general Tamil rule** — a
  nasal voices the stop that follows it (ந்த *nd* · ண்ட *ṇḍ* · ன்ற *ndr*) —
  rather than as a property unique to those two letters. Each of the three n's
  produces its own cluster, and the spelling tells you which: that is the three
  n's earning their keep. It is also why *naṉṟi*, which is how Chapter 1
  romanizes it, is so often written *nandri* in English.

### Checks

- `sounds:` ids reused from the track's existing vocabulary (`tamil-inherent-a`,
  `pulli`, `retroflex-n`, `dental-vs-alveolar-vs-retroflex-n`, `matra-i`,
  `gemination-kk`, `final-m`) rather than invented.
- `writing`-type lessons carry **no `concept_tag`**, per the convention.
- Every Tamil character used in the four lessons was checked against the data
  file. Eleven appear that aren't in it — ஃ ங ட த ப ர ள ஷ ஸ ீ ு — all inside
  **quoted words and examples**, never in a `[YOU WRITE: …]` directive. W03
  carries a **standing read-now-draw-later note for the whole track** rather than
  an enumerated list, since the list grows with each new example. Verified
  mechanically: every character the lessons ask the learner to *draw*
  (க ண ந ன ம ற வ ி ்) has a real entry in `tamil.json`.
- **Letter shapes verified by rasterising the vendored font, not by eye.** A
  throwaway zero-dependency TTF reader (cmap → loca → glyf) extracted each
  glyph's true outline and scan-converted it to a text bitmap, which is what the
  `components` and `strokeOrder` descriptions were written against. This caught
  three descriptions that were confidently wrong:
  - **ற** was described as "a single arch, like a Latin n". It has **two**
    arches and three legs, with the right leg continuing below the baseline,
    sweeping left and dropping into a long descender.
  - **ண vs ன**: ண is **ன with one extra arch** — both open with the same top
    bar and loop and close with the same straight vertical, and ண carries two
    arches between them where ன carries one.

    This one is worth recording as a caution, because an intermediate revision
    of these lessons **replaced that correct statement with a false one** —
    claiming the two letters differed only in a final curving stroke. The cause
    was a rasteriser windowed to x≤1030 when **ண's outline runs to x=1631**:
    it amputated 37% of the letter, including the final vertical, and the
    description was then written from the truncated picture. A measurement
    instrument can be as confidently wrong as a memory. The window is now
    derived from the glyph's own bounding box, and the test asserting this
    fails if the window ever clips again.
- Verified in the browser: Tamil renders with 11 letters and "inventory in
  progress"; console clean.

## Chapter 6 — Case endings, and the sentence with no subject

- **Chapter 6 authored** (`TA-C06-dative-ukku`, `-dative-subject`): the track's
  first **case ending**, and the first Indic/Dravidian chapter since the
  curriculum rotation was rebalanced — reviewing Ch.2/3/5 via `reviews_of`.
- **-உக்கு** (`TA-C06-dative-ukku`): the dative "to/for," taught as the doorway to
  **agglutination** rather than as vocabulary. The contrast that makes it: Tamil
  **adds** a suffix that carries **one** meaning, keeps its shape, sits in a fixed
  order and leaves the **seam visible** (*peyar* + *ukku*) — where a Latin ending
  like *-īs* **fuses** case *and* number *and* declension into one indivisible
  lump. A four-row table sets the two systems side by side. Includes the irregular
  pronoun stem (*nāṉ* → *en-* → **எனக்கு** *enakku*), and *vēlaikku* built on Ch.5's
  வேலை.
- **எனக்குத் தமிழ் தெரியும்** (`TA-C06-dative-subject`): "I know Tamil" — literally
  "**to-me Tamil is-known**," with **no nominative "I"** — the person moved into the
  dative (a **dative subject**: it behaves as subject without being in the subject
  case, while the theme *tamiḻ* stays unmarked) — set directly
  against Ch.5's *nāṉ tamiḻ pēsugiṟēṉ* ("**I** speak Tamil"). Explains the
  **dative-subject** rule: Tamil sorts what you *do* from what *happens to* you, so
  knowing, liking, wanting and being cold put the experiencer in the dative.
  English's surviving fossil — "**methinks**," where *me* is a dative — is used as
  the bridge.
- **The Dravidian family thread**, new in this chapter and the counterpart of the
  Romance one: *-ukku / -ku / -ge / -ikku* across Tamil, Telugu, Kannada and
  Malayalam are visibly the **same suffix**, and all four languages build "I know
  X" the same subjectless way.
- Taxonomy: namespaced `TA-CASE-DATIVE`, `TA-DATIVE-SUBJECT`.

## Chapters 3–5 — How-are-you, Farewells, First Verbs

- Three new chapters carry Tamil to Chapter 5, matching the leading tracks'
  greet→introduce→how-are-you→farewell→verbs arc. One word per lesson, atom-first,
  Tamil script inline; every root traced (`lessons/TA-C0{3,4,5}-*`,
  `book/chapters/ch0{3,4,5}-*.tex`). Concept tags reuse the universal `HL01`
  taxonomy; verbs namespaced (`TA-VERB-*`). The native-Dravidian-vs-Sanskrit
  thread runs throughout.
- **Ch. 3 — How Are You**: *eppaḍi* (how; the native *e-* question family) →
  *nīṅgaḷ eppaḍi irukkiṟīrgaḷ?* (the verb *iru* "to be" — the copula returns for
  states, where Ch.2's zero-copula couldn't reach) → *nāṉ* (I ← Proto-Dravidian,
  unrelated to *me*) → *nalam* (well ← *nal-* "good," the root of *naṉṟi*) →
  *paravāyillai* ("no harm" = you're welcome; the *iru*/*illai* pair) → practice.
- **Ch. 4 — Farewells**: *pō*/*vā* → *pōy varugiṟēṉ* ("I'll go and come back" —
  the Dravidian promise-of-return goodbye, tabled across Kannada/Telugu/Malayalam
  and the Indo-Aryan tracks) → *nāḷai pārkkalām* (see you tomorrow) → *mīṇḍum
  sandippōm* (we'll meet again; native *mīṇḍum* + Sanskrit *sandi* ← *sandhi*) →
  practice.
- **Ch. 5 — First Verbs**: *pēsu* (stem + tense + person) → *nāṉ tamiḻ pēsugiṟēṉ*
  (I speak Tamil; the signature retroflex *ḻ*; no gender in the 1st person,
  unlike Hindi) → *vāḻ* (to live/flourish) → *vēlai sey* (to work; noun + *sey*,
  the twin of Hindi's *karnā*) → practice. Book compiles clean with XeLaTeX
  (0 missing chars, 0 undefined refs).

## Chapter 2 — Introducing Yourself

- New chapter around the introduction dialogue (*eṉ peyar … / uṅgaḷ peyar
  eṉṉa?*), atom-first, Tamil script inline (`lessons/TA-C02-*`,
  `book/chapters/ch02-introductions.tex`). Every atom is **native Dravidian**
  and traced:
  - **பெயர்** peyar ("name") ← Proto-Dravidian *\*peyar* — pointedly **not** the
    Indo-European *name/nām*; the family fault-line made visible (cognate box
    across all four Dravidian tongues vs. Hindi/English).
  - **என்** eṉ ("my") ← *nāṉ* ("I"); no European cousin.
  - **என் பெயர் …** — **"my name is…"**; introduces the **zero copula** (Tamil
    has no word for "is" in an equational sentence).
  - **நீ / நீங்கள்** nī/nīṅgaḷ — "you," familiar/respectful; respect by the
    plural (the same mechanism as French *vous*).
  - **என்ன** eṉṉa ("what") ← Dravidian question-stem *\*yā-/\*e-*.
  - **உங்கள் பெயர் என்ன?** — **"what's your name?"** (still no "is").
  - **மகிழ்ச்சி** magiḻcci — "pleased to meet you" ("joy"); the rare ழ (*ḻ*).
  - **practice** — the whole dialogue.
- Book compiles clean with XeLaTeX.

## Chapter 1 — Greetings (Tamil script taught inline)

- New Tamil track on the HL00 framework — the **anchor** of the four Dravidian
  tracks. One word per lesson, slug ids, atom-first, derivations shown, LaTeX
  book. Uses the **vendored** Noto Sans Tamil font (loaded by relative `Path=`
  so local and CI builds match).
- **No reading course.** Per `HL00`'s inline-letters rule, Tamil is taught
  *inside* each word lesson: a *"The letters in this word"* section introduces
  exactly the letters that word needs, so reading and meaning arrive together.
  A Tamil script reference page is included in the book as a lookup, explicitly
  not a gated pre-course.
- Chapter 1 (`lessons/TA-C01-*`), greetings + conversational glue:
  - **வணக்கம்** vaṇakkam ("hello," from the native verb *vaṇaṅku*, "to bow") —
    teaches the inherent *a*, the puḷḷi (vowel-killing dot), the retroflex ண.
  - **நன்றி** naṉṟi ("thanks," literally "goodness," from *nal*) — introduces
    Tamil's three-way dental/alveolar/retroflex *n* (and the parallel *l*, *r*
    sets).
  - **ஆம்** ām ("yes") — independent word-initial vowels; verb-echo "yes."
  - **இல்லை** illai ("no / there isn't") — negation carried by a negative verb
    of existence, a deeply Dravidian habit.
  - **சரி** sari ("okay") — one Tamil letter standing for several stop-sounds
    (voicing read from position).
  - **practice** — recap + the *pōy varugiṟēṉ* / *pōy vā* farewell ("go and
    come back," never a bare "I'm leaving").
- The recurring thread: **Tamil's native word-stock vs. its sisters' Sanskrit
  borrowing** — each lesson carries an "Across the family" cognate box
  (English / Sanskrit / Hindi / Kannada / Telugu / Malayalam), every form
  supplied so nothing is assumed. Grounds against English + the Dravidian
  family + Sanskrit. Book compiles clean with XeLaTeX.
