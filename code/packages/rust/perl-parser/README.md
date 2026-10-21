# Perl parser (Rust)

`release_parser_grammar(id)` returns the compiled parser grammar of exactly
one checked release from `code/grammars/perl/releases.csv` (LANG84); the 117
pairs share 2 distinct grammars. `parse_perl` below keeps using the
unversioned pair. Release selection itself lives in `perl-iir-compiler`.

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


The separate partial 5.004_72 pair uses its own pinned archive. Its
`TMP_CRLF_PATCH` allows carriage returns as whitespace in the standalone
arithmetic-print subset; CR does not concatenate numeric tokens or admit
strings/heredocs. The own numeric scanner matches 5.004_71 and retains the
250-digit bound. Inventory-wide numeric/input probes and own-release CR and
unsupported-context probes cover this declared contract. Earlier release
whitespace contracts and LANG81 execution are unchanged. Full syntax and
exhaustive release coverage remain open.
