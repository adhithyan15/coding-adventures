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

The first 18 versioned pairs, from 1.000 through 5.003_03, now limit their
integer token to plain decimal forms (`0` or a nonzero first digit). Their
original tokenizers treat a leading zero as octal and reject `08` or `09`;
the pair grammar cannot yet represent octal values accurately, so it also
rejects valid octal forms such as `012` rather than reading them as decimal.
This boundary is grounded in the tagged Perl 1
[`perly.c`](https://github.com/Perl/perl5/blob/perl-1.0/perly.c) and Perl
[2](https://github.com/Perl/perl5/blob/perl-2.0/toke.c),
[3](https://github.com/Perl/perl5/blob/perl-3.000/toke.c),
[4](https://github.com/Perl/perl5/blob/perl-4.0.36/toke.c), and
[5](https://github.com/Perl/perl5/blob/perl-5.003_03/toke.c) tokenizers.

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
  5.002 while `toke.c` differs; this pair covers only the listed forms and
  rejects adjacent decrement `--` while allowing spaced binary/unary minus.
- `perl5.003.*` is a separate **partial** pair checked against the
  [`perl-5.003` source tag]'s `perly.y` and `toke.c`. Both tagged source files
  match 5.002_01, but this release retains distinct files and limited scope.
- `perl5.003_01.*` is a separate **partial** pair checked against the
  [`perl-5.003_01` source tag]'s `perly.y` and `toke.c`. Both tagged files
  differ from 5.003; this pair covers only the documented arithmetic forms.
- `perl5.003_02.*` is a separate **partial** pair checked against the
  [`perl-5.003_02` source tag]'s `perly.y` and `toke.c`. Its `perly.y` matches
  5.003_01; the `toke.c` changes concern PerlIO and constant subroutines,
  outside this bounded arithmetic and `print` subset. Distinct release files
  are still required, and this pair does not claim complete syntax coverage.
- `perl5.003_03.*` is a separate **partial** pair checked against the
  [`perl-5.003_03` source tag]'s `perly.y` and `toke.c`. Its `perly.y` matches
  5.003_02; the tokenizer changes only add a system include and replace
  `bcmp` with `memcmp` in quoted-term scanning. The listed arithmetic and
  `print` subset is unchanged, and this pair does not claim full coverage.
- `perl5.003_04.*` is a separate **partial** pair checked against the
  [`perl-5.003_04` source tag]'s `perly.y` and `toke.c`. Its `perly.y` matches
  5.003_03; the sole tokenizer change adds a strict-subs guard for `truncate`,
  outside this bounded arithmetic and `print` subset. This pair does not
  claim full Perl 5.003_04 coverage. It accepts plain decimal integers;
  multi-digit leading-zero forms are excluded because Perl treats them as
  octal, and `08`/`09` are invalid octal literals.
- `perl5.003_05.*` is a separate **partial** pair checked against the
  [`perl-5.003_05` source tag]'s `perly.y` and `toke.c`. Both tagged source
  files match 5.003_04; this release still has its own files and claims only
  the listed plain-decimal arithmetic and `print` subset. Leading-zero
  multi-digit forms remain excluded.
- `perl5.003_06.*` is a separate **partial** pair checked against the official
  [5.003_06 patch commit]. Its message introduces the 5.003_06 patch and its
  `patchlevel.h` identifies subversion 6. That commit's `perly.y` and `toke.c`
  blobs match the later 5.003_07 tag; only the listed arithmetic and `print`
  subset is represented, with leading-zero multi-digit forms excluded. The
  source repository has no 5.003_06 tag.
- `perl5.003_07.*` is a separate **partial** pair checked against the
  [`perl-5.003_07` source tag]'s `perly.y` and `toke.c`. Its tagged yacc file
  matches 5.003_05; tokenizer changes include line buffering and sort handling
  outside the listed arithmetic and `print` subset. It does not claim full
  release coverage. Leading-zero multi-digit forms remain excluded.
- `perl5.003_08.*` is a separate **partial** pair checked against the
  [`perl-5.003_08` source tag]'s `perly.y` and `toke.c`. The yacc file adds
  lexical `my` scope, and the tokenizer changes interpolation and octal/hex
  overflow handling. Those forms remain outside the plain-decimal arithmetic
  and `print` subset; leading-zero multi-digit forms are rejected.
- `perl5.003_09.*` is a separate **partial** pair checked against the
  [`perl-5.003_09` source tag]'s `perly.y` and `toke.c`. Its yacc changes
  rearrange lexical scope handling; tokenizer changes affect sigils,
  interpolation and numeric conversion. Only the plain-decimal arithmetic
  and `print` subset is represented, with leading-zero forms rejected.
- `perl5.003_10.*` is a separate **partial** pair checked against the
  [`perl-5.003_10` source tag]'s `perly.y` and `toke.c`. Its yacc blob matches
  5.003_09; tokenizer changes affect sigil spacing and built-in/identifier
  handling outside this plain-decimal arithmetic and `print` subset.
- `perl5.003_11.*` is a separate **partial** pair checked against the
  [`perl-5.003_11` source tag]'s `perly.y` and `toke.c`. Yacc changes operator
  token types and block bookkeeping; tokenizer changes cover sigils, patterns
  and numeric-locale setup outside this plain-decimal arithmetic/print subset.
- `perl5.003_12.*` is a separate **partial** pair checked against the
  [`perl-5.003_12` source tag]'s `perly.y` and `toke.c`. Its yacc changes to
  while/until block forms lie outside the accepted plain-decimal arithmetic
  and `print` subset; leading-zero forms remain rejected.
- `perl5.003_13.*` is a separate **partial** pair checked against the
  [`perl-5.003_13` source tag]'s `perly.y` and `toke.c`. Yacc is unchanged
  from 5.003_12; tokenizer label recognition changes lie outside this subset.
- `perl5.003_14.*` is a separate **partial** pair checked against the
  [`perl-5.003_14` source tag]'s `perly.y` and `toke.c`. Deprecated
  conditional/loop block forms are removed outside this arithmetic/print subset.
- `perl5.003_15.*` is a separate **partial** pair checked against the
  [`perl-5.003_15` source tag]'s `perly.y` and `toke.c`. Both tagged source
  blobs match 5.003_14, while the release keeps its own pair of files.
- `perl5.003_16.*` is a separate **partial** pair checked against the
  [`perl-5.003_16` source tag]'s `perly.y` and `toke.c`. Both tagged source
  blobs match 5.003_15; the release has its own files and validation row.
- `perl5.003_17.*` is a separate **partial** pair checked against the
  [`perl-5.003_17` source tag]'s `perly.y` and `toke.c`. Yacc matches 5.003_16;
  tokenizer cleanup at lex end lies outside the accepted print subset.
- `perl5.003_18.*` is a separate **partial** pair checked against the
  [`perl-5.003_18` source tag]'s `perly.y` and `toke.c`. Both tagged source
  blobs match 5.003_17; the release still has distinct files.
- `perl5.003_19.*` is a separate **partial** pair checked against the
  [`perl-5.003_19` source tag]'s `perly.y` and `toke.c`. Yacc matches 5.003_18;
  tokenizer changes to method/subroutine and quote/label recognition are
  outside the accepted plain-decimal arithmetic and `print` subset.
- `perl5.003_20.*` is a separate **partial** pair checked against the
  [`perl-5.003_20` source tag]'s `perly.y` and `toke.c`. Yacc changes subroutine
  forms; tokenizer changes shebang interpreter-path handling. Both lie outside
  the accepted plain-decimal arithmetic and `print` subset.
- `perl5.003_21.*` is a separate **partial** pair checked against the
  [`perl-5.003_21` source tag]'s `perly.y` and `toke.c`. Yacc changes format
  subroutine handling; tokenizer changes alternate shebang and braced-word
  handling outside the accepted plain-decimal arithmetic and `print` subset.
- `perl5.003_22.*` and `perl5.003_23.*` are separate **partial** pairs checked
  against their respective source tags. Their yacc and tokenizer changes
  concern subroutine setup, interpolation, shebangs and heredocs outside this
  plain-decimal arithmetic and `print` subset.
- `perl5.003_24.*` is a separate **partial** pair checked against the
  [`perl-5.003_24` source tag]. Its tagged `perly.y` and `toke.c` match 5.003_23.
- `perl5.003_25.*` is a separate **partial** pair checked against the
  [`perl-5.003_25` source tag]. Its yacc matches 5.003_24; `toke.c` changes
  a C prototype outside the accepted subset.
- `perl5.003_26.*`, `perl5.003_27.*` and `perl5.003_28.*` are separate
  **partial** pairs checked against their official source tags. The 5.003_26
  tokenizer explicitly rejects carriage returns, now reflected in all three
  token files and a negative parser probe. Later tokenizer changes to diagnostics,
  filters and interpolation lie outside this arithmetic and `print` subset;
  5.003_28's yacc change is comment-only.
- `perl5.38.2.*` is a distinct **partial** pair for the LANG81 print-arithmetic
  subset, checked against the [`v5.38.2` source tag] and Perl 5.38.2 runtime.
  It rejects syntax outside that pilot, including adjacent `--` and
  leading-zero integer forms, and does not claim full coverage.
- `perl5.44.0.*` and `perl5.45.3.*` are distinct **partial** pairs for the
  current stable and development endpoints in the pinned inventory. Their
  arithmetic and `print` subset is documented by each release's tagged
  `perlop.pod` and `perlfunc.pod`; they reject adjacent `--` and leading-zero
  integer forms and do not claim complete coverage.
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
[`perl-5.003` source tag]: https://github.com/Perl/perl5/tree/perl-5.003
[`perl-5.003_01` source tag]: https://github.com/Perl/perl5/tree/perl-5.003_01
[`perl-5.003_02` source tag]: https://github.com/Perl/perl5/tree/perl-5.003_02
[`perl-5.003_03` source tag]: https://github.com/Perl/perl5/tree/perl-5.003_03
[`perl-5.003_04` source tag]: https://github.com/Perl/perl5/tree/perl-5.003_04
[`perl-5.003_05` source tag]: https://github.com/Perl/perl5/tree/perl-5.003_05
[5.003_06 patch commit]: https://github.com/Perl/perl5/commit/9c6be91f691c6d72250718e13acfc7872e72a19f
[`perl-5.003_07` source tag]: https://github.com/Perl/perl5/tree/perl-5.003_07
[`perl-5.003_08` source tag]: https://github.com/Perl/perl5/tree/perl-5.003_08
[`perl-5.003_09` source tag]: https://github.com/Perl/perl5/tree/perl-5.003_09
[`perl-5.003_10` source tag]: https://github.com/Perl/perl5/tree/perl-5.003_10
[`perl-5.003_11` source tag]: https://github.com/Perl/perl5/tree/perl-5.003_11
[`perl-5.003_12` source tag]: https://github.com/Perl/perl5/tree/perl-5.003_12
[`perl-5.003_13` source tag]: https://github.com/Perl/perl5/tree/perl-5.003_13
[`perl-5.003_14` source tag]: https://github.com/Perl/perl5/tree/perl-5.003_14
[`perl-5.003_15` source tag]: https://github.com/Perl/perl5/tree/perl-5.003_15
[`perl-5.003_16` source tag]: https://github.com/Perl/perl5/tree/perl-5.003_16
[`perl-5.003_17` source tag]: https://github.com/Perl/perl5/tree/perl-5.003_17
[`perl-5.003_18` source tag]: https://github.com/Perl/perl5/tree/perl-5.003_18
[`perl-5.003_19` source tag]: https://github.com/Perl/perl5/tree/perl-5.003_19
[`perl-5.003_20` source tag]: https://github.com/Perl/perl5/tree/perl-5.003_20
[`perl-5.003_21` source tag]: https://github.com/Perl/perl5/tree/perl-5.003_21
[`perl-5.003_22` source tag]: https://github.com/Perl/perl5/tree/perl-5.003_22
[`perl-5.003_23` source tag]: https://github.com/Perl/perl5/tree/perl-5.003_23
[`perl-5.003_24` source tag]: https://github.com/Perl/perl5/tree/perl-5.003_24
[`perl-5.003_25` source tag]: https://github.com/Perl/perl5/tree/perl-5.003_25
[`perl-5.003_26` source tag]: https://github.com/Perl/perl5/tree/perl-5.003_26
[`perl-5.003_27` source tag]: https://github.com/Perl/perl5/tree/perl-5.003_27
[`perl-5.003_28` source tag]: https://github.com/Perl/perl5/tree/perl-5.003_28
[`v5.38.2` source tag]: https://github.com/Perl/perl5/tree/v5.38.2
