---
category: Rust
---

# A trusted kernel must not delegate token grammar to the host language's integer parser

`drat-check` has its own DIMACS/DRAT reader so that a parser bug cannot make
the solver and the checker misread a formula the same way. But the reader
turned tokens into numbers with `str::parse::<i64>()` and `parse::<usize>()`,
and Rust's integer parsers are more generous than DIMACS: they accept a
leading `+`, and they read `-0` as `0`. In a DIMACS reader `0` is the clause
terminator, so `-0` silently ended a clause. `p cnf 1 1` followed by `-0 1 0`
was read as two clauses, where every other reader reports a syntax error. The
kernel could therefore certify a formula that other tools would refuse or read
differently. It was found only by a spec audit that listed, token by token,
what the code accepts; the existing tests all used well-formed input.

The fix is to vet the token's bytes against the grammar you actually specify
(`-?[0-9]+`, no `-0`, header counts digits only) before handing it to a
library parser, and to write the grammar in the spec so each clause has a test.

Do differently: when a spec says "strict", test the lexer with the
near-misses the host parser tolerates (`+1`, `-0`, leading/trailing
whitespace, `1_0`, Unicode digits, `i64` limits), not just with bad garbage.
`from_str` is a convenience for trusted input, not a definition of your
format.
