# Engram Zero-Dependency Plan

## Goal

Remove **every third-party crate** from the Engram Rust stack so it honours the
repo's zero-dep / no-third-party-FFI policy. Each dependency is replaced by a
from-scratch, zero-dep Rust implementation (a repo crate), never by another
external crate.

This is a **large, multi-PR, multi-session program** — it involves writing, from
scratch: an on-disk SQLite reader/writer, a complete RFC-8878 Zstandard codec, a
protobuf codec, an FSRS scheduler, a regex engine, Unicode normalization, and a
serde replacement (with a `#[derive]` proc-macro). It is sequenced leaf-to-root
so each landed PR is independently green.

## The dependency surface (audited)

| Dep | Crate(s) | Prod/test | Zero-dep replacement | Effort | Interop risk |
|-----|----------|-----------|----------------------|--------|--------------|
| `rusqlite` (bundled **C** SQLite, FFI) | engram-anki-package (prod), engram-capi (**test-only, already dev-dep**) | prod in anki-pkg | new `sqlite-file` crate (reader+writer) | **L** (8 PRs) | writer output must be readable by real Anki/sqlite3 |
| `zstd_crate` | engram-anki-package | prod | repo `zstd` — **currently INCOMPLETE** (see below) | **L** | must bidirectionally interop with real zstd (Anki `.anki21b`) |
| `prost` (protobuf) | engram-anki-package | prod | new tiny `protobuf` codec (4 messages) | **S/M** | must byte-interop with Anki `meta`/`media` protobufs |
| `fsrs 6.6.1` | engram-core | prod | reimplement forward FSRS-5/6 (~200 LOC) | **M** | **numeric** — must match crate output |
| `regex` | engram-core (**✅ DONE — now test-only dev-dep**; was prod in search.rs) | dev | zero-dep `regex-engine` (Pike VM) — all boolean uses + media `replace_all` moved over (Phase D); `regex` kept only as the `html_scan` cross-check oracle | **S…L** | `re:` user-search + media extent both run on `regex-engine` now |
| `unicode-normalization` | engram-core (search.rs, template.rs) | prod | NFD/combining-class tables (+ NFC for dedup) | **M** | Unicode tables shared with regex whole-word |
| `serde` + `serde_json` | engram-core, -core-wasm, -capi, -anki-package | prod | repo `json-value`/`json-parser`/`json-serializer` + **new `json-derive` macro** | **L** | **wire-critical** — JS/Swift/Anki parse the exact bytes |
| `tempfile` | engram-capi (**test-only, already dev-dep**), engram-anki-package (test) | test | drop with rusqlite | trivial | — |
| `getrandom` (wasm) | engram-core | prod (wasm target) | evaluate `web-sys`-free path / repo rng | S | — |

**Already-clean note:** `engram-capi` declares `rusqlite`/`tempfile` only under
`[dev-dependencies]` — its production ABI has no direct third-party dep except
`serde_json`. It picks up bundled SQLite only *transitively* via
engram-anki-package, so the SQLite/zstd/prost work belongs in anki-package.

## Two strategic decisions surfaced by the audit

1. **zstd interoperability (blocks the zstd swap).** The repo `zstd` crate emits
   the correct zstd magic (`28 b5 2f fd`) and self-round-trips, but a bidirectional
   cross-compat test against real zstd **fails both directions**: repo cannot
   decode real-zstd frames, and real zstd rejects repo frames ("data corruption").
   Its own docs admit it only supports **predefined FSE tables**, not the full
   FSE/Huffman table-description path that real zstd (and Anki) use. So a drop-in
   swap would **break real-Anki `.anki21b` interop**. Options:
   - **(a)** Complete the repo `zstd` to full RFC-8878 interop (full FSE + Huffman
     table decode; a compliant encoder). Large, but benefits the whole repo. *(recommended)*
   - **(b)** If Engram does not require real-Anki `.anki21b` interop (only its own
     round-trips + legacy `.anki2` for Anki interop), accept the self-consistent
     repo zstd. Smaller, but a behaviour/compat change.
   This is a **product decision** and gates the zstd milestone.

2. **serde replacement strategy.** No `#[derive(ToJson)]` macro exists in the repo
   JSON stack. Hand-writing `to_json`/`from_json` for ~57 types = ~114 functions +
   ~280 call-site rewrites across a wire-critical 11.8k-line facade. The
   high-leverage move is to **build a zero-dep `json-derive` proc-macro first**
   (honouring `rename_all`, `default`, `skip_serializing_if`), collapsing the
   problem back to a `derive`-swap. Recommended before touching any facade.

## Phased roadmap (leaf-to-root)

Effort: S ≈ ½ day, M ≈ 1–2 days, L ≈ several days / multiple PRs.

### Phase A — protobuf (removes `prost`) — S/M — ✅ DONE (#7574, #7578)
- New `code/packages/rust/protobuf` (or inline in anki-pkg): varint + wire-type
  0/2 encode/decode for the 4 messages (`PackageMetadata`, `PackageVersion`,
  `MediaEntries`, `MediaEntry`). Cross-test vs `prost` before swap. Remove `prost`.
- Self-contained; no wire-format risk beyond standard protobuf. **Best first win.**
- **Landed:** zero-dep `protobuf` crate + hand-coded `encode_pb`/`decode_pb`;
  byte-for-byte cross-verified vs `prost` before removal. `prost` gone.

### Phase B — FSRS (removes `fsrs`) — M (numeric risk) — ✅ DONE
- New `code/packages/rust/fsrs`: forward FSRS-6 — initial S0/D0, R(t), next
  difficulty (mean-reversion + linear damping), next stability
  (after-success/after-failure/short-term), interval, `memory_state_from_sm2`,
  `current_retrievability`, param upgrade + clipping. ~200 LOC scalar `f32`.
- **Gate met:** transcribed upstream 6.6.1 scalar path exactly; a throwaway
  cross-check asserted **5,900+ comparisons** vs the live crate (grid of
  retention × elapsed × random `(S,D)`, plus sm2 + retrievability) within `1e-4`
  rel-tol, then froze the exact upstream outputs as unit-test snapshots and
  removed the dev-dependency. Cut engram-core over via a one-line Cargo.toml
  swap (zero source edits — identical API); all 167 tests pass; `burn` is gone.
- **Note:** upstream `fsrs` uses `burn` only for *training*; the scheduling path
  we consume is pure scalar arithmetic, so no tensor code was needed. The
  `reducer.rs:3251` reference in the original plan did not exist; the only oracle
  tests were `scheduler.rs:564,599`, which now exercise the zero-dep crate.

### Phase C — Unicode + non-`re` regex (removes `unicode-normalization`, most of `regex`) — M
- **C1 — ✅ DONE (removes `unicode-normalization`).** New zero-dep
  `code/packages/rust/unicode-normalize` (Unicode 17.0.0): NFD/NFC + combining
  class + `is_combining_mark`, generated tables + algorithmic Hangul. Cross-verified
  vs the live upstream crate across **every scalar value** (~1.1M code points) +
  200k random strings — zero mismatches. engram-core swapped (2 `use` lines +
  Cargo.toml); 167 tests pass; `unicode-normalization` gone.
- **C2 — ✅ PARTIAL (tag-strip scanner only).** New `engram-core::html_scan`
  reimplements the `<[^>]+>` tag-strip (`DUPLICATE_HTML_TAGS`) as an explicit
  scanner, byte-verified vs the live `regex` across 300k random strings. The
  `regex` **dep stays** (still used by the media pattern + the search-match
  pipeline).
  - **Scope correction:** the *media* pattern
    (`DUPLICATE_HTML_MEDIA_TAGS`) and the `*`/`_` glob were originally slated as
    hand-scanners here, but they rely on regex **backtracking** that is a rabbit
    hole to hand-emulate (quoted values crossing `>`; alternation × trailing-`>`
    backtracking; `\s*` overlapping `[^ >]`). The right mechanism is the general
    engine below — so media/glob/whole-word/`re:` **all move to Phase D**, which
    now removes `regex` in one coherent step.
- Character classes (`\p{Alphabetic}`/`\p{Mark}`/`\p{Nd}`) to add if/when the
  regex engine needs them; `is_combining_mark` (Mark) already shipped in C1.

### Phase D — zero-dep regex engine (removes `regex` entirely) — L (PRs D0–D4)
Build a small zero-dep **Pike VM** (Thompson NFA) regex engine — linear-time, so
DoS-immune on user `re:` patterns, unlike a backtracker. Decomposed:
- **D0 — ✅ DONE (`is_match` core).** New `code/packages/rust/regex-engine`:
  parser → bytecode → Pike VM. Supports literals, `.`, `\d\D\w\W\s\S` (ASCII),
  `[]`/`[^]` ranges, `()`/`(?:)`, `|`, `*+?` and `{m,n}` (greedy/lazy), `^$`,
  `\b\B`, `(?i)`/`(?s)`, `case_insensitive`. Public surface = `Regex`,
  `RegexBuilder`, `is_match`. Cross-verified vs live `regex` (`(?-u)`) across
  **100k+ random (pattern,input) pairs** + CI sweep — zero `is_match`
  divergences. Iterative epsilon-closure (no stack overflow) + compile-size cap
  (no `{0,huge}` blowup). Not yet wired.
- **D1 — ✅ DONE (Unicode classes).** Generated Unicode tables (`unicode_tables.rs`:
  WORD/DIGIT/SPACE/ALPHABETIC/MARK/ND) from the `regex` crate; `\w\d\s` are
  Unicode by default (`(?-u)`=ASCII), `\p{Alphabetic|Mark|Nd}` + `\P` supported,
  `\b` uses the Unicode word set, `(?u)`/`(?-u)` flags. Cross-verified vs live
  `regex` in default Unicode mode across 80k+ pairs (non-ASCII inputs) — zero
  divergences. Unicode CASE FOLDING deferred to D2 (needed when wiring the CI uses).
- **D2a — ✅ DONE (Unicode case folding).** Prereq added to regex-engine (v0.3.0):
  `(?i)` uses Unicode fold orbits, cross-verified vs live `regex` (60k+ pairs incl.
  tricky orbits — σ/ς/Σ, Kelvin/Ångström, long-s, titlecase digraphs).
- **D2 (wiring) — ✅ DONE.** engram-core's `search.rs` now imports
  `regex_engine::{Regex, RegexBuilder}` for all three boolean uses — `re:`
  (`build_search_regex`), whole-word (`build_whole_word_regex` + the runtime
  `whole_word_pattern_matches`), and glob (`search_pattern_regex_source` compiled
  in `contains_search_pattern` / `search_pattern_matches`). Added
  `regex_engine::escape` (v0.3.1) for the glob builder (replaces `regex::escape`).
  The `regex` crate stays only for the media `DUPLICATE_HTML_MEDIA_TAGS`
  `replace_all` (needs match extents → D4). Full `cargo test -p engram-core`
  (170 tests incl. the Anki text-modifier search suite) passes on the new engine.
- **D3a — ✅ DONE (`find`, overall extent).** `Regex::find` returns the leftmost
  match's byte range/substring, tracking one extra `usize` (the match start) per
  thread — O(instructions) per step, no per-group vectors, so the DoS argument for
  `is_match` still holds. Solved the **nullable-loop** priority problem: `e{n,}`
  loops back to the body start and `e*` with a nullable body compiles as an
  optional-plus, so an empty iteration routes to the exit at the correct priority
  (`(a?)*`/`(a*)*` ⇒ whole run; `(a??)*`/`(a??)+` ⇒ empty — all matching `regex`,
  pinned by hand-verified unit tests). Verified by **property**, not span-equality:
  the live `regex` crate's own unanchored `find` is quirky on adversarial patterns
  (skips matches its anchored matcher accepts), so it's the wrong oracle. Using
  `regex`'s *anchored* matcher as an independent oracle, **40k+** random cases
  (greedy+lazy quantifiers, alternation, nested groups, nullable loops; multibyte)
  confirm every span is a *valid* match at the *leftmost* start; a separate **35k+**
  check confirms `is_match` stays exact across the same space (the property engram
  relies on — it never calls `find`). Reported extents can differ from `regex` only
  on lazy/overlapping-greedy-alternation corners, which the media pattern avoids.
  regex-engine v0.4.0.
- **D3b — ✅ DONE (captures).** `Regex::captures` + `Captures` type. Capturing
  groups compile to `Save` instructions bracketing the body (slots `2g`/`2g+1`;
  `0`/`1` = overall); the `captures` run carries a **copy-on-write** (`Rc`) slot
  vector per thread (branches share until a `Save` writes). `Save` is an epsilon
  no-op for `is_match`/`find`, so those paths are unchanged. **MAX_GROUPS=1000**
  DoS cap (rejects at build). Cross-verified vs live `regex`: 72k existence + 39k
  full-group comparisons (group boundaries agree wherever the overall span does;
  the lazy/overlapping-greedy overall-match corner from D3a is skipped, not group-
  compared). regex-engine v0.5.0.
- **D3c — ✅ DONE (`replace_all` + iterators).** `Regex::replace_all` (Cow return;
  `Replacer` = closure `FnMut(&Captures)->String` *or* `$N`/`${N}`/`$$` string),
  `find_iter`, `captures_iter`. Non-overlapping iteration matches `regex` (resume
  at prev end; skip empty match at that end). Cross-verified vs live `regex`: 84k
  iteration checks + 84k replace-output comparisons (byte-identical where the two
  iterate identically). regex-engine v0.6.0.
- **D4 — ✅ DONE (swap media + drop `regex`).** `DUPLICATE_HTML_MEDIA_TAGS` now
  compiles on `regex_engine::Regex` and its `replace_all` closure takes
  `&regex_engine::Captures` (same `get()`/`Match::as_str()` API — the media
  pattern is greedy with disjoint quote-alternation, so the new engine's leftmost
  extents match the old crate exactly; the full `engram-core` suite, incl. the
  media-dedup tests, is byte-identical). `regex` is **removed from
  `[dependencies]`** and kept only as a **`[dev-dependency]`** — its sole
  remaining use is the `html_scan` cross-check test's independent oracle. No
  non-test `regex::` reference remains in `engram-core/src`. **`regex` gone from
  the runtime graph — Phase D complete; the whole regex removal is done.**
- Rationale for the split: `is_match` (D0) is the bulk of engram's use and is
  cross-verifiable independently; extents (D3) are a genuinely separate, harder
  sub-problem, so they get their own PR rather than blocking the core.

### Phase E — sqlite-file reader (unblocks rusqlite-free **import**) — L (PRs A1–A5)
Per `code/specs/storage-sqlite.md` + the Python `storage-sqlite` port:
- E1: `sqlite-file` skeleton + varint + record codec (port `varint.py`+`record.py`). M
- E2: header + in-memory pager (borrow `&[u8]`, slice page N; no journal/cache). S
- E3: table b-tree walk — leaf(0x0D)+interior(0x05)+**overflow chains** → `(rowid, [SqlValue])`. L
- E4: sqlite_schema reader + `read_table(bytes, name)` API. S
- E5: cut anki-pkg **reader** over (`open_serialized_v11_collection` +
  `read_v11_*`), delete the `ffi`/`OwnedData` unsafe block. M — import runs rusqlite-free.

Status update: E1-E4 are now landed in `sqlite-file`; E5 has started by cutting
`engram-anki-package`'s V11 import tables over to
`sqlite_file::read_table(bytes, name)`. The reader path no longer needs the
unsafe serialized-rusqlite open helper; remaining `rusqlite` usage is writer and
test-fixture work for Phase F.

### Phase F — sqlite-file writer (**removes rusqlite**) — L (PRs A6–A8)
- F1: record/header/b-tree **write** — smallest-serial-type rule, leaf-insert,
  page splits → interior pages, overflow-write for large `col` JSON. L
- F2: sqlite_schema writer + `build(tables)` API. M
- F3: cut anki-pkg **writer** over (`write_v11_collection_bytes_from_engram_state`),
  **delete `rusqlite`** from Cargo.toml; port engram-capi's test-only sqlite
  fixture builder to the new writer, drop its dev-dep. **rusqlite gone from repo.**

### Phase G — zstd (**removes `zstd_crate`**) — L, gated on Decision 1
- If (a): complete repo `zstd` to full RFC-8878 (FSE + Huffman table decode +
  compliant encoder), cross-tested vs real zstd both directions, then swap.
- If (b): swap to repo zstd as-is, document the `.anki21b` interop limitation.

### Phase H — serde/serde_json (**removes `serde` + `serde_json`**) — L
- H0: build `json-derive` proc-macro + `to_value`/`from_value` bridge + non-panicking parse. M
- H1: `engram-core` (leaf) — 39 derives, 7 call sites, feature-gated. S/M — add golden byte snapshots first.
- H2: `engram-capi` — 0 own types, ~61 sites. M.
- H3: `engram-core-wasm` — the 11.8k-line facade, ~214 sites + `ok_with` funnel. L. Resolve the `Map` key-ordering question empirically first (sorted vs insertion order).
- H4: `engram-anki-package` — 17 camelCase+skip-if-none types. M. Byte-match Anki.

## Verification discipline (every phase)

- **Interop gates before swap.** For each format-compat dep (zstd, protobuf,
  sqlite, serde-wire), cross-test the new impl against the *real* one (or a real
  fixture) **before** deleting the third-party crate. The zstd probe already
  proved this catches non-interoperable replacements.
- **Golden snapshots** for numeric (FSRS) and wire (serde) outputs, frozen before
  the reimplementation, asserted after.
- Full `cargo test -p <crate>` for the crate and every downstream consumer; the
  Engram SwiftUI/Qt/native apps re-link `engram-capi`, so run those builds.
- Per repo standards: spec-sync, CHANGELOG, README, 95%+ coverage, literate code.

## Ordering rationale

Leaf-first minimizes churn and lets each PR stay green: protobuf and FSRS are
self-contained (do first); Unicode/regex share tables (bundle); the SQLite
reader ships a rusqlite-free import path before the (larger) writer removes
rusqlite entirely; zstd waits on the product decision; serde is last because its
ripple is largest and most wire-sensitive. `engram-anki-package` is the crate
that carries the most third-party debt (rusqlite + zstd + prost) — it is only
fully zero-dep after Phases A, F, and G all land.
