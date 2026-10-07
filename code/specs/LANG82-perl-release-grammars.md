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
The initial `code/grammars/perl/releases.csv` inventory expands 763 numbered
releases from a pinned `perlhist.pod` snapshot. An official source-tag audit
added 11 release candidates omitted there, for 774 pending or partial release
rows. The `v5.17.7.0` tag is excluded because its own annotation says that
no such Perl release occurred. CPAN archives still need comparison before
treating the inventory as exhaustive.

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
For the modern arithmetic-print subset, accept plain decimal integer literals
without a leading zero (and `0` itself). A leading-zero literal may use octal
syntax; reject it until that release's actual numeric rules are implemented.

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
