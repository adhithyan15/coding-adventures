# HL117: Bounded async corpus loading for Windows whole-corpus checks

## Problem

The unchanged Windows `human-language-data` coverage suite on PR #18960 timed out
in 16 cases. An isolated gentle-ramp case took 258 seconds against its 180-second
budget, so worker contention alone is insufficient. On the same 63,257-lesson
checkout, compiled `loadEverything()` spent 100.8 seconds, while the existing
bounded `loadLessonsAsync()` read and parsed the lessons in 16.5 seconds. The
synchronous per-file source-read path is a shared bottleneck for whole-corpus
consumers.

## Contract

* Add `loadEverythingAsync(root?)` with the same result shape and validation as
  `loadEverything(root?)`. Only lesson loading changes: await the existing
  `loadLessonsAsync(root)`, whose per-track source reads are bounded to 16.
  Metadata, book, script, sound-tag, and letter-ledger loaders retain their
  existing order, checks, and exact data.
* A byte-for-byte lesson edit must be observed on the next call, including when
  the test-only content-addressed cache is enabled. No mtime-only shortcut or
  new trust in generated data is permitted.
* Move genuinely whole-corpus async-capable tests and commands to this API only
  where their behavior remains identical. Keep every lesson, assertion,
  security guard, per-test timeout, and >=85% coverage gate.
* For the gentle-ramp parity test, expose the existing snapshot-retirement and
  report-derivation operation over already-loaded source inputs. Both sides of
  the comparison still derive the complete report independently; only the
  redundant second filesystem traversal is removed. The public no-argument
  snapshot loader keeps reading and validating canonical sources itself.
* A read-only test file may share one awaited bounded corpus load across its
  cases. A test that edits corpus sources must not use that shared value:
  after an edit, the content-addressed loader must read the new bytes again.
* A one-track assertion may read that track's complete canonical lessons and
  curriculum directly when the measured function itself filters to that track.
  Keep independently shared policy and spine inputs, and fail if the track
  curriculum is absent rather than silently measuring an empty substitute.
* Verify sync/async result parity on a small fixture and real-track lesson
  identity/hash parity. Confirm isolated failing cases and then the unweakened
  full Windows suite on the exact final source before considering PR #18960
  ready. No wall-clock assertion is part of the correctness contract.

## Out of scope

This change does not alter curriculum content, lesson duration, pre-A1-to-C2
progression, book generation, or unrelated package tests. Synchronous public
APIs remain available for callers that cannot await.
