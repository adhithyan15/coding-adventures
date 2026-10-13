# HL44 — Japanese changelog direct owners

## 1. Problem

The Japanese track changelog is a newest-first Markdown document whose entries
are independently authored alongside unrelated lessons and script work. At the
post-#17427 measurement revision, twenty-two of the newest two hundred scoped
human-language commits touched its single 98 KB file. Git therefore makes every
Japanese author contend for one prepend location even though release-note
entries do not share semantic state.

## 2. Ownership boundary

`code/learning/human-languages/japanese/CHANGELOG.d/` is the source of truth.

- `_meta.md` owns only the document preamble.
- Each level-2 changelog entry owns one strict rank/slug/heading-digest file.
- The rank orders entries newest first. Parallel entries may use the same rank;
  their heading slug and digest give them distinct filenames and deterministic
  tie ordering.
- `CHANGELOG.md` is an ignored local rendered view, never a tracked aggregate.

Adding one Japanese changelog entry must add one file and must not edit
`_meta.md` or any existing entry owner.

## 3. Migration contract

The migration slices the existing document at level-2 headings and must
reconstruct every pre-migration byte exactly. It may not normalize prose,
spacing, punctuation, or heading text.

The existing document-shard protections remain in force: exact plan matching,
real-file and symlink checks, safe repository-relative paths, canonical strict
filenames, filename/body agreement, append-only history, deterministic sorting,
and exact split/join reconstruction.

## 4. Regression contract

Focused tests must prove:

1. Japanese is registered as a newest-first level-2 document-shard plan.
2. The historical Japanese monolith reconstructs byte-for-byte from its direct
   owners.
3. The tracked monolith is absent from a clean checkout.
4. Two same-rank Japanese entries produce different owner filenames, render in
   deterministic order, and leave the shared preamble unchanged.

## 5. Verification

Run the focused document-shard tests, the human-language-data TypeScript build,
the repository document-shard check, the full package suite and coverage, and
the deterministic HL42 hot-file report after merge.
