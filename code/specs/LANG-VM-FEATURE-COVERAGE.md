# LANG VM feature and backend coverage

Audit base: `cd73f3ad86` (2026-09-05); corpus counts updated for VM-047b. This is an inventory of the implemented
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

| Frontend | Unified rows | Declared standard cells | Additional proof boundary |
|---|---:|---:|---|
| Twig | 49 | 343 | 20 BEAM cells; dedicated heap/closure tests |
| Nib | 26 | 182 | Intel 4004 BCD simulator proof, separate from standard targets |
| Brainfuck | 6 | 42 | Dedicated WASM/JVM/CLR and JIT execution |
| Dartmouth BASIC | 51 | 357 | Random differential suite and frontend JIT tests |
| Oct | 12 | 84 | Frontend JIT control-flow tests |
| ALGOL 60 | 232 | 1624 | Separate owner; full-matrix CI exclusion remains VM-025 |
| FLOW-MATIC | 4 | 28 | Frontend EOF/record-stream JIT tests |
| COBOL-60 | 52 | 364 | Much larger frontend JIT/oracle suite |
| McCarthy Lisp | 0 | 0 | Dedicated 19-program capstone with nine runner lanes |
| Macsyma | 0 | 0 | Dedicated 21-program capstone with seven runner lanes |

The normal non-ALGOL capstone therefore declares 200 programs and 1420 cells.
The zeroes for McCarthy and Macsyma mean dedicated coverage, not absent support.
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
| [COBOL lowerer](../packages/rust/cobol-iir-compiler/src/lib.rs) | PICTURE/scaled arithmetic, DISPLAY/MOVE, condition names, IF/EVALUATE, PERFORM/GOTO, COMPUTE/power, size errors and signed/alphanumeric operations occur in the matrix, now expanded to 52 rows with ASCII reference modification: literal/computed bounds, comparisons, MOVE fitting and invalid-bound traps, plus STRING SIZE full-width copying, truncation and untouched tails, STRING delimiters and UNSTRING field fitting/empty fields/exhaustion, and pointer/overflow branches. The [JIT/oracle suite](../packages/rust/cobol-iir-compiler/tests/jit_e2e.rs) additionally exercises reference modification, STRING, UNSTRING and INSPECT families. | VM-045 adds reference-modification rows; VM-046a/b/c add STRING SIZE, delimiter/splitting and pointer/overflow rows; VM-047a adds ALL/CHARACTERS/LEADING tallying proofs; VM-047b adds replacement including first-match/non-rechaining; region proofs remain VM-047c. Validator acceptance is insufficient. Existing byte/character and category restrictions must remain explicit. |
| [McCarthy lowerer](../packages/rust/mccarthy-lisp-iir-compiler/src/lib.rs) | Quote/cons/CAR/CDR/ATOM/EQ/COND, direct and higher-order lambdas, captured variables, LABEL recursion and closure values. [Capstone](../packages/rust/lang-aot/tests/conformance.rs) tests 19 integer-result programs; [frontend run tests](../packages/rust/mccarthy-lisp-iir-compiler/tests/run_e2e.rs), [JIT](../packages/rust/lang-aot/tests/jit_mccarthy.rs) and dedicated per-backend lambda suites cover further shapes. | VM-036 enables the native capstone on Linux/macOS/Windows, with a required native-only Windows CI run. The capstone is not a proof for every closure shape; preserve dedicated closure suites in normal BUILD. |
| [Macsyma lowerer](../packages/rust/macsyma-iir-compiler/src/lower.rs) | v0 integers, unary/binary arithmetic, exact literal division, assignments, symbols and unevaluated symbolic Apply. [Oracle suite](../packages/rust/macsyma-iir-compiler/tests/oracle.rs) compares symbolic results with the evaluator. [Capstone](../packages/rust/lang-aot/tests/macsyma_conformance.rs) proves 21 integer-result programs on VM/JIT/WASM/CLR/JVM/LLVM/native when present. | No BEAM runner here (VM-038); symbolic result representation has VM oracle coverage, not portable capstone agreement (VM-048). Function definitions/calls, control flow, floats, lists, comparisons and power are explicit frontend rejections, outside implemented v0 parity. |

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

Reproduce the dedicated capstones with:

```sh
cargo test -p lang-aot --test conformance --test macsyma_conformance -- --nocapture
```

McCarthy always runs VM/JIT/WASM/CLR simulator. Java, clang, Erlang and real
CLR require their respective installed tools; native uses the host Linux/macOS/Windows compiler and linker.
Macsyma always runs VM/JIT/WASM/CLR simulator, gates JVM/LLVM/native on tools,
and has no real-CLR or BEAM runner. VM-049 adds a real-CLR arithmetic proof;
simulator success alone does not claim .NET execution.

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
