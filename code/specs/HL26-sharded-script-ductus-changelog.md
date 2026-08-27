# HL26 — Sharded Script Ductus changelog

## 1. Outcome

Script authors add independently owned release-note fragments without editing
`code/packages/typescript/script-ductus/CHANGELOG.md`. The historical changelog
remains byte-exactly recoverable as an optional local view, while the tracked
monolith is absent and cannot be reintroduced unnoticed.

This completes issue #13284 and the next measured conflict-removal tranche of
#13193. PR #13275 started a forward-only `CHANGELOG.d/` entry point after a live
overlap with #13272; PR #13279 immediately demonstrated that guidance alone did
not remove the old shared-file habit. HL26 migrates the history and enforces the
new ownership boundary.

## 2. Canonical storage

The canonical document is:

```text
code/packages/typescript/script-ductus/CHANGELOG.d/
  _meta.md
  NNNNN-HEADING-SLUG-<8-hex-heading-digest>.md
```

`_meta.md` contains the bytes above the first level-3 entry heading. Each other
file contains exactly one `###` entry and all bytes through the next entry.
Level-2 release markers remain attached to the preceding entry exactly as they
are in the pre-migration document; this migration does not normalize history.

The former aggregate is forbidden:

```text
code/packages/typescript/script-ductus/CHANGELOG.md
```

It may be rendered locally for searching, but it is ignored and never committed.

## 3. Ordering and identity

Script Ductus is newest-first. Section filenames therefore use descending
recency ranks: the lexically greatest valid filename renders first. Adding a
new note chooses a rank above the current maximum and never renames a neighbor.

The filename slug is readable but not unique. Identity comes from the lowercase
eight-hex SHA-256 prefix of the raw heading line, matching HL22. Two parallel
authors collide only when they independently write the same heading at the same
rank, which is duplicate release-note content rather than unrelated work.

Strict shard names are mandatory. New fragments without a positive fixed-width
rank, portable uppercase slug, and heading digest fail validation. Any legacy
pre-gate name may survive only behind an exact content hash pin; HL26 creates no
new exception.

## 4. Shared tooling boundary

Human Language Data's document sharder owns the single implementation. Add the
Script Ductus path to its fixed `DOC_SHARD_PLANS` registry with:

```text
headingLevel: 3
newestFirst: true
```

The registry remains an allowlist, so CLI arguments cannot select arbitrary
files. Existing realpath, symlink, filename, parse, ordering, and content guards
apply unchanged. `check:doc-shards` validates the new directory and rejects a
tracked or locally present aggregate unless it is the explicit ignored render
case defined by the existing tool.

Script Ductus exposes package-local `shard:docs`, `unshard:docs`, and
`check:doc-shards` commands that invoke the shared compiled tool with the fixed
path. Its BUILD dependency chain installs and builds Human Language Data before
running those commands; it does not copy the sharder.

## 5. Migration proof

After #13279 lands, split the fresh-main monolith and prove:

1. concatenating committed shards reproduces its bytes exactly;
2. the pre-migration SHA-256, byte length, and level-3 section count match pins
   recorded during migration;
3. every historical entry appears exactly once and in the same order;
4. the existing forward-only #13275 fragment is reconciled without duplicating
   its entry; and
5. deleting the monolith changes no package/runtime output because no runtime
   consumer imports it.

The proof values are taken only after #13279 clears the contested file. Tests
written before that merge specify the boundary but deliberately do not freeze a
stale intermediate hash.

## 6. Enforcement

CI and package checks fail when:

- `CHANGELOG.md` is tracked or recreated beside the canonical shards;
- a fragment name is malformed or a shard is a symlink/non-regular file;
- a fragment contains zero or multiple level-3 entries;
- a filename digest disagrees with its heading;
- shard order is ambiguous or does not reproduce document order; or
- a locally rendered aggregate differs from the shard reconstruction.

Contributor documentation points release-note authors at a new uniquely named
fragment, never at the monolith.

## 7. Acceptance

Completion requires:

1. the monolith is absent and every historical byte is recoverable;
2. new Script Ductus work touches one changelog fragment only;
3. package-local and repository document-shard checks enforce the boundary;
4. Human Language Data, Script Ductus, and affected-package builds pass;
5. an independent security review finds no unresolved filesystem or destructive
   migration issue; and
6. the PR is auto-merged only after all required CI gates pass.
