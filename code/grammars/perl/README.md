# Perl release grammars

`releases.csv` inventories 774 numbered releases from the official
`perlhist.pod` snapshot and annotated Perl source tags retrieved on 2026-10-07.
It expands historical range rows such as `1.001..10` into one row per release.
The `perlhist_line` column points to the source line, or names a `tag:v...`
when a release candidate is absent from that document. The history snapshot's
Git blob ID is in the CSV header. A tag comparison found 11 such additional
release candidates. The `v5.17.7.0` tag is excluded: its annotation explicitly
says that no version 5.17.7.0 was released; it only preserves Git ancestry.
On 2026-10-07, all 337 distinct version names from CPAN's `/src/5.0/`
`perl-<version>.tar.{gz,bz2,xz}` entries and all 11 legacy
`perl5.<version>.tar.gz` names were present in this inventory. This comparison
does not establish that the inventory covers every historical public release;
other archive names and pre-Perl-5 sources still need review.

The user requested a **separate `.tokens` and `.grammar` file for every
release**, including maintenance and development versions. The filenames are
`perl<V>.tokens` and `perl<V>.grammar`, with `<V>` from the `release` column.
An unchanged syntax release still gets its own checked pair. The CSV status is
`pending`, `partial`, or `complete`; `complete` requires source-backed syntax
coverage and version-specific positive and negative lexer/parser fixtures.
The Rust `perl-parser` tests load every non-pending pair from this inventory,
cross-check token references, and run shared accepted and malformed-source
probes. These probes verify the recorded subset, not full language coverage.

Current pairs:

- `perl1.000.*` is an explicitly **partial** arithmetic and `print` subset
  derived from the original [`perl-1.0` source tag]. It validates as a pair
  and parses representative accepted programs while rejecting unsupported
  syntax. It does not claim full Perl 1.000 syntax coverage.
- `perl1.0.15.*` and `perl1.0_16.*` are separate **partial** arithmetic and
  `print` subsets checked against the [`perl-1.0.15` source tag] and
  [`perl-1.0.16` source tag] yacc grammars, respectively.
  The two tagged `perl.y` files match, but each release has its own pair;
  neither pair claims complete syntax coverage.
- `perl2.000.*` and `perl2.001.*` are separate **partial** arithmetic and
  `print` subsets checked against the [`perl-2.0` source tag] and
  [`perl-2.001` source tag] yacc grammars and tokenizers. Both tagged
  `perl.y` files match; neither pair claims complete syntax coverage.
- `perl3.000.*` and `perl3.044.*` are separate **partial** arithmetic and
  `print` subsets checked against the [`perl-3.000` source tag] and
  [`perl-3.044` source tag] yacc grammars and tokenizers. They cover only the
  listed forms, not complete Perl 3 syntax.
- `perl4.000.*` and `perl4.036.*` are separate **partial** arithmetic and
  `print` subsets checked against the [`perl-4.0.00` source tag] and
  [`perl-4.0.36` source tag] yacc grammars and tokenizers. Their tagged
  `perly.y` and `toke.c` files differ; neither pair claims complete Perl 4
  syntax coverage.
- `perl5.000.*` is a separate **partial** arithmetic and `print` pair checked
  against the [`perl-5.000` source tag]'s `perly.y` and `toke.c`. It does not
  claim complete Perl 5.000 syntax coverage.
- `perl5.001.*` is a separate **partial** arithmetic and `print` pair checked
  against the [`perl-5.001` source tag]'s `perly.y` and `toke.c`. The tagged
  `perly.y` has the same Git blob as 5.000, while `toke.c` differs; both source
  tags still require separate checked pairs. This covers only the listed forms.
- `perl5.001n.*` is a separate **partial** arithmetic and `print` pair checked
  against the [`perl-5.001n` source tag]'s `perly.y` and `toke.c`. Its tagged
  `perly.y` still matches 5.001, while `toke.c` differs. This covers only the
  listed forms.
- `perl5.002.*` is a separate **partial** arithmetic and `print` pair checked
  against the [`perl-5.002` source tag]'s `perly.y` and `toke.c`. Both tagged
  files differ from 5.001n, and this pair covers only the listed forms.
- `perl5.002_01.*` is a separate **partial** pair checked against the
  [`perl-5.002_01` source tag]'s `perly.y` and `toke.c`. Its `perly.y` matches
  5.002 while `toke.c` differs; this pair covers only the listed forms.
- `perl5.38.2.*` is a distinct **partial** pair for the LANG81 print-arithmetic
  subset, checked against the [`v5.38.2` source tag] and Perl 5.38.2 runtime.
  It rejects syntax outside that pilot and does not claim full coverage.
- `perl5.44.0.*` and `perl5.45.3.*` are distinct **partial** pairs for the
  current stable and development endpoints in the pinned inventory. Their
  arithmetic and `print` subset is documented by each release's tagged
  `perlop.pod` and `perlfunc.pod`; they do not claim complete coverage.
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
[`perl-1.0.15` source tag]: https://github.com/Perl/perl5/tree/perl-1.0.15
[`perl-1.0.16` source tag]: https://github.com/Perl/perl5/tree/perl-1.0.16
[`perl-2.0` source tag]: https://github.com/Perl/perl5/tree/perl-2.0
[`perl-2.001` source tag]: https://github.com/Perl/perl5/tree/perl-2.001
[`perl-3.000` source tag]: https://github.com/Perl/perl5/tree/perl-3.000
[`perl-3.044` source tag]: https://github.com/Perl/perl5/tree/perl-3.044
[`perl-4.0.00` source tag]: https://github.com/Perl/perl5/tree/perl-4.0.00
[`perl-4.0.36` source tag]: https://github.com/Perl/perl5/tree/perl-4.0.36
[`perl-5.000` source tag]: https://github.com/Perl/perl5/tree/perl-5.000
[`perl-5.001` source tag]: https://github.com/Perl/perl5/tree/perl-5.001
[`perl-5.001n` source tag]: https://github.com/Perl/perl5/tree/perl-5.001n
[`perl-5.002` source tag]: https://github.com/Perl/perl5/tree/perl-5.002
[`perl-5.002_01` source tag]: https://github.com/Perl/perl5/tree/perl-5.002_01
[`v5.38.2` source tag]: https://github.com/Perl/perl5/tree/v5.38.2
