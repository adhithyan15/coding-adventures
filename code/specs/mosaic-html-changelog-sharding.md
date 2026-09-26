# Sharding the Mosaic and HTML Changelogs

## Goal

Move eleven Mosaic and HTML package changelogs into `CHANGELOG.d/`, using the
mechanism from [`lang-aot-changelog-sharding.md`](lang-aot-changelog-sharding.md),
the bullet entry shape from
[`doc-shard-bullet-entries.md`](doc-shard-bullet-entries.md), and the batch
procedure from [`remaining-changelog-sharding.md`](remaining-changelog-sharding.md).

Parallel PRs on the Mosaic emitters, the artifact builder and the HTML stack
keep conflicting at the top of these files. The user's standing direction is
that changelogs are split.

## How each mode was chosen

A plan's mode decides two things. It decides how history is cut, which is
cosmetic because the round trip is byte-exact. It also decides what a new shard
must start with, because `--check` rejects a section shard whose first line is
not the plan's entry start. That second effect is a convention for every future
author, so the mode follows what recent commits actually insert:

- `###` entries under an `## Unreleased` banner → `headingLevel: 3`;
- dated `## 2026-09-24 (topic)` sections at the top → `headingLevel: 2`;
- bare `- ` entries under the banner → `entryShape: "bullet"`.

Everything above the first entry becomes `_meta.md`. A mode that would leave
the live insertion point inside `_meta.md`, or inside one large shard, does not
fix the conflict and was rejected.

`newestFirst` is `true` for all eleven, from direct evidence: the commits
measured below insert at the top of the document, never at the bottom.

"Touches" is `git log --since="21 days ago" --oneline -- <path> | wc -l` on
2026-09-26.

## The eleven, measured

| document | mode | entries | recent inserts | touches | lines |
|---|---|---:|---|---:|---:|
| `mosaic-package-artifact-builder` | heading 3 | 54 | 8/8 `###` at line 5 | 54 | 2,317 |
| `mosaic-emit-compose` | heading 2 | 15 | 11/12 `##` dated at the top | 54 | 1,652 |
| `mosaic-emit-flutter` | heading 3 | 67 | 8/8 at line 8 (3 `###`, 5 bullets) | 35 | 1,876 |
| `mosaic-emit-xaml` | heading 3 | 90 | 7/8 at line 5-6 (5 `###`, 2 bullets) | 22 | 2,566 |
| `mosaic-emit-swiftui` | heading 2 | 11 | 4/5 `##` dated at line 3 | 20 | 1,068 |
| `mosaic-emit-html` | bullet | 84 | 6/8 bullets at line 5 | 18 | 590 |
| `mosaic-app-bindings` | heading 3 | 19 | 8/8 `###` at line 5 | 10 | 642 |
| `html-parser` | bullet | 196 | 8/8 bullets at line 8 | 15 | 674 |
| `programs/mosaic/venture-browser` | bullet | 104 | 8/8 bullets at line 3 or 5 | 37 | 364 |
| `programs/mosaic/engram-app` | heading 3 | 19 | 8/8 `###` at line 5 | 17 | 1,224 |
| `programs/mosaic/journal-app` | bullet | 20 | 8/8 bullets at line 5 | 16 | 104 |

The first nine rows are under `code/packages/rust/` unless a path says
otherwise.

### Shape notes

- **Compose and SwiftUI** switched conventions recently. Older entries are
  `###` under `## [Unreleased]`, and the newest are dated `##` sections above
  it. Level 2 follows the newest convention. The `## [Unreleased]` block then
  becomes one large shard of history. Level 3 would put every dated section
  into `_meta.md`, above all new entries.
- **Flutter and XAML** mix `###` entries with a few bare bullets written
  directly under the banner. Level 3 keeps every `###` entry whole. The one
  (Flutter) or two (XAML) bullets above the first `###` stay in `_meta.md`.
  After this change, new entries are `###` shards.
- **mosaic-emit-html and html-parser** take new entries as bare bullets above
  older `###` groups. Under bullet mode those `###` lines attach to the end of
  the preceding entry's shard and return to the same place on rejoin. Version
  markers already behave this way in `spice-netlist-parser`.

## Left whole, and why

| document | reason |
|---|---|
| `programs/mosaic/task-app` | Has a runtime reader. `release-task-app.yml` runs `taskapp_release.py validate-changelog --changelog code/programs/mosaic/task-app/CHANGELOG.md` on every PR that touches Mosaic emitters. Making the file generated would break that gate until the release script reads shards. |
| `mosaic-emit-qt`, `mosaic-emit-react` | Same mixed shape as Compose. Level 2 yields 7 sections, which is below the real-document test floor (`MIN_SHARDABLE_ENTRIES = 10`). Level 3 would put the 4 newest dated sections into `_meta.md`. Bullet mode would separate their 59 and 53 `###` headings from the entries they introduce. |
| `programs/mosaic/visicalc` | Three live conventions: dated `##`, `###` entries, bare bullets. Levels 2 and 3 yield 5 and 7 sections, below the floor. In bullet mode `_meta.md` would end with `## 2026-09-20`, so every new entry would render under that stale date. |
| `html-lexer`, `state-machine-tokenizer` | Keep-a-Changelog category groups (`### Fixed`, `### Added`) under `## Unreleased`, holding bullet entries. Bullet mode would file every new entry under whichever category is first. Level 3 yields 5 and 3 sections. Both files had 2 touches. |
| `mosaic-package-manifest` | Level 3 yields 6 entries, below the floor. Bullet mode would leave the 70 newest lines in `_meta.md`. 4 touches. |
| `mosaic-app-runtime` | 7 bullet entries; 3 touches. |
| `html-tree-builder` | 9 bullet entries, below the floor. |

If the floor is relaxed for documents with a measured hot spot, Qt, React and
html-tree-builder are the next candidates.

## Readers checked first

Every committed reference was searched by `<package>/CHANGELOG` and by
`CHANGELOG` inside each package, which covers both the short form and the full
path. For the eleven migrated documents, the only reader is a prose pointer in
`mosaic-emit-flutter/README.md` ("See `CHANGELOG.md` for the full feature
matrix"). It now points at `CHANGELOG.d/`. No `BUILD`, `Cargo.toml`, workflow,
test or script reads any of the eleven files.

## What each migration touches

These are the same places the earlier batch touched:

- a `DOC_SHARD_PLANS` entry with the measurement above;
- the generated `CHANGELOG.d/`, including `_meta.md`;
- a root `.gitignore` line for the rendered `CHANGELOG.md`;
- `git rm` of the tracked monolith;
- `doc_shard_globs` and `tracked_doc_monoliths` in
  `human-languages-books.yml`, pinned by `doc-shard.test.ts`.

Each document is round-tripped on its own. `--unshard` output is compared with
the pre-migration blob by `diff`, and the rendered file is then removed.
