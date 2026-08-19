# Changelog

## Unreleased -- Chapters 60-66: a third thirty-five

Malayalam stood at 119 headwords against the 300 the pre-A1 vocabulary floor asks
for -- tied with Tamil for the lowest of the twenty-two tracks, a second time.
These seven chapters answer with thirty-five more on the same terms as the two
tranches before them: **one new word per lesson**, and reuse of everything already
taught, unlimited and deliberate.

  60 Animals Around a House   പശു ആട് കോഴി കാക്ക ആന
  61 On the Kitchen Shelf     തേങ്ങ എണ്ണ തൈര് ശർക്കര മുളക്
  62 Made by Hand             കയർ നൂല് സൂചി ചൂല് മുറം
  63 How the Body Reports     വിശപ്പ് ക്ഷീണം വേദന ചുമ പനി
  64 Five Words That Join     പിന്നെ ഉടനെ ചിലപ്പോൾ മാത്രം എന്നാൽ
  65 Asking Well              അപേക്ഷ അനുവാദം സമ്മതം മര്യാദ വിശ്വാസം
  66 Out in the Paddy         നെല്ല് പുല്ല് കതിര് കലപ്പ കൊയ്ത്ത്

The vocabulary criterion moves 119 to 154 of 300 -- a shortfall of 181 falling to
146, by exactly the thirty-five lessons written and not one more. Each of the seven
pre-A1 spine nodes carries one chapter, all seven used a third time; a node is a
thing you can do rather than a slot that fills up, so a third pass over it is the
design working.

Chaining is unbroken. Chapter 60's first lesson picks up from കനൽ, each lesson
chains to the one before it, each chapter's second lesson still practises the
previous chapter's final word, and each fifth lesson is the payoff that says all
five. That chaining is why the ramp got *gentler* again: Malayalam's own R1
reinforcement ratio falls 0.3444 to 0.3007, its numerator unmoved at 83 while the
denominator grew 241 to 276. Whole-corpus R1 falls 0.2823 to 0.2800 with its
numerator likewise unmoved at 1181. Not one of the thirty-five new atoms misses its
R1 window, and `atomsNeverRevisited` holds at 51.

**Nine candidate words were written and then discarded before these thirty-five
survived.** Three fell to the never-re-teach check reading inside existing
headwords rather than only at whole words: കുട "umbrella" sits whole inside both
കുട്ടി and കുടുംബം, വിള "crop" sits whole inside വിളക്ക്, and വല "net" sits whole
inside വലിയ. One fell to a gloss that already exists: ഉറങ്ങൂ is glossed "sleep" in
the imperative chapter, so ഉറക്കം would have re-taught the word rather than added
one. Four fell to the taught-glyph filter -- ദാഹം "thirst" needs ഹ, ആരോഗ്യം
"health" and വേഗം "quickly" need ഗ, and കൃഷി "cultivation" needs ൃ, none of which
Malayalam's writing lessons teach.

The ninth is the interesting one, and it is the class no string search reaches: a
**gloss of a morpheme inside somebody else's etymology note**. മീൻ "a fish" scans
clean against every headword, every romanization and every lesson body -- but
`ML-C53-star` explains that the inherited Dravidian way of naming a star was to
call it a fish in the sky, and glosses the Tamil compound *viṇmīn* on the way past.
The morpheme *mīn* has already been handed to the reader with its meaning attached.
ആന "an elephant" took that slot instead.

**Ten more candidates were examined under the same rule and KEPT**, because a rule
that only ever rejects is a filter rather than a rule. In each of these the
colliding gloss is of an etymon, an English analogy or a grammatical label, never of
a Malayalam headword:

  - കലപ്പ "a plough" -- `ML-C48-farmer` glosses the SANSKRIT root *karṣ-* as "the
    plough pulled through the ground". An etymon, not a Malayalam word. Kept, and
    the new lesson names the connection outright.
  - ആന "an elephant" -- `ML-C54-branch` lists "the tusk of an elephant" among the
    senses of കൊമ്പ്. That is a sense-list of an already-taught headword. Kept, and
    made the chapter's payoff so the tusk-word finally gets its animal.
  - ആട് "a goat" -- `ML-C36-kutti` glosses the ENGLISH word "kid" as covering a
    child and a young goat. An English analogy for കുട്ടി. Kept.
  - തേങ്ങ "a coconut" -- `ML-C15` glosses ഇളനീർ "tender coconut water", a compound
    of ഇള and നീർ; neither morpheme is തേങ്ങ. Kept.
  - മുളക് "a chilli" -- `ML-C56-basket` mentions "a load of pepper to market" in
    English, with no Malayalam word attached. Kept.
  - അനുവാദം "permission" -- `ML-C32-kaanuka` uses "permission" as a label for one
    of the moods a Malayalam ending can carry. Kept.
  - അപേക്ഷ "a request" -- `ML-C08-dayavayi` uses "respectful request form" to name
    an ending, not a noun. Kept.
  - കൊയ്ത്ത് "the harvest" -- `ML-C52-flower` names the harvest festival in English
    while glossing പൂക്കളം. Kept, and the new lesson points back at it.
  - നെല്ല് "unhusked rice" -- `ML-C55-field` glosses വയൽ "a paddy field". Same
    English word, different referent: the field against the grain standing in it.
    Kept, and the lesson sets നെല്ല്, അരി and ചോറ് side by side as the three stages
    of one plant.
  - വിശപ്പ് "hunger" -- `ML-C06-dative-subject` lists "being hungry" among the
    things that arrive at a person. English prose, no Malayalam word. Kept.

Two romanization near-collisions were kept and turned into teaching rather than
avoided: *āṭŭ* sits inside *nāṭŭ* and *nellŭ* contains *ellŭ*, so the goat lesson
and the paddy lesson each say so and send the ear to the front of the word.

Every one of the thirty-five headwords, and every Malayalam citation in their
bodies, is spelled from the forty-nine characters Malayalam's writing lessons
teach. `scriptClosureViolations` holds at 43 and `exposureExemptedGlyphs` at 122:
the tranche adds zero to both. `forwardReferences` holds at 20, the rule-statement
count at its ceiling, and cross-chapter prose references at 46 -- the tranche names
no chapter by number at all. Cousin words are cited in romanization rather than in
Tamil or Kannada script, keeping the tranche's script surface to one writing system.

HL-C201: `ML-C59-ember` needed no rewording. Its "the run is closed" names that
chapter's five words rather than the book, and stays true with seven chapters
appended after it.

A whole-tree sweep (HL-C202/C203/C208) over all 460 files in the track -- reading
file bytes, classifying by Unicode BLOCK RANGE rather than by `unicodedata.name`
(which raises on unassigned codepoints) and rather than by `\w` (which drops
combining marks), with every fixture built from `chr()` in a source file asserted
pure ASCII -- reports zero mixed-script words, zero NUL, zero ZWJ/ZWNJ, zero bidi
controls, zero BOM, zero soft hyphens and zero replacement characters. The scanner
was self-tested against fifteen known-dirty and eleven known-clean controls, and
then run against the PRE-FIX bytes of the three files this changelog already
records as defective; it rediscovered all three before the clean result was
believed. The chillu letters remain atomic U+0D7B-U+0D7E throughout -- twenty-one
of them across the new lessons, not one spelled with a joiner.

All twenty-two books rebuild at exit 0 with zero missing characters, zero overfull
and zero underfull boxes. Malayalam's own book is now 401 pages.

## Unreleased -- Chapters 53-59: a second thirty-five, and three mixed-script repairs

Malayalam stood at 84 headwords against the 300 the pre-A1 vocabulary floor asks
for -- tied with Tamil for the lowest of the twenty-two tracks. These seven
chapters answer with thirty-five more, on the same terms as the first tranche:
**one new word per lesson**, and reuse of everything already taught, unlimited
and deliberate.

  53 Overhead                  ആകാശം സൂര്യൻ ചന്ദ്രൻ നക്ഷത്രം വെയിൽ
  54 A Tree, Part by Part      മരം കൊമ്പ് തടി വേര് വിത്ത്
  55 Ground Underfoot          പുഴ പാറ നാട് വഴി വയൽ
  56 Things About the House    പായ കൊട്ട കത്തി പാത്രം പെട്ടി
  57 Five More of the Body     കഴുത്ത് മുതുക് ചുണ്ട് നഖം എല്ല്
  58 Five More Short Answers   കൂടുതൽ കുറവ് കുറച്ച് വേണ്ട പോരും
  59 What the Air Carries      കാറ്റ് മണൽ ചെളി പുക കനൽ

The vocabulary criterion moves 84 to 119 of 300 -- a shortfall of 216 falling to
181, by exactly the thirty-five lessons written and not one more. Each of the
seven pre-A1 spine nodes carries one chapter, all seven used a second time;
reusing them is the point, since a node is a thing you can do rather than a slot
that fills up.

Chaining is unbroken. Chapter 53's first lesson picks up from മാല, each lesson
chains to the one before it, each chapter's second lesson still practises the
previous chapter's final word, and each fifth lesson is the payoff that says all
five. That chaining is why the ramp got *gentler* again rather than steeper: the
R1 reinforcement ratio falls 0.2780 to 0.2757, its numerator unmoved at 1123
while the corpus grew. Not one of the thirty-five new atoms misses its R1 window.

**Nineteen candidate words were written and then discarded before these
thirty-five survived.** Thirteen fell to the never-re-teach check, which reads
both directions and looks inside existing headwords rather than only at whole
words: മല "hill" sits whole inside മലയാളം, തീ "fire" inside തീർച്ചയായും,
ഇല "leaf" inside ഇല്ല, കുട്ട "basket" inside കുട്ടി, മഞ്ഞ് "mist" over the
colour-word മഞ്ഞ, and പുറം "back" was simply already glossed in the
setting-out lesson. Teaching any of them here would have made an earlier page
point forward, which is the failure the forward-reference count exists to catch;
that count holds at 508 and the rule-statement count holds at 30.

The other six fell to a **taught-glyph filter**. Malayalam's writing lessons
teach forty-nine characters, and a headword spelled with anything outside that
set either breaks script closure or gets laundered through the romanization
exemption. മേഘം "cloud" needs ഘ, ഗ്രാമം "village" and അഗ്നി "fire" need ഗ,
ഓല needs ഓ -- so the sky chapter ends on വെയിൽ instead, the village chapter on
നാട്, and the fire chapter on കനൽ. Every one of the thirty-five headwords and
every Malayalam word in their bodies is spelled from characters the reader has
already been taught, so script-closure violations hold at 756 and
exposure-exempted glyphs at 2220: the tranche adds zero to both.

Cousin words are cited in romanization throughout rather than in Tamil or
Kannada script, which keeps the tranche's script surface to one writing system.

### Three mixed-script repairs (HL-C202)

Three words in the committed track were each spelled across two scripts, so the
renderer emitted them over two font commands mid-word and they printed as
plausible wrong text while the build exited 0 with no missing characters:

  - `ML-C11-nirangal.md` -- Tamil நீல was carrying U+0D32 MALAYALAM LETTER LA
    in place of U+0BB2 TAMIL LETTER LA.
  - `ML-C37-mookku.md` -- Kannada ಮೂಗು was carrying U+0D41 MALAYALAM VOWEL
    SIGN U in place of U+0CC1 KANNADA VOWEL SIGN U.
  - `roadmap.md` -- Tamil எழுது was carrying U+0D41 MALAYALAM VOWEL SIGN U in
    place of U+0BC1 TAMIL VOWEL SIGN U.

One codepoint changed in each; the corrected spelling was taken from an
occurrence already committed elsewhere in the corpus rather than composed. The
four narration files that quote these lessons cleared on regeneration. A
whole-tree sweep -- reading file bytes, walking Unicode categories L/Mn/Mc/Me
rather than `\w`, and self-tested against twelve known-dirty and eleven
known-clean fixtures built from `chr()` codepoints -- now reports zero mixed-script
words, zero NUL, zero ZWJ/ZWNJ, zero bidi overrides or isolates, zero BOM and
zero soft hyphens across the whole Malayalam track. The chillu letters remain
atomic U+0D7B-U+0D7E throughout.

## Unreleased — Chapters 46-52: Thirty-five everyday words, one per lesson

Malayalam stood at 49 headwords against the 300 the pre-A1 vocabulary floor asks
for. These seven chapters answer with thirty-five, and with nothing else — **one
new word per lesson**, and reuse of everything already taught, unlimited and on
purpose.

  46 Things You Ask For        പഴം തുണി വിളക്ക് ഉപ്പ് പുസ്തകം
  47 The Leg and the Tooth     കാൽ പല്ല് മുടി വിരൽ വയറ്
  48 Who Someone Is            അധ്യാപകൻ വിദ്യാർത്ഥി വൈദ്യൻ കർഷകൻ അതിഥി
  49 Short Replies             സത്യം മതി തീർച്ചയായും ഒരുപക്ഷേ അങ്ങനെ
  50 Taking Leave              ഇപ്പോൾ മറ്റന്നാൾ യാത്ര പുറപ്പെടുക വിട
  51 Courtesy                  കൃതജ്ഞത ഉപകാരം ബഹുമാനം അനുഗ്രഹം വന്ദനം
  52 Welcoming a Guest         വാതിൽ കസേര കോലം പൂവ് മാല

Each chapter sits on one of the seven pre-A1 spine nodes, five lessons sharing
it, and each lesson chains to the one before — so a word introduced by a
chapter's payoff lesson is still being practised two lessons into the next
chapter. That is why the ramp got *gentler* rather than steeper: the R1
reinforcement ratio falls 0.3034 to 0.3005 even though the corpus grew. Not one
of the thirty-five new atoms misses its R1 window.

Every headword was checked against all 142 existing lessons before it was
written, in both directions, so none of the thirty-five re-teaches anything and
none of them makes an earlier page point forward — the corpus forward-reference
figure holds at its 500 ceiling, and so does the thirty-count on rule statements.

Two threads run the length of the seven chapters rather than sitting in one
lesson. The first is the chillu, the consonant written without a vowel and given
a letter of its own: ൽ closes കാൽ, വിരൽ, വാതിൽ and ജനൽ; ൻ marks the three
masculine role-words; ൾ closes ഇപ്പോൾ and മറ്റന്നാൾ; ർ sits inside വിദ്യാർത്ഥി,
കർഷകൻ and തീർച്ചയായും. Each is written as its own atomic character rather than as
a consonant plus virama plus a zero-width joiner, because U+200D belongs to no
Unicode script block at all and so falls through every font selection the book
makes — a trap the Kannada tranche fell into and this one is written to avoid.

The second is the standing division of labour between an inherited word and a
borrowed one. തുണി against വസ്ത്രം, മുടി against രോമം, നന്ദി against കൃതജ്ഞത,
വിളക്ക് against the Sanskrit lamp Kannada took instead: the native word does the
daily work and the borrowed one does the ceremony, again and again, until it
stops being a fact about five words and becomes a fact about the language.

Chapter 49 spends its last lesson collecting on a debt from the chapter on
pointing: അങ്ങനെ is എങ്ങനെ with the far-pointer swapped in for the question one,
so the reader who saw i- / a- / e- once gets the fifth reply for nothing. Chapter
52 does the same with വായ്, the mouth-word from the chapter on the face, which
Tamil's door-word வாயில் shows sitting inside വാതിൽ.

Chapters 43, 44 and 45 also move from a bare `unicodeScript` to the track's
`malayalam-comparisons` script set, matching chapters 6-42 (backlog HL-C200).
Their rendered output is byte-identical and their book hashes did not move,
because they happen to cite no cousin script today; what changes is that a
comparison table added to them later can no longer drop its glyphs silently into
the Latin font at exit 0.

## Unreleased — Chapter 41: Pointing, and Asking

Six words and the pattern behind them: ഇത് അത് ഇവിടെ അവിടെ ആര് എവിടെ

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

## Unreleased — the first 8 characters this book actually teaches

8 recognition segments, one character each, in chapters 6-13: ക ◌് ◌ി ◌ാ ന ൾ ◌ു ത

Until now this track taught **no letters at all**. Every word was printed in its
own script and the reader had no way in — HL12's measurement put the track at A2
by aspiration and pre-A1 by attainment, with the script strand simply missing.

Each segment names one character, says what it carries, and shows it inside four
words the reader **already says** — so nothing new has to be learned in order to
do the recognising. That is HL12 §2.1's rule made concrete: a lesson may sit at
the frontier of decoding or of meaning, never both, because a reader who fails
one that is new in both cannot tell which one they failed.

They teach recognition and not writing, and that is a sourcing fact rather than a
pedagogical preference. This script has **no cited stroke order** in the corpus —
its own script file says *"Recognition only"* — and HL11 §5 forbids a pen path
without one, because a learner cannot tell an invented stroke order from an
attested one and will drill it for years. So the reader is asked to trace the
printed shape, which needs no source, and the book says plainly that where to
start the character and which way to travel are not written down yet.

Each segment sits **last** in its chapter, after every word in that chapter that
contains its character — so it consolidates rather than pre-teaches, and it costs
the driving edition nothing: `drivablePrefixTotal` is unchanged corpus-wide.

## Unreleased — 24 words a reader can now say

Added `romanization` to 24 lessons that had none, so their headwords become
HL11 *exposure* — something the reader is shown and can use — rather than script
they are stuck on. Each is recovered from the pronunciation the lesson already
gives in its own prose, then checked against the headword's script so a wrong
grab cannot pass. Nothing is transliterated: a mechanical romanization of this
script disagrees with its own authors often enough to teach mispronunciations.

## Chapters 35-40 — Vocabulary wave 5: family, face, hearts, and drinks (2026-08-08)

- Added fifteen schema-v2 word lessons in six new chapters, closing the HL09
  §3.1 pre-A1 gate's **reinforcement** blocker outright and narrowing (not
  closing — this is one tranche of an ongoing program) the **vocabulary**
  blocker. Malayalam's **spine-nodes** criterion was already satisfied before
  this tranche — all seven pre-A1 spine nodes were already realized through
  existing content — so this wave went straight to vocabulary depth and
  reinforcement, per HL09's measured, not assumed, gate report.
- **Ch. 35 (Family and Friend)**: കുടുംബം *kuṭumbaṁ* (family, the Sanskrit
  collective word Chapter 12's six people-words never supplied), സുഹൃത്ത്
  *suhṛttŭ* (friend, Sanskrit *su* + *hṛd* "good-heart" — foreshadowing
  Chapter 38's own heart-word, with the everyday spoken alternative
  കൂട്ടുകാരൻ/കൂട്ടുകാരി named too).
- **Ch. 36 (Child, Son, Daughter)**: കുട്ടി *kuṭṭi* (child/kid, matching
  Tamil's *kuṭṭi*), മകൻ *makan* (son, Proto-Dravidian, matching Tamil almost
  exactly where Kannada lenited *-k-* to *-g-* and dropped the final *-n*),
  മകൾ *makaḷ* (daughter, **identical** to Tamil's own word, sound for sound
  — the closest match in the whole tranche).
- **Ch. 37 (Face Words)**: കണ്ണ് *kaṇṇŭ* (eye, near-identical across all four
  literary Dravidian languages), ചെവി *cevi* (ear, Proto-Dravidian *\*kewi*
  — Malayalam, Tamil and Telugu all palatalized the initial *k-*; Kannada
  alone kept it, ಕಿವಿ *kivi*), മൂക്ക് *mūkkŭ* (nose, matching Tamil and
  Telugu, with Kannada's usual *-k-*/*-g-* softening), വായ് *vāy* (mouth,
  DEDR 5352 — the word Chapter 33 already named once, only to rule out that
  വായിക്കുക, "to read," is built on it).
- **Ch. 38 (The Two Hearts)**: ഹൃദയം *hṛdayaṁ* (heart, Sanskrit तत्सम, and a
  genuine — if very distant — Indo-European cousin of English *heart*, via
  the same root as Greek *kardía* and Latin *cor*), നെഞ്ച് *nenchŭ* (chest,
  native Dravidian, cognate with Tamil's நெஞ்சு/நெஞ்சம் — the word idiom
  actually reaches for, e.g. നെഞ്ചിടിപ്പ് "chest-beat").
- **Ch. 39 (Tea, Coffee, Milk)**: ചായ *chāya* (tea, Chinese *chá* carried
  overland through Persian and Hindi-Urdu — the land route), കാപ്പി *kāppi*
  (coffee, from English, itself from Arabic *qahwah* via Turkish and Italian
  — the sea route; tea and coffee arrive at the same tea-shop counter from
  opposite directions), പാൽ *pāl* (milk, native, matching Tamil **exactly**
  and Kannada by the same *p*-to-*h* law documented at *pōkuka*/*hōgu* — the
  mirror image of Chapter 15's വെള്ളം, which broke from Tamil's water-word
  entirely).
- **Ch. 40 (The Meal)** — the tranche's payoff: ഊണ് *ūṇ* (a meal, native
  Dravidian *\*uHṇ-*, the noun behind ഉണ്ണുക *uṇṇuka*, a **second eat-verb**
  this track had never taught, standing beside Chapter 32's everyday
  തിന്നുക; Tamil's cognate உணவு *uṇavu* generalised to "food," Kannada's ഊട
  *ūṭa* narrowed to "a meal" like Malayalam's own ഊണ്). Closes the
  SPINE-POLITE-REQUEST-REPAIR arc: ദയവായി (Ch. 8) asks, ക്ഷമിക്കണം (Ch. 9)
  repairs, and both are folded back in alongside Chapter 6's dative and
  Chapter 34's *iṣṭamāṇŭ* frame.
- **Every etymological claim was checked, and several assumptions going in
  were corrected against the corpus and general Dravidological knowledge
  rather than taken on trust.**
  - ചെവി/ಕಿವಿ are **cognate**, both from Proto-Dravidian *\*kewi* — not two
    unrelated roots. Malayalam, Tamil and Telugu palatalized the inherited
    *k-* before this front vowel; Kannada alone kept it. The lesson names
    this as a regular correspondence on this one word, not a systemic sound
    law across the whole lexicon (the family's usual subgrouping does not
    cleanly predict it).
  - കുടുംബം turned out to be a **genuine gap**: Chapter 12's own id is
    `kudumbam`, but its actual headword is the six people-words
    (അച്ഛൻ/അമ്മ/ചേട്ടൻ/അനിയൻ/ചേച്ചി/അനിയത്തി) — the word "family" itself,
    കുടുംബം, had never been taught. This mirrors Kannada's own
    KA-C35-kutumba finding exactly, and the parallel is real: both
    languages nativize the same Sanskrit loan, but Malayalam adds its own
    **‑അം** neuter-noun ending (as it already does on ഇഷ്ടം, സുഖം) where
    Kannada borrows the bare Sanskrit stem.
  - The bare Chinese character 茶 and the bare Greek **καρδία** were
    **removed** from lesson prose after they produced `Missing character`
    warnings against Latin Modern Roman during the book build — Kannada's
    own chai and heart lessons already establish the precedent of citing
    such forms by romanization only (*chá*, *kardía*), never in their native
    script, and this tranche now follows it.
  - A drafting mistake, not a factual one: several Sanskrit vocalic-*r*
    spellings (*hṛd*, *suhṛttŭ*, *kṛpā*-family words) were first typed as
    the **decomposed** sequence `r` + U+0325 (combining ring below) instead
    of the **precomposed** `ṛ` (U+1E5B) the rest of the corpus uses
    throughout. The decomposed form is invisible in an editor but has no
    glyph in Latin Modern Roman, and produced eight `Missing character`
    warnings in the first build. Fixed corpus-wide across all fifteen
    lessons; the book now compiles with zero missing characters.
- **Reinforcement discipline.** Every lesson's `practises.knowledge` reaches
  back through the two immediately preceding lessons (closing the R1/R2
  windows for every new atom this tranche introduces — measured, not just
  intended: the pre-A1 thin-atom count is confirmed at **zero** after this
  tranche), and specific lessons reach further back to rescue the sixteen
  pre-A1 atoms the HL09 gate report had flagged as under-reinforced before
  this wave: ML-C35-kudumbam and ML-C36-kutti rescue Chapter 12's
  കുടുംബം-atoms; ML-C36-makal rescues Chapter 19's വയസ്സ്; ML-C37-kannu and
  ML-C38-nenchu rescue Chapter 13's ശരീരഭാഗങ്ങൾ; ML-C39-chaaya rescues
  Chapter 6's dative atoms and Chapter 9's ക്ഷമിക്കണം atoms (repeated again
  in ML-C40-oon); ML-C39-paal rescues Chapter 15's വെള്ളം/അരി atoms.
  Malayalam's pre-A1 reinforcement blocker — 16 atoms revisited fewer than
  twice — is fully closed by this tranche.
- **Atom budget.** Each of the six new chapters introduces at most 8 new
  atoms (Chapter 37's four lessons; every other new chapter introduces 3-6),
  against the 12-atom-per-chapter ceiling, and no lesson introduces more
  than 3. Zero atom-budget violations, corpus-wide, after this tranche.
- **Vocabulary.** Malayalam's distinct-headword count rises from **69 to
  84** overall, and from **34 to 49** at or below pre-A1 — fifteen new
  one-headword lessons moving the count by exactly fifteen, per
  `vocabularyOf()`'s 1:1 lesson-to-headword accounting. The pre-A1 shortfall
  against the 300-word target narrows from 266 to 251; closing it fully
  will take further tranches, consistent with every prior wave in this
  program.
- Wiring: `ML-PATH-029` through `ML-PATH-034` and six new
  `ML-EXT-029`.. `ML-EXT-034` extensions in
  [`curriculum.json`](./curriculum.json) — three attached to
  `SPINE-EXCHANGE-NAMES`, two to `SPINE-CHECK-WELLBEING`, two to
  `SPINE-POLITE-REQUEST-REPAIR` — six new chapter-capability entries in
  [`chapters.json`](./chapters.json), six new `core/book-generation.json`
  targets, the generated `book/chapters/ch35..ch40-*.tex`, six new `\input`
  lines in `book.tex`, and generated narration for all fifteen lessons.
- The 166-page XeLaTeX build has **zero missing characters** and zero
  errors (two minor underfull-hbox warnings on the Chapter 38 heading,
  consistent with the small number of pre-existing underfull boxes already
  tracked elsewhere in the track).
- `npm run check:modality`, `check:books` and `check:narration` are all
  clean. The six corpus-wide snapshot tests
  (`tests/{chapters,continuity,levels,modality-manifest,narration,ramp}.test.ts`)
  pin exact whole-corpus totals from before this wave and are **expected**
  to fail until the orchestrating merge re-measures across all of wave 5;
  every other test, including `tests/integration.test.ts` and
  `tests/cli.test.ts`, passes.

## Chapters 33-34 — The eight verbs (2026-08-07)

- Added eight schema-v2 lessons carrying the eight canonical concepts fifteen
  other tracks already teach — `VERB-THINK`, `VERB-UNDERSTAND`, `VERB-READ`,
  `VERB-WRITE`, `VERB-TAKE`, `VERB-ASK`, `VERB-HELP`, `VERB-LIKE-LOVE` — making
  Malayalam the sixteenth track to hold all of them, and the fourth Dravidian
  contributor after Tamil, Kannada and Telugu.
- **Two chapters of four, never one of eight.** Chapter 33 introduces 9 atoms
  and Chapter 34 introduces 9, both under the `maxNewAtomsPerChapter` budget of
  12 that one chapter of eight would have blown. Each carries its own `canDo`
  and its own payoff, and both payoffs assess **every** atom their chapter
  introduces — 9/9 and 9/9, representativeness **1.00** against the 0.5 floor.
- **Ch. 33 (The Mind and the Page)**: ചിന്തിക്കുക, മനസ്സിലാക്കുക, വായിക്കുക,
  എഴുതുക. Its spine is a diagnostic the learner can hear: the **ഇ** of
  **‑ഇക്കുക**, and the **‑ഇച്ചു** past that goes with it, mark a Sanskrit
  borrowing. മനസ്സിലാക്കുക comes apart into *manassŭ* + locative *‑il* +
  *ākkuka* — "to put in the mind" — and its intransitive twin **എനിക്ക്
  മനസ്സിലായി** puts the understander in the dative, joining Ch. 6's *enikku
  malayāḷaṁ aṟiyāṁ*. എഴുതുക is Tamil **எழுது** unchanged, carrying the **ഴ**
  that Kannada and Telugu lost — and with it the verb, which is why they write
  with the line-drawing root Malayalam kept for വരയ്ക്കുക, "to draw."
- **Ch. 34 (Taking, Asking, Helping, Liking)**: എടുക്കുക, ചോദിക്കുക,
  സഹായിക്കുക, എനിക്ക് മലയാളം ഇഷ്ടമാണ്. എടുക്കുക corrects Ch. 32's rule rather
  than breaking it — the stamp is the **ഇ**, not the doubled **ക്ക** a native
  stem carries by itself. ചോദിക്കുക is Sanskrit *cud*, "to urge," and the
  reason a loan was needed at all is structural: Proto-Dravidian *\*kēḷ‑*
  covered hearing **and** asking, Tamil's கேள் still does both, and Malayalam
  narrowed കേൾക്കുക to hearing alone. The same shape recurs four times across
  the two chapters — ഓതുക, ഓർക്കുക, കേൾക്കുക, ഉതവി each gave up an everyday
  slot to a Sanskrit word — which is the Maṇipravāḷam era's lexical footprint.
- **Every etymology was checked against sources rather than taken on trust, and
  four briefed or inherited claims were corrected in the process.**
  - വായിക്കുക is **not** from വായ് "mouth". Gundert marks വായന a *tadbhava* of
    Sanskrit *vac* and gives the Tamil verb as *vācikka*; DEDR 5352 (*vāy*
    "mouth") carries no reading sense in any Dravidian language. The mouth story
    is folk etymology, and the lesson says so.
  - The *c* → *y* is **not** a Malayalam sound law. It happened in the middle
    Indo-Aryan stage the *tadbhava* passed through; Malayalam holds the same
    Sanskrit word borrowed straight as വാചകം, so the doublet is visible in the
    language itself.
  - എഴുതുക is **not** filed under എഴു "to rise". Burrow–Emeneau keep them as
    separate entries, so the lesson names the link and marks it unproven — while
    എടുക്കുക genuinely *is* filed inside the rise-family, which is where that
    fact belongs.
  - English *mind* descends from *\*ménti‑*, not from the *\*ménos* that gave
    Sanskrit मनस्; the lesson claims root-level cognacy only, and names मति and
    Greek *ménos* as the exact matches.
  Two further claims are hedged rather than asserted: Monier-Williams marks the
  *saha* + *aya* reading of सहाय "probable", and whether Kannada/Telugu *ettu*
  is the cognate of എടു is a point Gundert and DEDR disagree on.
- **Reinforced at two cadences.** Every lesson's `practises.knowledge` names
  atoms from the immediately preceding one to three lessons, across the chapter
  seam; each payoff reaches several chapters back. Malayalam's never-revisited
  atoms fall from **72 of 78 (92%)** to **46 of 96 (48%)** — 29 previously
  orphaned atoms rescued, spanning Chapters 6, 7, 8, 9, 10, 13, 15, 16, 18, 19,
  20, 24, 26 and 32. The three that remain from this tranche belong to the
  final lesson of the track, which no later lesson exists to retrieve.
- All eight use the canonical `## The letters in this word` heading. That block
  classifies as `script`, which is **detachable**, so every lesson derives
  `modality: sight` with `coreModality: voice` — the driving edition is intact.
  (`core/lesson-modality.json` reports `drivable` from the whole-lesson modality
  rather than the core, so the published manifest understates this; that is a
  known bug in `modality-manifest.ts`, not a property of these lessons.)
- All eight sit under the 300-second effective ceiling (285-299s computed).
- Wiring: `ML-PATH-027`/`ML-PATH-028` and `ML-EXT-027-MIND-VERBS`/
  `ML-EXT-028-DOING-VERBS` in [`curriculum.json`](./curriculum.json), all eight
  concepts dropped from the `SPINE-SAY-WHAT-I-DO` omission ledger (36 omits down
  to 28), Chapter 33 and 34 ledger entries in [`chapters.json`](./chapters.json),
  two `core/book-generation.json` targets, the generated
  `book/chapters/ch33-mind-and-page.tex` and
  `book/chapters/ch34-taking-asking-helping-liking.tex`, `\input` in `book.tex`,
  and generated narration for both chapters.
- The 136-page XeLaTeX build has **zero missing characters** and zero errors.
  Three underfull boxes remain, two of them pre-existing in Chapters 6 and 30;
  the third is the Chapter 34 payoff's section heading, where the Malayalam
  script at 14.4pt forces an awkward break. The track is `null` in
  `core/latex-warning-baseline.json`, so nothing is re-pinned.

## Chapter 32 — The Core Verbs (2026-08-06)

- Added six schema-v2 core-verb lessons, the track's first A2 material and its
  first realization of `SPINE-SAY-WHAT-I-DO`: `ML-C32-undu` (VERB-BE),
  `ML-C32-pokuka` (VERB-GO), `ML-C32-varuka` (VERB-COME), `ML-C32-tinnuka`
  (VERB-EAT), `ML-C32-kaanuka` (VERB-SEE), `ML-C32-ariyuka` (VERB-KNOW). All
  six take canonical spine concept tags, so the track goes from four namespaced
  verb concepts and none canonical to six canonical ones.
- The chapter is built around the fact that makes Malayalam unlike its three
  Dravidian sisters: **its verb carries no person marking at all**. Chapter 5
  observed this for one verb; Chapter 32 turns it into the chapter's spine.
  `ML-C32-undu` sets up the two-slot machine (stem + tense) against Tamil's
  three (stem + tense + person); `ML-C32-pokuka` shows that each tense form is
  therefore the *whole* conjugation; `ML-C32-varuka` locates the entire
  irregularity budget in the past (*varu-* → *vann-*); `ML-C32-kaanuka` shows
  the freed slot spent on mood (*kāṇāṁ*, *kāṇaṇaṁ*, *kāṇarutŭ*); and
  `ML-C32-ariyuka` closes by taking the person out of the subject slot too,
  giving Chapter 6's **എനിക്ക് മലയാളം അറിയാം** the verb it was always built on.
- Two genuinely conservative facts are recorded rather than glossed: Malayalam
  kept **both** of the family's be-verbs (ഉണ്ട് = Telugu ఉండు for existing and
  having, ഇരിക്കുക = Tamil இரு / Kannada ಇರು for being somewhere), and it kept
  the inherited *tiṉ-* and *kāṇ-* as its everyday eat- and see-words where
  Tamil, Kannada and Telugu each moved on. `ML-C32-tinnuka` also names the
  **-ഉക / -ഇക്കുക** split, which marks a native verb off from a Sanskrit
  borrowing turned into one.
- Every non-Malayalam form is supplied in full — no lesson assumes the reader
  knows another target language — and every cognate claim stays inside
  Dravidian, with Sanskrit material flagged as borrowing.
- Wired the chapter through the pipeline: `ML-PATH-026` in
  [`curriculum.json`](./curriculum.json) (dropping the six concepts from
  `SPINE-SAY-WHAT-I-DO`'s `omits`), a Chapter 32 ledger entry in
  [`chapters.json`](./chapters.json), a `core/book-generation.json` target, the
  generated `book/chapters/ch32-core-verbs.tex`, and `\input` in `book.tex`.
- All six lessons are **voice** modality, so Chapter 32 is drivable end to end,
  and all six sit under the 300-second effective ceiling (272–294s computed).
  The 114-page XeLaTeX build has zero missing characters and adds no over- or
  underfull boxes.

## Chapter capability ledger for Chapters 6–31 (2026-08-06)

- Added [`chapters.json`](./chapters.json), the track's HL05 chapter capability
  ledger: one `canDo` promise and one validated payoff for each of Chapters
  6–31. Titles and labels are copied from `core/book-generation.json` so the two
  agree until HL-C04 inverts that dependency; `spineNodes` are derived from
  `curriculum.json`'s path segments; every `payoff.assesses` atom is taken from
  the payoff lesson's own `practises.knowledge`, never invented.
- Derived from the lessons and `curriculum.json` rather than from
  [`roadmap.md`](./roadmap.md) or [`session-map.md`](./session-map.md), which
  still lag the canonical Chapters 6–31 (known debt, HL-M04).
- **Chapters 1–5 are deliberately absent.** Their terminal practice lessons
  (`ML-C01-practice` … `ML-C05-practice`) are still schema v1 and declare no
  `practises.knowledge`, so no payoff can name an atom without fabricating one.
  Those five chapters also have no `book-generation.json` target to copy a title
  from. The gap is recorded in the file's own `note` and stays visible to the
  HL05 gap report rather than being filled with a placeholder.
- No chapter from 6 on ends in a `practice` lesson, so every payoff is that
  chapter's last lesson by `sequence`. Where that terminal lesson is an
  `etymology` lesson (Chapters 23, 24, 31) the payoff is typed `task` and its
  summary describes the sorting the reader actually does.

## Warning-free complete book (2026-08-03)

- Added explicit static bold and italic faces for Malayalam and every
  comparison script, plus bookmark-safe Unicode commands, eliminating all
  font-shape and Hyperref warnings without dropping multilingual examples.
- Made the five handwritten recap labels unique and shortened only the running
  titles that exceeded the text block. Small sentence-level copy-flow repairs
  in the generated family, number, and colour chapters remove the remaining
  horizontal overflows while preserving the canonical teaching sequence.
- Added natural page bottoms for deliberately short micro-lessons and made
  open-right chapter versos truly empty, without a running header or page
  number.
- The forced 107-page build now has zero missing glyphs, overfull or underfull
  boxes, duplicate destinations, Hyperref warnings, LaTeX warnings, or font
  warnings. All 107 pages were rendered and visually inspected.
- The 33 top-level and 97 total outline entries, title and author metadata,
  generated source hashes, and zero schema or generator leaks remain intact.

## Canonical Chapters 6–31 in the book (2026-08-03)

- Migrated all thirty-three Malayalam lessons after Chapter 5 to the strict
  schema-v2 curriculum contract: canonical spine nodes, unique
  prerequisite-safe sequence, explicit sub-five-minute budgets, typed block
  boundaries, and closed knowledge introductions and assessments.
- Generated twenty-six LaTeX chapters from those canonical lessons instead of
  copying app content into a separate book source. The committed source-hash
  manifest is independently checked against Language Ladder for Chapters 6–31.
- Added a reusable Malayalam comparison-font set for Malayalam, Tamil, Telugu,
  Kannada, Devanagari, and Arabic-script examples. The 107-page PDF has zero
  missing glyphs and preserves the full 33-entry top-level chapter outline.
- Rendered and inspected all 107 pages, including dense case, calendar,
  etymology, daypart, and register sections. No teaching content is clipped,
  colliding, accidentally omitted, or replaced by generator metadata.
- The expanded artifact's cleanup baseline is 17 overfull boxes, four
  underfull horizontal boxes, ten underfull vertical boxes, four duplicate
  practice labels, 108 Hyperref warnings, and seven font warnings. `HL-B27`
  tracks those warnings and the running headers on intentionally empty versos.
- The single all-books publication gate still compiles and catalogs all twenty
  downloadable volumes successfully.

## Sub-five-minute lesson remediation (2026-08-02)

- All thirty-seven Malayalam duration violations are resolved. Thirty-three
  lessons already computed below five minutes and now declare an honest
  four-minute budget without changing their teaching content.
- Four long lessons become gentle prerequisite pairs: **ഉച്ച** noon →
  **പാതിരാ** midnight; Sanskrit *divasam/dinam* → native **നാൾ**; Sanskrit
  **രാത്രി** → native *iravŭ/iruḷ*; formal **ശുഭ മധ്യാഹ്നം** → the three-language
  convergence map. The eight steps compute between 141 and 235 seconds.
- The four support lessons bring the Malayalam track to 64 lessons with zero
  unknown prerequisite ids; downstream lessons now require the moved concept
  before using it.
- A forced book build succeeds at 31 pages with no missing glyphs. Canonical
  lessons continue through Chapter 31 while the book stops at Chapter 5
  (`HL-B26`); existing layout, bookmark, duplicate-label, and font warnings are
  tracked in `HL-B27`; roadmap and session-map drift is tracked in `HL-M04`.

## Chapter 6 — Case endings, and the sentence with no subject

- **Chapter 6 authored** (`ML-C06-dative-ikku`, `-dative-subject`): the track's
  first **case ending** — reviewing Ch.2/3/5 via `reviews_of`.
- **-ിക്ക്/-ിന്** (`ML-C06-dative-ikku`): the dative "to/for," taught as the doorway
  to **agglutination**. Malayalam **adds** a suffix carrying **one** meaning with
  the **seam visible** (*jōli* + *kku*), where a Latin ending like *-īs* **fuses**
  case+number+declension inseparably; the two shapes are **one case** chosen by the
  noun's ending. Includes *ñān* → **എനിക്ക്** *enikku*, flagged as worth memorising
  cold — it opens a great many everyday Malayalam sentences.
- **എനിക്ക് മലയാളം അറിയാം** (`ML-C06-dative-subject`): "I know Malayalam" — literally
  "**to-me Malayalam is-knowable**" — *aṟiyām* being *aṟiy-* "know" plus the
  **abilitative** *-ām*, not a passive — with **no nominative "I"** (contrast Ch.5's
  *ñān malayāḷam saṁsārikkunnu*). Explains the **dative-subject** rule with
  English's "**methinks**" as the bridge.
- **The Dravidian family thread**, new in this chapter: *-ikku / -ukku / -ku / -ge*
  are visibly the **same suffix**, with the extra observation that **Malayalam's
  *enikku* and Tamil's *enakku* are nearly the same word** — the two languages
  separated most recently of the four, and it shows.
- Taxonomy: namespaced `ML-CASE-DATIVE`, `ML-DATIVE-SUBJECT`.

## Chapters 3–5 — How-are-you, Farewells, First Verbs

- Three new chapters carry Malayalam to Chapter 5, matching the leading tracks'
  arc. One word per lesson, atom-first, Malayalam script inline; every root traced
  (`lessons/ML-C0{3,4,5}-*`, `book/chapters/ch0{3,4,5}-*.tex`). Concept tags reuse
  the universal `HL01` taxonomy; verbs namespaced (`ML-VERB-*`). Malayalam's
  double character — Tamil's closest sister, yet the deepest in Sanskrit, and the
  only one with a real copula — runs throughout.
- **Ch. 3 — How Are You**: *eṅṅane* (how; the native *e-* questions) → *sukhamāṇō?*
  ("are you well?" — the Ch.2 copula *āṇŭ* + the question particle *-ō*) → *ñān*
  (I ← Proto-Dravidian; **can't be dropped**, since Malayalam verbs don't mark
  person) → *sukham* (well ← Sanskrit *sukha*, the *su-* that is Greek *eu-*) →
  *sāramilla* ("no matter" = you're welcome; Sanskrit *sāraṁ* + native *illa*) →
  practice.
- **Ch. 4 — Farewells**: *pōkuka*/*varika* → *pōyi varāṁ* ("I'll go and come back,"
  tabled across the family) → *nāḷe kāṇāṁ* (see you tomorrow; *nāḷ* "day" + *kāṇ*
  "see" + the "let's" *-āṁ*) → *vīṇḍuṁ kāṇāṁ* (we'll meet again; native *kāṇ*,
  where Tamil borrowed Sanskrit *sandi*) → practice.
- **Ch. 5 — First Verbs**: *saṁsārikkuka* (Sanskrit-derived; native twin
  *paṟayuka*) → *ñān malayāḷaṁ saṁsārikkunnu* (I speak Malayalam; the *-unnu*
  present — **the verb never changes for person**, Malayalam's great
  simplification) → *tāmasikkuka* (to live; postposition *-il*) → *jōli ceyyuka*
  (to work; *ceyyuka* is the *same root* as Tamil *sey*) → practice. Book compiles
  clean with XeLaTeX (0 missing chars, 0 undefined refs).

## Chapter 2 — Introducing Yourself

- New chapter around the introduction dialogue (*enṟe pēru … āṇŭ / ninṟe pēru
  entāṇŭ?*), atom-first, Malayalam inline (`lessons/ML-C02-*`,
  `book/chapters/ch02-introductions.tex`). Every atom traced:
  - **പേര്** pēru ("name") ← Proto-Dravidian *\*pēr* — twin of Tamil *peyar*,
    **not** the Indo-European *name/nām*.
  - **എന്റെ** enṟe ("my") ← *ñāṉ* ("I").
  - **ആണ്** āṇŭ ("is") — Malayalam's **copula**, from the verb *āka*. The
    standout: Tamil/Kannada/Telugu use the **zero copula**, but Malayalam,
    Tamil's closest sister, grammaticalised a "to be" verb.
  - **എന്റെ പേര് … ആണ്** — **"my name is…"**; verb last (unlike Tamil).
  - **നീ / നിങ്ങൾ** nī/niṅṅaḷ — "you," familiar/respectful; respect by plural.
  - **എന്ത്** entŭ ("what") ← Dravidian question-stem *\*yā-/\*e-*.
  - **നിന്റെ പേര് എന്താണ്?** — **"what's your name?"** (*entŭ* + *āṇŭ* fused).
  - **സന്തോഷം** santōṣam — "pleased to meet you," a **Sanskrit** loan (Malayalam
    borrows selectively: native *nandi* for thanks, Sanskrit here).
  - **practice** — the whole dialogue.
- Example names are invented (Mira / Arun). Book compiles clean with XeLaTeX.

## Chapter 1 — Greetings (Malayalam script taught inline)

- New Malayalam track on the HL00 framework — the last of the four Dravidian
  tracks. One word per lesson, slug ids, atom-first, derivations shown, LaTeX
  book. Uses the **vendored** Noto Sans Malayalam font (relative `Path=`, shaped
  via `Script=Malayalam`, no polyglossia language module needed).
- **No reading course.** Per `HL00`'s inline-letters rule, Malayalam is taught
  *inside* each word lesson.
- Chapter 1 (`lessons/ML-C01-*`), greetings + conversational glue:
  - **നമസ്കാരം** namaskāram ("hello," **Sanskrit** namas + kāra) — inherent
    *a*, vowel signs, the chandrakkala, the സ്ക conjunct, anusvāram ം.
  - **നന്ദി** nandi ("thanks," **native**, root *nal*) — the twin of Tamil
    *naṉṟi*; the ന്ദ conjunct.
  - **അതെ** athe ("yes," native, "that [is so]") — yes/no as demonstratives;
    the *e*-sign written before its consonant.
  - **ഇല്ല** illa ("no / isn't," native, root *il*) — the twin of Tamil
    *illai*; negation by a negative existential verb.
  - **ശരി** śari ("okay," native) — the family word *sari* with Sanskrit ശ.
  - **practice** — recap + the *pōyi varām* farewell (nearly the same words as
    Tamil's *pōy varugiṟēṉ*).
- The recurring thread: **Malayalam is Tamil's closest sister** — four of the
  five everyday words are shared with Tamil (nandi, athe, illa, śari) — **with a
  heavy Sanskrit overlay** (namaskāram; the largest alphabet in the family).
  Each lesson carries an "Across the family" cognate box (English / Sanskrit /
  Hindi / Tamil / Kannada / Telugu), every form supplied so nothing is assumed.
  Book compiles clean with XeLaTeX. Completes the four Dravidian first chapters.
