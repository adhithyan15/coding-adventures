# for-body-fuse — `for(…){a();b();}` → `for(…)a(),b();`

Input: `for(;x;){a();b();}`

At SIMPLE, closurec fuses a `for` loop's block body — when every statement is a
plain expression statement — into a single comma-sequenced expression statement,
dropping the braces. The comma operator runs the statements left-to-right with
the same side effects, and a loop body discards the value, so the rewrite is
behaviour-preserving.

Expected (SIMPLE): `for(;x;)a(),b();` — byte-identical to the reference Closure
Compiler. The proof the pipeline ran (not a WHITESPACE_ONLY fallback, which
would keep `for(;x;){a();b()}`) is the absence of the body braces.

A body carrying a declaration / `break` / `continue` / `return` / nested `if` or
loop is left as a block. See `closure-pass-fold-control-flow`'s
`fold_for_statement`.
