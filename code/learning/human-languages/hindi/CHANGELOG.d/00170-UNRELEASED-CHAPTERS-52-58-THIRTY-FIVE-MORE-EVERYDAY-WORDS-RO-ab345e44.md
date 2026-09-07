## Unreleased — Chapters 52-58: Thirty-five more everyday words, round two

The second HL-C198 tranche for Hindi, in the shape the first one fixed. Hindi
stood at 85 headwords against the 300 the pre-A1 vocabulary floor asks for;
these seven chapters answer with thirty-five more, one new word per lesson,
reusing everything already taught.

  52 In the Kitchen        चावल दाल तेल चीनी सब्ज़ी
  53 More of the Body      कंधा घुटना माथा पीठ होंठ
  54 Animals               गाय घोड़ा मछली चिड़िया बकरी
  55 The Sky               सूरज चाँद तारा बादल आसमान
  56 The Road              सड़क गाँव बाज़ार दुकान खेत
  57 A Gift and a Feast    तोहफ़ा दावत थाली ख़ुशबू मिठास
  58 Inside the House      दीवार छत खिड़की सीढ़ी चादर

Chapters 52-58 append after chapter 51 and chain from HI-C51-sweet, reusing all
seven pre-A1 spine nodes a second time — one node per chapter, five lessons
sharing it. **Pre-A1 vocabulary 85 -> 120 of 300**, a rise of exactly
thirty-five, which puts Hindi at the front of the corpus ahead of Spanish's 118.

The ramp got gentler again while the book got longer. R1 falls 0.2818 to 0.2793
with the numerator **held at 1106** — not one of the thirty-five new atoms
misses its R1 window. Hindi's forward-reference count holds at **11**, exactly
where the previous tranche left it: none of the new words makes an earlier page
point forward at something it has not met.

Sixty-four candidates were screened against all 192 existing lessons before any
were written, using the forward-reference detector's own word-boundary predicate
rather than a substring search. Only one was discarded — चाबी, which chapter 36
already spends on the room it unlocks. The low discard rate is a property of the
script rather than of the screening: a Devanagari headword collides far less
readily than a Latin-script one, because an independent vowel (आ) and a vowel
sign (ा) are different characters, so आम cannot hide inside नाम the way *aam*
hides inside *naam*.

Two defects were caught by the gates rather than by reading, and both are
recorded because the reasoning is the valuable part.

The first: HI-C57-fragrance came out classified `sight` rather than `voice`, on
the cue "see the" in *once you can see the seam in the middle*. Nothing in that
lesson needs eyes — it is about hearing *khush-* inside *khushbū*, and the drill
line three lines below it already said **hear**. The sentence was wrong on its
own terms before it was wrong for the detector, and it now reads *hear the
seam*. All thirty-five lessons are `voice`, so the whole wave stays drivable.

The second is a policy this tranche had to learn the hard way. Seven of the new
lessons pointed back at earlier material by number — "since chapter 37", "back
in chapter 46" — and each number was checked against the corpus before it was
written. Checking them was the wrong instinct: HL-C102 pins cross-chapter prose
references per track precisely because a number that is correct when written
goes stale the next time a chapter splits, and the gate holds Hindi at 20. The
fix is never a fresher number, it is to name the thing, so the seven now read
"when the ear was named", "at the end of the welcome chapter", "when you first
counted to five". Hindi stays at 20 and the tranche adds none.

A third defect was caught by CI rather than locally, because no local gate could see
it. The sun lesson cited the reconstructed PIE form with a combining ring below
(U+0325), which Latin Modern has no glyph for, so XeLaTeX dropped it and the LaTeX
warning gate failed on `missing_character`. The books compiled; the character simply
never reached the page. The citation now names the Proto-Indo-European word without
spelling it, keeping the *sōl* / *hḗlios* / *sun* cousins intact. There is no LaTeX
toolchain in the authoring container, so the only check that reads a book.log runs in
CI alone.

The etymologies keep the double-inheritance thread the earlier chapters set up,
and refuse it where it does not hold. गाय with *cow*, सूरज with *sun*, चाँद with
*candle* and तारा with *star* are four clean PIE inheritances. Against them:
घोड़ा displaced the inherited अश्व that is Latin's *equus*, and चावल, चिड़िया,
सड़क, खिड़की and सीढ़ी are set down as genuinely unsettled rather than given a
plausible-sounding ancestor. बादल is marked as probable-not-settled in the one
chapter where three clean etymologies in a row might otherwise have made a
fourth look safe.

