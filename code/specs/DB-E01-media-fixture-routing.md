# DB-E01 — Exact Media Fixture Routing

DB-E01 registers the first two shared database-media vectors with the Go build
selector, package hash inputs, and Kiln graph export. For a changed fixture, the
current readers are exactly:

| Changed input | Python reader | TypeScript reader | C reader |
|---|---|---|---|
| `code/specs/fixtures/database-media-v1/nand-flash.json` | `python/nand-flash` | `typescript/nand-flash` | `c/nand-flash` (DB-E05) |
| `code/specs/fixtures/database-media-v1/hdd-simulator.json` | `python/hdd-simulator` | `typescript/hdd-simulator` | `c/hdd-simulator` (DB-E06) |

With all languages selected, editing one vector selects only the readers in
its row. A language-filtered build selects that fixture's reader in the
requested language if one exists. Of the database-media fixtures, each package exports only
its own as a repository input, so the same exact path enters its fallback
package hash and Kiln `repo_inputs`. The sibling vector, lookalike filenames and directories, and
unrelated database-media files do not select these consumers or enter their
input lists. No recursive glob is used.

The repository source-input boundary ledger authorizes each set of readers to read
only its matching shared vector. Its generated language projections, digest pins,
and count assertions must stay synchronized with the canonical JSON ledger.

Focused native Go tests pin positive selection, language filtering, sibling and
near-miss rejection, exact per-package input lists, and the corresponding Kiln
exports. This is build routing and cache-key behavior; it does not change media
simulation semantics.
