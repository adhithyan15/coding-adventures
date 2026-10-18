# HL59 — Sanskrit A1 media and communication words

## Status

Proposed for issue #17554 under #13415 and #13295.

## Problem

The project-defined Sanskrit A1 point `SA-A1-M-06` says that press,
telephone, radio, television, the internet, and letters are absent. That note
is mostly stale. The corpus already owns **पत्रिका** (*patrikā*, “a newspaper,
a magazine”), **दूरभाषः** (*dūrabhāṣaḥ*, “a telephone”), **दूरदर्शनम्**
(*dūradarśanam*, “a television”), and **लेखः** (*lekhaḥ*, “a letter, an
article”).

The two remaining lexical gaps are radio and internet. Kosha Sanskrit Today
explicitly attests modern **आकाशवाणी** (*ākāśavāṇī*) for “radio” and
**अन्तर्जालम्** (*antarjālam*) for “internet.” Because **आकाशवाणी** also has
an older “voice from the sky” sense, this tranche owns only the explicitly
sourced modern radio meaning.

Sources:

- https://kosha.sanskrit.today/word/en/radio
- https://kosha.sanskrit.today/word/en/Internet
- https://kosha.sanskrit.today/glossary/computer-internet

## Contract

Append one Sanskrit-owned chapter after chapter 268 with six lessons:

1. retrieve the four already-owned media and communication words;
2. teach **आकाशवाणी** only in the sourced modern radio sense;
3. teach **अन्तर्जालम्** only in the sourced modern internet sense;
4. distinguish and retrieve all six meanings in a guided round;
5. complete a first model-free six-word retrieval round; and
6. complete a second model-free round in a different order.

This is a bounded lexical-field tranche. It does not teach operating a device,
understanding news, writing correspondence, broadcasting, navigating the
internet, or digital literacy.

## Knowledge ownership

The chapter owns exactly two new atoms:

- `SA-LEX-C269-MEDIA-01` — **आकाशवाणी**, radio only in this tranche;
- `SA-LEX-C269-MEDIA-02` — **अन्तर्जालम्**, internet.

The second and third lessons introduce the new atoms at the blocks where their
sourced modern meanings first appear. Every later lesson retrieves both. The
last two lessons introduce no atoms, require and practise every point atom, and
show no copyable Sanskrit answer before the learner responds.

The exact `SA-A1-M-06` probe is:

- `SA-LEX-C222-THG222-05`
- `SA-LEX-C104-THINGS104-05`
- `SA-LEX-C269-MEDIA-01`
- `SA-LEX-C105-THINGS105-01`
- `SA-LEX-C269-MEDIA-02`
- `SA-LEX-C120-THINGS120-03`

## Ownership and sharding

Lesson files, chapter ledger, curriculum extension/path/membership owners,
book target, generated modality/book/narration owners, corpus evidence, and the
Sanskrit changelog entry must be Sanskrit-owned or single-record additive
files. The one inventory point and existing Sanskrit aggregate pins are the
only same-track files whose measured contract changes. No cross-language
mutable aggregate may be introduced.

## Evidence boundary

This slice closes one project-defined editorial A1 point. It proves two
model-free retrievals of six media and communication meanings. It does not
prove device use, news comprehension, correspondence, broadcasting, internet
navigation, digital literacy, learner attainment, human validation, or exam
readiness. The A1 inventory remains partial.

## Acceptance

- A Sanskrit-owned red-before-green test pins all six lessons, their order,
  exact ownership, duration ceilings, source citations, radio sense boundary,
  chapter payoff, and exact inventory probe.
- No owned press, telephone, television, or letter vocabulary is retaught.
- Every lesson stays within five minutes, and the final two introduce nothing.
- Both final prompts say that no Sanskrit model is shown before the learner
  answers.
- Sanskrit A1 coverage moves from 151/164 to 152/164, with 12 unmapped and
  zero partial points.
- Corpus, curriculum, shard, document-shard, modality, generated-book,
  narration, lesson, affected-package, and full book checks pass.
- Independent security and data-integrity review passes on the exact pushed
  head before every push.
