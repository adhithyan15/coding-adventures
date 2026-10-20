# Closure Compiler on Rust — parity backlog

**Status:** active. Reset on 2026-10-19 after an eight-day stall (see below).
**Driver:** one Claude loop session. It works this queue top to bottom, one
item per PR. Clean, green PRs squash-merge automatically (owner authorization,
2026-10-19).
**Upstream audit base:** `google/closure-compiler` at
`10ca677aff381d2c2e6e1b254ba32861e503173d` (2026-09-17), release `v20260915`.
**History:** the full pre-reset file, including every loop-record row and
evidence log, is at commit `74ffd48a0bc0c1d71cb001a53952ea43d25ba51c`
(`git show 74ffd48a:code/specs/CLOSURE-COMPILER-RUST-BACKLOG.md`).

## Why the backlog was reset

From 2026-09-19 to 2026-10-07, small Closure PRs merged several times a day.
Then nothing merged from 2026-10-07 to 2026-10-19. Four rules in this file
caused that, and the reset removes each one.

1. **One item gated all the others.** On 2026-10-07 provenance (CCR-065) was
   made "a completion requirement". Its acceptance test covers nearly the whole
   compiler, so source maps, `language_in`, destructuring and ADVANCED value
   propagation all waited behind it. Provenance is now one ordinary item. It
   ships in slices like everything else.
2. **Everything went into one PR.** "Preserve one active Closure/shared-stack
   PR" was read as "put every change in that PR". PR #17288 grew to 1,146
   commits and 909 files. About 55 of those files were Closure work; the rest
   were mermaid, diagram, ALGOL and CI repairs absorbed while catching up with
   main. It conflicted with main in 15 non-Closure files. CI never completed
   green on it: 27 of 30 runs were cancelled by newer pushes, and the 3 that
   finished failed in unrelated packages. It was closed on 2026-10-19. The lexer
   source-interval work was salvaged as #17803.
3. **The bar for "done" rose faster than work could meet it.** It required an
   exact-head, whole-affected-workspace re-validation after every merge from
   main, and a final Miri run before anything counted as complete. Main lands
   many commits a day, so the PR never caught up.
4. **Status logs replaced the plan.** Checkpoint notes overwrote the PR
   description, the epic #15830 description (restored from edit history on
   2026-10-19) and this file.

## Working rules

These rules exist to keep the loop moving. If a rule and progress conflict,
stop and ask the owner. Don't stretch the PR to satisfy both.

1. **One item, one PR.** A PR contains the selected item's code, tests, spec,
   README and changelog shard, and nothing else. Aim for under ~1,500 changed
   lines. If an item is bigger, split it into numbered slices, each a PR.
2. **Never fix unrelated breakage inside a Closure PR.** If CI fails in a
   package the item does not touch, check whether main fails the same way.
   - If it does, open an issue, or a tiny separate fix PR when it blocks
     merging, then continue.
   - If only the Closure PR fails, the item caused it. Fix it in the item's own
     files.
3. **Sync with main only when needed.** Sync when GitHub reports a conflict or
   a required check needs newer main, and use a real merge. Never copy main's
   content into the branch as ordinary commits.
4. **Done means the item's acceptance tests pass, and required CI is green on
   the PR.** Run `cargo test -p` locally for the crates the PR touches, plus
   `closurec` when a pipeline crate changes. CI covers the rest. Don't add Miri
   gates unless the PR adds `unsafe`.
5. **Short status, in the right place.** The PR description says what changed
   and how it was tested, in plain prose. Progress notes go in issue
   *comments*, never in issue or epic descriptions. This file changes only when
   an item is selected, merged or added.
6. **Timebox.** If a PR has gone 24 hours or three CI rounds without turning
   green, stop and report to the owner instead of widening its scope.
7. **No hidden gates.** An item waits only when its row says `Blocked by` and
   names the blocking item.
8. **Shared crates.** At most one open PR at a time may touch a given shared
   crate (`javascript-ast`, `javascript-parser`, `lexer`,
   `closure-pass-pipeline`, `cli-builder`). Other PRs may proceed.
9. **Owner decisions.** Items marked `Decision` need the owner's answer before
   any code. Ask once, then skip to the next item until it arrives.

Standing owner directions remain in force:

- Match Closure on **observable output**, not on internal pass structure.
- **Refuse what Closure refuses** (CCR-075, decided 2026-10-19). If the
  pinned upstream rejects an input, closurec rejects it the same way: the same
  diagnostic, location and exit status, and no output. closurec does not
  compile a superset of what Closure compiles.
- Base ESTree is the canonical serialized AST boundary, using our own types
  and no external AST crate (#15840).
- No new crates.io dependencies without a recorded decision. `serde`,
  `serde_json` and `regex` are tracked for removal (CCR-069, CCR-070).

## Where it stands

"Parity" is tracked at five levels: surface (CLI), language (parse, print,
map), pass (optimization behavior), build (modules, chunks, externs, maps,
reports) and operational (deterministic on Linux, macOS and Windows).

The differential ladder (`tests/diff/ladder_*`, 58 rungs × 3 levels, #15837)
measures agreement with the pinned oracle:

| Level | Rungs agreeing |
|---|---:|
| `WHITESPACE_ONLY` | 55 / 58 |
| `SIMPLE` | 36 / 58 |
| `ADVANCED` | 24 / 58 |

WHITESPACE_ONLY is strong. SIMPLE is real but partial. ADVANCED is mostly
scaffolding: the pass slots exist, but most of the transformations that define
that level are missing.

The 462-fixture golden corpus reports 100% agreement. It measures regressions,
not parity.

Other known gaps:
- Source maps are placeholders.
- The type checker is a passthrough.
- BUNDLE and TRANSPILE_ONLY are identity operations.
- Collapse-properties changes nothing.

## ID allocation

CCR IDs live in the GitHub issue tracker. Every item is an issue titled
`[closurec] CCR-NNN …`. To mint a new ID, take the next number above the highest
`CCR-` in any issue title, open or closed. Don't infer it from this file.
CCR-011–014, 016–021, 023–026, 028–030 and 032–040 predate that rule and exist
only as rows in the pre-reset file.

## Ordered queue

Work top to bottom. When an item merges, mark it done, then re-check whether
anything newly found outranks the next row.

Ranking:
1. Miscompiles, crashes and security.
2. Fake success: output that claims more than the compiler did.
3. Cheap ladder wins.
4. Front-end and output contract.
5. ADVANCED semantics.
6. Workflows and flags.

### Q0 — miscompiles, crashes, broken tooling

| # | ID | Item | Acceptance | Status |
|---:|---|---|---|---|
| 1 | #16008 | A top-level binding shadowed by a `for`-head binding is deleted, so the output throws a `ReferenceError`. | A ladder rung or regression test that fails before the fix; the output runs. | Ready |
| 2 | #16009 | Stack overflow (SIGABRT) on a long binary-expression chain, about 60 KB of input. | Parsing, the passes and emit stay iterative or bounded, and the hostile input compiles or fails cleanly with a diagnostic. | Ready |
| 3 | #15939 | `cargo clippy -- -D warnings` can't pass: `Cargo.toml` allows a lint that clippy 0.1.94 doesn't know. | Strict clippy passes on `closurec`. | Ready |
| 4 | #15942 | Add a ladder rung for an array spread before an index read, the gap the CLOC28 miscompile slipped through. | The rung is committed and agrees at every level. | Ready |
| 5 | CCR-078 #15863 | At SIMPLE, closurec propagates a const that upstream keeps. This output has never been validated. | Either match upstream, or record the divergence as intentional with runtime-equivalence evidence. | Ready |
| 5.5 | CCR-075 #15860 | closurec compiles input that upstream refuses, starting with private class elements. Refuse it the same way upstream does. | The `private_field` rungs and the CCR-081 `upstream_refuses` fixtures produce upstream's diagnostic key and message, location and exit status 2, with no output or sidecars. The ladder harness gains a "both declined identically" verdict that counts as agreement. | Ready |

### Q1 — cheap ladder wins (emitter and DCE parity)

Each of these is a small change to the emitter or DCE. Each also has to check
its blast radius across the 462 goldens.

| # | ID | Item | Status |
|---:|---|---|---|
| 6 | CCR-080 #15867 | The WHITESPACE_ONLY path drops the program's final terminator after `do…while`. | Ready |
| 7 | CCR-084 #15919 | Keep the `;` on a block-final `throw`. | Ready |
| 8 | CCR-083 #15918 | Brace-wrap a `do…while` used as a substatement. | Ready |
| 9 | CCR-087 #15924 | Unwrap a single-statement block under a label. | Ready |
| 10 | CCR-088 #15925 | Unwrap single-statement substatement blocks for every statement kind, not just expression statements. | Ready |
| 11 | CCR-076 #15861 | Drop braces around a single-statement body. | Ready |
| 12 | CCR-077 #15862 | Match upstream spacing and class-body punctuation. | Ready |
| 13 | CCR-085 #15920 | Strip `"use strict"` unless `--emit_use_strict` is set; re-emit it single-quoted. | Ready |
| 14 | CCR-082 #15878 | Truncate dead code after `continue`. | Ready |
| 15 | CCR-086 #15923 | `dce_program` truncates the dead tail after a top-level `throw`. | Ready |
| 16 | CCR-074 #15855 | The `while`→`for` rewrite hoists the preceding initializer into the `for` header. | Ready |

### Q2 — front end and output contract

| # | ID | Item | Status |
|---:|---|---|---|
| 17 | CCR-065 #16970 | Exact lexical source intervals in `lexer` and `javascript-lexer`. | In review — #17803 |
| 18 | CCR-007 #15624 | Real VLQ source maps through parser spans, passes, emitter and outputs. Build on #17803's intervals; ship parser spans, emitter mappings and multi-file/wrapper support as separate slices. | Blocked by #17803 |
| 19 | CCR-006 #15611 | An execution-backed capability matrix, so users can tell parse-only flags from implemented ones. | Ready |
| 20 | CCR-015 #15674 | Diagnostics plumbing: levels, groups, formatting, counts, streams and exit codes. Slice it. | Ready |
| 21 | CCR-042 #15571 | Match the legacy-octal warning diagnostics. | Blocked by CCR-015 |
| 22 | CCR-008 #15626 | Enforce `language_in` syntax boundaries. | Ready |
| 23 | CCR-079 #15866 | Derive the ladder's divergence ledger from captured oracle runs. | Ready |
| 24 | CCR-089 #15935 | Recapture the upstream-refusal evidence on the pinned JVM. | Ready |

### Q3 — ADVANCED semantics

| # | ID | Item | Status |
|---:|---|---|---|
| 25 | CCR-068 follow-up #15940 | Literal-property propagation should fire on more than one binding per program. | Ready |
| 26 | CCR-022 #15856 | Renaming covers loop locals, catch parameters and labels. | Ready |
| 27 | CCR-067 #15840 | ESTree migration: retire the hand-written bridge, one node family per PR. | Ready (spec first) |
| 28 | CCR-010 #15628 | Destructuring binding targets end to end. | Blocked by CCR-067 for the bridge part |
| 29 | CCR-065 #16875 | Connect the remaining call and composite fold lineage. | Ready |
| 30 | CCR-065 epic #15830 | Provenance slices P2–P6 from the epic: typed edges, per-node contributions, per-pass adoption (`inline` and `rename` first), a query surface, and a join with source maps. One slice per PR. | Ready after #29; P6 needs CCR-007 |
| 31 | CCR-009 #15627 | `language_out` lowering and TRANSPILE_ONLY, starting with optional chaining and nullish coalescing. | Ready |

### Q4 — workflows and flags

These are ordered by how widely they are used. Each issue carries its own
acceptance test.

CCR-025 (dependency modes, `goog.provide`/`goog.require`, ES modules), then
CCR-027 #15636 (BUNDLE), CCR-031 #15637 (JSON streams), CCR-045 #15638 (stdin),
CCR-051 #15705 (flagfiles), CCR-046 #15673 (charset), CCR-044 #15632,
CCR-049 #15703, CCR-052 #15710, CCR-048 #15693, CCR-050 #15704, CCR-054 #15726,
CCR-055 #15731, CCR-056 #15732, CCR-057 #15733, CCR-058 #15734, CCR-059 #15735,
CCR-060 #15737, CCR-061 #15738, CCR-062 #15739, CCR-015A #15724, CCR-047 #15683.

The larger pre-reset rows are CCR-014 (type checker), CCR-017 (collapse
properties), CCR-018–021 and CCR-023–024 (ADVANCED passes), and CCR-026–040
(chunks, externs, polyfills, reports, conformance, fuzzing, benchmarks, the
migration trial). Each one starts with a survey PR that splits it into issues.

### Decisions needed from the owner

None open.

### Filler (only while waiting on CI)

CCR-069 #15843 (remove serde), CCR-070 #15845 (in-repo regex), CCR-072 #15832
(CLI audit hash projection). None of them may displace a queue item.

### Recently completed

- **Before the stall:**
  - CCR-001–005, CCR-043, CCR-041, CCR-053, CCR-063, CCR-066 and CCR-073.
  - CCR-068 slices #15964, #16011 and #16019.
  - CCR-081.
  - The CCR-065 foundations: #16866, #16881, #16905 and #16964.
- **Since the reset:** none yet.

## Loop record

Add one line per selection or merge.

| Date | Event |
|---|---|
| 2026-10-19 | Backlog reset. #17288 closed; lexer intervals salvaged as #17803. Provenance is no longer a gate. Next: #16008. |
| 2026-10-19 | Owner decided CCR-075: refuse what Closure refuses. Added to Q0 as rank 5.5. |
