# Python grammars

This directory holds three kinds of files:

- `python.tokens` and `python.grammar`: the unversioned pair.
- `python2.7`, `python3.0`, `python3.6`, `python3.8`, `python3.10` and
  `python3.12` pairs: legacy **minor-line** pairs. They serve only the existing
  `parse_python(source, "3.12")` API. LANG84 keeps them until the per-release
  pairs of their line exist, then deletes them. They are never selectable
  through the release API.
- `releases.csv`: the LANG84 release inventory. It has one row per upstream
  tagged CPython release, and per-release pairs (`python<release>.*`) will be
  built against it, earliest first. See
  [LANG84](../../specs/LANG84-grammar-driven-versioned-frontends.md).

The CSV header declares the legacy pairs with
`legacy-minor-line-pairs: ...`, so `release_inventory.py check` does not
mistake them for release pairs.

**Name collision:** `python2.7.*` and `python3.0.*` have the same file names
that releases `2.7` and `3.0` (tags `v2.7` and `v3.0`) will need. Before either
release gets its own pair, the legacy file must be moved or deleted, and its
`legacy-minor-line-pairs` entry dropped.

## What the inventory holds

On 2026-10-20, `git ls-remote --tags https://github.com/python/cpython` listed
677 tags. 657 are rows. The 20 excluded tags are `2.0` … `2.7`, `3.0` … `3.10`
and `legacy-trunk`. Their annotations say "Final head of the former X branch"
or "Python 3.10 end of life": they mark retired branch heads, not releases.

| Channel | Rows |
| --- | ---: |
| `final` | 282 |
| `rc` (`cN` before 2.6, `rcN` after) | 181 |
| `alpha` | 108 |
| `beta` | 86 |

Release IDs are the tag names without the `v`, such as `2.3c1`, `3.0`,
`3.13.0rc2` and `0.9.8`. The earliest row is `0.9.8` (1993-01-10?) and the
newest is `3.16.0a1` (2026-10-13). `source_ref` is the tag's GitHub URL.
`source_sha` is the tagged commit, peeled for annotated tags.

## Dates

- **597 rows** take the date from the release's own `Release date` line. The
  line is read from `Misc/NEWS`, `Misc/NEWS.d/<release>.rst`
  (`.. release date:`), or `Misc/HISTORY`, at every release tag and at
  `v2.7.18`/`v3.15.0`. When documents disagree, the date nearest the tag date
  is used. That happened for 9 releases:

  | Release | Documented dates | Chosen |
  | --- | --- | --- |
  | 2.4.5c1 | 2008-03-02 / 2008-03-20 | 2008-03-02 |
  | 2.5.6c1 | 2010-04-17 / 2011-04-17 | 2011-04-17 |
  | 2.7a1 | 2009-12-04 / 2009-12-05 | 2009-12-05 |
  | 2.7.2 | 2011-05-29 / 2011-06-11 | 2011-06-11 |
  | 3.1.5 | 2012-02-23 / 2012-04-08 | 2012-04-08 |
  | 3.2.3rc1 | 2011-02-24 / 2012-02-24 | 2012-02-24 |
  | 3.4.6 | 2017-01-16 / 2017-01-17 | 2017-01-16 |
  | 3.5.0a2 | 2015-03-08 / 2015-03-09 | 2015-03-08 |
  | 3.5.3 | 2017-01-16 / 2017-01-17 | 2017-01-16 |

  Of these 597 dates, 583 are within a day of the tag date.
- **60 rows** carry a `?`. They have no dated NEWS entry: every release from
  0.9.8 through 2.1.2, plus 2.5a0, 2.7.6, 3.1a2, 3.1.1rc1, 3.1.5rc1, 3.1.5rc2,
  3.2.6rc1, 3.3.6rc1, 3.4.4rc1 and 3.4.4, whose entry is a placeholder such as
  `XX-XXX` or is missing. Their
  date is the annotated tag's date or, for older tags, the date of the tagged
  commit or of the cvs2svn tag-creation commit. python.org's release API could
  not be reached from the inventory environment, so these dates have not been
  cross-checked against announcements.

No row has an empty date.

## Lineage (`predecessor`, `lineage`)

- `line`: the previous release on the same `X.Y` line, in inventory order.
- `branch-point`: the first release of a line. Its predecessor is the most
  recent earlier release of the nearest lower line whose tagged commit is a
  git ancestor of this release's commit. For example, `3.13.0a1` follows
  `3.12.0b1`, where 3.12 branched.
- `first`: `0.9.8`.

## Gaps

Untagged releases are not rows, because LANG84 v1 covers tags only (open
question 10). They are recorded in the CSV header:

- Releases with a NEWS section but no tag: 2.2a1, 2.2a2, 2.2a4, 2.2b1, 2.2b2,
  2.2c1, 2.3a1, 2.3a2, 2.3b1, 2.3b2, 2.3 final, 2.6.7rc1 and 2.6.7rc2.
- Releases older than the tags, or never tagged: 0.9.0–0.9.7, 1.0.0 and 1.6
  final.

Every row is `pending`, stage `none`. No per-release pair exists yet.

`python3 code/scripts/release_inventory.py check` validates this file.
