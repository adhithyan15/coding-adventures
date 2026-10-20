# HL58 — Sanskrit A1 service locations

## Status

Proposed for issue #17547 under #13415 and #13295.

## Problem

The project-defined Sanskrit A1 point `SA-A1-M-08` says that post, bank,
health service, and police are absent. That note is mostly stale. The corpus
already owns **पत्रालयः** (*patrālayaḥ*, “post office”), **चिकित्सालयः**
(*cikitsālayaḥ*, “hospital”), **आरक्षकः** (*ārakṣakaḥ*, “policeman”), and the
familiar **… कुत्र अस्ति?** (“Where is …?”) location-question frame.

The remaining lexical gap is a financial bank. The existing **तीरम्** owner
means a riverbank and must not be reused. Modern Sanskrit sources explicitly
attest **वित्तकोषः** (*vittakoṣaḥ*) for a financial bank: Kosha Sanskrit Today
lists the word under “Places — Bank” with a locative usage example, and the
Sanskrit Documents combined dictionary gives `वित्तकोषः = (m) bank`.

Sources:

- https://kosha.sanskrit.today/glossary/places-bank
- https://sanskritdocuments.org/dict/dictallcheck.pdf

## Contract

Append one Sanskrit-owned chapter after chapter 267 with six lessons:

1. retrieve the three already-owned service words and the familiar location
   question;
2. teach **वित्तकोषः** only in the sourced financial-institution sense and
   contrast it explicitly with the already-owned riverbank word;
3. ask and answer where the post office and hospital are;
4. ask and answer where the financial bank and policeman are;
5. complete a first model-free four-service location round; and
6. complete a second model-free round from a different familiar prompt.

The answers use only already-owned here/there language. The chapter does not
teach directions, transactions, emergency requests, or institutional systems.

## Knowledge ownership

The chapter owns exactly two new atoms:

- `SA-LEX-C268-SERVICE-01` — **वित्तकोषः**, financial bank only;
- `SA-PATTERN-C268-SERVICE-LOCATION-02` — the bounded four-service location
  exchange using the already-owned **… कुत्र अस्ति?** frame.

The second lesson introduces the bank atom at the block where the sourced
financial sense first appears. The third lesson introduces the exchange atom.
Every later lesson retrieves both. The last two lessons introduce no atoms,
require and practise every point atom, and show no copyable Sanskrit answer
before the learner responds.

The exact `SA-A1-M-08` probe is:

- `SA-LEX-C120-THINGS120-02`
- `SA-LEX-C268-SERVICE-01`
- `SA-LEX-C84-PLACES-02`
- `SA-LEX-C238-PPL238-04`
- `SA-LEX-C14-DEIXIS-06`
- `SA-LEX-ASTI`
- `SA-PATTERN-C268-SERVICE-LOCATION-02`

## Ownership and sharding

Lesson files, chapter ledger, curriculum extension/path/membership owners,
book target, generated modality/book/narration owners, corpus evidence, and the
Sanskrit changelog entry must be Sanskrit-owned or single-record additive
files. The one inventory point and existing Sanskrit aggregate pins are the
only same-track files whose measured contract changes. No cross-language
mutable aggregate may be introduced.

## Evidence boundary

This slice closes one project-defined editorial A1 point. It proves recall of
four service referents and one small location exchange. It does not prove
directions, postal or banking transactions, medical communication, emergency
communication, police-station vocabulary, institutional knowledge, learner
attainment, human validation, or exam readiness. The A1 inventory remains
partial.

## Acceptance

- A Sanskrit-owned red-before-green test pins all six lessons, their order,
  exact ownership, duration ceilings, source citation, financial/riverbank
  distinction, chapter payoff, and exact inventory probe.
- No owned post-office, hospital, policeman, location-question, here, there,
  or copula vocabulary is retaught.
- Every lesson stays within five minutes, and the final two introduce nothing.
- Both final prompts say that no Sanskrit model is shown before the learner
  answers.
- Sanskrit A1 coverage moves from 150/164 to 151/164, with 13 unmapped and
  zero partial points.
- Corpus, curriculum, shard, document-shard, modality, generated-book,
  narration, lesson, affected-package, and full book checks pass.
- Independent security and data-integrity review passes on the exact pushed
  head before every push.
