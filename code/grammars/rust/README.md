# Rust grammars

This directory will hold the versioned Rust front ends of
[RS01](../../specs/RS01-rust-grammars.md): one `.tokens` and one `.grammar`
file for every published Rust release, and for every edition that release
supports. **Slices A2 and A3 wrote the inventory and the evidence tooling;
slice A4 wrote the first four pairs, the lexical pairs of release 1.97.0.**
Only the 1.97.0 row has left `none` (it is at stage `G`, still `pending`);
every other row is `pending`.

What is here today:

- `releases.csv`: the LANG84 release inventory, 158 rows.
- `subsets.csv`: the two subsets RS01 names, `rs-token-trees` and
  `rs-confined`, with the new `allowlist` column.
- `pair-evidence.csv`: four `reviewed` rows, one per edition of 1.97.0 (see
  "Pair evidence").
- `rust1.97.0-e{2015,2018,2021,2024}.{tokens,grammar}`: the first pairs (see
  "Pair files").
- `unicode/xid-17.0.0.ranges`: the `XID_Start` and `XID_Continue` ranges the
  pairs' identifier classes are built from.
- `corpus/rs-token-trees/`: 56 cases that probe the lexical pairs.

The pair files and the Unicode data are **generated** by
`code/scripts/rust_lexical_pairs.py`. Do not edit them by hand.

`code/scripts/release_inventory.py check` validates both on every pull
request. See [LANG84](../../specs/LANG84-grammar-driven-versioned-frontends.md)
"Amendment RS01" for the edition rules the checker enforces.

## What the inventory holds

The release unit is the upstream tag in `rust-lang/rust`. On 2026-11-12,
`git ls-remote --tags https://github.com/rust-lang/rust` listed 165 tags.
158 are rows.

| Group | Releases | Rows | Channel |
| --- | --- | ---: | --- |
| Pre-1.0 finals | 0.1, 0.2, 0.3, 0.4 ... 0.10, 0.11.0, 0.12.0 | 12 | `final` |
| Pre-1.0 patch | 0.3.1 (tag `release-0.3.1`) | 1 | `patch` |
| 1.0 pre-releases | 1.0.0-alpha, 1.0.0-alpha.2, 1.0.0-beta | 3 | `alpha`, `alpha`, `beta` |
| 1.x minors | 1.0.0 ... 1.99.0 | 100 | `final` |
| 1.x patches | 1.12.1 ... 1.99.1 | 42 | `patch` |

Release IDs are the tag names, except 0.3.1. They are never compared as
semantic versions; rows are in date order.

**Excluded tags.** `release-0.1` ... `release-0.7` point at exactly the same
commits as `0.1` ... `0.7` (checked when the inventory was written). They are
aliases, not releases, and are listed in `releases.csv`'s header comments with
their commits. `release-0.3.1` is kept because it is a real release with no
other spelling.

**1.100.0 is not a row yet.** `RELEASES.md` lists it (2026-11-12) but no tag
existed. The first inventory change after the tag appears adds the row
(RS01 decision Q13).

**Not in scope.** Nightly toolchains, beta-channel snapshots of 1.x releases
and untagged 1.0 beta refreshes are not releases here, and the checker refuses
any release ID that looks like one.

## Dates

| `date_source` | Rows | Source |
| --- | ---: | --- |
| `releases-md` | 156 | the `Version X (YYYY-MM-DD)` heading of `RELEASES.md` at the repository's main branch, read on 2026-11-12 |
| `tag-commit` | 1 | 0.3.1: not in `RELEASES.md`; dated `2012-07?` from its tag. The `?` marks the date uncertain, and RS01 section 11 lists it as not verified |
| `announcement` | 1 | 1.0.0-beta: not in `RELEASES.md`; 2015-04-03 as RS01 section 1.2 records from the release announcement's source |

0.3.1 sits after 0.3 (2012-07-12) because a row's predecessor must be an
earlier row. A date known only to the month allows any day of July, so the
order check accepts that position.

## Lineage and editions

`predecessor` and `lineage` follow RS01 section 1.2:

- a 1.x minor: the previous `X.Y.0` (`branch-point`);
- a 1.x patch: `X.Y.(z-1)`, or `X.Y.0` for the first patch (`line`);
- 1.0.0-alpha: 0.12.0 (`branch-point`); 1.0.0-alpha.2, 1.0.0-beta and 1.0.0
  follow one another on the 1.0 line (`line`);
- 0.3.1: 0.3 (`line`); 0.4 branches from 0.3, not from 0.3.1.

The `editions` column is derived, never trusted. The header comment
`# editions: 2015@1.0.0-alpha 2018@1.31.0 2021@1.56.0 2024@1.85.0` names each
edition's first release, and a row's editions are its predecessor's plus any
edition that starts at the row. Lineage carries the set, so 1.30.1 stays
2015-only although 1.31.0 came earlier.

| Releases | Editions | Rows | Pairs |
| --- | --- | ---: | ---: |
| 0.1 ... 0.12.0, 0.3.1 | none (`-`) | 13 | 13 |
| 1.0.0-alpha ... 1.30.x | 2015 | 45 | 45 |
| 1.31.0 ... 1.55.x | 2015, 2018 | 34 | 68 |
| 1.56.0 ... 1.84.x | 2015, 2018, 2021 | 43 | 129 |
| 1.85.0 ... 1.99.1 | 2015, 2018, 2021, 2024 | 23 | 92 |
| **Total** | | **158** | **347** |

## Pair evidence

`pair-evidence.csv` holds one row per (release, edition) pair that has files
here, and `code/scripts/rust_release_pairs.py` keeps the rows and the files
honest. Slice A3 added the tooling and slice A4 the first four rows, the
`reviewed` pairs of 1.97.0: the same seven `syntax_blobs` (the lexer, symbol,
accepted-feature, parser, macro-by-example and `Cargo.lock` objects of tag
1.97.0, from a blobless depth-1 fetch), one note per edition, and `-` for the
oracle until slice C2.

Each row has one of three bases (RS01 section 6.3):

| Basis | Meaning | Who writes the files |
| --- | --- | --- |
| `reviewed` | written by hand from the tag's sources, as a reviewed delta | a person |
| `copy:<pair>` | every recorded syntax blob equals the source pair's | `rust_release_pairs.py write` |
| `semantic-copy:<pair>` | a 1.x minor that copies its predecessor's pair; the `accepted.rs` blob and features, the `RELEASES.md` blob and section, and the oracle sweep's toolchain SHA-256 and result digest are all recorded | `rust_release_pairs.py write` |

A pair is named `<release>` before 1.0 and `<release>-e<edition>` from 1.0,
for example `0.4` or `1.85.0-e2021`. A source must be an earlier edition of
the same release, or the same edition of a release on the target's
`predecessor` chain; a `semantic-copy` must be a 1.x minor and name its direct
predecessor.

```text
python3 code/scripts/rust_release_pairs.py check   # verify; exit 1 on any violation
python3 code/scripts/rust_release_pairs.py write   # (re)write copy and semantic-copy files
```

A copy's header names its own release and edition and cites its evidence, and
its body is its source's, byte for byte. `check` fails when a copy's body or
header drifts, a `semantic-copy` lacks any of its three conditions, a
`reviewed` row lacks its blobs, note or commit, a source is missing, later or
cyclic, a pair file has no row, or a row has no files. It reads only local
files, refuses symlinks and paths that leave this directory, and runs in CI
beside `release_inventory.py check`.

## Pair files

Pre-1.0 releases will use `rust<release>.tokens` and `.grammar`; 1.0 and later
use `rust<release>-e<edition>.*`. A row's claims hold for every edition it
lists, so a row past stage `none` needs both files for each of its editions.

**The four 1.97.0 pairs** (slice A4) are complete lexers and token-tree
grammars (the `rs-token-trees` subset): the `.tokens` file produces every token
`rustc_lexer` does, and the `.grammar` accepts token trees and delimiter
balance and nothing else. They are at stage `G`: `grammar-tools validate`
passes (its "defined but never used" warnings are intended; the `.grammar`
header says why) and each header cites the tag, its commit and the syntax
blobs. Stage `P` needs `release_probes` (slice A5), so the row cannot be
`partial` yet.

| Pair | Keywords added | Lexical differences |
| --- | --- | --- |
| `-e2015` | (base set; `dyn` is a soft keyword) | `async`, `await`, `try` are identifiers |
| `-e2018` | `async await dyn try` | |
| `-e2021` | | C strings and raw C strings, raw lifetimes `'r#a`, reserved prefixes (`k#x`, `k"x"`, `k'x'`, `'a#`) refused |
| `-e2024` | `gen` | guarded strings `#"x"#` and `##` refused |

What the lexers do that is worth knowing:

- **Block comments nest**, tracked by lexer groups; line comments are tokens
  that the A5 hook `rust_block_comments` deletes. Neither that hook nor
  `rust_strip_shebang` exists yet, so a raw lexer run leaves comment tokens in
  the stream and lexes `#!/usr/bin/env x` as punctuation.
- **Raw strings** have one regex arm per hash count, 0 to 255.
- **Identifiers** use explicit `XID_Start` and `XID_Continue` classes for Unicode
  17.0.0, never `\p{XID_Start}`.
- **Malformed input is a refused token**, not an `errors:` entry: the engine
  never reads `errors:`.
- **A decoder still has work to do**: the lexer accepts `0b102`, `'ab'` and bad
  escapes as tokens (RS01 divergence RD6).

### Generating and checking

```text
python3 code/scripts/rust_lexical_pairs.py write        # rewrite the eight pair files
python3 code/scripts/rust_lexical_pairs.py check        # fail if any differs; CI runs this
python3 code/scripts/rust_lexical_pairs.py import-xid TABLES_RS
                                                        # regenerate unicode/xid-17.0.0.ranges
python3 -m unittest discover -s code/scripts/tests -p 'test_rust_lexical_pairs.py'
```

The generator reads `releases.csv` and `pair-evidence.csv` for each header, so
the files are byte-reproducible. `import-xid` takes `src/tables.rs` of the
`unicode-ident` 1.0.24 crate (it is in the cargo registry once any Rust crate
that depends on it has been built) and refuses a file whose SHA-256 is not the
pinned one. Nothing is downloaded.

## The `rs-token-trees` corpus

`corpus/rs-token-trees/` holds one case per RS01 section 4.2 hard case, or
several: nested and doc block comments, strings that contain `/*`, shebangs,
raw strings with 0 to 255 hashes, byte and C strings, lifetimes against
characters, numeric suffixes and forms, raw identifiers, per-edition keywords
and reserved prefixes. A case is:

| File | Meaning |
| --- | --- |
| `<name>.rs` | the source |
| `<name>.expect` | the tokens, one per line: `TYPE value`, with a leading `+` when the token follows trivia (RS01's F11 flag). Values escape `\n \r \t \\` and `\u{..}`, so the file is ASCII. The tokens are those after the two C3 hooks of slice A5 |
| `<name>.expect@1.97.0-e2021` | the tokens for one pair, when it differs |
| `<name>.reject@1.97.0-e2021` | `Lex` or `Parse`: that pair refuses the case |
| `<name>.reject` | every pair refuses it (`doc-block-nested-refused`: `rustc` accepts, divergence RD4) |
| `<name>.cite` | the oracle record: `rustc 1.97.0 (2d8144b78)`'s verdict per edition, and the pair's |

Every case agrees with `rustc` 1.97.0 in every edition, except the one RD4
case. A5's lexer tests must reproduce the `.expect` files.

## Subsets

| Subset | Purpose |
| --- | --- |
| `rs-token-trees` | lexical-pair probes (RS01 slice A4) |
| `rs-confined` | the VER25a proof front end's subset (RS01 slice B1) |

`allowlist` names a file in this directory (never inside `corpus/`) holding
the subset's gate, or `-` for none. `rs-confined.allowlist` is written by
slice B1; until a row claims `rs-confined`, the checker does not require it to
exist.
