# HL71 — Nine-track changelog direct owners

## 1. Problem

Arabic, Chinese, French, German, Italian, Latin, Persian, Portuguese, and
Russian still keep independently authored release notes in one tracked
`CHANGELOG.md` apiece. In the exact-main first-parent 200-commit audit after
#17696, each aggregate was still live: the nine files were touched between
eleven and nineteen times. Unrelated same-track authors therefore contend for
one prepend location even though their release notes do not share semantic
state.

Japanese and the Indian-language tracks already use direct changelog owners.
HL71 closes the same ownership seam for the remaining nine measured tracks.

## 2. Ownership boundary

For each migrated track, `CHANGELOG.d/` is the source of truth.

- `_meta.md` owns only the document preamble.
- Each level-2 changelog entry owns one strict rank/slug/heading-digest file.
- The rank orders entries newest first. Parallel entries may use the same rank;
  their heading slug and digest give them distinct filenames and deterministic
  tie ordering.
- `CHANGELOG.md` is an ignored local rendered view, never a tracked aggregate.

Adding one changelog entry must add one file and must not edit `_meta.md` or any
existing entry owner.

## 3. Migration contract

The migration slices each existing document at level-2 headings and must
reconstruct every pre-migration byte exactly. It may not normalize prose,
spacing, punctuation, or heading text.

The existing document-shard protections remain in force for all nine tracks:
exact plan matching, real-file and symlink checks, safe repository-relative
paths, canonical strict filenames, filename/body agreement, case-fold collision
rejection, append-only history, deterministic sorting, and exact split/join
reconstruction.

## 4. Regression contract

Focused tests must prove for every migrated track:

1. the changelog is registered as a newest-first level-2 document-shard plan;
2. its historical monolith reconstructs byte-for-byte from direct owners;
3. the tracked monolith is absent from a clean checkout; and
4. repository policy covers both its append-only owner glob and ignored local
   projection, so deleted owners and resurrected aggregates fail closed.

The general document-shard suite continues to prove malformed, linked,
non-canonical, and case-fold-colliding owners fail closed; adding these paths to
the authoritative plan makes those protections apply without track-specific
copies of the same adversarial fixtures.

## 5. Verification

Run the focused document-shard tests, the human-language-data TypeScript build,
the repository document-shard check, the full package suite and coverage, and
the deterministic hot-file report after merge.
