# Changelog — engram-core

## Unreleased

### Fixed — an imported archive name could make two provenance records collide

`external_source_merge_key` joined its four parts with `\u{1f}`, and two of them
are attacker-influenced when the collection came from an `.apkg`: a media
record's `target_id` is `format!("anki-media:{archive_name}")`, where the archive
name is a zip entry, and `original_id` carries the Anki `notes.guid`.

Moving a `\u{1f}` across the boundary between two parts produced the same key
from genuinely different records. Measured, not argued — both of these render as
`Media\u{1f}anki-media:art\u{1f}anki-v11\u{1f}later\u{1f}`:

| `target_id` | `source` |
| --- | --- |
| `anki-media:art\u{1f}anki-v11` | `later` |
| `anki-media:art` | `anki-v11\u{1f}later` |

`upsert_by` **replaces** on a key match, so one record silently displaced the
other.

The key is length-prefixed now — `{len}:{value}` per part — which makes the
encoding injective for any content at all, rather than for any content that
happens to avoid one character. A guard on the separator would have closed this
instance; the prefix closes the class, and needs nothing at the producers.

**Fixed at the encoding, which is the opposite of the choice made for the same
shape in card ids.** `engram-core-wasm` guards `{note_id}::{template_id}` rather
than re-encoding it, because a card id is written into saved collections,
snapshots and exported packages, so changing its shape rewrites data already on
disk. This key is derived per merge, is private to `merge.rs`, and is persisted
nowhere — so re-encoding costs nothing.

The target is length-prefixed too. Its `Debug` is a closed set of variant names
and no one of them is a prefix of another, so leaving it bare would in fact be
safe — but that is a case analysis a reader has to redo every time a variant is
added, and uniformity costs three characters.

Three tests, and the third is the one that makes the other two mean something.
Mutation-tested:

| mutation | fails |
| --- | --- |
| the old separator-joined key | collision, all-pairs |
| a key unique per record | **self-merge** |

A fix that made every key distinct would satisfy everything the first row checks
while silently turning de-duplication off, so re-importing a package would
accumulate duplicate provenance rows forever. The all-pairs test runs 10³
field-triples — separators at each end and in the middle, a `:` that could pass
for the length delimiter, digits that could pass for a length — and asserts the
count, so the loop is known to have covered what it claims.

**Scope, stated because the reviewer who found this was careful to:** this is
provenance metadata. No `Card`, `Note`, `NoteType` or `CardTemplate` id is ever
derived from a media asset id or an archive name, so a collision cost a row of
"where this came from", never a card. `merge_media_assets` nearby already renames
on an id clash rather than displacing, and is untouched.

### Fixed -- `rebuild_filtered_deck` moved cards into decks that do not exist

Rebuilding a filtered deck moves cards *out of the decks they are in*, and
`rebuild_filtered_deck_from_card_ids` did it without checking the destination
exists. It now declines, returning the state unchanged.

The visible refusal belongs to the callers -- `engram-core-wasm` checks first
and returns a JSON error -- but guarding only there left the mutation itself
willing to strand every card it touched. Two paths reach it without passing a
facade: `reduce(EngramCommand::RebuildFilteredDeck)`, which cannot report an
error because `reduce` returns `AppState`, and `rebuild_filtered_deck` itself,
which is publicly re-exported. A future consumer of either would have inherited
the bug rather than the fix, which is precisely how this family of defects kept
surfacing one route at a time.

Declining silently here is not trading loud for silent: there is no channel to
be loud on at this layer, so the choice is between doing nothing and corrupting
the collection. The callers that can speak still do.

### Fixed -- a deeply nested search query aborted the process

`SearchParser` is recursive descent with no depth bound. Every `(` costs a
`parse_primary` -> `parse_or` -> `parse_and` -> `parse_unary` -> `parse_primary`
cycle of stack frames, and every space-separated leading `-` costs a
`parse_unary` frame, so the query was a stack-depth dial that whoever supplied
the query got to turn.

This is not a panic. Verified locally: it is `fatal runtime error: stack
overflow, aborting`, SIGABRT, which `catch_unwind` cannot contain and no caller
can recover from. The browser build's stack is roughly 1 MB, a few thousand
frames.

It matters beyond a badly typed query because Engram renders browser props on
every props build -- including the one that follows a snapshot restore. A saved
query is therefore parsed with no interaction at all, so a corrupt or hostile
snapshot would abort on open, and abort again on every subsequent launch, with
no way back into the app.

Nesting past `MAX_SEARCH_DEPTH` (64) is now a `SearchError`. 64 is far past any
query a person writes and far short of any stack; a test pins both sides of the
boundary, since a limit that rejected ordinary queries would be a worse bug than
the one it fixes.

- Added `get_deck_stats_for_all_decks`, which computes every deck's stats in a
  single pass. `get_deck_stats_for_state` rebuilds its card-progress and
  imported-schedule indexes on each call, so asking it once per deck to render
  a deck list cost O(decks x cards) on every event -- measured at 12ms for one
  deck and 48ms for a hundred over the same 20,000 cards, now flat at ~10ms.
  The per-card classification is shared between both walks so they cannot
  drift, and a test asserts they agree deck for deck including nested decks.

### Search matching now runs on the zero-dep `regex-engine` (Phase D2)

The three **boolean** regex uses in `search.rs` — user `re:` patterns
(`build_search_regex`), whole-word matching (`build_whole_word_regex` and the
runtime `whole_word_pattern_matches`), and `*`/`_` glob matching
(`search_pattern_regex_source` compiled in `contains_search_pattern` /
`search_pattern_matches`) — now use `regex_engine::{Regex, RegexBuilder}`
(linear-time Pike VM, zero third-party deps) instead of the `regex` crate. The
glob-source builder's `regex::escape` calls become `regex_engine::escape`. All
170 `engram-core` tests, including the Anki text-modifier search suite (words,
combining marks, clozes, and `re:`), pass unchanged on the new engine.

The `regex` crate is **still a dependency** — the media-tag
`DUPLICATE_HTML_MEDIA_TAGS` pattern uses `replace_all` (a match *extent*, not a
boolean), which the engine gains in a later, separately-verified step (Phase D4,
after `regex-engine` adds `find`/`captures`/`replace_all`). Part of the Engram
zero-dependency program (`code/specs/engram-zero-dep-plan.md`, Phase D2).

### HTML tag-strip no longer uses `regex` (zero-dep step)

`rendered_search_text` (search) stripped HTML tags with the `regex` pattern
`(?is)<[^>]+>`. That step is now the hand-written `html_scan::strip_tags`
scanner, byte-for-byte verified against the live `regex` across 300k random
strings. The `regex` dependency is **not yet removed** — the media-tag pattern
and the search-match pipeline (glob/whole-word/`re:`) still use it and are
scheduled for the zero-dep regex engine (Phase D). Part of the Engram
zero-dependency program (`code/specs/engram-zero-dep-plan.md`, Phase C2).

### Removed third-party `unicode-normalization` — NFD/NFC is now zero-dep

Accent-stripping and canonical de-duplication in `search.rs` and `template.rs`
used the third-party `unicode-normalization` crate for `nfd()`, `nfc()`, and
`is_combining_mark`. That dependency is now the repository's own zero-dependency
`unicode-normalize` crate (`code/packages/rust/unicode-normalize`), a from-scratch
NFD/NFC implementation for Unicode 17.0.0. The consumed surface is identical
(`UnicodeNormalize` trait + `char::is_combining_mark`), so the swap is a two-line
`use` change plus the `Cargo.toml` dependency.

Before the cutover a cross-check asserted the new crate matches the live upstream
crate across **every Unicode scalar value** (~1.1M code points) and 200,000
random multi-character strings — zero mismatches. All 167 `engram-core` tests
pass unchanged; `unicode-normalization` is gone from the dependency tree.

### Removed third-party `fsrs` — FSRS scheduling is now zero-dep

FSRS-6 review scheduling (`scheduler.rs`) and retrievability ranking
(`search.rs`) previously used the third-party [`fsrs`](https://crates.io/crates/fsrs)
crate, which pulls in the `burn` tensor framework and dozens of transitive
crates in order to support parameter *training*. Engram never trains — it only
schedules — and the scheduling path is pure scalar `f32` arithmetic.

The dependency is now the repository's own zero-dependency `fsrs` crate
(`code/packages/rust/fsrs`), a from-scratch, forward-only reimplementation of
exactly that path. The public surface Engram consumes (`FSRS::new`,
`next_states`, `memory_state_from_sm2`, `current_retrievability`,
`DEFAULT_PARAMETERS`, `FSRS6_DEFAULT_DECAY`, `MemoryState`, `ItemState`) is
identical, so the swap is a one-line `Cargo.toml` change with no source edits.

Before the cutover a cross-check asserted the new crate matches the live upstream
`fsrs` 6.6.1 across **5,900+ comparisons** (within a `1e-4` relative tolerance;
in practice bit-for-bit). All 167 `engram-core` tests — including the FSRS
scheduler oracle tests — pass unchanged, and `burn` is gone from the dependency
tree.
