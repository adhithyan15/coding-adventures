# Sharding `algol-iir-compiler`'s Changelog

## Goal

Second application of the mechanism specified in
[`lang-aot-changelog-sharding.md`](lang-aot-changelog-sharding.md). That
document carries the design, the gate analysis, and the reasoning about why the
registry's two hand-kept guard lists must be extended alongside it; none of
that is repeated here.

`code/packages/rust/algol-iir-compiler/CHANGELOG.md` is 2,382 lines and 275
newest-first `## <version> — <date> — <title>` sections. Every PR prepends one.
That is the same shape as the Language Ladder and lang-aot plans, so it splits
at `headingLevel: 2` with `newestFirst: true`.

## Measurement

Re-derived on current `main` rather than carried over — the repo moves fast
enough that a three-day-old ranking is a different ranking. Median PR
open→merge is now **1.16h** across the last 100 merged PRs (it was 1.19h when
lang-aot was measured).

Over 21 days, counting touches that land within that window of the previous
touch, restricted to files still on `main`:

| close | total | ratio | path |
|---:|---:|---:|---|
| 63 | 111 | 57% | `code/specs/data/adj-facts-stdlib/CHANGELOG.md` |
| 57 | 79 | 73% | `code/specs/spice-full-implementation-plan.md` |
| 45 | 94 | 48% | `code/specs/LANG-FULL-IMPLEMENTATION.md` |
| 45 | 61 | 75% | `code/specs/RUST-CPU-SIMULATOR-BACKLOG.md` |
| 44 | 60 | 75% | `code/specs/oauth.md` |
| **41** | 89 | 47% | **`code/packages/rust/algol-iir-compiler/CHANGELOG.md`** |

`lang-aot/CHANGELOG.md`, which led this table at 90/143, has dropped out of it
entirely — it is sharded and no longer on `main`. The fix shows up in the
measurement, which is the cheapest available confirmation that the metric
tracks the thing it claims to.

### Why the higher-scoring rows are not the target

- **`adj-facts-stdlib/CHANGELOG.md`** scores higher and cannot be sharded by
  this tool at all: 10,541 lines with exactly **one** `##` heading and 323
  top-level `- ` bullets. `splitDocument` takes `headingLevel: 2 | 3`, so it
  would emit one useless shard. Teaching the tool to split bullet lists is
  separate work.
- **`spice-full-implementation-plan.md`, `LANG-FULL-IMPLEMENTATION.md`,
  `RUST-CPU-SIMULATOR-BACKLOG.md`, `oauth.md`** are prose documents (5–11 `##`
  sections across 700–5,600 lines), not entry lists. Their contention is
  plausibly *semantic* — two people editing the same argument — which sharding
  does not fix and would disguise. They are deliberately not migrated on this
  evidence.

## A caveat this file has and lang-aot did not

`algol-iir-compiler/src/lib.rs`, `Cargo.toml` and `CHANGELOG.md` score
**identically**: 41 close touches of 89, same last-touched day. Identical
scores mean they are always touched together — every PR to this package edits
all three.

So sharding the changelog removes **one of three** conflict surfaces here, not
the whole problem. It is still worth doing, because the three are not equal: a
changelog conflict is pure noise whose resolution is always "keep both", while
a `src/lib.rs` conflict is genuine work that a human should look at.

lang-aot was a stronger case on this axis — its changelog scored 90 against
`lang_matrix.rs`'s 81 and `Cargo.toml`'s 62, meaning PRs touched the changelog
*more* often than its siblings. Recording the difference so the next reader
does not infer a bigger win than this delivers.

## Scope

One registry row, one shard directory, one `.gitignore` line, and one entry in
each of `doc_shard_globs` and `tracked_doc_monoliths`. The tests added by
#15215 pin both guard lists against `DOC_SHARD_PLANS`, so omitting either fails
a test rather than silently shipping unguarded history.
