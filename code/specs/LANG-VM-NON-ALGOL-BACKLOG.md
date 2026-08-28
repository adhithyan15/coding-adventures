# LANG VM non-ALGOL completion backlog

Status date: 2026-08-27

This is the execution backlog for completing the shared LANG VM platform while
the ALGOL campaign is owned separately. It complements
`LANG-FULL-IMPLEMENTATION.md`; when the two disagree about landed behavior,
executed tests and current package changelogs are authoritative until the older
roadmap is reconciled.

## Prioritization policy

Re-rank before selecting every work item, and whenever current work discovers a
new gap. Apply this order:

1. Red executed conformance cells or silent backend skips.
2. Missing CI protection for already-working conformance.
3. Incorrect roadmap/status documentation that could send work down a dead path.
4. Missing backend parity for an already-implemented language feature.
5. New frontend semantics and historical-machine fidelity.

Each item must add or strengthen an executed cross-backend proof. One item uses
one fresh worktree and one PR; remove the worktree after merge.

## Ranked backlog

| Rank | ID | Status | Work item | Completion proof |
|---:|---|---|---|---|
| — | VM-001 | done ([#13306](https://github.com/adhithyan15/coding-adventures/pull/13306)) | Preserve COBOL scale-12 decimal intermediates on JVM. | The existing `A / B + C` LANG matrix cell prints `000533` on real Java and the full matrix is green. |
| — | VM-004 | done ([#13340](https://github.com/adhithyan15/coding-adventures/pull/13340)) | Normalize signed process results in `macsyma_conformance` for LLVM and native AOT. | `-7$` compares as `-7`, not process status 249, and the full Macsyma conformance suite passes on every available backend. |
| — | VM-005 | done ([#13348](https://github.com/adhithyan15/coding-adventures/pull/13348)) | Link Linux LLVM matrix executables with the host math library. | Dartmouth BASIC `trunc` and non-ALGOL math cells link and pass under `lang_matrix` on Linux; ALGOL-native gaps remain separately owned. |
| — | VM-002 | done ([#13326](https://github.com/adhithyan15/coding-adventures/pull/13326)) | Protect every CI-green `lang-aot` integration suite in `lang-aot/BUILD`, including `e6d7a_wasm_closures`; replace stale exclusions with the current Macsyma and Linux-matrix failures. | The normal build command runs every CI-green top-level integration target and documents both red exclusions precisely. |
| — | VM-003 | done ([#13356](https://github.com/adhithyan15/coding-adventures/pull/13356)) | Reconcile `LANG-FULL-IMPLEMENTATION.md` with landed Twig E6 and McCarthy work. | Roadmap statuses match current executed suites and name only genuine gaps. |
| 1 | VM-015 | in progress | Reconcile the McCarthy compiler README with the completed L3/W16 backend campaign. | The package README no longer calls L3 future work and links the authoritative eight-backend completion matrix. |
| 2 | VM-010 | queued | Close the remaining Twig dynamic VM/JIT gaps: interned symbols/quote and forward-referenced globals, including boxed global arithmetic. | The existing symbol and dynamic-global matrix cells run on VM and JIT in addition to the five code-generation backends. |
| 3 | VM-011 | queued | Close Dartmouth BASIC's non-ALGOL semantic tail: remaining math builtins and dynamic string data paths. | `SIN`, `COS`, `LOG`, `EXP`, `RND`, and remaining string `READ`/`DATA` cases have seven-backend executed cells. |
| 4 | VM-012 | queued | Implement Nib BCD storage semantics and Intel-4004 RAM mapping. | Hardware-faithful programs agree across the portable matrix and the 4004 simulator. |
| 5 | VM-013 | decision required | Define portable semantics for Oct's Intel-8008 intrinsics (`in`, `adc`, `sbb`, rotations, carry, parity). | The accepted semantics are documented and every intrinsic has executed portable and 8008 proofs. |
| 6 | VM-014 | queued | Extend the unified matrix to every wired frontend, especially FLOW-MATIC and Macsyma/JIT. | Every `Language` variant has an executed baseline on every applicable standard backend, with explicit exclusions only where architectural. |

## Discovery log

- **VM-D001 — confirmed 2026-08-27:** JVM scalar concretization narrowed
  COBOL's explicit `i64` constant `10^12` to `i32`, so nested fixed-point
  division printed `000000` instead of `000533`. Promoted to VM-001.
- **VM-D002 — confirmed 2026-08-27:** `lang-aot/BUILD` still describes the
  closure suite as red even though all five tests pass, and excludes the full
  LANG matrix. Promoted to VM-002.
- **VM-D003 — confirmed 2026-08-27:** the LANG-FULL roadmap still marks Twig
  lists/closures/records/unions and McCarthy backend completion as open despite
  landed executed tests. Promoted to VM-003.
- **VM-D004 — confirmed 2026-08-27:** comparing top-level `lang-aot/tests/*.rs`
  targets with the explicit BUILD command found `macsyma_conformance` omitted
  in addition to the two documented exclusions. Running it exposed LLVM process
  status 249 for the expected signed result -7. Promoted to VM-004 and ranked
  above CI/documentation work as a red executed conformance suite.
- **VM-D005 — confirmed 2026-08-27:** enabling `lang_matrix` in Linux CI exposed
  19 failures: LLVM links without `libm` for cross-language
  `pow`/`floor`/`trunc` calls (including Dartmouth BASIC), while native AOT
  refuses newly proven four-dimensional ALGOL array cases. The linker portion
  is promoted to VM-005; the array refusals remain with the separate ALGOL
  campaign. `lang_matrix` stays explicitly excluded from `lang-aot/BUILD`
  until both streams make it Linux-green.
- **VM-D006 — confirmed 2026-08-27:** the roadmap's E6/Twig checklist predates
  landed seven-engine proofs for list operations, records, unions, and closures,
  and still presents McCarthy backend completion as open even though W16 proves
  F1–F7 on all eight applicable backends. Promoted to VM-003. The audit narrowed
  VM-010 to the two Twig cells that still exclude VM/JIT: symbols/quote and
  forward-referenced dynamic globals (plus the documented boxed-global arithmetic
  follow-up).
- **VM-D007 — confirmed 2026-08-27:**
  `mccarthy-lisp-iir-compiler/README.md` still says L3 is the next phase even
  though the authoritative McCarthy platform matrix marks W1–W16 complete.
  Promoted to VM-015 as a separate package-documentation reconciliation.

## Ownership boundary

Do not select ALGOL items from this backlog while the separate ALGOL agent is
active. Cross-cutting fixes may touch shared infrastructure used by ALGOL, but
must preserve its tests and avoid changing ALGOL semantics or roadmap ownership.
