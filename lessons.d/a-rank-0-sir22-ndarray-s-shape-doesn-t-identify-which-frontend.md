# A rank-0 SIR22 `NDArray`'s shape doesn't identify which frontend produced it — gate display conventions on `source_language`, not value shape

**Context:** Fixing `semantic-ir-to-javascript`'s three APL monadic-scalar-atom
bugs (`neg` printing ASCII `-5` instead of APL's high-minus `¯5`; `neg` on a
rank ≥ 1 array giving `NaN`; `sign`/`recip`/`ceil`/`floor` crashing outright) —
bugs found by `apl-to-semantic-ir/tests/oracle.rs`. The task's own suggested
fix (verified before implementing, per its own explicit instruction) was: "make
`neg` check whether its operand IS a genuine NDArray, and if so, preserve the
box (any rank including 0) so `formatSeen` routes it through the high-minus
`ArrayRt.display` path; only fall back to the old behavior for a non-NDArray
operand."

**Mistake almost made:** That fix looks locally correct and is exactly what a
"generalize from the bug report" pass would produce — but a rank-0 `{shape: [],
data}` NDArray is genuinely NOT unique to APL. `matlab-to-semantic-ir`'s `^`/
`.^` unconditionally lower to `ElementwiseOp::Pow` even for two literals (no
scalar fast path for power), so a plain MATLAB `2 ^ 2` reaches the byte-for-byte
identical rank-0 representation an APL scalar does by the time it reaches
`neg`. `matlab-to-semantic-ir/tests/oracle.rs`'s own `unary_minus_on_power` case
(`-2 ^ 2` must print ASCII `-4`) already exercises exactly this shape. Applying
the suggested fix (box-preserve ANY rank, including 0) would have flipped that
MATLAB case to high-minus `¯4`, silently breaking an existing, currently-green
downstream oracle test — a real regression that a purely value-shape-driven
fix cannot avoid, because the two languages' scalars are representationally
indistinguishable at that point.

**How it was caught:** Traced the actual generated JS for `-2 ^ 2` (a scratch
`cargo test`-based dump of the real compiled output, not just reading source)
*before* writing the fix, and separately traced the actual generated JS for a
bare APL `-5` (also via a scratch dump) — which revealed the operand there is
a BARE `IntLit`-shaped number, never an NDArray at all, meaning the "box
preservation" idea wouldn't even fix the literal bug-report example. Both
traces disproved a plausible-sounding, not-yet-tested design before it was
implemented.

**Fix / rule:** Only the SOURCE LANGUAGE (`module.metadata.source_language`),
never the runtime value's own shape, can decide a *display convention* when
two unrelated frontends can legally produce the identical runtime
representation. The existing precedent for this is `SIR_DISPLAY_RUBY` (a
per-module boolean baked into the emitted JS via a `__SIR_DISPLAY_RUBY__`
placeholder substitution, gating `true`/`false` vs `#t`/`#f`) — the fix here
added a second, analogous `SIR_DISPLAY_APL_HIGH_MINUS` flag rather than
inventing a new mechanism. Concretely: keep VALUE-computing functions
(`neg`/`sign`/`recip`/`ceil`/`floor`) unaware of source language entirely — a
rank-0 operand always unwraps to a bare number exactly as before, for every
language — and make ONLY the actual glyph decision, inside `formatSeen`
(the shared `print`/`puts`/`format` display path), consult the new flag. This
cleanly separates "is the VALUE correct" (source-language-independent, safe to
fix unconditionally — e.g. the rank ≥ 1 NDArray branch, which no frontend
besides APL ever prints raw anyway) from "is the DISPLAY SPELLING correct"
(source-language-dependent, needs the flag). Before trusting a "box-preserve
the value to fix the display" pattern for a value that flows through a SHARED
backend, check whether every consumer that can construct that same runtime
shape agrees on how it should be displayed — grep for other frontends
lowering to the same builtin/node and read their own oracle/e2e test
expectations, don't assume the bug report's one example generalizes.
