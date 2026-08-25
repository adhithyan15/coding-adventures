### Added - BACKLOG.md and CHANGELOG.md are sharded documents (HL22)

- Extend the HL21 `X.d/` convention from JSON ledgers to Markdown documents:
  `src/doc-shard.ts` partitions a document at heading boundaries, and
  `src/doc-shard-cli.ts` provides `--shard` / `--unshard` / `--check` as
  `shard:docs`, `unshard:docs` and `check:doc-shards`.
- Migrate the two files every human-languages author touches — `BACKLOG.md`
  (100 of the last 200 human-languages commits, 107 sections) and this
  `CHANGELOG.md` (75 of 200, 451 entries) — so a new entry is a NEW FILE and
  five parallel level-authoring agents no longer serialize on two files. The 23
  per-language `<track>/CHANGELOG.md` files are already partitioned by track
  and are deliberately left alone.
- Number the shards by RECENCY RANK rather than by position, because both
  documents are newest-first: the topmost section takes the highest ordinal, so
  prepending an entry is an append in ordinal space instead of a reach into a
  shrinking gap that two agents would both grab.
- Identify each shard by an 8-hex SHA-256 digest of its heading, not by its
  ASCII-folded slug: `source-verified Tamil ர` and `source-verified Tamil த`
  fold to the same slug, and the digest is what keeps them apart.
- Introduce NO normalization. The rebuild is concatenation of verbatim slices,
  so both regenerated monoliths are byte-identical to the pre-migration files,
  asserted by `git diff --exit-code` and by a test over the real documents.
- Gate both monoliths with `check:doc-shards` in
  `.github/workflows/human-languages-books.yml` and
  `verify-human-languages.sh`, beside `check:shards`.

