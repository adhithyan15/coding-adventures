# VM-058: preserve both INSPECT region boundaries

Status: semantic design and baseline verification, 2026-09-19.

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
