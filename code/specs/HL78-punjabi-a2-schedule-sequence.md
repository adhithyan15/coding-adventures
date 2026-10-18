# HL78 — Punjabi A2 schedule and sequence prerequisite

## Status

Accepted implementation slice for issue #17742. This is a prerequisite for
Punjabi A2 mock 1 (#17739), not a mock or a readiness decision.

## Problem

A complete draft of Punjabi A2 mock 1 exposed 281 distinct Gurmukhi tokens
that do not occur in any Punjabi lesson. Five of the highest-reuse structural
gaps are **ਵਜੇ** (o'clock), **ਤੋਂ** (from), **ਤੱਕ** (until/up to), **ਨਾਲ**
(with), and **ਬਾਅਦ** in the sequence **ਤੋਂ ਬਾਅਦ** (after). Scoring those forms
before teaching them would break the track's gentle evidence order.

## Required sequence

Chapter 178 owns five lessons, each no longer than 300 seconds:

1. `PA-C178-vaje` teaches **ਅੱਠ ਵਜੇ** as one bounded whole-hour frame.
2. `PA-C178-ton` teaches **ਘਰ ਤੋਂ ਸਕੂਲ ਜਾਣਾ**, placing **ਤੋਂ** after its
   source noun.
3. `PA-C178-tak` joins the already-taught numbers and the first two new forms
   as **ਅੱਠ ਵਜੇ ਤੋਂ ਦਸ ਵਜੇ ਤੱਕ**.
4. `PA-C178-naal` teaches **ਅਮਨ ਨਾਲ ਕੰਮ ਕਰੋ**, placing **ਨਾਲ** after the
   companion noun.
5. `PA-C178-baad` teaches **ਦਸ ਵਜੇ ਤੋਂ ਬਾਅਦ** only after **ਤੋਂ** and closes
   with the two-sentence schedule note **ਅਮਨ ਨਾਲ ਅੱਠ ਵਜੇ ਤੋਂ ਦਸ ਵਜੇ ਤੱਕ ਕੰਮ
   ਕਰੋ। ਦਸ ਵਜੇ ਤੋਂ ਬਾਅਦ ਘਰ ਆਉਣਾ।**

Each introduced atom has exactly one owner lesson. Supporting Punjabi must
already occur in earlier <=300-second lessons. The final lesson retrieves the
bounded schedule through listening, speaking, reading, and covered-model
Gurmukhi writing.

## Evidence boundaries

- The chapter closes only these five exact forms. It does not teach every use
  of any postposition, oblique-case paradigm, half-hour expression, tense, or
  daily-routine construction.
- It is the first discovered A2 prerequisite tranche and does not by itself
  unblock mock 1.
- It provides no mock completion, readiness, attainment, accreditation,
  external-equivalence, recording-quality, or qualified-human-validation
  evidence.

## Sources

- Ranbir Johal, *Speaking and Writing Punjabi* (2018), pp. 67–70, 76, and
  84–85: **ਤੋਂ … ਤੱਕ**, whole hours with **ਵਜੇ**, and post-nominal **ਨਾਲ**.
- *Online Punjabi Teaching*, "Adverb", for an explicit clausal
  **ਤੋਂ ਬਾਅਦ** example. The lesson deliberately teaches only the bounded time
  phrase required here.

## Validation

The focused corpus owner pins lesson order, duration, single ownership,
source-before-postposition ordering, exact cumulative forms, four-skill
payoff, curriculum/chapter membership, and explicit non-readiness boundaries.
Normal shard, modality, narration, book-generation, compile, and full package
checks remain mandatory.
