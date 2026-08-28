# LANG VM non-ALGOL completion backlog

Status date: 2026-08-28

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
| — | VM-015 | done ([#13367](https://github.com/adhithyan15/coding-adventures/pull/13367)) | Reconcile the McCarthy compiler README with the completed L3/W16 backend campaign. | The package README no longer calls L3 future work and links the authoritative eight-backend completion matrix. |
| — | VM-010 | done ([#13380](https://github.com/adhithyan15/coding-adventures/pull/13380)) | Close the remaining Twig dynamic VM/JIT gaps: interned symbols/quote and forward-referenced globals, including boxed global arithmetic. | The existing symbol and dynamic-global matrix cells run on VM and JIT in addition to the five code-generation backends. |
| — | VM-011 | decomposed | Close Dartmouth BASIC's non-ALGOL semantic tail: remaining math builtins and dynamic string data paths. | Split into VM-016 through VM-018 because deterministic transcendental wiring, mixed-type `DATA`, and portable randomness have independent designs and risk. |
| — | VM-016 | done ([#13392](https://github.com/adhithyan15/coding-adventures/pull/13392)) | Wire Dartmouth BASIC's deterministic `SIN`, `COS`, `LOG`, and `EXP` builtins to the existing shared transcendental IIR ops. | One discriminating program executes all four functions on NativeAOT, LLVM, WASM, JVM, CLR, VM, and JIT. |
| — | VM-019 | done ([#13419](https://github.com/adhithyan15/coding-adventures/pull/13419)) | Reconcile the Dartmouth BASIC compiler README with landed general exponentiation, seven-backend string arrays, and current dynamic-string limitations. | The README no longer directs work toward already-landed power or string-array support and names only executed current gaps. |
| — | VM-014 | decomposed | Extend the unified matrix to every wired frontend, especially FLOW-MATIC and Macsyma/JIT. | Split into VM-020 and VM-021 because FLOW-MATIC needs an executed matrix baseline while Macsyma needs new universal-JIT runtime glue. |
| 1 | VM-020 | in progress | Add FLOW-MATIC's first unified matrix baseline. | A field `MOVE` plus `WRITE-ITEM` program prints `0` on NativeAOT, LLVM, WASM, JVM, CLR, VM, and JIT. |
| 2 | VM-021 | queued | Add Macsyma to the universal JIT and its cross-backend conformance suite. | Macsyma's integer arithmetic corpus agrees across VM, NativeAOT, LLVM, WASM, JVM, CLR, and JIT. |
| 3 | VM-017 | queued | Add mixed numeric/string Dartmouth BASIC `DATA`, `READ`, and `RESTORE` semantics. | Scalar and array string reads preserve source order with numeric values and execute on all applicable standard backends. |
| 4 | VM-012 | queued | Implement Nib BCD storage semantics and Intel-4004 RAM mapping. | Hardware-faithful programs agree across the portable matrix and the 4004 simulator. |
| 5 | VM-018 | decision required | Define and implement portable Dartmouth BASIC `RND` semantics. | The accepted seed/repeatability contract is documented and executed consistently across all standard backends. |
| 6 | VM-013 | decision required | Define portable semantics for Oct's Intel-8008 intrinsics (`in`, `adc`, `sbb`, rotations, carry, parity). | The accepted semantics are documented and every intrinsic has executed portable and 8008 proofs. |

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
- **VM-D008 — confirmed 2026-08-27:** the discriminating Twig forward-global
  arithmetic cell returned the tagged word for 42 (`336`, observed as process
  exit 80) on native AOT and LLVM because `lower_dynamic_arith` boxed the
  helper's result without propagating `ref<any>` through its return/call
  signature. BEAM refused the same representation-only `box` op even though
  Erlang integers are already dynamic terms. Retained inside VM-010 because
  boxed global arithmetic was an explicit part of that item.
- **VM-D009 — confirmed 2026-08-28:** VM-011 combined three independent
  implementation classes. `SIN`/`COS`/`LOG`/`EXP` already exist as shared IIR
  operations on every standard backend and need only Dartmouth frontend wiring;
  string `READ`/`DATA` needs a mixed-type ordered pool; `RND` needs an explicit
  portable RNG contract and new substrate. Decomposed into VM-016, VM-017, and
  VM-018 before selecting the deterministic VM-016 slice.
- **VM-D010 — confirmed 2026-08-28:** the Dartmouth BASIC compiler README still
  calls general exponentiation and the final NativeAOT/JVM/CLR string-array lanes
  future work even though their seven-backend matrix proofs have landed. Promoted
  to VM-019 for a focused package-documentation reconciliation.
- **VM-D011 — confirmed 2026-08-28:** the same README broadly calls richer
  dynamic string expressions future work even though the matrix executes
  `str_concat` over two runtime `INPUT` strings on all seven backends. Retained
  inside VM-019; the corrected limitation is mixed numeric/string `DATA` and
  `READ`, already tracked as VM-017.
- **VM-D012 — confirmed 2026-08-28:** VM-014 combined two different gaps.
  FLOW-MATIC is wired into `Language` and already has package-level JIT and
  backend acceptance tests but no unified matrix cell. Macsyma's separate
  conformance suite executes VM, NativeAOT, LLVM, WASM, JVM, and CLR while
  explicitly excluding the universal JIT because its callback glue is still
  McCarthy-specific. Decomposed into VM-020 and VM-021; selected VM-020 first
  because it closes a zero-coverage frontend without changing runtime semantics.

## Ownership boundary

Do not select ALGOL items from this backlog while the separate ALGOL agent is
active. Cross-cutting fixes may touch shared infrastructure used by ALGOL, but
must preserve its tests and avoid changing ALGOL semantics or roadmap ownership.
