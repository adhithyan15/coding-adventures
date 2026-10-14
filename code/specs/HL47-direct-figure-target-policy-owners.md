# HL47 — Direct figure-target policy owners by writing system

**Status:** specification, 2026-10-13

**Extends:** HL46's direct-document ownership boundary and HL43's direct-owner
corpus evidence. Tracks #17456 and the parent contention program #13193.

## 1. Problem

`human-language-data/src/figure-targets.ts` combines stable target derivation
with independently changing writing-system facts: enabled tracks, written sign
order, fused pairs and runs, apart clusters, headline composition, font
exceptions, citations, and script-specific limits. The matching real-corpus
test combines evidence for every writing system in one file. A change for one
script therefore collides with unrelated work on another script.

The exact post-#17452 HL42 report at
`0bc039d953cd0853276a0c37a9393c59d2937281` measured 17 touches to the
1,115-line source module and 14 touches to the 555-line real-corpus case in the
newest 200 scoped first-parent commits.

## 2. Stable core and compatibility facade

The stable core owns only script-independent behavior:

- recognizing a writing lesson and its landing block;
- decomposing a headword into candidates under a supplied policy;
- enforcing shared safety bounds;
- merging cited derived targets with explicit targets;
- rendering captions and placing images; and
- preserving all current public exports.

The historical `figure-targets.ts` import path remains a compatibility facade.
It may compose and re-export direct owners, but it must not regain per-script
tables, citations, Unicode ranges, font facts, or real-corpus expectations.

## 3. Direct writing-system owners

Each independently evolving writing-system policy lives in one direct source
file under `src/figure-target-policies/`. An owner may declare:

- the language tracks that activate its script id;
- whether cited base letters may compose as separate letters in a word;
- written-side rows for cited marks;
- cited fused sign pairs or fused letter runs;
- cited clusters whose printed pieces remain apart;
- shared-headline letter, sign, consonant, and split-glyph rules; and
- source citations and writing-system-specific limits.

A stable composition boundary validates owners fail closed. Script ids and
track ids must be unique, cited source rows must be well formed, and a missing
policy supplies no permission. Existing-script changes edit only that
writing-system owner. The composition boundary is not a count pin, manifest,
generated index, or independently maintained duplicate of owner contents.

## 4. Fail-closed derivation

The current rule remains absolute: no cited and validated policy means no
derived figure. Unicode normalization behavior, exact written order, maximum
sequence and phrase bounds, list handling, placement rules, explicit-target
precedence, and the requirement that every derived piece has a cited ductus
are preserved.

Public compatibility constants are projections of validated owners, not a
second policy copy. The migration must preserve the exact target set at the
change base, including intentional refusals.

## 5. Direct real-corpus evidence

The shared real-corpus case is replaced by direct case files owned by the
writing systems whose facts they assert. Tamil, Gujarati, Devanagari, Telugu,
Bengali, Malayalam, Gurmukhi, Latin, and the remaining separate-letter scripts
have independent evidence owners. Generic switched-track closure and printed
chapter placement have their own cases because they are cross-script core
invariants rather than writing-system policy.

The test loader discovers case files from the filesystem. There is no aggregate
test, owner manifest, central case list, or total-owner count. Existing-script
evidence changes only that script's case file.

## 6. Anti-regression evidence

A structural test proves that:

1. policy owners are direct regular TypeScript files with safe script slugs;
2. reserved aggregate names such as `index`, `manifest`, and `registry` are
   rejected;
3. the stable core and compatibility facade contain no writing-system policy,
   source citation, font-version fact, or real-corpus expectation;
4. two independent existing-script policy edits require no change to the
   composition boundary; and
5. the retired real-corpus aggregate does not return.

Behavioral tests continue to hold policy rows to the cited script inventory,
font, Unicode, generated-figure, and book evidence.

## 7. Acceptance

Completion requires:

1. direct writing-system source and real-corpus test owners replace both hot
   files without a replacement aggregate;
2. all public exports and the exact resolved target set are preserved;
3. focused policy, ownership, corpus, generated-figure, and book checks pass;
4. the full Human Language Data suite, build, and coverage gate pass;
5. the sharded changelog and lessons validations pass; and
6. an independent security/data-integrity review reports no unresolved issue
   before every push.
