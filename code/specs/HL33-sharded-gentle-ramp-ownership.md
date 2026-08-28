# HL33 — Sharded gentle-ramp ownership

**Status:** specification, 2026-08-27

**Extends:** HL17's learner-first gentle-ramp report, HL21's deterministic
filesystem rules, and HL32's exact generated-owner closure. Tracks #13376,
parent #13193, and program #12206.

## 1. Outcome

Changing two independent measurements in one language must not rewrite one generated
language aggregate. Gentle-ramp snapshots therefore use stable direct owners:

```text
core/gentle-ramp-snapshots/
  arabic.d/
    _meta.json
    metrics/
      atomMeasurableLessons.json
      ...
      reinforcementMissesByWindow-R1.json
      reinforcementMissesByWindow-R4.json
    findings/
      duration.json
      ...
      measurement-blind.json
  ...
  urdu.d/
```

Every registered language owns exactly 37 files: one metadata owner, 26 metric
owners, and ten always-present finding-kind owners. The 23 current languages
therefore produce 851 files. That total is derived from the registry rather than
hard-coded.

The 26 metric identities are the 23 stored `TrackGentleRamp` fields other than
`language`, `lessonCount`, `findings`, and `next`, with
`reinforcementMissesByWindow` flattened into the fixed R1, R2, R3, and R4 identities.
`lessonCount` is derived from independently proven lesson identities. `findings` is
reconstructed in `GENTLE_RAMP_PRIORITIES` order, and `next` is its first item or null.

The old `core/gentle-ramp-snapshots/<language>.json` aggregates are neither tracked,
read, nor emitted.

## 2. Exact identity closure

Completeness is established before owner contents may define the report:

1. `core/languages.json` supplies the exact language-directory set.
2. Parsed canonical lesson Markdown supplies the exact lesson-id set and language
   assignment.
3. `narrationLessonIdentityIndex` supplies the independent generated-narration
   lesson-id set and language assignment.

All three projections must agree exactly, including global duplicate and case-fold
checks. Per-language totals are insufficient: two mismatched identities can preserve
the same count. Only after the sets agree may their cardinality become `lessonCount`.

At the filesystem boundary, the snapshot root contains only registered
`<language>.d/` directories. Each has exactly `_meta.json`, `metrics/`, and
`findings/`. Metrics and findings contain exactly the fixed regular direct-child JSON
owners. Flat aggregates, unknown entries, nesting, traversal-shaped or reserved
identities, duplicate or case-fold aliases, symbolic links, directories in file
positions, and other non-regular entries fail closed.

Each file contains an exact-key identity-bearing object. Directory language,
filename identity, embedded language, and embedded metric or finding kind must agree.
All numbers are non-negative safe integers; finding payloads must be positive,
well-formed, and exactly derivable from their metrics. Reinforcement totals equal the
R1-R4 sum, atom measured plus blind lessons equals `lessonCount`, and writing counts,
position, and prefix agree. Recursive dangerous keys are rejected. Canonical bytes are
two-space JSON plus one trailing newline.

## 3. Reconstruction without aggregates

The strict fold builds `TrackGentleRamp` with the historical property insertion order,
rebuilds reinforcement windows in `REINFORCEMENT_WINDOWS` order, and rebuilds findings
in learner-first priority order. Its serialized result must be byte-for-byte equal to
the freshly derived public track. The global work queue and summary remain projections
of those reconstructed tracks; they are not tracked shared files.

There is no legacy fallback. A flat language aggregate beside direct owners is an
error even when its bytes happen to match. `check:gentle-snapshots` proves exact path
sets, exact identity sets, canonical bytes, metric arithmetic, finding derivation, and
public reconstruction. Clean deletion, stale owners, copied owners, and aggregate
resurrection all fail.

## 4. Generated migration

`generate:gentle-snapshots` replaces the family as one staged transaction:

1. derive the current report and exact source/narration identities;
2. when legacy aggregates exist, require the exact registry-language set and exact
   current canonical bytes;
3. write all expected owners under a private same-filesystem staging root using only
   whitelisted paths;
4. strict-read the staged tree and require exact public reconstruction;
5. rename the previous snapshot directory to a recovery path, install the complete
   staged tree, and verify it again;
6. remove the recovery path only after installed verification succeeds.

An interrupted or failed install restores the previous complete directory. Legacy
aggregates are therefore removed only after their direct-owner replacement has already
passed strict validation; they are never deleted one by one.

The capability manifest grants writes only to
`*.d/_meta.json`, `*.d/metrics/*.json`, and `*.d/findings/*.json`. It grants no flat
`gentle-ramp-snapshots/*.json` target, so filesystem policy enforces the same
no-aggregate boundary as the checker and reader.
