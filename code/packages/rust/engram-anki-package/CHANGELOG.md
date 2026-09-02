# Changelog — engram-anki-package

## Unreleased


### Fixed — the Anki fixtures' scheduling state was still ours

The corpus added for #13940 was exported by Anki but had its scheduling columns
**assigned by hand first** (`card.type = 2`, `card.ivl = 21`, `card.left =
1001`), so the container was Anki's while the semantics were still our own
belief in an Anki wrapper — one level in from the problem the corpus exists to
solve. The generator's own comment said as much: "Anki packs two numbers into
`left`. Our importer has an opinion about that encoding which nothing has ever
checked against a real file" — immediately before writing that opinion into the
file.

Fixtures are now produced by answering cards through Anki's scheduler. Two
values changed in ways that matter:

- learning `left` was `1001`, the older packed step encoding. Anki 26.08.1
  writes **`2`**.
- filtered `odue` was `0`. Anki writes **`5`** (the original due day), with
  `due` set to `-100000`. With `odue` at zero, a reader that ignored the column
  entirely would still have passed.

### Added — the import tests now assert against those fixtures

Five of the seven Anki-authored fixtures were committed and never read by any
test; only `anki-modern` and `anki-review-scheduled` were, and the latter only
for its schema version. `tests/anki_authored_oracle.rs` asserts our reader
against what Anki actually wrote: the day-number/timestamp split between review
and learning `due`, the distinct negative `queue` values for suspended and
buried, cloze expansion to one card per deletion, `odid`/`odue` on a filtered
card, and both members of the media map.

This is #13940's "done when" — real Anki-produced fixtures, asserted against.

**Modern `.anki21b` / `.colpkg` packages now actually import.** They previously
decompressed and then failed with `invalid Anki V11 JSON in col.conf: EOF while
parsing a value at line 1 column 0`.

The zstd work made the payload readable; the schema underneath is a different
database. At **schema 18** the `col` table's `conf`, `models`, `decks` and
`dconf` columns are **empty**, and the same data lives in relational tables:
`notetypes` + `fields` + `templates`, `decks` + `deck_config`, `config`, `tags`.
So `col.conf` is not malformed JSON — it is the empty string, exactly as Anki
wrote it.

Dispatch is on `col.ver`, not on which archive member was found. Real Anki
legacy exports ship **both** `collection.anki21` and `collection.anki2` in one
archive, so the member name says how the bytes were packaged while the version
says what they contain.

Two things made this more than a parser branch. Five of the tables — `fields`,
`templates`, `config`, `tags`, and `graves` — are `WITHOUT ROWID`, which SQLite
stores as index b-trees; `sqlite-file` reads those, but through a different
entry point, and calling the ordinary reader returns `unexpected b-tree page
type`. `graves` is the trap, because its *columns* are identical to V11's, so
it fails one level below where you look. `notes`, `cards` and `revlog` keep
their rowids and are read by the existing code unchanged.

And the configuration columns are **protobuf**, not JSON. Field numbers were
decoded from archives produced by real Anki rather than recalled: the cloze
discriminator was settled by exporting a collection with one Basic and one
Cloze note and diffing the blobs, which showed `kind` is field 1 and — the part
that matters — that a **normal note type omits it entirely**, since protobuf
does not encode defaults. Treating absent as "unknown" would have misclassified
every ordinary note type.

Verified against the real-Anki corpus: all seven fixtures import, and the
modern one is asserted on content — CSS from `notetypes.config`, field names in
ordinal order, and both template formats — rather than on having parsed.

Still using the repo's own zero-dependency `protobuf` crate; no third-party
protobuf enters the graph.

**Modern `.anki21b` / `.colpkg` packages now work in the browser**, and the
`modern-format` feature is gone.

That feature existed for one reason: the only zstd that could read real frames
was the third-party crate, which reaches libzstd's C sources through `zstd-sys`
and cannot target `wasm32-unknown-unknown`. So the browser build turned it off
and every modern package was refused with an explanatory message. The message
was honest, but it described a capability gap, not a design.

The repo's own `zstd` now decodes Huffman literals and FSE table descriptions
per RFC 8878 — what real encoders actually emit — and is pure Rust with no build
script. `decode_package_payload` and `encode_package_payload` route through it
unconditionally, so the feature, the `cfg` branches, the
`MODERN_FORMAT_UNAVAILABLE` message, and the split target-dependency in
`engram-core-wasm` all go away together. **Every target now gets the same
package layer with the same capabilities**, rather than a reduced browser
configuration that had to be kept honest separately.

The third-party crate stays as a **dev-dependency oracle**, the role `rusqlite`
already holds here. Modern-package fixtures are compressed by real libzstd and
read back by our decoder; a second test compresses with our encoder and decodes
with real libzstd, which is the direction that decides whether an exported
`.colpkg` opens in Anki. Building fixtures with our own encoder would only prove
we can read back our own bytes — the exact circularity that let the decoder ship
without Huffman support and still show green.

Our encoder emits raw literals with predefined FSE tables: valid Zstandard that
any decoder reads, but larger than libzstd at the same nominal level. That is a
deliberate trade — a correct frame on every target beats a smaller frame only
native builds can write.

The test that asserted the old refusal is replaced by one asserting the failure
mode that still matters: a modern member whose payload is not a zstd frame is
refused at decode rather than imported as a plausible-looking collection.

**Media expansion is now budgeted**, with the budget clamped at both ends and
computed in `u64`. An earlier draft used `usize::saturating_mul`, which on
`wasm32` saturates to `usize::MAX` for any archive over ~82 MiB — and since the
running total saturated to the same value, the comparison became
`usize::MAX > usize::MAX` and never fired. The control was inert at exactly the
sizes where it mattered. The arithmetic now has its own unit test, which fails if
the ceiling clamp is removed.

The ceiling is target-aware: 32 MiB on wasm, 256 MiB elsewhere. That is not
arbitrary — returning state through the JSON facade amplifies media roughly 24x
in transient memory (`Vec<u8>` serialises as a JSON number array through an
intermediate `Value` tree), so the wasm ceiling is set against the amplified
peak. It is a visible limit on browser media size; removing the amplification is
tracked in #13671.

Bounding each decode to the *remaining* budget, rather than measuring after the
allocation, is tracked in #13672.

**The lookup by name is no longer linear.** `ZipReader::read_by_name` scanned
entries; `read_media_files` calls it once per media entry, so the pair was
quadratic in entry count — and entry count is linear in archive size. A ~8 MB
archive with ~148,000 aliased entries sharing a long name prefix is on the order
of 10^10 string comparisons: a frozen tab, with no memory pressure and no error
to show for it, which the media budget cannot catch because the payloads are
tiny. `ZipReader` now builds a name index once.

 Nothing in the ZIP format stops a central
directory from listing thousands of entries pointing at the *same* local header,
and `read_media_files` read and **retained** each one. A ~1 MB archive was measured
expanding to over 1.3 GB of retained memory (a 1278x ratio) with plain stored
members; a DEFLATE bomb behind each alias multiplies that further.

Survivable natively — a failed allocation is an `Err`. Not survivable in a
browser, where `panic = "abort"` turns it into a module abort that takes the
user's unsaved collection with it. `read_media_files` now caps total decompressed
output at `max(16 MiB, 50 x archive size)` and fails with a clear message. The
budget is on output bytes rather than entry count, because entry count is not
what exhausts memory.

**Anki packages now work in the browser.** `zstd_crate` moves behind a new
default-on `modern-format` feature, making it optional. That is what finally lets
this crate build for `wasm32`.

The blockers were two C dependencies. `rusqlite` went when the export moved onto
`sqlite-file`'s writer; `zstd_crate` reaches libzstd through `zstd-sys`, and
`clang` has no `wasm32-unknown-unknown` target, so its build script failed
outright. Removing the first was necessary but not sufficient — the build simply
moved on to failing at the second.

Legacy V11 `.apkg` never needs zstd: `write_legacy_apkg` stores its members
uncompressed through the repo's own `zip` crate, and `decode_package_payload`
copies them straight back. So a build without `modern-format` still does full
legacy import and export — a genuine importer in the browser rather than the
previous state, where the whole package layer was compiled out of wasm and every
APKG call returned "handled by native hosts for WASM shells".

Modern `.anki21b` / `.colpkg` returns an explicit, actionable error there, naming
the legacy format as the way through. Deliberately an error rather than a
fallback to the raw bytes: handing back a zstd frame as if it were a collection
would surface later as corrupt data of unclear origin.

Native builds are unchanged — the feature is on by default, so full modern Anki
compatibility is preserved with no caller changes.

Verified in both configurations: the legacy golden `.apkg` round-trips pass with
`--no-default-features` (the configuration wasm uses), a new test asserts the
modern path fails with an actionable message rather than importing partial data,
and `cargo build --target wasm32-unknown-unknown` succeeds for this crate and for
`engram-wasm` with the package layer included.

**Fixed two DoS paths in revlog id assignment, both reachable from an imported
file.** Review ids come straight from the revlog b-tree's cell rowids, and
`walk_table` sorts but does not deduplicate, so a crafted `.anki2` controls them
entirely.

`unique_review_id` resolved collisions by linear probing. That did not terminate
at `i64::MAX` — `saturating_add` became a no-op and the loop spun at 100% CPU
forever. It was also **quadratic**: the function normalises every id `<= 0` to
`1`, so N revlog rows with rowid `0` all start at the same candidate and row *k*
probes *k* times. Measured at ~91s for 80k rows and about an hour at 500k, from
an `.apkg` of tens of kilobytes once zipped, since the rows compress well.

Since the ids only need to be unique, a collision now jumps past the highest id
handed out so far rather than walking to it — O(1) per review, one step always
sufficient because `highest + 1` is free by construction. `checked_add` keeps
exhaustion an error rather than a hang.

Regression tests cover both: two ids at `i64::MAX` must error rather than loop,
and 50000 rows all colliding on candidate 1 must produce 50000 distinct ids (that
test alone would take roughly 35 seconds under the old probe; it now runs in
0.06s).

Pre-existing rather than introduced by the port, fixed because the export path
calls it.

**The V11 export no longer uses `rusqlite`.** `write_v11_collection_bytes_from_engram_state`
built its `.anki2` database by opening an in-memory `rusqlite` connection, running
a `CREATE TABLE` batch, executing five `INSERT` loops, and calling `serialize()`.
It now builds rows directly and emits the file with `sqlite-file`'s
zero-dependency writer (`write_multi_table_db_with`).

`rusqlite` and `tempfile` move to **dev-dependencies**. The production graph
therefore links no bundled C SQLite, which is what the `wasm32` build needs — the
C amalgamation cannot target it. `rusqlite` stays as a test **oracle**, the same
role it holds in `sqlite-file`'s cross-check suite: the import tests build genuine
`.anki2` fixtures with the real C library and assert our reader decodes what
SQLite actually wrote. Replacing that with our own writer would make it circular.

The five `CREATE TABLE` statements are preserved verbatim as constants — they are
stored in `sqlite_schema.sql` and SQLite reparses them on open, so they are
load-bearing bytes.

**The rowid-alias trap.** `col`, `notes`, `cards`, and `revlog` declare
`id integer primary key`, which SQLite treats as an alias for the rowid: the
record stores NULL and the value travels in the cell's rowid. `graves` has no such
column, so all three of its values are stored and its rowids are synthesised.

This one is worth spelling out because it fails silently. Writing the id as an
integer instead of NULL was tried deliberately, and **all 42 tests passed** —
including both golden `.apkg` round-trips, `SELECT id FROM notes`, and
`PRAGMA integrity_check` on the export opened in real SQLite. SQLite reads the
rowid for an alias column and never consults the stored value, so no SQL-level
check can see the difference. The export is now gated by an assertion on what is
actually *stored* (`sqlite_file::read_table` returns the record's columns
alongside the rowid), which does fail on that mutation.

New test `exported_v11_collection_opens_in_real_sqlite` hands the export to
bundled-C SQLite and asserts `integrity_check`, `user_version = 11`, all five
tables reparsing from their stored DDL, row counts, and the rowid-alias contract.

Note that this does **not** yet make the crate `wasm32`-buildable: `zstd_crate`
remains an unconditional production dependency and its `zstd-sys` build script
cannot target wasm either. Gating that behind a feature is tracked separately.

## Unreleased

### Started sqlite-file reader cutover for Anki V11 imports

`parse_v11_collection_bytes` now reads the `col`, `notes`, `cards`, `revlog`,
and `graves` tables directly from raw SQLite bytes through the repo
`sqlite-file` reader instead of deserializing the collection into an in-memory
`rusqlite` connection. This removes the unsafe serialized-rusqlite import path
(`sqlite3_malloc64` + `OwnedData::from_raw_nonnull`) while keeping the existing
owned Anki V11 representation and APKG round-trip tests unchanged.

`rusqlite` remains in this crate for the SQLite writer and test fixtures until
Phase F of the zero-dependency roadmap replaces collection export as well.

### Removed third-party `prost` — Anki `meta`/`media` protobuf is now zero-dep

The Anki `.apkg` `meta` (package version) and `media` (filename/size/sha1 map)
messages were encoded/decoded via the third-party `prost` derive. They are now
hand-coded against the repo's zero-dependency `protobuf` wire crate
(`code/packages/rust/protobuf`), removing `prost` from this crate's dependencies.

The hand-coded `encode_pb`/`decode_pb` implementations follow proto3 semantics
(implicit-presence scalar fields omitted at their default; explicit-`optional`
`legacy_zip_filename` emitted when `Some`). Before the cutover, a cross-compat
gate asserted they produce **byte-for-byte identical output to `prost`** and
round-trip its bytes across edge cases (empty maps, zero-size entries, `Some(0)`
optionals, non-ASCII filenames, multi-entry maps) — guaranteeing continued
real-Anki `.anki21b` interoperability. All 41 crate tests, including the APKG
round-trip and the checked-in golden fixture, pass unchanged.

Phase A of the Engram zero-dependency plan
(`code/specs/engram-zero-dep-plan.md`). Remaining third-party deps in this crate
(`rusqlite`, `zstd_crate`, `serde`/`serde_json`) are removed in later phases.
