# HL48 — Direct etymology-gap pin owners by language track

**Status:** specification, 2026-10-13

**Extends:** HL43's direct-owner corpus evidence and HL47's stable composition
boundary. Tracks #17463 and the parent contention program #13193.

## 1. Problem

`human-language-data/tests/root-ledger.test.ts` combines stable root-ledger
behavior with an independently changing etymology debt pin for every language
track. When one track lowers its measured debt, its author edits the shared
`ETYMOLOGY_GAP_PINS` object and appends track-specific provenance prose to the
same test file as every other owner.

The exact post-#17459 HL42 report at
`b7ab881795f1b8d90dab877042723878d21df934` measured 15 touches to the
431-line test in the newest 200 scoped first-parent commits. Fourteen recent
touches are unrelated track-by-track etymology closures. The test has therefore
become a cross-track serialization point even though the underlying lessons
already have direct owners.

## 2. Stable generic test boundary

The root-ledger test keeps only script- and track-independent behavior:

- build and rendering semantics for a supplied lesson set;
- one corpus load and one etymology-coverage measurement;
- the exact-pin ratchet and its rise/fall diagnostics; and
- root-ledger summary consistency and source hygiene.

No language-keyed pin map or track-specific closure narrative remains in the
shared test. Adding or improving an existing track does not edit its stable
algorithm or test harness.

## 3. One direct owner per language track

Each language owns one regular TypeScript module under
`tests/etymology-gap-pin-owners/`. The filename and declared language must be
the same canonical lowercase track slug. An owner declares exactly:

- `language`;
- `withoutHook`, the exact number of word lessons without an
  `etymology_hook`;
- `withoutVisible`, the exact number without learner-visible etymology; and
- a non-empty `rationale` containing the track-specific migration baseline or
  provenance for a lowered pin.

The migration preserves every current numeric pin exactly. Track-specific
provenance moves with its values rather than remaining in the shared harness.

## 4. Discovery and fail-closed validation

The test runner discovers owner modules through a bounded `import.meta.glob`.
A stable loader validates the matching filesystem directory before composing
the owners. It rejects:

- a missing, symlinked, or non-directory owner root;
- symlinked, non-regular, unsafe, non-lowercase, reserved, or unexpected child
  names;
- filename/declaration mismatches and case-folded collisions;
- missing, duplicate, unknown, or extra track owners;
- modules with extra or missing exports or owner fields;
- negative, fractional, unsafe, or otherwise malformed pin values; and
- empty or malformed rationale paragraphs.

The expected track set comes from the measured corpus, not from a second count
pin, manifest, ordered registry, or hand-maintained index. A track missing from
either side fails closed.

## 5. Exact-pin semantics

Each direct owner still drives two generated tests. A rise names the newest
offending lessons and requires fixing the lessons rather than raising the pin.
A fall asks the author to lower only that track's owner in the same change.
The shared diagnostic must point to the direct-owner path, never instruct an
author to edit a retired aggregate map.

The corpus is loaded and measured once at module scope. Direct-owner discovery
must not add a second corpus parse or a separate aggregate test run.

## 6. Anti-regression evidence

Structural and behavioral tests prove that:

1. all current corpus tracks have exactly one valid direct owner;
2. unsafe roots, names, links, modules, fields, values, rationales, and track
   sets are refused;
3. all migration-boundary values equal the pre-HL48 map;
4. `root-ledger.test.ts` contains no language-keyed pin map or track-specific
   etymology closure narrative; and
5. the direct-owner directory contains no shared index, manifest, registry, or
   aggregate owner.

## 7. Acceptance

Completion requires:

1. all current etymology-gap pins and their rationale have direct owners;
2. exact pin values, ratchet behavior, diagnostics, and one-corpus-load
   behavior are preserved;
3. focused ownership and root-ledger tests pass;
4. the full Human Language Data suite, build, and coverage gate pass;
5. changelog, document-shard, lesson, and book checks pass proportionately;
   and
6. an independent security/data-integrity review reports no unresolved issue
   before every push.
