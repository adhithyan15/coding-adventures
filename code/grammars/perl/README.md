# Perl release grammars

`releases.csv` inventories 774 numbered releases from the official
`perlhist.pod` snapshot and annotated Perl source tags retrieved on 2026-10-07.
It expands historical range rows such as `1.001..10` into one row per release.
The `perlhist_line` column points to the source line, or names a `tag:v...`
when a release candidate is absent from that document. The history snapshot's
Git blob ID is in the CSV header. A tag comparison found 11 such additional
release candidates. The `v5.17.7.0` tag is excluded: its annotation explicitly
says that no version 5.17.7.0 was released; it only preserves Git ancestry.
Compare this inventory with CPAN archives before calling it exhaustive.

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
- `perl5.38.2.*` is a distinct **partial** pair for the LANG81 print-arithmetic
  subset, checked against the [`v5.38.2` source tag] and Perl 5.38.2 runtime.
  It rejects syntax outside that pilot and does not claim full coverage.
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
[`v5.38.2` source tag]: https://github.com/Perl/perl5/tree/v5.38.2
