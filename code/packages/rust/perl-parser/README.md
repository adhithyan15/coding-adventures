# Perl parser (Rust)

Parses LANG81's bounded Perl 5.38 `print` arithmetic subset from the
checked-in `code/grammars/perl/perl.grammar` definition. The parser returns
a grammar tree for direct lowering to InterpreterIR. This is not a full Perl
parser.

Historical release files under `code/grammars/perl/` are separate,
explicitly partial token/grammar pairs. Their inventory statuses describe
only the bounded syntax each pair tests; they do not claim complete Perl
release coverage.
