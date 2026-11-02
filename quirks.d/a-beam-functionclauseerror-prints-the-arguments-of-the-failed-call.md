---
package: elixir/ct-compare
lane: elixir
rung: E0
status: mitigated
---

# A BEAM FunctionClauseError prints the arguments of the failed call

When no clause of an Elixir function matches, BEAM raises
`FunctionClauseError`, and the top frame of its stack trace carries the **full
argument list**. `Exception.format` prints it. For `ct_select_bytes` that meant
both secret byte strings reached any crash log when a caller passed the wrong
type.

**Handled:** every public `ct-compare` function has a catch-all clause that
dispatches on public type only and raises `ArgumentError`; an error raised in a
function body does not record its arguments. A test checks that no secret
appears in the formatted error. The same pattern applies to any Elixir or
Erlang code that handles secrets: guard-only clauses need a catch-all.
