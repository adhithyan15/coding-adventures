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
   *comments*, never in issue or epic descriptions. Record merges in this file
   at least once a day, as a small docs-only PR of its own: mark each merged
   row done with its PR number and close its issue. Don't edit this file
   inside item PRs, because concurrent PRs would conflict on it.
6. **Timebox an item, never the loop.** An item that has gone 24 hours, three
   CI rounds or three security-review rounds without converging is *parked*:
   - Keep its branch local. Push it only if its latest `/security-review`
     passed; never push a branch whose latest review failed or is missing.
     Parking never skips security review or CI.
   - Post one comment on its issue saying where it stands and what blocks it.
   - Mark its row `Parked`, and tell the owner in one line.
   - Then take the next row.
   If the way forward is a design question, write the spec as its own docs PR
   and mark the row `Decision`. The loop itself never goes idle while any
   queue row is Ready.
7. **No hidden gates.** An item waits only when its row says `Blocked by` and
   names the blocking item.
8. **Shared crates.** At most one open PR at a time may touch a given shared
   crate (`javascript-ast`, `javascript-parser`, `lexer`,
   `closure-pass-pipeline`, `cli-builder`). Other PRs may proceed.
9. **Owner decisions.** Items marked `Decision` need the owner's answer before
   any code. Ask once, then skip to the next item until it arrives.
10. **Near-root crates need the full build.** A PR that changes `lexer`,
    `parser` or `grammar-tools` rebuilds almost the whole repository, and one
    CI runner runs out of disk doing that. Add the `ci-full-build` label when
    you open such a PR, so CI shards the build.

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
| `WHITESPACE_ONLY` | 56 / 58 |
| `SIMPLE` | 37 / 58 |
| `ADVANCED` | 25 / 58 |

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
2. Rejecting valid JavaScript that Closure compiles, and fake success (output
   that claims more than the compiler did).
3. Cheap ladder wins.
4. Front-end and output contract.
5. ADVANCED semantics.
6. Workflows and flags.

### Q0 — miscompiles and wrong output

| # | ID | Item | Status |
|---:|---|---|---|
| 1 | F12 phase 0 | Add the ExpressionStatement lookahead to the ES grammars (ECMA-262 §14.5), so `{ a }⏎/foo/g.exec(s)` fails to parse instead of compiling to a different program. See [F12](F12-parser-driven-lexical-goals.md) §5. | Ready |
| 2 | #17907 | The emitter turns non-directives (`"use\x20strict"`, `` `use strict` ``, `("use strict")`) into a real `"use strict"` directive. | Ready |
| 3 | #17940 | A function declaration in a class `static {}` block parses as an expression (same family as #17877). | Ready |
| 4 | #17939 | WHITESPACE_ONLY drops the restricted-production line break after `yield`. | Ready |
| 5 | #17887 | The emitter prints `a<!--b`, line comments swallow `\r`, and WHITESPACE_ONLY drops a line break after the identifier `yield`. One PR per bug. | Ready |
| 6 | #17883 | WHITESPACE_ONLY drops the empty-statement `;` after `with(...)` or a label before `}`, so the output no longer parses. | Ready |
| 7 | #17888 | The BLOCK_COMMENT pattern misses comments ending in `**/`, and REGEX admits line terminators. | Ready |
| 8 | #17882 | ASI insertion takes tens of seconds on about 400 labeled blocks. Make it linear or bounded. | Ready |

### Q0.5 — valid JavaScript that closurec rejects

| # | ID | Item | Status |
|---:|---|---|---|
| 9 | #17854 | Any template literal with a substitution fails the typed parse at SIMPLE/ADVANCED. | Ready |
| 10 | #17941 | Object method shorthand and async function expressions fail to parse. | Ready |
| 11 | #17852 | Adjacent class members and reserved-word property names fail to parse. | Ready |
| 12 | F12 phases 1–4 | Parser-driven lexical goals, so regex vs. division is decided by the parser. Phase 1 `lexer` (snapshot, mode override, deferred errors); phase 2 `parser`/`grammar-tools` (`relex:` section, token source, undo, memo); phase 3 `javascript-parser` (relex rules, byte-offset ASI, fix B's inputs as fixtures); phase 4 `closurec` (resolved token stream; WHITESPACE_ONLY parses). One PR per phase; phases 1–3 need `ci-full-build`. | Decision — phase 1 waits for the owner to confirm F12's *Decisions* section |
| 13 | #17884 | Support Annex B HTML-like comments (`<!--` and line-leading `-->`) instead of refusing them. Coordinate with #17914, another session's fix for the `-->` test262 regression. | Ready after #17914 merges |
| 14 | CCR-075 #15860 | Refuse what Closure refuses. Slice 1 (private class elements) merged as #17876; find and ship the remaining upstream refusals. | Ready |

### Q1 — cheap ladder wins (emitter and DCE parity)

Each of these is a small change to the emitter or DCE, and each has to check
its blast radius across the 462 goldens.

| # | ID | Item | Status |
|---:|---|---|---|
| 15 | CCR-088 #15925 | Unwrap single-statement substatement blocks for every statement kind, not just expression statements. | Ready |
| 16 | CCR-076 #15861 | Drop braces around a single-statement body. | Ready |
| 17 | CCR-077 #15862 | Match upstream spacing and class-body punctuation. | Ready |
| 18 | CCR-085 #15920 | Strip `"use strict"` unless `--emit_use_strict` is set; re-emit it single-quoted. Do it after #17907. | Ready |
| 19 | CCR-074 #15855 | The `while`→`for` rewrite hoists the preceding initializer into the `for` header. | Ready |
| 20 | CCR-084, CCR-083, CCR-087, CCR-086 | Their fixes merged (see below), but the issues are still open. Check that each is fully done, close it or file what's left. | Ready |

### Q2 — front end and output contract

| # | ID | Item | Status |
|---:|---|---|---|
| 21 | CCR-007 #15624 | Real VLQ source maps through parser spans, passes, emitter and outputs, building on #17803's intervals. Ship parser spans, emitter mappings and multi-file/wrapper support as separate slices. | Ready |
| 22 | CCR-006 #15611 | An execution-backed capability matrix, so users can tell parse-only flags from implemented ones. | Ready |
| 23 | CCR-015 #15674 | Diagnostics plumbing: levels, groups, formatting, counts, streams and exit codes. Slice it. | Ready |
| 24 | CCR-042 #15571 | Match the legacy-octal warning diagnostics. | Blocked by CCR-015 |
| 25 | CCR-008 #15626 | Enforce `language_in` syntax boundaries. | Ready |
| 26 | CCR-079 #15866 | Derive the ladder's divergence ledger from captured oracle runs. | Ready |
| 27 | CCR-089 #15935 | Recapture the upstream-refusal evidence on the pinned JVM. | Ready |

### Q3 — ADVANCED semantics

| # | ID | Item | Status |
|---:|---|---|---|
| 28 | CCR-068 follow-up #15940 | Literal-property propagation should fire on more than one binding per program. | Ready |
| 29 | CCR-022 #15856 | Renaming covers loop locals, catch parameters and labels. | Ready |
| 30 | CCR-067 #15840 | ESTree migration: retire the hand-written bridge, one node family per PR. | Ready (spec first) |
| 31 | CCR-010 #15628 | Destructuring binding targets end to end. | Blocked by CCR-067 for the bridge part |
| 32 | CCR-065 #16875 | Connect the remaining call and composite fold lineage. | Ready |
| 33 | CCR-065 epic #15830 | Provenance slices P2–P6 from the epic: typed edges, per-node contributions, per-pass adoption (`inline` and `rename` first), a query surface, and a join with source maps. One slice per PR. | Ready after #32; P6 needs CCR-007 |
| 34 | CCR-009 #15627 | `language_out` lowering and TRANSPILE_ONLY, starting with optional chaining and nullish coalescing. | Ready |

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

F12 phases 1–4 (row 12). On 2026-10-22, while restarting the loop, Claude settled F12's
four open questions using the spec's own recommendations. The owner had not
answered them; the spec's *Decisions* section records this. Phase 1 waits until the
owner confirms them. Phase 0 does not depend on them.

### Filler (only while waiting on CI)

CCR-069 #15843 (remove serde), CCR-070 #15845 (in-repo regex), CCR-072 #15832
(CLI audit hash projection). None of them may displace a queue item.

### Recently completed

- **Before the stall:**
  - CCR-001–005, CCR-043, CCR-041, CCR-053, CCR-063, CCR-066 and CCR-073.
  - CCR-068 slices #15964, #16011 and #16019.
  - CCR-081.
  - The CCR-065 foundations: #16866, #16881, #16905 and #16964.
- **Since the reset (2026-10-19 to 2026-10-22):**
  - **Miscompiles:**
    - #16008 for-head scope (#17829)
    - #17836 switch-case scope (#17859)
    - #17840 WHITESPACE_ONLY newline ASI (#17875, which also completes CCR-080)
    - #17841 hoisted declarations after `return` (#17845)
    - #17842 nested `break` in a collapsed switch (#17861)
    - #17877 declarations in blocks (#17899)
    - #17879 shared switch declarations (#17894)
    - #17886 template, comment and regex token ends (#17901)
  - **Crash and tooling:**
    - #16009 bounded expression chains (#17833)
    - #15939 strict clippy (#17835)
  - **Parity:**
    - #15942 spread ladder rung (#17846)
    - CCR-078 SIMPLE `const` (#17848)
    - CCR-075 slice 1 (#17876)
    - CCR-082 (#17844)
    - CCR-083 (#17860)
    - CCR-084 (#17839, #17900)
    - CCR-086 (#17874)
    - CCR-087 (#17916)
  - **Front end:** CCR-065 #16970 exact lexical source intervals (#17803).
- **Parked:** fix B, a lexer-only regex-vs-division context stack on local
  branch `claude/closurec-regex-after-brace`. It was abandoned after three
  non-converging security-review rounds, and F12 replaces it.

## Loop record

Add one line per batch of merges or selections.

| Date | Event |
|---|---|
| 2026-10-19 | Backlog reset. #17288 closed; lexer intervals salvaged as #17803. Provenance is no longer a gate. Next: #16008. |
| 2026-10-19 | Owner decided CCR-075: refuse what Closure refuses. Added to Q0 as rank 5.5. |
| 2026-10-19 | Selected #16008 (PR #17829) and #16009 (bounded chains, CLOC42). |
| 2026-10-22 | Recorded 20 Closure PRs merged since the reset. The loop had stopped after parking fix B, so rule 6 now parks the item, not the loop. Merges are recorded in a daily docs PR (rule 5), and near-root crate PRs use `ci-full-build` (rule 10). F12 phase 0 queued; Claude settled phases 1–4's open questions provisionally. New front-end findings ranked into Q0/Q0.5. Next: F12 phase 0. |
