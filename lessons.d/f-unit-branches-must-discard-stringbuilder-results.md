# F# unit branches must discard StringBuilder results

### The compiler failure

An F# `if` used only for side effects expects its branches to return `unit`.
`StringBuilder.Clear()` returns the builder, so mixing it with a unit-valued
branch causes a type constraint error.

### The fix

Explicitly discard the fluent result with `|> ignore` when clearing a builder
as a statement. This keeps the branch's type visibly `unit`.
