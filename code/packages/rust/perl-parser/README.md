# Perl parser (Rust)

Parses LANG81's bounded Perl 5.38 `print` arithmetic subset from the
checked-in `code/grammars/perl/perl.grammar` definition. The parser returns
a grammar tree for direct lowering to InterpreterIR. This is not a full Perl
parser.

Historical release files under `code/grammars/perl/` are separate,
explicitly partial token/grammar pairs. Their inventory statuses describe
only the bounded syntax each pair tests; they do not claim complete Perl
release coverage.

The distinct partial 5.004_71 pair uses its own pinned historical archive.
Inventory-wide parser probes cover its bounded arithmetic/`print` acceptance,
250/251-digit boundary, and unsupported numeric/input forms; own-release
probes reject constant-hook and keyword-override contexts. These files do not
change LANG81 execution or claim full 5.004_71 syntax. Its complete numeric
scanner matches 5.004_70; keyword-warning, overload-helper and regex-hook
changes stay outside the standalone subset.
