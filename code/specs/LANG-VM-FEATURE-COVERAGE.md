# LANG VM feature and backend coverage

Audit base: `cd73f3ad86` (2026-09-05). This is an inventory of the implemented
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
| Oct | 9 | 63 | Frontend JIT control-flow tests |
| ALGOL 60 | 232 | 1624 | Separate owner; full-matrix CI exclusion remains VM-025 |
| FLOW-MATIC | 1 | 7 | Frontend EOF/record-stream JIT tests |
| COBOL-60 | 21 | 147 | Much larger frontend JIT/oracle suite |
| McCarthy Lisp | 0 | 0 | Dedicated 19-program capstone with nine runner lanes |
| Macsyma | 0 | 0 | Dedicated 21-program capstone with seven runner lanes |

The normal non-ALGOL capstone therefore declares 163 programs and 1161 cells.
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
| [Oct lowerer](../packages/rust/oct-iir-compiler/src/lib.rs) | u8 arithmetic/masking, bitwise/logical operations, functions, local/global state, if/while/loop/break and stdout `out`. Matrix covers output, wrap, short circuit and shared globals; [JIT suite](../packages/rust/oct-iir-compiler/tests/jit_e2e.rs) separately executes while loops and returned function values. | Add observable loop/break and returned-call standard-column proofs (VM-044). `in`, carry arithmetic and rotations are explicit intrinsic errors; VM-013 owns portable machine-state design. Body-local static and floats are not implemented parity gaps. |
| [ALGOL lowerer](../packages/rust/algol-iir-compiler/src/lib.rs) | Scalar integer/boolean/real/string operations, arrays, procedures, by-name specializations, switches and nonlocal control flow have a substantial evolving corpus. [Frontend JIT](../packages/rust/algol-iir-compiler/tests/jit_e2e.rs) and [AOT smoke](../packages/rust/algol-iir-compiler/tests/aot_smoke.rs) are separate proofs. | Full LANG matrix remains excluded for the recorded native-array failure. VM-025 and the separate ALGOL owner control fixes and detailed feature expansion; 232 declarations do not mean 232 green Linux programs. |
| [FLOW-MATIC lowerer](../packages/rust/flow-matic-iir-compiler/src/lib.rs) | MOVE, COMPARE/IF/OTHERWISE, GO TO/JUMP, STOP, READ-ITEM/EOF and WRITE-ITEM. The unified row proves only scalar move/output. [JIT stream tests](../packages/rust/flow-matic-iir-compiler/tests/jit_e2e.rs) run read/process/write to EOF through custom `input_more`/`input_i64` builtins. | VM-037 adds discriminating control-flow matrix rows. VM-039 provides portable EOF-aware input before promoting record streams to code-generation columns. TRANSFER and tape control are clean frontend rejections, not secretly implemented file I/O. |
| [COBOL lowerer](../packages/rust/cobol-iir-compiler/src/lib.rs) | PICTURE/scaled arithmetic, DISPLAY/MOVE, condition names, IF/EVALUATE, PERFORM/GOTO, COMPUTE/power, size errors and signed/alphanumeric operations occur in the 21-row matrix. The [JIT/oracle suite](../packages/rust/cobol-iir-compiler/tests/jit_e2e.rs) additionally exercises reference modification, STRING, UNSTRING and INSPECT families. | These later string families lack unified executable rows; validator acceptance is insufficient. VM-045/046/047 separate reference modification, STRING/UNSTRING and INSPECT backend proofs. Existing byte/character and category restrictions must remain explicit. |
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
