# Perl release grammars

`releases.csv` inventories 763 numbered releases recorded by the official
`perlhist.pod` snapshot retrieved on 2026-10-07. It expands historical range
rows such as `1.001..10` into one row per release. The `perlhist_line` column
points to the original source line; the snapshot's Git blob ID is in the CSV
header. Compare this inventory with tagged source and CPAN archives before
calling it exhaustive, since a history document may omit an interim release.

The user requested a **separate `.tokens` and `.grammar` file for every
release**, including maintenance and development versions. The filenames are
`perl<V>.tokens` and `perl<V>.grammar`, with `<V>` from the `release` column.
An unchanged syntax release still gets its own checked pair. The CSV status is
`pending`, `partial`, or `complete`; `complete` requires source-backed syntax
coverage and version-specific positive and negative lexer/parser fixtures.

Current pairs:

- `perl1.000.*` is an explicitly **partial** arithmetic and `print` subset
  derived from the original [`perl-1.0` source tag]. It validates as a pair
  and parses representative accepted programs while rejecting unsupported
  syntax. It does not claim full Perl 1.000 syntax coverage.
- `perl.tokens` and `perl.grammar` are the unversioned **partial** Perl 5.38
  execution subset used by LANG81. They are not a substitute for the required
  release-specific `perl5.38.*` files.

Do not route an unknown version to the unversioned pilot or a nearby release.
The version selector must fail explicitly until that release has its own
validated pair. Keep syntax coverage separate from executable semantics;
LANG81's IIR compiler accepts less syntax than a future complete Perl grammar.

Sources: [Perl history], [CPAN source releases], [Perl version policy].

[Perl history]: https://perldoc.perl.org/perlhist
[CPAN source releases]: https://www.cpan.org/src/
[Perl version policy]: https://perldoc.perl.org/perlpolicy
[`perl-1.0` source tag]: https://github.com/Perl/perl5/tree/perl-1.0
