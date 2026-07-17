# while-to-for — `while (cond) body` → `for (; cond; ) body`

Input: `while(x)a();`

At SIMPLE, closurec canonicalizes every live `while` loop to the equivalent
`for` loop — the form the reference Closure Compiler always emits. A `while`
and a `for` with an empty init *and* empty update are exactly equivalent (no
init runs, and `continue` targets the test in both), so the rewrite is a pure
spelling change.

Expected (SIMPLE): `for(;x;)a();` — byte-identical to the reference Closure
Compiler. The proof the pipeline ran (not a WHITESPACE_ONLY fallback, which
would keep `while(x)a();` verbatim) is that the output spells the loop `for`.

See `closure-pass-fold-control-flow` `fold_while_statement`.
