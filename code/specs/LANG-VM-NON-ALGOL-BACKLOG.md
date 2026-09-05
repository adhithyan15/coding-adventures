# LANG VM non-ALGOL completion backlog

Status date: 2026-09-05

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

## Completion loop and acceptance criteria

The loop keeps one active implementation PR. Before selecting another item,
fetch `origin/main`, inspect the previous PR's actual merge state, record new
discoveries, and rerank this backlog. Fix CI failures and merge conflicts on the
active PR. Publish ready-for-review PRs, never drafts, so draft-specific check
filters cannot suppress validation. Enable auto-merge only after its current-head checks finish green and
no conflicts or blocking reviews remain; do not bypass checks. After merge,
record completion and repeat from a clean feature worktree.

Completion means the scoped language features have executable conformance on
every applicable backend, those proofs run in normal CI, historical-machine
semantics are explicit and tested, and the status documents agree with the
code. A green example is not full language completion. The separate ALGOL
campaign retains ownership of ALGOL semantics. The platform vision's broader
tooling, transpilation, GC, and performance aspirations need their own audited
acceptance inventory (VM-029), rather than an implicit claim of completion.

### 2026-09-05 prioritization

The initial audit selected a directly observed CI gap: `lang-aot/BUILD` runs only the BASIC random differential
tests from `lang_matrix`, so the new non-ALGOL conformance rows are not protected.
The full matrix remains excluded for separately owned ALGOL Linux native-AOT
array failures. VM-024 will run every non-ALGOL `PROGRAMS` row and its declared
backends through the existing strict runner, preserving the full matrix and its
diagnostic single-cell interface. Missing tools remain explicit skips; compiler,
linker, runtime, and result failures remain failures. Report executed and skipped
cells and reject an empty selection. Validate on the local host and all PR CI
hosts before enabling auto-merge. Promote any discovered runtime failure ahead
of further coverage or semantic work.

Executing that selection exposed VM-030 on the very first Windows native cell.
PR #14265 repairs it; 62 library tests and all-target Clippy pass. Both LLVM
`lld-link` and Microsoft `link.exe` run the Windows smoke suite successfully
(eight reported tests, six exercised tests and two explicit GC early returns).
The original failing LANG cell `0:NativeAot` passes with the single-cell sentinel.
Current-head CI remains the merge gate. The subsequent matrix run reproduced
VM-033 (Windows text-output newlines), which now precedes coverage work. After
that repair, missing Windows runtime CI protection (VM-032) precedes the broader non-ALGOL matrix gate:
otherwise this exact regression can recur despite a green Windows job.

## VM-032 implementation contract (selected after #14289 merged)

PR #14289 merged as `e6765d7711` after all applicable checks passed on
`92250cbe12`. VM-033 and VM-034 are complete. Reprioritization now selects
VM-032 ahead of VM-024: Windows must execute its native regression on affected
PRs before widening the general matrix coverage.

Derive a dedicated LANG Windows runtime flag from the build plan's Windows
`affected_packages` override, falling back to the top-level affected closure
only when there is no Windows override. Select `rust/twig-aot` or
`rust/lang-aot`; the planner already propagates changes through their dependency
closure. A null or missing closure means full coverage; an empty closure means
no affected packages. Reject malformed plans. Changes to the workflow, selector,
its tests, or the MSVC bootstrap must self-select the gate even for an empty
package plan.

Wire the flag into Windows runner selection, Rust setup, and the existing MSVC
developer-environment setup. Add a dedicated Windows step running
`cargo test -p twig-aot --test windows_x86_64_smoke -- --nocapture` with
`LANG_REQUIRE_WINDOWS_AOT=1`. In that mode absence of a real linker must fail,
not silently skip. Preserve optional local-toolchain skips without the flag and
the two explicit, separately tracked precise-GC early returns (VM-031).

Validate selection against Windows overrides, unrelated and full plans, and
self-changes. Exercise the real Windows suite with the required flag, then run
its scalar test binary with no linker on PATH and prove a nonzero result.
Validate workflow wiring and the existing planner/metadata contracts. Actual
hosted Windows runtime execution on the PR is the final acceptance evidence.

## Ranked backlog

### VM-033 implementation contract (selected after #14265 merged)

PR #14265 merged as `0e18482307` after every applicable current-head CI check
passed. Reprioritization selects the independently reproduced `352:Llvm`
stdout mismatch before VM-032 and VM-024 coverage work.

`Expect::Stdout` describes text lines for the text-output languages: compare
CRLF as LF, on either host, without removing lone carriage returns, spaces,
tabs, empty lines, or Unicode characters. Brainfuck is the byte-oriented
exception and retains the existing exact comparison. Keep the raw observed
stdout in failure diagnostics. Apply the same comparison boundary to the BASIC
full-value differential tests, so the matrix and differential harness agree.
This changes test interpretation only; do not change compiler or runtime output.

Validate positive LF/CRLF equivalence and negative cases for altered content,
lone CR, and Brainfuck output. Execute the existing BASIC multi-line real,
mixed DATA, and RND rows on every available declared backend, requiring every
available runner to return a result. Add those focused tests to normal BUILD;
the complete non-ALGOL corpus remains VM-024 and full ALGOL coverage VM-025.

The new RND regression exposed VM-034 before publication. The focused repair
must include this prerequisite rather than excluding that backend or program:
the JVM's simulator-compatibility pass must preserve integer widths in modules
that use floating-point values. Such modules cannot run on the legacy 32-bit
integer-only simulator anyway. Keep this decision module-wide so helper calls
and globals agree; prove that RND's `i64` product remains wide and execute the
existing seed/repeat/advance transcript on real Java. This is a generic mixed
numeric-type boundary, not a BASIC-specific RNG special case. Preserve the
existing integer-only simulator path and run its regression suite.

Local validation on Windows: the LF/CRLF regression fails before the fix and
passes after it; three focused tests pass with 18 executed backend cells and
three missing-tool skips. The library and five JVM suites pass 24 tests,
including the integer-only simulator and real-Java Lisp tests. BASIC T7 tests
exercise 400 generated programs with 864 cross-engine agreements. The ALGOL
real-procedure and real-for-variable matrix regressions pass on available
backends, and all-target Clippy is clean. Full-matrix completion is not claimed.

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
| — | VM-020 | done ([#13428](https://github.com/adhithyan15/coding-adventures/pull/13428)) | Add FLOW-MATIC's first unified matrix baseline. | A field `MOVE` plus `WRITE-ITEM` program prints `0` on NativeAOT, LLVM, WASM, JVM, CLR, VM, and JIT. |
| — | VM-021 | done ([#13442](https://github.com/adhithyan15/coding-adventures/pull/13442)) | Add Macsyma to the universal JIT and its cross-backend conformance suite. | Macsyma's integer arithmetic corpus agrees across VM, NativeAOT, LLVM, WASM, JVM, CLR, and JIT. |
| — | VM-017 | done ([#13452](https://github.com/adhithyan15/coding-adventures/pull/13452)) | Add mixed numeric/string Dartmouth BASIC `DATA`, `READ`, and `RESTORE` semantics. | Scalar and array string reads preserve source order with numeric values and execute on all applicable standard backends. |
| — | VM-022 | done ([#13762](https://github.com/adhithyan15/coding-adventures/pull/13762)) | Repair the TypeScript Dartmouth BASIC parser `BUILD` so it runs that package's tests instead of ending in the generic parser package. | `BUILD` executes the Dartmouth parser suite itself and includes a mixed numeric/string `DATA` regression. |
| — | VM-023 | done ([#13773](https://github.com/adhithyan15/coding-adventures/pull/13773)) | Audit and repair the same stateful-directory build defect across non-ALGOL TypeScript parser and lexer frontends. | Every affected package's normal and Windows build scripts run that package's own tests, with an automated guard against ending in a dependency directory. |
| — | VM-012 | done ([#13785](https://github.com/adhithyan15/coding-adventures/pull/13785)) | Implement Nib BCD storage semantics and Intel-4004 RAM mapping. | Hardware-faithful programs agree across the portable matrix and the 4004 simulator. |
| — | VM-018 | done ([#13802](https://github.com/adhithyan15/coding-adventures/pull/13802)) | Define and implement portable Dartmouth BASIC `RND` semantics. | Merged as `8ceb8efc60`; the matrix includes negative reseeding, positive advancement, zero repeat, and shared `DEF FN` state on seven backends. |
| — | VM-030 | done ([#14265](https://github.com/adhithyan15/coding-adventures/pull/14265)); discovered by VM-024 | Repair Windows native-AOT CRT linking. | Merged `0e18482307`; both Windows linkers execute the smoke suite, 62 unit tests and Clippy pass, and Linux/macOS/Windows CI and both CI gates finished green. |
| — | VM-033 | done ([#14289](https://github.com/adhithyan15/coding-adventures/pull/14289)) | Define portable text-output comparison in the LANG matrix. | Windows LLVM BASIC cell `352:Llvm` prints correct values with CRLF but the LF expectation fails; normalize only the accepted host text-newline difference, preserve meaningful output bytes, and run discriminating multi-line cases on available backends. |
| — | VM-034 | done ([#14289](https://github.com/adhithyan15/coding-adventures/pull/14289)) | Preserve JVM integer arithmetic in mixed floating-point modules. | RND cell `360:Jvm` must produce `22`, `85032`, `85032`, `601352`; its `48271 * 48271` intermediate stays i64, while the existing integer-only simulator tests remain green. |
| 0 | VM-032 | selected | Protect LANG native Windows execution in PR CI. | An affected `twig-aot`/LANG dependency change selects an actual Windows executable smoke run, with its toolchain present; assert real execution rather than a green Clippy-only job. Preserve platform-plan gating and keep the known GC early returns explicit. |
| 1 | VM-024 | queued behind VM-032 | Protect the non-ALGOL language matrix in normal CI. | Every non-ALGOL `PROGRAMS` row executes on each available declared backend; no empty selection or silent failure-to-skip conversion; full ALGOL diagnostics remain available. |
| 2 | VM-025 | queued; coordinate with ALGOL owner | Reproduce the full matrix's current Linux failures and restore full CI coverage after their repair. | Record exact failing cells and owning PRs; remove the full-matrix exclusion only after the complete target runs green on supported CI hosts. |
| 3 | VM-026 | queued | Reconcile stale Twig, frontend-count, and completed-work claims in LANG-FULL and LANG-PLATFORM status documents. | Every remaining gap links a current source/test boundary; landed VM-010, VM-017, VM-018, VM-020, and VM-021 work is no longer described as missing. |
| 4 | VM-027 | queued | Audit feature-by-backend coverage for all ten wired frontends, including dedicated McCarthy and Macsyma suites. | Inventory implemented features, declared/refused backend cells, executable proofs, and CI commands; split every uncovered implemented feature into a bounded parity item. |
| 5 | VM-013 | design required | Define portable semantics for Oct's Intel-8008 intrinsics (`in`, `out`, `adc`, `sbb`, rotations, carry, parity). | Document machine state and I/O contracts, then split into PR-sized operations with both portable and real 8008-simulator proofs; resolve only essential language-design questions with the user. |
| 6 | VM-028 | queued | Audit Nib's remaining Intel-4004 arithmetic and control-flow parity beyond BCD storage. | Compare implemented operations against the 4004 backend and simulator; file bounded missing-operation or refusal tests and close them with executed proofs. |
| 7 | VM-029 | queued | Audit the broader platform vision and native-runtime roadmaps into explicit completion milestones. | Separate landed capabilities from GC, tooling, IR-bridge/transpilation, and measured-performance work; give every retained milestone a testable acceptance criterion and owner. |
| 7a | VM-031 | queued under VM-029 | Reconcile and complete native precise-GC frame-walk proofs. | Windows smoke has two explicit early-return differentials for the Rust/Twig frame boundary; inventory supported-host GC gaps, track each refusal, and execute live-byte reclamation proofs before calling native GC complete. |

## Discovery log

- **VM-D024 — confirmed 2026-09-05:** VM-033's real multi-line/RND proof
  exposed JVM output `22`, `-914968`, `-914968`, `-398672`. A fresh process
  using the pre-VM-033 matrix binary reproduces `360:Jvm`, so normalization
  did not cause it. `concretize_scalar_any_for_jvm` narrows RND's explicit
  `i64` product to `i32` because every literal fits in i32 and BASIC's newer
  floating-point PRINT is not the old `print_i64` builtin. The second RNG
  product overflows before modulo. Promote VM-034 as VM-033's prerequisite;
  keep mixed floating-point modules out of the integer-only simulator rewrite
  and retain all three selected multi-line regressions.

- **VM-D023 — confirmed 2026-09-05:** with VM-030 applied, the proposed
  non-ALGOL matrix ran for 308 seconds before failing on LLVM Dartmouth BASIC
  multi-line real output. Expected `3.14\n.25\n-2.5`, observed
  `3.14\r\n.25\r\n-2.5`. A fresh single-cell process with
  `LANG_MATRIX_ONLY_CELL=352:Llvm` reproduces the mismatch in 0.35 seconds.
  This is a host text-output comparison failure, independent of CRT symbol
  linking. Promote VM-033 ahead of coverage work; retain the exact content and
  control-character contract rather than broadly trimming away discrepancies.

- **VM-D022 — confirmed 2026-09-05:** `.github/workflows/ci.yml` selects a
  Windows job when `needs_rust` is true, but `Build and test affected packages`
  on Windows additionally requires Swift or Elixir. Its dedicated Windows-only
  Rust step is Clippy only and does not select the portable `twig-aot` crate.
  Therefore a green Windows job for a Rust-only CRT repair does not execute
  `windows_x86_64_smoke`. Promote VM-032 as the first coverage follow-up. Local
  runs with both real linkers provide VM-030's Windows runtime evidence while
  that gap is repaired separately.

- **VM-D021 — confirmed 2026-09-05:** executing the proposed non-ALGOL
  matrix on Windows failed on the very first native Twig `42` cell with
  duplicate `__vcrt_InitializeCriticalSectionEx`, defined by both
  `libvcruntime.lib` and `vcruntime.lib`. The static library was added for an
  earlier custom `/ENTRY:main` path; that custom entry was removed by
  `bc2cb05594`, but the static library and its old explanation remained.
  Promote VM-030 ahead of VM-024. Remove the obsolete static CRT selection,
  preserve the compiler's normal startup, and run actual Windows executables
  before enabling additional matrix coverage. VM-024's draft patch is held
  outside the checkout until this prerequisite lands.

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
- **VM-D013 — confirmed 2026-08-28:** VM-021 did not require the anticipated
  McCarthy tagged-runtime callback generalization. Macsyma's v0 integer corpus
  becomes typed arithmetic after the existing `lower_dynamic_arith` pass, while
  generic VM/JIT `box`/`unbox` are identity operations. The dedicated
  `run_macsyma_on_jit` runner therefore adds no language-specific runtime
  callbacks; all 21 programs execute and agree across seven engines.
- **VM-D014 — confirmed 2026-08-28:** VM-017 needs one dynamically ordered
  stream without imposing a new tagged-value ABI on seven backends. Parallel
  kind, `array<f64>`, and `array<str>` pools retain one source index and shared
  `RESTORE` pointer. Every `READ` first bounds-checks the kind array and then
  validates the target type; a mismatch deliberately enters the existing
  cross-backend array-bounds trap contract instead of reading a placeholder.
- **VM-D015 — confirmed 2026-08-28:** the shared Dartmouth BASIC grammar is
  embedded in generated parser artifacts outside the Rust LANG VM frontend.
  CI caught the required Ruby regeneration; auditing the same source boundary
  found Python, Lua, and TypeScript parser artifacts with the old numeric-only
  `DATA` rule. VM-017 regenerates all four alongside Rust so no checked-in
  parser silently disagrees with the authoritative grammar.
- **VM-D016 — confirmed 2026-08-28:** the TypeScript Dartmouth BASIC parser's
  `BUILD` chains relative `cd` commands through its dependencies and finishes
  in `../parser`, so its reported 117 tests belong to the generic parser while
  the Dartmouth package's own 61-test suite never runs. Promoted to VM-022 and
  ranked above new semantics as a missing-test-protection defect.
- **VM-D017 — confirmed 2026-08-31:** VM-022's sibling audit found the same
  top-level, stateful `cd ../...` pattern in 27 TypeScript parser `BUILD`
  scripts and 22 lexer `BUILD` scripts, with 20 and 18 corresponding Windows
  scripts respectively. Promoted to VM-023 for a non-ALGOL fleet audit and an
  automated package-directory guard; it remains separate from VM-022 so the
  Dartmouth repair has a focused executed proof.
- **VM-D018 — confirmed 2026-08-31:** after excluding the separately owned
  ALGOL fronts and the already-repaired Dartmouth parser fronts, VM-023 found
  470 stateful sibling-directory commands across 81 parser/lexer build files.
  Dependency installs now run in subshells, and a CI-discovered repository
  test rejects any future top-level sibling `cd` in non-ALGOL `BUILD` or
  `BUILD_windows` fronts.
- **VM-D019 — confirmed 2026-08-31:** the 4004 simulator models 256 main RAM
  characters plus 64 status characters, while the old backend had no RAM ops
  and compiled functions independently. VM-012 uses the complete 320-nibble
  space and builds a module-wide global-slot map before lowering functions, so
  every function agrees on a static's physical address.
- **VM-D020 — confirmed 2026-08-31:** every standard backend already executes
  typed module globals, cross-function calls, exact `i64` multiplication/modulo,
  and `i64`↔`f64` conversion. VM-018 therefore needs no random host ABI: one
  Park–Miller helper can keep a module-global state shared by `main` and
  `DEF FN`. The accepted contract fixes seed 1 at program start, makes negative
  arguments reseed-and-advance, zero repeat, and positive arguments advance.

## Ownership boundary

Do not select ALGOL items from this backlog while the separate ALGOL agent is
active. Cross-cutting fixes may touch shared infrastructure used by ALGOL, but
must preserve its tests and avoid changing ALGOL semantics or roadmap ownership.
