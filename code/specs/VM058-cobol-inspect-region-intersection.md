# VM-058: preserve both INSPECT region boundaries

Status: implemented and locally validated, 2026-09-19; PR pending.

## Contract

For each INSPECT item, allow at most one BEFORE and one AFTER boundary. Find
both delimiters against the original inspected field before comparison starts.
BEFORE supplies the exclusive end (field length if absent); AFTER supplies the
start immediately after its first match (empty window if absent). Apply both
restrictions; clamp a crossed window to empty. Keyword order must not discard
either boundary. Delimiters themselves are outside the eligible window.

Micro Focus defines each first occurrence against identifier-1 before the first
comparison cycle (General Rules 13b/13c). IBM permits one phrase of each kind.
Combined TALLYING/REPLACING executes the tally phase first.
Sources:
- https://www.microfocus.com/documentation/reuze/60d/lhpdf90c.htm
- https://www.ibm.com/docs/en/cobol-zos/6.5.0?topic=statements-inspect-statement

## Discriminating observations

TALLYING ALL "0" on `0A00B0`:
- BEFORE "B" AFTER "A", or reversed phrase order: 2.
- BEFORE "Z" AFTER "A": 3 (no upper delimiter).
- BEFORE "B" AFTER "Z": 0 (no lower delimiter).
- BEFORE "A" AFTER "B": 0 (crossed bounds).
- On `00X00`, BEFORE "X" AFTER "X": 0, replacing the current wrong 2.

Include repeated-delimiter cases with an upper delimiter before and after the
lower delimiter; first occurrence is chosen from the entire original field.
REPLACING ALL "0" BY "*" in the first window yields `0A**B0`. CONVERTING
"0" TO "*" uses the same window. Test LEADING and CHARACTERS, multi-item
and multi-counter readers, combined phases, and identifier delimiters too.
Duplicate BEFORE or duplicate AFTER must reject explicitly. Preserve current
single-character delimiter limitations.

## Implementation plan

Current audit finds seven compiler child_node inspect_region readers and eight
runtime readers, not the nine total mentioned by the older backlog. Centralize
collection/validation in each engine, then route every reader through it.
Represent both boundaries explicitly and update the shared runtime window helper
and compiler window emitter. Clamp crossed bounds before length subtraction or
slicing. Preserve per-item windows and original-source matching in replacement.

First run the existing first-region-only regression as baseline evidence. Then
replace it with expected observations independent of oracle agreement. Run
compiler JIT/oracle tests, runtime tests and Clippy. Only after these pass, add
a bounded matrix row and execute all declared backend cells, updating counts
and documentation from actual results. Check downstream use of public Region
before changing its representation. Security review precedes publication.

## Results

Both engines now collect every region sibling through one reader per engine.
All seven compiler and eight runtime call sites use the new reader. Runtime
Region holds independent before/after operands; compiler InspectRegion holds
copyable AST borrows. The existing single-boundary emitter is reused for each
boundary, then intersected and clamped. No remaining in-repository use of the
old runtime RegionKind or kind/delim fields was found.

Eight boundary observations, eleven reader-family observations and two duplicate
rejections pass with expected output checked separately on oracle and compiled
VM/JIT harness. This harness may use VM fallback; it is not proof of machine JIT.
The complete compiler/runtime suites pass (83 compatibility, 22 compiler unit,
640 compiled/oracle integration, 219 runtime tests and 1 runtime doc test).
The new matrix row was then executed on NativeAOT, LLVM, WASM, JVM, real CoreCLR,
VM, JIT harness and real Erlang before declaration. CoreCLR required adding the
installed Framework64 ilasm directory to this process's PATH; no files installed.
Coverage guard, Clippy and document-shard validation pass. COBOL declarations
are now 59 rows / 472 cells; non-ALGOL 211 rows / 1688 cells. The full pre-existing
matrix was not rerun on every backend in this slice.
