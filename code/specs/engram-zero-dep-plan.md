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
| `regex` | engram-core (search.rs) | prod | hand-scanners + glob matcher; mini regex engine only for `re:` | **S…L** | `re:` user-search is the only true-regex need |
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
- New shared Unicode tables: combining-class / `is_combining_mark`,
  `\p{Alphabetic}`/`\p{Mark}`/`\p{Nd}`, canonical decomposition, NFC composition.
- Hand-write HTML-tag scanners + `*`/`_` glob matcher (replaces most regex uses).
- Wire NFD-fold (template + `nc:` search) and NFC dedup onto the tables.

### Phase D — `re:` regex engine (removes `regex` entirely) — L
- Mini backtracking/NFA engine for the subset Anki users type in `re:` search
  (`. * + ? [] () | ^ $ \d \w`, case-insensitive) + whole-word Unicode boundary.
- Consider feature-gating/deferring `re:` if it lets `regex` drop after Phase C.

### Phase E — sqlite-file reader (unblocks rusqlite-free **import**) — L (PRs A1–A5)
Per `code/specs/storage-sqlite.md` + the Python `storage-sqlite` port:
- E1: `sqlite-file` skeleton + varint + record codec (port `varint.py`+`record.py`). M
- E2: header + in-memory pager (borrow `&[u8]`, slice page N; no journal/cache). S
- E3: table b-tree walk — leaf(0x0D)+interior(0x05)+**overflow chains** → `(rowid, [SqlValue])`. L
- E4: sqlite_schema reader + `read_table(bytes, name)` API. S
- E5: cut anki-pkg **reader** over (`open_serialized_v11_collection` +
  `read_v11_*`), delete the `ffi`/`OwnedData` unsafe block. M — import runs rusqlite-free.

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
