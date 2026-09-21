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

