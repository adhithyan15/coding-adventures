### Fixed - every ledger read in `src/` goes through `shard.ts` (#12564, #12734)

- **The seventeen bare `JSON.parse(readFileSync(...))` sites in `loader.ts` are
  gone**, along with the last two outside it (`figure-cli.ts`,
  `track-progress-cli.ts`). They predate `shard.ts` and picked up none of the
  guards it added: the symlink refusal, the
  `__proto__`/`constructor`/`prototype` rejection, and the parse-error scrubbing
  that keeps V8 from splicing file bytes into a CI log. A test asserts the form
  does not reappear anywhere in `src/`, because a rule stated only in a comment
  is a rule that comes back — this file acquired seventeen of them one at a
  time, each reasonable on its own.
- **`readLedgerFile` now refuses a monolith whose `X.d/` exists**, which is the
  part that is not merely defence in depth. Since HL21 (#12690)
  `<track>/chapters.d/`, `<track>/curriculum.d/` and `core/book-generation.d/`
  are the source of truth and the `.json` beside each is a *generated artifact*.
  Between an edit to a shard and the next `--check`, that monolith holds stale
  bytes which parse, validate and look complete — so a direct read returned a
  plausible wrong answer with no error anywhere. It now throws, naming the
  directory that actually holds the data.
  - Refusing rather than guessing a merge is deliberate: three merge shapes
    exist today (`mergeMetaAndList`, `mergeSectionedShards`,
    `mergeGroupedShards`) and inventing one would emit a document no generator
    would produce. Callers that know the shape use `readMaybeSharded`.
  - The check is on **layout, not on a content comparison**. Comparing would
    mean merging the shards to find out — the work the caller was meant to do —
    and would let a read pass today and fail tomorrow for reasons no diff
    explains.
  - `shard-cli`'s `shardLedger` is the single caller entitled to opt out, via
    `readLedgerFile(path, { allowShardedSibling: true })`. `--shard` exists to
    rebuild `X.d/` *from* the monolith, so "the shards already exist" is the
    ordinary case there rather than the error.
- **The partial migration is preserved and now tested.** `chapters` is sharded
  in 20 of 23 tracks and `curriculum` in 22 of 23; `french`, `japanese` and
  `marwadi` keep their monoliths. A test loads the real corpus and asserts those
  three are still present with chapters, because `loadTrackChapters` treats an
  absent ledger as honest un-authored debt — so dropping them would have shrunk
  the corpus in silence and left every gate green on the remainder.
- **`isSharded` no longer reports "not sharded" for errors it did not
  investigate** (#12734). It collapsed *every* `lstat` errno into `false`. Only
  `ENOENT` and `ENOTDIR` mean absent; `EACCES`/`EPERM`, `EBUSY` (on Windows the
  search indexer, antivirus, or a sync client), `EMFILE`/`ENFILE` (which a
  102-file parallel vitest run genuinely reaches), `EIO` and `ELOOP` all mean
  "I could not tell", and now throw naming the errno. Reporting them as absence
  silently routes every reader to the generated monolith — the same failure
  class as the item above. The sibling bug in `doc-shard.ts` (#12731) made
  `--check` print "BACKLOG.d is missing" for a directory holding 109 shards;
  `SHARD_PLANS` covers 44 ledgers, so this path had 44 chances per run to do it.
  - The classification is the exported pure predicate `isAbsentErrno`, shared
    with `shard-cli`'s `statIfPresent` — which had already drawn the line
    correctly, so the right and wrong patterns sat side by side in one feature.
    One definition now, rather than two that can drift. Pure because `vi.spyOn`
    cannot patch a `node:fs` export under ESM, so this is the only way to test
    the errnos that matter without provoking them.
  - A **file squatting at `X.d`** also reported as "missing", sending the reader
    to restore a directory whose name is already taken. `shardLedger` refused
    that on the write side; the reader let it through. It now throws.
- `loadTrackGrammarCells` allowlists its `language` argument. It interpolated an
  unvalidated, caller-supplied id straight into a path, and `join` *normalises*
  an embedded `..` rather than refusing it, so the trailing `grammar-cells.json`
  was no protection. Every sibling loader that interpolates a track id already
  guards it; this exported one was the gap.
- `readLedgerFile`, `isAbsentErrno` and `ReadLedgerOptions` are exported from
  the package index, so a consumer outside it has somewhere to go other than a
  bare parse.

