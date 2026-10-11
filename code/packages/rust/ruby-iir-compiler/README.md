# ruby-iir-compiler

This pilot parses Ruby 3.0 source with the repository's Rust Ruby parser,
lowers a bounded integer-expression and `puts` subset directly to
InterpreterIR, then runs it on the Rust `vm-core` interpreter. Semantic IR and
a host Ruby runtime are outside the execution path.

`rubyvm <file.rb>` runs a source file. The bounded subset accepts one-argument
`puts(1 + 2)` and Ruby's bare `puts 1 + 2` with plain decimal integer tokens,
grouping, unary minus, and `+`, `-`, `*`, `/`. An exact empty `puts()` call emits
one newline. An exact parenthesized two-integer call such as `puts(1, 2)`
emits each value on its own line. Every intermediate must fit in `i64`;
division uses Ruby's floor rule. Strings containing digits and legacy
leading-zero octal literals are rejected. Other syntax is rejected. See
[LANG80](../../../specs/LANG80-ruby-direct-iir-pilot.md).

`compile_source` limits input to 64 KiB. `compile_ast` also limits directly
supplied tree items, nesting, text bytes, and module-name length before
lowering.
