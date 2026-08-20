# HL17 — The Super-Gentle Ramp Gate

**Status:** Active specification

**Depends on:** HL05, HL08, HL09, HL11, HL16

**Scope:** Every active Human Languages track, pre-A1 through C2

## 1. The promise

Every instructional lesson takes at most five minutes. Every new burden arrives
in a small prerequisite-safe step. Writing begins in the opening lesson and
advances as gently as listening, speaking and reading. Finishing a book must
prepare a learner for its real external examination, or for the clearly labelled
project-defined equivalent required by HL16.

This promise applies to every language. Adding a good new track does not make an
older steep track acceptable, and touching a CEFR label does not constitute
attainment.

## 2. Why another report is necessary

The repository already has strong individual measurements:

- duration and the five-minute ceiling;
- knowledge atoms per lesson and chapter;
- new target-script glyphs and writing systems per lesson;
- prerequisite and review order;
- target-language forward references;
- expanding retrieval windows;
- load-bearing glyph closure;
- writing modality; and
- chapter capability and payoff closure.

Before HL17 they were separate tables. That made a language's worst learner cliff
a manual join and made cross-language prioritization fragile. HL17 joins those
measurements per track and emits a corpus-wide work queue.

## 3. No composite gentleness score

HL17 must not add unlike units. Seconds, atoms, glyphs, prerequisite edges and
retrieval windows are not exchangeable. A weighted score lets a severe opening
cliff disappear inside a long calm course and lets a large easy-to-count category
dominate a small catastrophic one.

Every queue row therefore retains:

1. the language;
2. the named debt kind;
3. the count;
4. the unit; and
5. the concrete remediation.

The queue is ordered by learner dependency, then worst count, then language id.
Duration comes first because a lesson that cannot fit the session contract must
split before its internal load can be trusted. Forward prerequisites and language
come before later coverage work because the learner meets them first. Unknown
measurement comes last in the ordering but remains debt, never evidence of a
gentle lesson.

## 4. Five minutes means a maximum

An effective duration of exactly 300 seconds is compliant. A duration strictly
greater than 300 seconds is a violation. The effective duration remains the
greater of the authored declaration and the independent duration estimate, so
short metadata cannot hide long learner work.

## 5. Writing starts on lesson one

Each track records the first reading-order position containing writing practice.
A writing lesson, a script-delivery lesson, or an authored `writing`/`script`
block counts as evidence. A track with no such evidence is not credited for
showing target script in a headword; exposure is not practice.

HL17 measures the opening boundary only. The complete progression — observe or
trace, guided copy, delayed copy, and independent production — is the writing
stage coverage contract tracked separately by the HL16 assessment work. That
more detailed gate may consume this report but must not weaken its lesson-one
rule.

## 6. Report-only migration

Existing debt is reported rather than converted into a corpus-wide failing gate.
Each track may become strict once its own baseline reaches zero. New work must not
worsen a track's named debt, and every rewrite tranche should reduce at least one
queue row without increasing an earlier one.

The machine-readable report is available as `gentleRamp` inside the curriculum
gap report. `renderGentleRamp` prints the complete cross-language queue.

## 7. Follow-on measurements

HL17 deliberately exposes two dependencies instead of pretending they are done:

- explicit writing-stage coverage is required before a track can prove the full
  writing ramp; and
- authentic external task shapes are required before simultaneous task novelty
  can be bounded without guessing from headings.

Until those land, an atom-measurement-blind lesson or an unmodelled task shape is
unmeasured, not clean.
