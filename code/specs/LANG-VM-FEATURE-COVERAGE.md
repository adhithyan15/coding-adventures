# LANG VM feature and backend coverage

Audit base: `cd73f3ad86` (2026-09-05); corpus counts updated for VM-047b, VM-057, VM-047c, VM-039b and VM-061. This is an inventory of the implemented
frontend families and their executable proof boundaries, not a claim that the
historical languages or every backend are complete. Follow-up IDs live in the
[completion backlog](LANG-VM-NON-ALGOL-BACKLOG.md).

## Reading the evidence

The [driver](../packages/rust/lang-aot/src/lib.rs) wires ten `Language` variants.
The [unified corpus](../packages/rust/lang-aot/tests/lang_matrix.rs) has eight.
Its seven standard columns are NativeAOT, LLVM, WASM, JVM, CLR, VM and JIT.
Only 20 Twig rows additionally declare BEAM. All counts below are source
**declarations**, not counts of executions on every host. External tools can be
absent; actual runner failures must fail, rather than turn into skips.

Frontend `backend_compat` validators and `backend_encode` byte generation are
useful checks, but neither establishes runtime behavior. The driver performs
shared lowering passes after frontend compilation, so a raw frontend validator
refusal also does not imply the complete driver refuses that feature.

| Frontend | Unified rows | Declared cells (all backends, Beam included) | Additional proof boundary |
|---|---:|---:|---|
| Twig | 49 | 363 | 20 of those cells are BEAM; dedicated heap/closure tests |
| Nib | 26 | 208 | All eight columns including real BEAM u4/u8 and BCD storage |
| Brainfuck | 6 | 42 | Dedicated WASM/JVM/CLR and JIT execution |
| Dartmouth BASIC | 51 | 357 | Random differential suite and frontend JIT tests |
| Oct | 12 | 96 | All eight columns, including real BEAM stdout and u8 wrap; frontend JIT control-flow tests |
| ALGOL 60 | 233 | 1631 | Separate owner; full-matrix CI exclusion remains VM-025; not re-audited by VM-061 (see below) |
| FLOW-MATIC | 8 | 60 | Four output/control-flow rows on eight columns; four input/EOF rows on seven |
| COBOL-60 | 58 | 422 | Much larger frontend JIT/oracle suite |
| McCarthy Lisp | 0 | 0 | Dedicated 19-program capstone with nine runner lanes |
| Macsyma | 0 | 0 | Dedicated 21-program capstone with eight runner lanes plus real CoreCLR |

The normal non-ALGOL capstone therefore declares 210 programs and 1548
declared cells (sum of the non-ALGOL rows above), which now matches a fresh
`non_algol_matrix_every_proven_cell_agrees` run exactly: 1338 cells exercised
plus 210 skipped (missing local `ilasm`) = 1548. The "Declared cells" column
counts every backend a row proves, Beam included — the convention Nib, Oct,
FLOW-MATIC and COBOL-60's numbers already used. Twig was the one holdout:
its old "343" was `49 rows × 7 standard backends`, silently excluding its 20
Beam cells (VM-D030/**VM-061**), which is why this is the only cell count
that changed in this pass. Every other non-ALGOL row's declared row count
and cell count were independently re-derived from `PROGRAMS` in
`lang_matrix.rs` (a `Prog { lang: Language::X, .., backends: &[..] }` per
row; cells = `rows.map(|p| p.backends.len()).sum()`) via a brace-balanced
parse of the whole array, not a hand count or a quick regex, and all six
matched the doc exactly except Twig. That derivation is now also pinned as
`feature_coverage_doc_counts_match_programs_source` in `lang_matrix.rs`: it
asserts each of these seven non-ALGOL row/cell pairs against the live
`PROGRAMS` corpus, plus that McCarthy Lisp and Macsyma still have zero rows
there (both use the dedicated capstone files below instead), so a future
slice that adds or removes a row without updating this table fails a normal
`cargo test -p lang-aot --test lang_matrix` run instead of drifting silently
again. ALGOL 60's row is intentionally left unrecomputed and unasserted: it
is owned by a separate, actively developing campaign (see "Ownership
boundary" in the completion backlog), and this table's own audit trail
(VM-D030/VM-061) does not extend a mandate to correct or pin its count. The
zeroes for McCarthy and Macsyma mean dedicated coverage, not absent support.
CLR-real is an additional runner lane for the same CLR backend in McCarthy's
capstone, not a tenth universal backend.

## Implemented feature families and remaining proofs

| Frontend/source | Implemented family and current executable evidence | Boundary and next work |
|---|---|---|
| [Twig lowerer](../packages/rust/twig-ir-compiler/src/compiler.rs) | Scalars, variadic arithmetic, lexical bindings, calls, cons/list operations, symbols, globals, records/unions, closures and source-inferred strings. Unified rows cover heap arithmetic, list helpers, quote equality, records/match, forward and boxed globals, capturing closures, literal/local/parameter strings and a bounds trap. | BEAM covers 20 selected rows, not all strings/records/closures. Dynamic or captured/reassigned strings exceed the source-local fast path. VM-040 inventories the remaining BEAM families; VM-041 isolates dynamic-string lowering from existing literal metadata. |
| [Nib lowerer](../packages/rust/nib-iir-compiler/src/lib.rs) | Integer arithmetic, narrow masking, wrapping/saturating addition, bitwise/logical operations, branches/loops, calls, const/static initialization and BCD storage. Unified rows execute standard-backend cases. [JIT tests](../packages/rust/nib-iir-compiler/tests/jit_e2e.rs) independently exercise compiled functions. | Standard-target parity does not establish 4004 arithmetic/control-flow fidelity. Existing VM-028 owns that audit; VM-012 proves only its landed BCD storage slice. BEAM remains undeclared (VM-040). |
| [Brainfuck compiler](../packages/rust/brainfuck-iir-compiler/src/compiler.rs) | All eight commands, wrapped tape cells/pointer movement, nested loops and input/EOF. Unified rows plus [WASM](../packages/rust/brainfuck-iir-compiler/tests/wasm_e2e.rs), [JVM](../packages/rust/brainfuck-iir-compiler/tests/jvm_e2e.rs), [CLR](../packages/rust/brainfuck-iir-compiler/tests/clr_e2e.rs) and [JIT](../packages/rust/brainfuck-iir-compiler/tests/jit_smoke.rs) execution. | BEAM tape support is intentionally excluded in the frontend README. Preserve that scope, but pin an actual driver refusal rather than treating a missing table cell as a refusal test (VM-042). |
| [BASIC lowerer](../packages/rust/dartmouth-basic-iir-compiler/src/lib.rs) | f64 arithmetic/general power/transcendentals, deterministic RND, scalar/string input and output, branches, FOR, GOSUB/RETURN, DEF FN, numeric/string arrays, mixed DATA/READ/RESTORE. All 51 rows declare seven columns; random differential tests supplement fixed results. | DEF FN global access and historical print zones are frontend semantics, not already-implemented parity. BEAM remains undeclared (VM-040). Two-dimensional numeric DIM already has a seven-column matrix proof; the stale one-dimensional-only README wording is corrected in this audit. |
| [Oct lowerer](../packages/rust/oct-iir-compiler/src/lib.rs) | u8 arithmetic/masking, bitwise/logical operations, functions, local/global state, if/while/loop/break and stdout `out`. Matrix covers output, wrap, short circuit, shared globals, loop-carried wrapping returned from a function, conditional break and nested break targets; [JIT suite](../packages/rust/oct-iir-compiler/tests/jit_e2e.rs) separately executes while loops and returned function values. | VM-044 adds observable loop/break and returned-call standard-column proofs. `in`, carry arithmetic and rotations are explicit intrinsic errors; VM-013 owns portable machine-state design. Body-local static and floats are not implemented parity gaps. |
| [ALGOL lowerer](../packages/rust/algol-iir-compiler/src/lib.rs) | Scalar integer/boolean/real/string operations, arrays, procedures, by-name specializations, switches and nonlocal control flow have a substantial evolving corpus. [Frontend JIT](../packages/rust/algol-iir-compiler/tests/jit_e2e.rs) and [AOT smoke](../packages/rust/algol-iir-compiler/tests/aot_smoke.rs) are separate proofs. | Full LANG matrix remains excluded for the recorded native-array failure. VM-025 and the separate ALGOL owner control fixes and detailed feature expansion; 232 declarations do not mean 232 green Linux programs. |
| [FLOW-MATIC lowerer](../packages/rust/flow-matic-iir-compiler/src/lib.rs) | MOVE, COMPARE/IF/OTHERWISE, GO TO/JUMP, STOP, READ-ITEM/EOF and WRITE-ITEM. Four unified rows prove scalar move/output, a taken EQUAL, false LESS/GREATER reaching OTHERWISE, and a jump chain. [JIT stream tests](../packages/rust/flow-matic-iir-compiler/tests/jit_e2e.rs) run read/process/write to EOF through custom `input_more`/`input_i64` builtins. | VM-037 adds terminating, output-discriminating control-flow rows; positive LESS/GREATER on nonzero input still requires VM-039. VM-039 provides portable EOF-aware input before promoting record streams to code-generation columns. TRANSFER and tape control are clean frontend rejections, not secretly implemented file I/O. |
| [COBOL lowerer](../packages/rust/cobol-iir-compiler/src/lib.rs) | PICTURE/scaled arithmetic, DISPLAY/MOVE, condition names, IF/EVALUATE, PERFORM/GOTO, COMPUTE/power, size errors and signed/alphanumeric operations occur in the matrix, now expanded to 58 rows with ASCII reference modification: literal/computed bounds, comparisons, MOVE fitting and invalid-bound traps, plus STRING SIZE full-width copying, truncation and untouched tails, STRING delimiters and UNSTRING field fitting/empty fields/exhaustion, pointer/overflow branches, a self-referential `STRING` proof (VM-057), and INSPECT TALLYING/REPLACING BEFORE/AFTER region proofs including the not-found asymmetry and BEFORE+AFTER used together across one combined statement's independently-regioned halves (VM-047c). The [JIT/oracle suite](../packages/rust/cobol-iir-compiler/tests/jit_e2e.rs) additionally exercises reference modification, STRING, UNSTRING and INSPECT families. | VM-045 adds reference-modification rows; VM-046a/b/c add STRING SIZE, delimiter/splitting and pointer/overflow rows; VM-047a adds ALL/CHARACTERS/LEADING tallying proofs; VM-047b adds replacement including first-match/non-rechaining; VM-047c adds BEFORE/AFTER region proofs. A single delimiter phrase carrying BOTH `BEFORE` and `AFTER` together (the ISO two-delimiter intersection) parses but currently reads only the first region clause on both the oracle and the compiler (VM-D027); genuine intersection support is a separate follow-up. Validator acceptance is insufficient. Existing byte/character and category restrictions must remain explicit. |
| [McCarthy lowerer](../packages/rust/mccarthy-lisp-iir-compiler/src/lib.rs) | Quote/cons/CAR/CDR/ATOM/EQ/COND, direct and higher-order lambdas, captured variables, LABEL recursion and closure values. [Capstone](../packages/rust/lang-aot/tests/conformance.rs) tests 19 integer-result programs; [frontend run tests](../packages/rust/mccarthy-lisp-iir-compiler/tests/run_e2e.rs), [JIT](../packages/rust/lang-aot/tests/jit_mccarthy.rs) and dedicated per-backend lambda suites cover further shapes. | VM-036 enables the native capstone on Linux/macOS/Windows, with a required native-only Windows CI run. The capstone is not a proof for every closure shape; preserve dedicated closure suites in normal BUILD. |
| [Macsyma lowerer](../packages/rust/macsyma-iir-compiler/src/lower.rs) | v0 integers, unary/binary arithmetic, exact literal division, assignments, symbols and unevaluated symbolic Apply. [Oracle suite](../packages/rust/macsyma-iir-compiler/tests/oracle.rs) compares symbolic results with the evaluator. [Capstone](../packages/rust/lang-aot/tests/macsyma_conformance.rs) proves 21 integer-result programs on VM/JIT/WASM/CLR/JVM/LLVM/native when present; [`clr_real_macsyma.rs`](../packages/rust/lang-aot/tests/clr_real_macsyma.rs) additionally proves the same corpus on **real CoreCLR** (`dotnet`+`ilasm`), not only the in-repo CLR simulator (VM-049). | BEAM executes the same 21 integer programs when Erlang is present (VM-038); symbolic result representation has VM oracle coverage, not portable capstone agreement (VM-048). Function definitions/calls, control flow, floats, lists, comparisons and power are explicit frontend rejections, outside implemented v0 parity. |

## CI and host boundaries

[lang-aot/BUILD](../packages/rust/lang-aot/BUILD) explicitly includes the
`conformance`, `macsyma_conformance`, `jit_mccarthy`, per-backend lambda/heap,
and `cargo_archive_path` targets in its first cargo command. It separately runs:

```sh
cargo test -p lang-aot --test lang_matrix t7_differential_random_basic_
cargo test -p lang-aot --test lang_matrix portable_text_stdout_
cargo test -p lang-aot --test lang_matrix non_algol_matrix_every_proven_cell_agrees -- --exact --nocapture
```

The complete unfiltered `lang_matrix` target is excluded (VM-025). Frontend
BUILD files run their own package tests, including FLOW-MATIC's stream JIT and
COBOL's JIT/oracle suite. Such frontend executions do not establish LLVM or
native execution of the same source. The [CI workflow](../../.github/workflows/ci.yml)
uses affected-package planning; Windows additionally has a selected native
executable smoke gate. A green Windows job does not imply every LANG test
executes there. VM-036 adds the native-only McCarthy corpus to that actual Windows execution
command, including the required-linker assertion.

`lang-aot/BUILD` declares `# needs-toolchain: dotnet` (VM-049/VM-D028): its own
bucket language is "rust", so without this declaration the planner never set
CI's `needs_dotnet` flag for a PR touching only this crate, and
`actions/setup-dotnet` plus the `ilasm` NuGet restore — both gated on that
flag in `ci.yml` — never ran. The CLR-real tests (`clr_real_*.rs`,
`clr_real_macsyma.rs`) always skipped *correctly* without the toolchain, but
that meant the CLR-real column effectively never executed on its own PR
merge-gate CI, only on a forced main-branch full build (which sets every
toolchain flag). The declaration follows the exact pattern
`java-to-semantic-ir/BUILD` already uses for its own extra Python dependency.

Reproduce the dedicated capstones with:

```sh
cargo test -p lang-aot --test conformance --test macsyma_conformance --test clr_real_macsyma -- --nocapture
```

McCarthy always runs VM/JIT/WASM/CLR simulator. Java, clang, Erlang and real
CLR require their respective installed tools; native uses the host Linux/macOS/Windows compiler and linker.
Macsyma always runs VM/JIT/WASM/CLR simulator, gates JVM/LLVM/native on tools,
and now gates a BEAM runner on Erlang (VM-038). VM-049 added a real-CLR arithmetic proof
(`tests/clr_real_macsyma.rs`, gated on `dotnet`+`ilasm`, over the identical
21-program corpus the simulator column already agrees on) so simulator
success is no longer the only claim of CLR execution; the simulator column
itself is unchanged and remains the required always-on floor. VM-049 also
fixed VM-D028: `lang-aot/BUILD` lacked a `needs-toolchain: dotnet` declaration,
so no PR touching only this rust-bucketed crate ever made hosted CI install
`ilasm`, and the CLR-real column — including McCarthy's pre-existing one —
never actually executed on its own PR merge-gate CI, only on a forced
main-branch full build.

## Executed audit validation

At the audit base, the dedicated command above passed on Windows: McCarthy reported
19 programs across VM/JIT/WASM/CLR/JVM/LLVM/BEAM (133 agreements); native-AOT
was excluded by its macOS guard and CLR-real was unavailable. Macsyma reported
21 programs across VM/JIT/WASM/CLR/JVM/LLVM/native-AOT (147 agreements), plus
its process-result decoder test. These are actual local executions, not inferred
from declaration counts. Hosted CI remains the merge gate.

VM-036 locally executes the 19-program native corpus with both Microsoft and
LLVM Windows linkers. The required-linker negative invocation fails with zero
programs run when PATH is empty. Linux/macOS and hosted Windows execution are
still checked by PR CI before merge.

VM-037 locally executed all 21 newly declared FLOW-MATIC cells (three programs
on seven backends), each with a fresh-process execution sentinel and no skips.

VM-044 locally executed all 21 newly declared Oct cells, each in a fresh process
with its execution sentinel and no skips. The loop returns 24 after wrapping
250 + 30, conditional break prints 3, and nested breaks preserve output 4/2/7.

VM-045 locally executed all 49 new COBOL cells (rows 401–407), with zero skips
and fresh-process ran-cell sentinels. The run exposed and repaired native and
LLVM computed-slice refusals and WASM runtime-slice receiver output (VM-050–052).
The frontend oracle suite separately passed 66 reference-modification tests.

VM-046a locally executed all 21 new STRING SIZE cells (408–410) with zero
skips and ran-cell sentinels. Repeated writes exposed WASM's same-block
last-literal substitution (VM-053), repaired with runtime value propagation.
The frontend STRING/UNSTRING oracle selection separately passes 121 tests.

VM-046b locally executed all 35 new delimiter cells (411–415), with zero skips
and fresh-process sentinels. The run exposed native loop-index constant folding
and LLVM runtime-index refusal (VM-054/055); both now use checked runtime byte
indexing where needed. The frontend oracle selection passed 121 tests.

VM-046c locally executed all 56 pointer/overflow cells (416–423) in fresh
processes with ran-cell sentinels and zero skips. All 121 frontend
STRING/UNSTRING oracle tests passed; this slice needed no runtime change.


VM-047a locally executed all 21 tallying cells (424–426) in fresh processes
with positive sentinels and zero skips. The same three programs pass the
frontend oracle comparison; the full INSPECT-filtered suite passes 287 tests.

VM-047b locally executes all 35 replacement cells (427–431) with positive
sentinels and zero skips after the VM-056 WASM concat alias repair. All 292
INSPECT oracle tests and 236 WASM package tests including doctests pass.

The VM-047b/056 full local non-ALGOL run passed all 200 programs and 1420
cells, with zero skips (635.72 seconds).

VM-057 adds one seven-backend cell (row 432) for a self-referential COBOL
`STRING S DELIMITED BY SIZE INTO S`, after confirming and repairing a real
WASM `str_slice` destination-aliases-source defect. All 632 frontend
JIT/oracle tests and all 237 `iir-to-wasm` package tests including doctests
pass.

VM-047c locally executed all 30 available new cells (rows 433–437 × six
tool-present backends) in fresh processes via the single-cell re-verification
path with positive ran-cell sentinels and zero failures; CLR is an explicit
missing-tool skip on this host (`ilasm` not locatable), reproduced identically
on the pre-existing row 432. All 649 `cobol-iir-compiler` package tests pass,
including six new INSPECT BEFORE/AFTER and VM-D027 regressions. Investigating
BEFORE/AFTER used together on one delimiter phrase exposed VM-D027 — both the
oracle and the compiler silently honor only the first of two grammar-legal
region clauses — promoted to VM-058 rather than fixed in this slice. The full
non-ALGOL matrix passed 206 programs, 1256 cells exercised and 206 skipped
(every program's CLR cell, matching the host-wide missing `ilasm`), zero
failures, in 501.12 seconds.

VM-049 added `clr_real_macsyma.rs`'s toolchain-independent
`macsyma_emits_valid_cil_text_for_full_corpus` test, which locally compiles all
21 Macsyma programs to textual CIL and passes (no lowering change was needed:
`iir-builtin-lowering::dynamic_arith` already expands Macsyma's `call_builtin
"+"/"-"/"*"/"/ "` to `unbox`/`add`/`box` before `emit_il` runs). The real-CoreCLR
test itself reports the expected honest skip on this host (`dotnet`/`ilasm`
both absent, consistent with VM-047c). Investigating why the pre-existing
McCarthy `clr_real_*` lane's tool gate always reported a skip rather than a
real pass exposed VM-D028 — `lang-aot/BUILD` never declared
`needs-toolchain: dotnet`, so hosted CI's `ilasm` restore step never ran for a
PR touching only this crate; confirmed against a recent merged PR's Linux job
log (`needs_dotnet=false`). Fixed by adding the declaration, following the
same pattern `java-to-semantic-ir/BUILD` already uses. Hosted CI on this PR
is therefore the first actual proof (or disproof) that the CLR-real column —
Macsyma's new lane and McCarthy's pre-existing one — executes on real CoreCLR
rather than skipping.

VM-038: the full 21-program Macsyma corpus passed on real Erlang locally with
zero skips. Every source also compiles unconditionally in `macsyma_beam_corpus`.
Hosted execution is requested through BUILD's Elixir/setup-beam declaration;
this does not extend the scalar proof to symbolic values (VM-048).

VM-039a adds four FLOW-MATIC input/EOF rows (438–441), each declaring native
AOT and LLVM only. All eight cells passed locally; a direct production C test
also verifies stable non-consuming peeks. The corpus now has 442 programs,
including eight FLOW-MATIC programs; these rows add eight declared cells, not
28. WASM/JVM/CLR input adapters and the shared VM/JIT harness remain VM-039
follow-ups. Existing frontend callbacks already prove VM/JIT record streams.

VM-039b: the same four FLOW-MATIC input programs now pass on WASM. Current
main has one additional ALGOL row, so their current indices are 439–442 and
the complete corpus contains 443 programs. This adds four declared cells and
no programs. JVM/CLR and VM/JIT matrix adapters remain subsequent slices.

VM-039c JVM: all four input/EOF rows and five BASIC input regressions passed
on a real JVM with positive execution sentinels and zero skips. The shared
Java host regression also checks repeated peeks, mixed numeric/string reads
and I/O failure propagation. CLR and VM/JIT EOF matrix coverage remain pending.

VM-039c CLR: four input/EOF rows and five BASIC input rows passed on real
CoreCLR with execution sentinels. Direct IIR probes verify repeated peeks,
mixed string/numeric reads, 32/64-bit widths and numeric EOF. Encoded CIL
simulator input is not included; VM/JIT matrix EOF callbacks remain pending.

VM-039d: all four input/EOF sources passed across the seven standard columns
(28 executions, zero skips), plus ten BASIC VM/JIT input cells. A direct
compiled JIT peek proof passed with a callback counter and failing fallback.
The normal JIT matrix remains a tiered pipeline; it is not a claim that every
source entry runs compiled. Encoded CIL input remains VM-059.

VM-040 Oct: all twelve BEAM cells executed in fresh processes with positive
single-cell sentinels and no skips. The dedicated real-BEAM corpus separately
checks stdout and zero return values. Intel-8008 semantics remain VM-013.
