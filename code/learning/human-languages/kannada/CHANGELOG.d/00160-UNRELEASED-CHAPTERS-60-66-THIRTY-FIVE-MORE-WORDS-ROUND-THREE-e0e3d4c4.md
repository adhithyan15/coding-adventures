## Unreleased — Chapters 60-66: Thirty-five more words, round three

Two rounds took Kannada from 47 pre-A1 headwords to 117, and that was still last
of the seven tracks being carried to the 300-word floor together — spanish 118,
malayalam 119, tamil 119, hindi 120, sanskrit 139, telugu 151. These seven
chapters answer with thirty-five more on the same terms: **one new word per
lesson**, and unlimited reuse of everything already taught.

  60 Animals of the House       ಹಸು ಮೇಕೆ ಕೋಳಿ ಕಾಗೆ ಹಕ್ಕಿ
  61 In the Kitchen             ಮೊಸರು ತುಪ್ಪ ಎಣ್ಣೆ ಬೆಲ್ಲ ಸಕ್ಕರೆ
  62 Things People Make         ನೂಲು ಸೂಜಿ ಹಗ್ಗ ಪೊರಕೆ ಕೊಡೆ
  63 Hunger, Sleep and Health   ಹಸಿವು ನಿದ್ದೆ ಜ್ವರ ನೋವು ಆರೋಗ್ಯ
  64 Joining Words              ಆಮೇಲೆ ಕೂಡ ಇನ್ನೂ ಬೇಗ ಹಾಗಾದರೆ
  65 More Good Manners          ವಿನಂತಿ ಅನುಮತಿ ಸಂಕೋಚ ನಂಬಿಕೆ ಸೌಜನ್ಯ
  66 The Field                  ಹೊಲ ಹುಲ್ಲು ಭತ್ತ ಬೆಳೆ ಸುಗ್ಗಿ

Attainment 117 → 152 of 300, a rise of exactly thirty-five. All seven pre-A1
spine nodes are used a third time, one per chapter, in the same order round two
used them, and the chain runs unbroken from ಬೆಂಕಿ at the end of the weather run
to ಸುಗ್ಗಿ at the end of this one — each chapter's second lesson still practising
the previous chapter's last word.

The ramp got gentler again. R1 falls 0.2664 → 0.2642 with its numerator held at
exactly 1140: not one of the thirty-five new atoms misses its first
reinforcement window, so the whole improvement is a larger denominator.

### The collisions, and which check caught each

Every candidate was checked in four classes: in a lesson body, inside an
existing headword, inside a romanization, and — the one no string search
reaches — inside the GLOSS of a morpheme or a stock phrase.

**Inside an existing headword.** ದಾರ, the obvious word for thread, sits whole
inside ದಾರಿ from the river-and-road chapter. ಕಣ, a threshing floor, sits whole
inside ಕಣ್ಣು from the face chapter, which is why chapter 66 ends on ಸುಗ್ಗಿ
instead. ಸಹ for "also" sits inside ಸಹಾಯ; ಮತ್ತೆ is the first word of the headword
ಮತ್ತೆ ಸಿಗೋಣ; ಎಷ್ಟು is inside the headword ನಿನ್ನ ವಯಸ್ಸು ಎಷ್ಟು?.

**Inside a romanization.** ಕನ್ನಡಿ, a mirror, spells *kannaḍa* inside itself, and
that string is on more pages of this book than any other.

**Inside a gloss — the largest class again, six of them, and none reachable by
grep.** This is HL-C207 holding on a fourth track.

  ಅಕ್ಷರ     "a written character"  all 24 writing lessons say "the single character X"
  ಸ್ವಾಗತ     "welcome"              ಪರವಾಗಿಲ್ಲ has been glossed "you're welcome" since the how-are-you chapter
  ಮಾತ್ರ      "only"                 the replies chapter glosses the emphatic -ē as "only that one"
  ಬೆಳಕು      "light"                the morning lesson glosses beḷaku, "light", outright
  ಬಲ        "strength"             the age lesson glosses vayas as "age, youth, vigor, strength"
  ನಿಧಾನ     "slowly"               every guided-practice block says "it once more, slowly"

ಅಕ್ಷರ is the same shape as sanskrit's *shiilam* and telugu's *aksharam*: the
collision is with a phrase twenty-four files independently use, not with any one
lesson. ಸ್ವಾಗತ is the sharpest of the six, because "welcome" and "you're welcome"
are two different acts wearing one English word, and the reader has had the
second since chapter 3.

ಎಣ್ಣೆ was examined under the same rule and KEPT. The friend-lesson glosses the
Sanskrit *snēha* as "oil, unctuousness, smoothness", but that gloss teaches a
Sanskrit etymon, not a Kannada headword; ಎಣ್ಣೆ is Kannada's own and had never been
on the page. The lesson makes the pair explicit rather than pretending it is not
there. ಪ್ರೀತಿ, "affection", is the word that gloss really does block, and it is not
in this tranche.

Three near-misses were kept and turned into teaching instead of discarded, each
because the two words are genuinely distinct in Kannada letters and only look
close in a romanization: *hakki* against *akki*, ಕೊಡೆ against ಕೊಡು, and ಕೂಡ
against ಕೂದಲು. ಹುಲ್ಲು against ಹಲ್ಲು is the closest pair in the whole book — one
vowel sign — and chapter 66 says so out loud rather than hoping.

### Script, and what the taught-glyph filter cost

Every headword is spelled entirely from glyphs the track's own writing lessons
teach, so the tranche adds no script-closure violation and nothing is laundered
through an exposure-exempt headword: Kannada's exempted-glyph count is unmoved at
162 and its violation count at 37. The same filter shaped the prose — ಉಪ್ಪು is
cited in romanization in the jaggery lesson, because ಉ is a letter no writing
lesson has taught. Forward references hold at 511, rule statements at thirty, and
Kannada's cross-chapter prose references at their pinned 80.

### What the chapters actually do

Chapter 61 is a chain rather than a list: ಹಸು gives ಹಾಲು, ಹಾಲು gives ಮೊಸರು and
ತುಪ್ಪ, and ಎಣ್ಣೆ is the one that comes from a ಬೀಜ instead — so the chapter teaches
where five kitchen words come from, not just what they are.

Chapter 63 teaches one sentence shape five times. ನನಗೆ ಹಸಿವು has exactly the
build of ನನಗೆ ಕನ್ನಡ ಗೊತ್ತು from the knowing lesson, and ನೋವು then attaches to
every body part the book has taught, which is the best trade in the tranche: one
new word, a sentence for each part of the body.

Chapter 66 completes a set three rounds in the making — ಭತ್ತ in the field, ಅಕ್ಕಿ
in the sack, ಅನ್ನ on the plate, one plant under three names that Kannada keeps
strictly apart.

The last lesson deliberately makes no claim about the book ending (HL-C201): it
says these five words end where a farming year does, which stays true however
many chapters come after it.

