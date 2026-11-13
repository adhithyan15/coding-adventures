# Rust grammars

This directory will hold the versioned Rust front ends of
[RS01](../../specs/RS01-rust-grammars.md): one `.tokens` and one `.grammar`
file for every published Rust release, and for every edition that release
supports. **Slices A2 and A3 wrote the inventory and the evidence tooling.**
There are no grammar files yet; every row is `pending`.

What is here today:

- `releases.csv`: the LANG84 release inventory, 158 rows.
- `subsets.csv`: the two subsets RS01 names, `rs-token-trees` and
  `rs-confined`, with the new `allowlist` column.
- `pair-evidence.csv`: header only, until the first pair files exist (see
  "Pair evidence").

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
honest. There are no rows yet: slice A3 (this change) adds the tooling and the
header-only file, and slice A4 writes the first pairs.

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

## Pair files (none yet)

Pre-1.0 releases will use `rust<release>.tokens` and `.grammar`; 1.0 and later
use `rust<release>-e<edition>.*`. A row's claims hold for every edition it
lists, so a non-pending row needs both files for each of its editions.

## Subsets

| Subset | Purpose |
| --- | --- |
| `rs-token-trees` | lexical-pair probes (RS01 slice A4) |
| `rs-confined` | the VER25a proof front end's subset (RS01 slice B1) |

`allowlist` names a file in this directory (never inside `corpus/`) holding
the subset's gate, or `-` for none. `rs-confined.allowlist` is written by
slice B1; until a row claims `rs-confined`, the checker does not require it to
exist.
