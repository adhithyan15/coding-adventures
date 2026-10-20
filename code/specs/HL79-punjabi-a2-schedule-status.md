# HL79 — Punjabi A2 schedule status and day-part prerequisite

## Status

Accepted implementation slice for issue #17749. This is the second content
prerequisite for Punjabi A2 mock 1 (#17739), not a mock or readiness decision.

## Problem

After chapter 178 landed, exact-main closure still found 277 distinct Gurmukhi
forms in the preserved mock draft that do not occur in Punjabi lessons. The
highest-reuse coherent public-schedule group is **ਸਮਾਂ** (time), **ਸਵੇਰੇ** (in
the morning), **ਬੰਦ** (closed), **ਦੁਪਹਿਰ** (at noon/in the afternoon), and
**ਖੁੱਲ੍ਹਾ** (open). Together they account for 43 unsupported mock occurrences.

## Required sequence

Chapter 179 owns seven lessons, each no longer than 300 seconds:

1. `PA-C179-samaa` teaches **ਸਮਾਂ** as the bounded noun "time".
2. `PA-C179-savere` teaches **ਸਵੇਰੇ ਅੱਠ ਵਜੇ** as one morning time.
3. `PA-C179-khullha` teaches **ਬਾਗ਼ ਸਵੇਰੇ ਖੁੱਲ੍ਹਾ ਹੈ** as one open-status line.
4. `PA-C179-dupahir` teaches **ਦੁਪਹਿਰ ਦੋ ਵਜੇ** as one afternoon time and
   retrieves the open line.
5. `PA-C179-band` teaches **ਬਾਗ਼ ਬੰਦ ਹੈ** as the contrasting closed status.
6. `PA-C179-status-review` retrieves all five forms without adding an atom.
7. `PA-C179-payoff` closes with **ਬਾਗ਼ ਸਵੇਰੇ ਅੱਠ ਵਜੇ ਖੁੱਲ੍ਹਾ ਹੈ। ਦੁਪਹਿਰ ਦੋ
   ਵਜੇ ਬੰਦ ਹੈ।** through listening, speaking, reading, and covered-model
   Gurmukhi writing.

Each new atom has exactly one owner. Supporting Punjabi must already occur in
earlier <=300-second lessons. Two atom-free review lessons give even the last
introduced forms two honest later retrievals instead of postponing continuity
debt to the next tranche.

## Evidence boundaries

- The chapter teaches only these five exact forms and their one opening-hours
  payoff. It does not teach every day part, appointment expression, adjective
  agreement pattern, business-hours convention, or public-notice register.
- It is the second discovered A2 prerequisite tranche and does not by itself
  unblock mock 1.
- It provides no mock completion, readiness, attainment, accreditation,
  external-equivalence, recording-quality, or qualified-human-validation
  evidence.

## Sources

- NCERT, Punjabi activity page, **ਗਤੀਵਿਧੀ ਟਾਈਮ**, for **ਸਮਾਂ**, **ਸਵੇਰ ਦਾ
  ਸਮਾਂ**, and **ਦੁਪਹਿਰ ਦਾ ਖਾਣਾ** in a daily-time activity.
- UK Government, *A Home Office Guide to Living in Asylum Accommodation —
  Punjabi*, for the attested public-hours contrast **ਸਵੇਰੇ ਖੁੱਲ੍ਹੇ ... ਦੁਪਹਿਰ
  ... ਬੰਦ**.
- RCPLT, Punjabi University Patiala, *Punjabi-English Dictionary*, entry
  **ਬੰਦ**, for the adjective "closed, shut"; the University's teaching
  vocabulary also publishes the pair **ਖੁੱਲ੍ਹਾ/ਬੰਦ**.

## Validation

The focused corpus owner pins sequence, duration, single ownership, exact
cumulative forms, supporting-token closure, two later retrievals per new atom,
four-skill payoff, A2 practical-text placement, and explicit non-readiness
boundaries. Normal shard, modality, narration, book-generation, compile, and
full package checks remain mandatory.
