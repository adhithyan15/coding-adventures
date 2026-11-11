# HL118: Bounded curriculum-membership owner reads

## Problem

On the 63,257-lesson Windows corpus, `loadLanguageCurricula()` takes about
42 seconds. It synchronously opens and parses one direct-owner JSON file per
lesson. This makes otherwise bounded track-progress and completion-plan checks
exceed their unchanged test budgets. Issue #18982 blocks the Windows suite
repair in #18751.

## Contract

* Add an asynchronous curriculum loader for whole-corpus consumers. Preserve
  the synchronous public loader and the exact `LanguageCurriculum` result.
* Derive expected owner identities from canonical lesson filenames, not owner
  metadata, generated output, or an earlier cache. Require exactly one owner
  per lesson plus `_meta.json`, with no extra, missing, duplicate, case-folded,
  unsafe, symlinked, junctioned, or non-regular direct entries.
* Read each owner relative to a pinned directory handle using a bounded number
  of simultaneous opens. The directory and files must not be redirected by a
  concurrent path replacement. Decode UTF-8 strictly and retain canonical JSON
  byte checks, exact keys, lesson-ID matching, extension validation, source
  paths, and error-on-corruption behavior.
* Do not cache source bytes across calls. A subsequent source edit must be
  observed. Do not increase test timeouts, skip lessons, lower coverage, or
  weaken corpus/build/security gates.
* Use the asynchronous loader in the track-progress command after parity and
  adversarial tests pass. Extend the completion-plan command only if its
  unchanged clean-corpus cases still need the same fix.
* Verify fixture sync/async parity, missing/extra/malformed/noncanonical
  owner rejection, and symlink/junction-swap refusal. Then rerun the affected
  Windows cases and full unweakened suite on the exact final source.

## Out of scope

This does not change curriculum content, lesson duration, pre-A1-to-C2
progression, authored owner format, or synchronous caller behavior.
