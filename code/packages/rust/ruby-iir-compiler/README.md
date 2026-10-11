# ruby-iir-compiler

This pilot parses Ruby 3.0 source with the repository's Rust Ruby parser,
lowers a bounded integer-expression and `puts` subset directly to
InterpreterIR, then runs it on the Rust `vm-core` interpreter. Semantic IR and
a host Ruby runtime are outside the execution path.

`rubyvm <file.rb>` runs a source file. The bounded subset accepts one-argument
`puts(1 + 2)` and Ruby's bare `puts 1 + 2` with plain decimal integer tokens,
grouping, unary minus, and `+`, `-`, `*`, `/`. An exact empty `puts()` call emits
one newline. Exact parenthesized two- and three-integer calls such as
`puts(1, 2)` and `puts(1 + 2, -7 / 2, 3 * 4)` emit each value on its own line.
All expressions execute in source order; each call checks its combined output
size before appending any of its lines. Every intermediate must fit in `i64`;
division uses Ruby's floor rule. Strings containing digits and legacy
leading-zero octal literals are rejected. Four or more arguments, bare
multi-argument calls, splats, keyword arguments, and other syntax are rejected. See
[LANG80](../../../specs/LANG80-ruby-direct-iir-pilot.md).

`compile_source` limits input to 64 KiB. `compile_ast` also limits directly
supplied tree items, nesting, text bytes, and module-name length before
lowering.

Direct AST arithmetic and grouping require canonical punctuation token kinds,
exact spelling and no named effective-type override. Grouping also requires
the actual `factor` shape with exactly one `expression` between parentheses.
Forged syntax tokens fail before producing IIR; this contract covers these
syntax positions rather than every field of an arbitrary caller-built tree.
