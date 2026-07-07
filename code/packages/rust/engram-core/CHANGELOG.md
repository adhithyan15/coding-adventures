# Changelog — engram-core

## Unreleased

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
