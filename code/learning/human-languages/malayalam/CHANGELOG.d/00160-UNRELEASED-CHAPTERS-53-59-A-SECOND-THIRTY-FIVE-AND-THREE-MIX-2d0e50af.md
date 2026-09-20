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

