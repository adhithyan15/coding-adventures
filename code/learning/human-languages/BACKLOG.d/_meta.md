# Human Languages — backlog

Note on provenance: this file was EMPTY in git for its whole history until now.
Findings from the pre-A1 tranche work were recorded in commit messages and PR
bodies, which are durable and searchable, but a reader opening this file found
nothing. The entries below are the ones that change how the work is done.

This directory is the committed backlog. Each numbered Markdown file is one
entry, ordered newest-first by its descending ordinal. To build a temporary
single-file view from the human-language-data package, run its `unshard:docs`
script with `code/learning/human-languages/BACKLOG.md` as the argument.
The rendered `BACKLOG.md` is intentionally ignored: committing it would make
every otherwise independent curriculum branch edit the same generated file.

**Adding an entry.**

Backlog ids after rank `05000` are concurrency-safe. Choose the subject first,
then allocate its id from the package root:

```sh
npm run backlog:id -- "the exact subject after the em dash"
```

The command keeps the next readable decimal (for example `HL-C412`) and adds an
eight-hex fingerprint of the NFC subject. Two concurrent branches may therefore
choose the same decimal and rank without claiming the same id. Copy the exact
subject into `## <allocated id> — <subject>`; `npm run check:doc-shards` rejects
a missing or stale fingerprint. Historical ranks through `05000` remain
unchanged because this directory is append-only.
