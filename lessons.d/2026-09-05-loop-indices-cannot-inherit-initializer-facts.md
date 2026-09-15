# 2026-09-05 — Loop indices cannot inherit initializer facts

A string-folding pass that sees `j = 0` before a back edge cannot substitute
that zero into every `str_index(s, j)`. Exclude multiply defined integer
registers from literal metadata and route unknown indices through the checked
runtime helper. Run delimiter scans to validate loop-carried byte reads; a
frontend oracle or literal-index test alone misses this native/LLVM gap.


### WASM runtime string concatenation must preserve aliased operands (2026-09-05)

INSPECT replacement exposed `str_concat result = result, character`: assigning
the fresh handle to result before copying overwrote the input handle, producing
NUL output or a trap. Preserve operand locals through all length and byte reads;
write the destination last. Exercise left, right and double aliasing explicitly.

Direct WASM IIR tests must give `str_eq` an i64/i32 result hint; `bool` is
rejected by the backend validator before execution. Use the established test ABI.


### The destination-before-read hazard was not unique to str_concat (2026-09-07)

VM-056 fixed `str_concat`; the same "assign the destination local, then read
an operand local that may be the same local" shape existed in `str_slice`'s
runtime path too, guessed at as a suspected defect (VM-057) rather than
confirmed. A raw IIR-level probe (a `str_slice` whose dest and source share a
variable name) reproduced wrong output before any fix — corrupted bytes
instead of the correct slice — proving it was a real, not merely suspected,
bug. The repair is the identical pattern: address the fresh block through the
not-yet-advanced bump global while the source local is still needed, and
`local.set` the destination only after the last read.

Before trusting "just one op was buggy," grep every other lowering site that
shares the same bump-allocate-and-copy shape (`ARRAY_BUMP_GLOBAL` +
`memory.copy`) for the same ordering hazard. `alloc_array` copies no existing
operand's bytes (only a requested length), so it was never exposed; the LLVM
backend's `str_slice`/`str_concat` already read every operand into a value
before overwriting the destination `env` entry, with a comment recording that
was deliberate — so the class of bug is backend-lowering-shape-specific
(mutable-local reuse keyed by variable name), not something to assume
recurs in every backend just because one had it.

Finding the actual real-world trigger mattered: COBOL reference-modification
`MOVE base(i:j) TO dst` does NOT reach this hazard (`ref_mod_slice` always
materializes into a fresh temp before the final reshape write), but
`STRING <item> DELIMITED BY SIZE INTO <same item>` does — a lone sending
field's register is returned directly by `string_source` with no temporary,
so the truncating `str_slice`'s destination and source are the identical
local. Guessing at "a MOVE-shaped example" instead of tracing the actual
register data flow would have produced a regression that never touched the
buggy path.
