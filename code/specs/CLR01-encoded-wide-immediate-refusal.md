# CLR01: refuse lossy encoded integer immediates

Selected 2026-09-19 during encoded CLR input prerequisite audit.

The encoded backend narrows Operand::Int(i64) with unchecked as i32 in const
and direct call arguments. The simulator stores Int(i32), and its literal
instruction set currently stops at ldc.i4. Full input_i64 support therefore
requires more than registering a host reader.

Prioritize preventing silent wrong artifacts before expanding the scalar ABI.
First reproduce source 4294967296 becoming 0 on the actual encoded simulator.
Then reject out-of-i32 integer immediates during encoded validation, naming the
function, operation, offending value and supported range. Do this before closure
early-accept paths; all Operand::Int positions must be range checked, including
call arguments and field ordinals. Retain checked conversions at literal emission
sites. Preserve valid i32 boundaries and the separate textual CoreCLR int64 path.

Tests cover both out-of-range neighbors, i64 extrema, direct call immediates,
and exact i32 boundaries; source-level encoded compilation must refuse while
text emission still preserves the full-width literal. Run backend and encoded
simulator consumer tests, Clippy and document metadata checks before ready PR.

This is an explicit refusal, not implemented full-width arithmetic or host input.
Next audit ldc.i8, locals/arguments, arithmetic/comparisons, boxing and call/return
before choosing a safe full-width representation slice. Runtime arithmetic can
still exceed i32 even when inputs fit; do not claim this gate solves that separate
representation gap. No change to textual CoreCLR or the eight-column corpus.

CI follow-up: COBOL nested division emits scale-12 constants (1000000000000)
and a scale-10 constant (10000000000). Its backend acceptance test previously
mistook encoded validator acceptance for representability. Preserve WASM/JVM
acceptance, assert explicit encoded CLR refusal and retained textual int64
emission for this program. Audit downstream frontend compatibility suites when
tightening a shared validator; the original backend-only checks missed this.
