---
category: Compiler / VM / language pipeline
---

# A GrammarParser depth refusal that an optional or repetition backtracks past surfaces as a furthest-failure message, not the depth-limit message

`GrammarParser::with_max_depth` refuses to recurse past the cap and sets
`depth_exceeded`, but `parse()` only reports the dedicated message
(`input nests deeper than the supported limit (N)`) when the *top-level*
rule fails. If the refusal happens inside an optional or a repetition, the
enclosing rule backtracks and succeeds on less input (a `program = { ... }`
rule happily matches nothing), and the error comes from the leftover-token
check instead: `Expected NEWLINE or input within the supported nesting
limit, got "("`.

Found by the Ruby release probes (LANG84 C5): `puts((((…1…))))` 5,000 deep
came back as an ordinary `Parse` error, not a `Budget` error.

**Superseded (LGE-PARSER-DEPTH):** `parse()` now checks the sticky flag
unconditionally and always returns the dedicated message, so this symptom no
longer occurs. Match with `GrammarParseError::is_depth_exceeded()` (or
`DEPTH_EXCEEDED_MESSAGE`). The furthest-failure marker advice below is
obsolete; keeping it as a fallback is harmless.

What to do (obsolete): when mapping parser errors to a budget category, match both the
dedicated message and the guard's furthest-failure marker
(`input within the supported nesting limit`), and keep a hostile-nesting
probe that asserts the category, not just `is_err()`.
