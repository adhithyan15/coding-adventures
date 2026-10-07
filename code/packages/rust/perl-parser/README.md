# Perl parser (Rust)

Parses LANG81's bounded Perl 5.38 `print` arithmetic subset from the
checked-in `code/grammars/perl/perl.grammar` definition. The parser returns
a grammar tree for direct lowering to InterpreterIR. This is not a full Perl
parser.
