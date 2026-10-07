# ruby-iir-compiler

This pilot parses Ruby 3.0 source with the repository's Rust Ruby parser,
lowers a bounded integer-expression and `puts` subset directly to
InterpreterIR, then runs it on the Rust `vm-core` interpreter. Semantic IR and
a host Ruby runtime are outside the execution path.

`rubyvm <file.rb>` runs a source file. The first slice accepts parenthesized
one-argument `puts` calls with decimal integers, grouping, unary minus,
and `+`, `-`, `*`, `/`. Every intermediate must fit in `i64`; division uses
Ruby's floor rule. Other syntax is rejected. See
[LANG80](../../../specs/LANG80-ruby-direct-iir-pilot.md).
