# HL42 — A machine-readable human-language hot-file report

**Status:** specification, 2026-10-13

**Tracks:** #13193, #17416

---

## 1. Purpose

The human-language conflict-removal queue is ordered by measured contention,
not file size. Until this specification, each measurement was an ad hoc shell
pipeline whose revision, path scope, rename treatment, and commit window lived
only in the operator's terminal. A later audit could reproduce the numbers only
by guessing those choices.

The repository therefore provides a read-only command:

```sh
python3 code/scripts/human_language_hot_files.py --commits 200 --revision HEAD
```

It emits deterministic JSON to stdout. It writes no report file: a tracked
snapshot would itself become another aggregate that every author has to update.

## 2. History window and scope

The report resolves `--revision` to a full commit object before walking history.
It then selects the newest `N` first-parent commits that touched at least one
configured scope. The default scopes are:

- `code/learning/human-languages`
- `code/packages/typescript/human-language-data`
- `code/programs/typescript/language-ladder`
- `code/packages/typescript/script-ductus`

Repeated `--path` arguments replace those defaults. Paths are repository-
relative POSIX paths. Empty, absolute, dot-segment, NUL-containing, and
backslash-separated values are rejected before Git runs. Revision and path
arguments are passed as process arguments, never interpolated into a shell.

The JSON records the requested revision, resolved revision, requested and
effective window sizes, newest and oldest selected commits, traversal mode, and
the exact path scope. No wall-clock generation timestamp is emitted, so the
same repository state and arguments produce byte-identical output.

## 3. Counting rules

For each selected commit, the tool asks Git for NUL-delimited name/status data
with rename detection enabled. Each resulting path is counted at most once in
that commit.

- add, modify, type-change, copy, and delete statuses count against the path
  named by that status;
- a rename counts once against the destination path, not once against each of
  the old and new names;
- every rename is also recorded explicitly with commit, source, destination,
  and similarity score;
- an unsupported or malformed Git status is an error, not a skipped row.

File rows contain the path, number of commits touching it, status counts, the
full commit identities that contributed, and `present_at_revision`. The latter
is false when the newest selected status for that path is deletion. Deleted hot
spots stay visible as historical evidence without being mistaken for current
work. They sort by descending touch count and then raw Unicode code-point path
order. Status keys and commit lists have deterministic order.

## 4. Direct-owner families

A directory component ending in `.d` is a direct-owner family boundary. The
report aggregates touches beneath the first such boundary separately from file
rows. Each family reports:

- commits touching any member;
- total member touches;
- distinct member files touched;
- touches to `_meta.*` members;
- the maximum touch count of one member and every member tied at that maximum.

This distinction prevents a healthy family with 200 independently owned shards
from looking like one file touched 200 times. It also exposes a hot `_meta`
owner or a single repeatedly edited shard instead of hiding it behind the
family total. Files outside `.d` families remain ordinary file rows and cannot
be made to disappear through aggregation.

## 5. Output contract

The top-level object has `schema_version: 1` and these stable fields:

```text
schema_version
repository
revision
window
path_scope
files
owner_families
renames
```

Repository is the canonical absolute worktree root reported by Git. Paths
inside the report are repository-relative strings. Git output is decoded as
strict UTF-8; a non-UTF-8 repository path fails closed because emitting an
interoperable JSON filename would otherwise be ambiguous. Whitespace, tabs,
newlines, quotes, and other valid Unicode characters are preserved and escaped
by the JSON encoder.

Git failures, an unresolved revision, a non-positive commit count, malformed
name/status output, and unsafe scope arguments produce a concise stderr message
and a non-zero exit. Diagnostics never echo file contents.

## 6. Verification

Tests create disposable Git repositories rather than mocking Git. They cover:

- the exact recent-human-language commit window and scope filter;
- stable file and family ordering;
- rename accounting without source/destination double-counting;
- an empty scoped history;
- filenames containing spaces, tabs, newlines, quotes, and non-ASCII text;
- invalid counts, revisions, path arguments, and malformed status streams;
- byte-identical output for repeated runs at one resolved revision.

After implementation, the command is run at a named `origin/main` revision and
the resulting priorities are recorded on #13193. The generated JSON remains an
ephemeral measurement, not a committed artifact.
