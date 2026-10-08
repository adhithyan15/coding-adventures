# LANG82 — Perl release grammar and token history

**Status:** Draft, 2026-10-07. This is a separate syntax-coverage track from
LANG81's executable Perl 5.38 arithmetic pilot.

## Version range and evidence

Cover every public, numbered Perl release from 1.000 (18 December 1987)
through the newest available release. As of this spec, Perl 5.44.0 is the
current stable release and 5.45.3 is the newest development release. Include
maintenance, development, and release-candidate releases in the inventory.
Perl 0 was internal, so it is not a public release target. Raku is a separate
language and is outside this Perl grammar line.

Use the official Perl release history and, for each release, its tagged source
and release documentation as the syntax evidence. Early Perl 1 syntax can be
checked against the `perl-1.0` tag's `perl.y` and original manual; later
releases can use their release tags, `perly.y`, and versioned `perlsyn`,
`perlop`, and delta documents. Record source revisions and any uncertainty
beside the relevant grammar change. A modern installed Perl is an oracle only
for syntax it still accepts; it cannot establish old-version syntax alone.

References:

- https://perldoc.perl.org/perlhist
- https://www.cpan.org/src/
- https://perldoc.perl.org/perlpolicy
- https://github.com/Perl/perl5/tree/perl-1.0

## Files and completeness

Each release gets its **own** materialized `code/grammars/perl/perl<V>.tokens`
and `perl<V>.grammar` pair. No release is represented only by an alias to
another release's files, including maintenance releases with unchanged syntax.
The filename uses the release's exact public version spelling, normalized only
where a filesystem character is illegal. The release inventory records each
version's spelling, date, history line or tag, and coverage state. Pair paths
follow the filename convention above; source revisions and the validated
subset are recorded in each pair's comments and the release-grammar README.
Inventory release IDs must be unique and safe to use in those file paths.
The corrected `code/grammars/perl/releases.csv` inventory expands 765 release
entries from a pinned `perlhist.pod` snapshot. An official source-tag audit
added 11 release candidates omitted there, for 776 pending or partial release
rows. The correction adds `p54rc1` and `p54rc2` from the pinned history's
lines 264 and 265. Historical source archives for both are listed in the
Perl archive index. The `v5.17.7.0` tag is excluded because its
own annotation says that no such Perl release occurred. CPAN archives still
need comparison before treating the inventory as exhaustive.

For `p54rc1` and `p54rc2`, use the separate historical source archives,
not an alias to 5.004. Both archives' `perly.y` files match the 5.004
release; `p54rc2`'s `toke.c` also matches 5.004, while `p54rc1` differs
only in the later addition of the regex `/c` modifier. Keep the two
release pairs distinct and explicitly partial. Their accepted surface is
the same plain-decimal `print` arithmetic subset with the 250-digit
numeric bound; reject leading-zero literals, adjacent decrement, carriage
returns and unsupported constructs. Check each archive's identity and run
positive and negative parser probes before changing either inventory row
from pending to partial.

A pair must not claim to describe the complete language while it covers only
a pilot subset. Mark partial pairs explicitly, list their accepted constructs,
and reject unsupported syntax. Full coverage requires the release's actual
lexical and grammatical forms, including context-sensitive constructs that
cannot be expressed by a simple regular-expression lexer alone. Add lexical
modes or parser hooks where the shared grammar system needs them, rather than
quietly accepting a modern approximation.
For release pairs that expose arithmetic subtraction, lex adjacent `--` as
Perl's decrement operator and reject it if decrement semantics are outside the
partial grammar; spaced binary/unary minus (`1- -2`) remains a separate form.
For a release pair that declares a plain-decimal arithmetic-print subset,
accept integer literals without a leading zero (and `0` itself). A
leading-zero literal may use octal syntax; reject it until that release's
actual numeric rules are implemented. Audit the already landed 1.000 through
5.003_03 partial pairs against their release tokenizers: each currently uses
`[0-9]+`, which accepts `08` and `09` as decimal integers. Perl 1's
[`perly.c`](https://github.com/Perl/perl5/blob/perl-1.0/perly.c), and the
Perl [2](https://github.com/Perl/perl5/blob/perl-2.0/toke.c),
[3](https://github.com/Perl/perl5/blob/perl-3.000/toke.c),
[4](https://github.com/Perl/perl5/blob/perl-4.0.36/toke.c), and
[5](https://github.com/Perl/perl5/blob/perl-5.003_03/toke.c) tokenizers
instead enter octal mode after a leading zero and reject `8` and `9` there.
Keep those existing pairs partial and limit their integer rule to plain
decimal forms until versioned octal tokenization is implemented. Preserve a
positive `0`/nonzero-decimal probe and add negative `08`, `09`, and `012`
probes for every affected pair.

The existing unversioned `perl.tokens` and `perl.grammar` remain the bounded
Perl 5.38 LANG81 execution grammar until a separately validated versioned
pair replaces them. A version selector must never silently fall back to those
files for a different release.

## Delivery order and gates

1. Inventory numbered releases and their primary sources. Start with 1.000,
   then build forward in chronological slices; keep the newest stable and
   development releases visible in the inventory so the endpoint cannot drift.
2. For each release, add the two files, document differences from the previous
   release, and validate both files with the repo grammar tools. Even if syntax
   is unchanged, retain an independently addressable pair and verify that
   against release evidence.
3. Add positive and negative parsing fixtures for each new syntax boundary.
   Run generated-grammar regeneration checks where a Rust package consumes the
   pair. Do not infer full executable Perl semantics from syntax coverage.
4. Mark a release complete only when the syntax inventory, lexer/parser
   behavior, fixtures, and primary-source comparison support the claim.

Keep this work in bounded, spec-first PRs. It may be prepared locally while
another implementation PR is in CI, but there is only one active implementation
PR at a time. LANG81 source-to-IIR execution continues to use its declared
Perl 5.38 subset until a release pair and semantic lowering are ready.

The next local historical slice selects 5.003_07. The official source repository
has no `perl-5.003_06` tag; that release initially remained pending. Its source
was then located in official commit `9c6be91f`, whose message introduces the
5.003_06 patch and whose `patchlevel.h` identifies subversion 6. Use that
commit to ground a separate 5.003_06 pair. The 5.003_07 pair is checked against
its own tag; unchanged yacc syntax does not remove the separate-file rule.

The following local slice selects 5.003_08. Its tagged yacc and tokenizer files
both differ from 5.003_07, especially around lexical `my` scope, interpolation,
and octal/hexadecimal overflow. Keep the initial release pair limited to the
arithmetic and `print` forms that those changes do not extend; record it as
partial and retain negative syntax probes.

The next slice selects tagged 5.003_09. Its yacc changes rearrange lexical
scope handling, while tokenizer changes mainly concern sigils, interpolation,
patterns, and locale-aware numeric conversion. Preserve only the bounded
plain-decimal arithmetic and `print` subset; reject leading-zero multi-digit
forms and do not infer full syntax coverage from the pair.

The following slice selects tagged 5.003_10. Its yacc source matches 5.003_09;
the tokenizer changes sigil spacing and a few built-in/identifier decisions,
outside this bounded print-arithmetic subset. It still needs its own pair and
separate source note.

The next slice selects tagged 5.003_11. Its yacc changes operator token type
annotations and block bookkeeping; tokenizer changes cover sigils, patterns
and numeric-locale setup. Keep the pair partial and independently addressable,
with the same plain-decimal print-arithmetic limits and negative probes.

This bounded follow-up installment materializes distinct partial pairs through
5.003_15, including 5.003_06 from its identified source commit. The next
local group begins at 5.003_16; none of these pairs claims full release syntax
or changes LANG81's executable Perl 5.38 grammar.

The next bounded installment is 5.003_16 through 5.003_28. Check each
release against its own official source tag, retain separate token and
grammar files even where source blobs match, and record only the
plain-decimal arithmetic and `print` subset. Parser probes must reject
adjacent decrement, leading-zero integer forms, and unsupported syntax.
5.003_26 introduces explicit carriage-return refusal; preserve that
boundary for 5.003_26 through 5.003_28. LANG81 execution is unchanged.

Prepare the following bounded installment as twelve distinct, explicitly
partial pairs for 5.003_90 through 5.003_97d. Check each release against
its own official source tag. Retain the plain-decimal arithmetic and
`print` syntax boundary, reject adjacent decrement and leading-zero
forms, and keep carriage returns outside the accepted subset. This stage
does not extend LANG81 execution or claim complete historical syntax.

Prepare a subsequent seven-release installment for 5.003_97e through
5.003_97j and 5.003_98. Check each release against its own official source
tag and keep its token and grammar files distinct even when the source
syntax is unchanged. Continue to accept only plain-decimal arithmetic and
`print`; reject leading-zero forms, adjacent decrement, carriage returns,
and other unsupported syntax. The 5.003_97i tokenizer introduces a bounded
decimal scan, so its partial pair and the later pairs must accept at most
250 decimal digits and reject longer literals. This stage does not change
LANG81 execution or claim complete release syntax.

For tagged 5.003_97i, `toke.c` adds a decimal scan bound using its 256-byte
`tokenbuf` and an end pointer six bytes before the buffer end. The partial
plain-decimal token rule may accept at most 250 digits and must reject a
251-digit literal before parsing. Test both sides of that boundary. This is a
lexer bound for the historical syntax pair, not a claim that LANG81 can
execute a 250-digit integer or that other Perl numeric forms are supported.

Prepare the next bounded installment as nine distinct partial pairs for
5.003_99, 5.003_99a, 5.004, 5.004_01, 5.004_01-t2, 5.004_01_01 through
5.004_01_03, and 5.004_02. Ground tagged releases in their own official
source tags and the four trials in their own historical archives. Keep each
token and grammar file separate even where source blobs agree. Retain the
plain-decimal `print` arithmetic subset and the 250-digit bound; reject
leading-zero forms, adjacent decrement, carriage returns, and unsupported
syntax. Leave 5.004_01-t1 pending until source evidence for that trial is
established. This installment does not extend executable LANG81 semantics or
claim full historical release syntax.

The historical 5.004_01-t2 source archive is independently available even
though the inspected official Git tag list has no t2 tag. Its `patchlevel.h`
identifies patchlevel 4, subversion 1; its `perly.y` and `toke.c` are
byte-identical to the tagged final 5.004_01 files. Give t2 distinct partial
token and grammar files and retain the plain-decimal 250-digit boundary
probes. Keep 5.004_01-t1 pending until its own source can be established.

The historical 5.004_01_01 trial archive has the same `perly.y` as
5.004_01-t2. Its `toke.c` changes quoted-curly disambiguation, word
operator expectations, warnings, and quote delimiter handling; these do not
expand the plain-decimal `print` arithmetic subset. Give this numbered
trial its own partial pair, retain the 250-digit decimal bound, and keep
unsupported constructs rejected.

The historical 5.004_01_03 trial archive keeps the same `perly.y` as
5.004_01_02. Its `toke.c` changes debugger line hooks only, outside the
bounded decimal `print` arithmetic subset. Give the trial a distinct
partial pair with the inherited 250-digit bound and negative probes.

The tagged final 5.004_02 release has `perly.y` and `toke.c` identical
to the 5.004_01_03 trial archive. It still needs a distinct partial pair;
retain the tested decimal boundary and unsupported-form rejection.

The historical 5.004_01_02 trial archive again has the same `perly.y`.
Its tokenizer changes command-line `-p` expansion and quoted delimiter
handling outside the partial decimal `print` arithmetic subset. Record
a distinct pair with the inherited 250-digit bound and reject unsupported
syntax rather than claiming full release coverage.

Prepare a later bounded maintenance installment with separate, explicitly partial
pairs for `5.004_02_01`, `5.004_03-t2`, and `5.004_03`. The historical
`perl-5.004_02_01.tar.gz` archive (SHA-256
`e2fdda04633175d078cd2312a1e49374dbda9a7d35f8d6b95c993f20ee9d5420`)
contains `perly.y` and `toke.c` byte-identical to 5.004_02. The separate
`perl-5.004_03-t2.tar.gz` archive (SHA-256
`077ca5870518c79c49a1fd2be9e39bdfbf23ad793cc2e1b87ad7168474e081c4`)
retains those source blobs. The 5.004_03 historical archive (SHA-256
`78b1905fdad1a0c5e1782651c898667b9c9b2a1ead88228021fc9c7752a2c85e`)
matches the official `perl-5.004_03` tag's `perly.y` and `toke.c` Git blobs;
its tokenizer only adds input-handle cleanup relative to the t2 archive.

Each release still needs its own token and grammar file. Retain only the
plain-decimal `print` arithmetic subset, its 250-digit lexical boundary, and
negative probes for leading-zero forms, decrement adjacency, carriage returns,
and unsupported characters. This stage does not extend executable LANG81
semantics or claim full Perl release syntax. Keep `5.004_01-t1` pending until
its own primary source evidence is found.

A 2026-10-07 inventory audit reparsed the pinned `perlhist.pod` blob,
including uncertain-date suffixes and expanded range rows. All 765 release
identifiers found there are present in `releases.csv`; the other 11 rows are
the separately annotated RC tags already cited by that inventory. This
checks coverage of those pinned sources only. It does not prove that the
inventory contains every public release after the snapshot or every
historical distribution not named there; keep new discoveries explicit and
never treat a pending or partial row as complete syntax.

Prepare a separate bounded installment for `5.004_04-t1`, `-t2`, `-t3`,
`-t4`, and final `5.004_04`. Their own historical source archives have
SHA-256 digests `a159185d580f409b14bd286baca770ef3ca6457c7db851e3663438cb60cb03b0`,
`c4c806d35feda5dd177880ccf13562610eb32afac64af4dfc86a0e56d2171030`,
`3337caa03309b6f772dedde87b40600c04911e795cf64c9efa6949a9c2cae744`,
`57a25365088de1d57a27516b84f39cc10f92403394e537f3405ec3cbfa151968`,
and `a33e436a815e7dc16ea539eb566d7e5ac4852f2ed16bbcdf4105c6df748bcef0`,
respectively. All five archives have byte-identical `perly.y` and `toke.c`;
the final archive's blobs also match the official `perl-5.004_04` tag.
Relative to 5.004_03, its yacc changes loop source-line construction,
while its tokenizer changes `glob` classification and quote-delimiter
whitespace handling. Those changes do not extend the accepted plain-decimal
`print` arithmetic subset. Give every release a distinct token and grammar
file; preserve the inherited 250-digit lexical bound and negative probes for
leading zero, decrement adjacency, carriage returns, and unsupported input.
Do not claim full Perl syntax or add a release alias.

Prepare a following bounded maintenance installment with four separate,
explicitly partial pairs for `5.004_04-m1` through `-m4`. Each release has
its own historical source archive at
`https://mirrors.develooper.com/perl/historical-perl/perl-<release>.tar.gz`.
Their respective SHA-256 digests are
`2782e92619e296052d45c5b9fc565e7c5558530ab37d39b761e8c9dd246c7605`,
`78e7ef63abec177ae1287ba3d20fc9e079dcbf1401eb3229db5a785bf7ef5827`,
`ce0c07bd7507cc0664d9da358f895f7bd3b67ea09ed92f432e1b0263ebf2acf0`,
and `8e883ceed74188b2cf69a8cb6247f80e8e8178c8904b9e1591c0342e20148241`.
The four `perly.y` blobs are byte-identical to each other. Relative to final
5.004_04, m1 adds postfix `for` and hash-element scalarization; later yacc
blobs retain those changes. The tokenizer changes across the four releases
concern interpolation and lexical state, regex modifiers, input handling,
subscript disambiguation, threading, and filters. Their comment-stripped
`scan_num` function remains identical to final 5.004_04, including its
250-digit decimal scan bound. These changes do not widen the accepted
plain-decimal `print` arithmetic subset. Retain its negative probes for
leading zero, decrement adjacency, carriage returns, and unsupported input;
give each release its own token and grammar files without aliases or a claim
of complete historical syntax.

Prepare a separate bounded installment with six distinct, explicitly partial
token/grammar pairs for `5.004_05-MT5` through `-MT9` and final `5.004_05`.
Their own historical archives at
`https://mirrors.develooper.com/perl/historical-perl/perl-<release>.tar.gz`
have SHA-256 digests, in that order,
`6832685e6bcb4993fab589f3b122de90ae2862bbc0cb1f5fdcac93bc9612d218`,
`34a6a5e8ddaa5cf800729e5319ba65fba66b57697612507ed42e368576e34d9f`,
`4dd052992d9cc3eed9abbc5f7f6bd1952493a30874eb12e2fc9f2ae722ab6373`,
`f7da20ab60e70f1b485c52c6d9556fdb9e03bfc31a497d5678fb4a00e3207a13`,
`29ab6b9332ee4021c0736fafd6340e19f6da6c24c4054208e7c4fa7e943296e9`,
and `26ca43d9f1067f601c05ebeb3488bc3d473ee1cc772d57085dc43a4a9d5f225`.
All six `perly.y` blobs match 5.004_04-m4. MT5's tokenizer changes input
carriage-return handling, regex modifiers, and symbol lookup; MT6 changes
filter cleanup and format-line carriage-return handling. MT7, MT8, MT9, and
final 5.004_05 have byte-identical `perly.y` and `toke.c` to MT6. The final
archive's two blobs also match the official `perl-5.004_05` Git tag. All seven
releases from 5.004_04-m4 through 5.004_05 have a byte-identical `scan_num`
function with its 250-digit decimal bound. None of those changes widen this
stage's plain-decimal `print` arithmetic subset. Keep carriage returns,
leading-zero literals, decrement adjacency, and unsupported characters
rejected by the partial grammar, with separate files for every release and
no complete historical syntax claim.

Prepare the next development-release installment as a distinct, explicitly
partial token/grammar pair for `5.004_50`. Its own historical source archive
is `https://mirrors.develooper.com/perl/historical-perl/perl-5.004_50.tar.gz`
with SHA-256
`458f5850e8b36f9280fcd713210f83472901577e930b41e3f8dfd243a64b9938`.
Compared with final 5.004_05, its `perly.y` changes loop and subroutine
productions while `toke.c` has broader lexical-state differences; neither
widens this stage's plain-decimal `print` arithmetic subset. The
comment-and-whitespace-stripped `scan_num` body is identical to 5.004_05,
including the 250-digit decimal scan bound. Accept at most 250 digits and
reject 251, leading-zero literals, adjacent decrement, carriage returns,
and unsupported characters. Keep its two files separate, without aliasing
another release or claiming complete historical syntax.

Extend that bounded development installment with a separate, explicitly
partial `5.004_51` pair. Its own historical archive is
`https://mirrors.develooper.com/perl/historical-perl/perl-5.004_51.tar.gz`
with SHA-256
`940e33d409ea7c5eb93dfabac3b53f5822a7a8522f0ced54e721a0853a80ba2f`.
Its `perly.y` and `toke.c` blobs are byte-identical to the 5.004_50 archive,
including the comment-stripped numeric scanner and 250-digit decimal bound.
It still requires its own token and grammar files and the same negative probes;
shared source blobs do not make the two public releases aliases or complete
syntax implementations.

Continue the development-release installment with a separate, explicitly
partial `5.004_52` token/grammar pair. Its own historical archive is
`https://mirrors.develooper.com/perl/historical-perl/perl-5.004_52.tar.gz`
with SHA-256
`f5edcffd4bf28db7bad95562470e37d57da65503822abb77de049b4d18cddd09`.
Its `perly.y` is byte-identical to 5.004_51, while `toke.c` changes input
handling and diagnostics outside the plain-decimal `print` arithmetic subset.
The comment-and-whitespace-stripped `scan_num` body remains identical to
5.004_51, retaining the 250-digit decimal scan bound. Accept at most 250
digits and reject 251, leading-zero literals, adjacent decrement, carriage
returns, and unsupported characters. Keep both files distinct and labeled
partial; source continuity does not establish full historical syntax.

Add a separate, explicitly partial `5.004_53` token/grammar pair. Its own
historical archive is
`https://mirrors.develooper.com/perl/historical-perl/perl-5.004_53.tar.gz`
with SHA-256
`6c8cc39262bc4134be38db1782d6a745bd45b1b6d605084f76c43eeaa3584c7e`.
Compared with `5.004_52`, its `perly.y` changes loop actions and `toke.c`
changes `glob` keyword classification and delimiter whitespace handling;
none widens the bounded plain-decimal `print` arithmetic subset. The
`scan_num` body is byte-identical to `5.004_52`, retaining its 250-digit
decimal scan bound. Accept 250 digits, reject 251, leading-zero forms,
adjacent decrement, carriage returns, and unsupported characters. Keep
release-specific files and do not claim complete Perl syntax.

Add a separate, explicitly partial `5.004_54` token/grammar pair from its own
`https://mirrors.develooper.com/perl/historical-perl/perl-5.004_54.tar.gz`
source archive, SHA-256
`77f8b07832e8e99f53d9c18be133d7d63e4417f4370a2bf25a82ca6102a84c8d`.
Its `perly.y` changes an action function prototype. Most `toke.c` changes
convert K&R signatures to ANSI prototypes; its thread-magical handling and
interpolation changes are outside plain-decimal arithmetic `print`. The
`scan_num` body remains byte-identical to `5.004_53`, including its 250-digit
bound. Keep the separate pair partial, accepting 250 digits and rejecting
251, leading-zero literals, adjacent decrement, carriage returns, and other
unsupported syntax.

Add a separate, explicitly partial `5.004_55` token/grammar pair from its own
`https://mirrors.develooper.com/perl/historical-perl/perl-5.004_55.tar.gz`
source archive, SHA-256
`1eb2d7a6f70e7501fe62d8838a6e575f446e12e60cf1f6ad6d6ebeeaa6ed700b`.
Its `perly.y` changes a BEGIN/END/INIT subname action, outside this standalone
plain-decimal `print` arithmetic subset. Its `scan_num` section is byte-identical
to `5.004_54`, retaining the 250-digit decimal bound. Other `toke.c` changes
concern thread variables, imported keyword overrides, regex and filters.
The `WIN32CHEAT` branch changes carriage-return handling; this partial grammar
excludes carriage returns from its accepted subset on every platform and makes
no claim of matching each platform's rejection behavior. Accept the bounded
plain-decimal examples, including 250 digits, and reject 251 digits,
leading-zero forms, adjacent decrement, carriage returns, and other unsupported
input in the partial grammar. Keep its own distinct files and pending releases
pending; do not claim full historical Perl syntax.

Add separate, explicitly partial `5.004_56` and `5.004_57` token/grammar
pairs, each grounded in its own historical source archive:
`https://mirrors.develooper.com/perl/historical-perl/perl-5.004_56.tar.gz`
(SHA-256 `72bc8c0944c85eb372e3d071d4e4f61eee3efe7f5c4146b7b3602ea904051b4a`)
and `https://mirrors.develooper.com/perl/historical-perl/perl-5.004_57.tar.gz`
(SHA-256 `86a35b731294a6ba7d161af68074a7be7fdfc9815a1ccad457fa7519eda60dcb`).
Between 5.004_55 and 5.004_56, `perly.y` changes only the
BEGIN/END/INIT subname action; `toke.c` changes word-handling parameter
spelling, the `sort` word path, and debugger error initialization. Between
5.004_56 and 5.004_57, `perly.y` is byte-identical and `toke.c` changes
environment, PerlIO, and filter handling. Both releases retain the same
`scan_num` body and source-backed 250-digit decimal bound as 5.004_55. None
of those source changes widens the standalone plain-decimal arithmetic
`print` subset represented by the partial files. Accept 250 digits; reject
251, leading-zero forms, adjacent decrement, carriage returns, and other
unsupported input in the partial grammar. As for 5.004_55, excluding CR
from this subset makes no claim about platform-wide historical rejection
behavior. Keep separate files, leave later releases pending, and do not
claim complete historical syntax.

Add a separate, explicitly partial `5.004_58` token/grammar pair from its own
`https://mirrors.develooper.com/perl/historical-perl/perl-5.004_58.tar.gz`
source archive, SHA-256
`3aea97f0fcd26b867512710d93625f9810dbb6c31c579035d24198fef1bdb977`.
Compared with `5.004_57`, `perly.y` changes the `OP_GELEM` action for a
symbol-table expression. `toke.c` changes lexical-state restoration, regex
interpolation, method lookup, filters, hash-brace disambiguation, and heredoc
line tracking. Those paths are outside this standalone plain-decimal arithmetic
`print` subset. Its `scan_num` body is byte-identical to `5.004_57`, retaining
the 250-digit decimal bound. Accept 250 digits; reject 251, leading-zero forms,
adjacent decrement, carriage returns, and other unsupported input in the
partial grammar. Keep its files separate, leave later releases pending, and do
not claim full historical syntax or platform-wide rejection parity.

Add a separate, explicitly partial `5.004_59` token/grammar pair from its own
`https://mirrors.develooper.com/perl/historical-perl/perl-5.004_59.tar.gz`
source archive, SHA-256
`76425638c9ca1502947e26728c4b9a3dd1982a538727629c5f5c4339757231cc`.
Its `perly.y` is byte-identical to `5.004_58`. In `toke.c`, `sublex_push`
replaces `push_scope()` with `ENTER`, and `sublex_done` replaces
`pop_scope()` with `LEAVE`; the source from `scan_num` onward is byte-identical
to `5.004_58`. Those sublexical scope changes do not widen the standalone
plain-decimal arithmetic `print` subset. Retain the source-backed 250-digit
decimal bound and reject 251 digits, leading-zero forms, adjacent decrement,
carriage returns, and unsupported input in the partial grammar. Keep its
files distinct, leave later releases pending, and make no full-syntax or
platform-wide rejection claim.

Add a separate, explicitly partial `5.004_60` token/grammar pair from its own
`https://mirrors.develooper.com/perl/historical-perl/perl-5.004_60.tar.gz`
source archive, SHA-256
`912091e555a293955797efc07709116860850655bbe294f137f5cb99e5d372ca`.
Its `perly.y` and `toke.c` are byte-identical to `5.004_59`; this source
comparison supports the same bounded standalone plain-decimal arithmetic
`print` subset and its 250-digit numeric limit. Keep an independent file pair
for this release. Accept 250 digits; reject 251 digits, leading-zero forms,
adjacent decrement, carriage returns, and other unsupported input in the
partial grammar. Leave later releases pending and make no full-syntax,
exhaustive-inventory, or platform-wide rejection claim.

Add a separate, explicitly partial `5.004_61` token/grammar pair from its own
`https://mirrors.develooper.com/perl/historical-perl/perl-5.004_61.tar.gz`
source archive, SHA-256
`ddddc1b0c5b832c9337691e70c585a98010d66a65fd68df171722a3292310347`.
Its `perly.y` is byte-identical to `5.004_60`; `toke.c` only simplifies the
old-style conditional prototype of `lop` before `scan_num`. The source from
`scan_num` onward is byte-identical, retaining the source-backed 250-digit
plain-decimal arithmetic `print` subset. Keep an independent file pair for
this release. Accept 250 digits; reject 251 digits, leading-zero forms,
adjacent decrement, carriage returns, and other unsupported input in the
partial grammar. Leave later releases pending and make no full-syntax,
exhaustive-inventory, or platform-wide rejection claim.

Add a separate, explicitly partial `5.004_62` token/grammar pair from its own
`https://mirrors.develooper.com/perl/historical-perl/perl-5.004_62.tar.gz`
source archive, SHA-256
`18a6f01ee34399326376c5132d0acbc0fc13d198ec9de17e50f6650f39ec3b82`.
Relative to `5.004_61`, `perly.y` adds `expr FOR expr`, outside this subset.
`toke.c` has substantial changes in package and bareword handling, quoted
forms, `for`, and heredoc paths; do not claim byte identity or full-language
parity. The complete `scan_num` function has an identical C token sequence
after removing comments and whitespace, and `keywords.h` is byte-identical.
This supports retaining only the bounded standalone plain-decimal arithmetic
`print` subset and its source-backed 250-digit numeric limit. Keep its own
file pair. Accept 250 digits; reject 251 digits, leading-zero forms, adjacent
decrement, carriage returns, and other unsupported input in the partial
grammar. Leave later releases pending and make no full-syntax,
exhaustive-inventory, or platform-wide rejection claim.

Add a separate, explicitly partial `5.004_63` token/grammar pair from its own
`https://mirrors.develooper.com/perl/historical-perl/perl-5.004_63.tar.gz`
source archive, SHA-256
`13e89ca65507f22ea6dcf503ffc525ade51c13eb42f0c15fd9c3ad4d495c6a92`.
Its `perly.y`, `toke.c`, and `keywords.h` are each byte-identical to their
`5.004_62` counterparts. Keep an independent file pair for this release and
retain only the source-backed, bounded standalone plain-decimal arithmetic
`print` subset with its 250-digit numeric limit. Accept 250 digits; reject
251 digits, leading-zero forms, adjacent decrement, carriage returns, and
other unsupported input in the partial grammar. Leave later releases pending
and make no full-syntax, exhaustive-inventory, or platform-wide rejection
claim.

Add a separate, explicitly partial `5.004_64` token/grammar pair from its own
`https://mirrors.develooper.com/perl/historical-perl/perl-5.004_64.tar.gz`
source archive, SHA-256
`aae74c445d5035a9af3e6146ca16e07220e50c89c8569b83e4c7ad3080a78df5`.
Relative to `5.004_63`, `perly.y` and `keywords.h` are byte-identical.
`toke.c` changes only the error text for a bad qualified name, outside this
subset; its `scan_num` function and everything after it are byte-identical.
Keep an independent file pair for this release and retain only the
source-backed, bounded standalone plain-decimal arithmetic `print` subset
with its 250-digit numeric limit. Accept 250 digits; reject 251 digits,
leading-zero forms, adjacent decrement, carriage returns, and other
unsupported input in the partial grammar. Leave later releases pending and
make no full-syntax, exhaustive-inventory, or platform-wide rejection claim.

Add a separate, explicitly partial `5.004_65` token/grammar pair from its own
`https://mirrors.develooper.com/perl/historical-perl/perl-5.004_65.tar.gz`
source archive, SHA-256
`aa774928afb0b0b2402c397d1e171523e5d3be4f8e022f1a42093274d78b1b72`.
Relative to `5.004_64`, `perly.y` and `keywords.h` are byte-identical.
The `toke.c` changes are confined to `scan_const` documentation and its
non-pattern `leaveit` expression, plus pattern/substitution flag handling;
the decimal scanner, arithmetic-print token path, and their grammar
productions are unchanged. Keep an independent file pair for this release
and retain only the source-backed, bounded standalone plain-decimal
arithmetic `print` subset with its 250-digit numeric limit. Accept 250
digits; reject 251 digits, leading-zero forms, adjacent decrement, carriage
returns, and other unsupported input in the partial grammar. Leave later
releases pending and make no full-syntax, exhaustive-inventory, or
platform-wide rejection claim.

Add a separate, explicitly partial `5.004_66` token/grammar pair from its own
`https://mirrors.develooper.com/perl/historical-perl/perl-5.004_66.tar.gz`
source archive, SHA-256
`55de2641c3dda692c868fc9d86d375ad874c44f23a884927c1005865aa36a061`.
Relative to `5.004_65`, `perly.y` and `keywords.h` are byte-identical.
`toke.c` changes static linkage declarations, object-build plumbing, and
non-subset regex, curly-brace, heredoc, and input cleanup paths; the complete
`scan_num` function body is byte-identical. Keep only the bounded standalone
plain-decimal arithmetic `print` subset and its source-backed 250-digit
numeric limit. Keep a distinct file pair for this release. Accept 250 digits;
reject 251 digits, leading-zero forms, adjacent decrement, carriage returns,
and other unsupported input in the partial grammar. Leave later releases
pending and make no full-syntax, exhaustive-inventory, or platform-wide
rejection claim.
