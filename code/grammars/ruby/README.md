# Ruby grammars

`ruby.tokens` and `ruby.grammar` are the unversioned pair used by the current
Ruby frontend. `releases.csv` is the LANG84 release inventory: one row per
upstream tagged Ruby release, the list that per-release pairs
(`ruby<release>.tokens` / `ruby<release>.grammar`) will be built against,
earliest first. See
[LANG84](../../specs/LANG84-grammar-driven-versioned-frontends.md) for the
schema and the status stages.

## What the inventory holds

On 2026-10-20, `git ls-remote --tags https://github.com/ruby/ruby` listed
1,196 tags. 1,189 are rows. 7 are excluded, each with its reason in the CSV
header:

- branch and freeze markers: `v1_1d-start`, `v1_2_stable`,
  `v1_8_5_freeze_candidate`, `v1_9_0_yarv_migration_base`
- CVS import and vendor tags: `v1_0_r2` (the repository's initial import) and
  `v1_1_8` (a third-party import on vendor branch `SHIGEO`; no Ruby 1.1.8 exists)
- a conversion copy of a mistaken svn tag: `v1_8_5_55@13008`

| Channel | Rows | Meaning |
| --- | ---: | --- |
| `final` | 172 | `X.Y.Z` releases, `X.Y.Z-p0` (the first patch level of a teeny release), `1.0-961225` (Ruby 1.0) and repacks |
| `patch` | 832 | `X.Y.Z-pN` with N > 0, and the dated 1.0 maintenance snapshots |
| `preview` | 77 | `-previewN` and `1.2.2-pr1` |
| `rc` | 31 | `-rcN` and the 1.1c series, which led to 1.2 |
| `beta` | 22 | the 1.1b series |
| `dev` | 55 | 0.x, `1.9.0-N`, the 1.1d series, and the odd-numbered development series 1.3, 1.5 and 1.7 |

The earliest row is `0.49` (1994-07-18) and the newest is `4.1.0-preview1`
(2026-10-12). Ruby 3.5 became 4.0: the tags go `v3_5_0_preview1`, then
`v4.0.0-preview2`.

## Release IDs

Release IDs follow ruby-lang.org spelling. They are derived from tag names
like this:

| Tag | Release |
| --- | --- |
| `v4.0.1`, `v4.0.0-preview2` | unchanged without the `v` |
| `v3_3_0`, `v3_3_0_rc1` | `3.3.0`, `3.3.0-rc1` |
| `v1_8_7_374` | `1.8.7-p374` (`v2_0_0_0` is `2.0.0-p0`) |
| `v1_9_0_5` | `1.9.0-5`, matching the upstream tarball `ruby-1.9.0-5` |
| `v1_3_1_990126`, `v1_0_961225` | `1.3.1-990126`, `1.0-961225` (dated snapshots) |
| `v1_1b9_07`, `v1_1c9_1` | `1.1b9_07`, `1.1c9_1` |
| `v1_2_1repack`, `v1_3_6_repack` | `1.2.1-repack`, `1.3.6-repack` |
| `v1_3_1a_990201` | `1.3.1a-990201` (the CVS tag was `v1_3_1"_990201`) |
| `v0_95` | `0.95` |

`source_ref` is the tag's GitHub URL and `source_sha` is the commit the tag
names.

## Dates

- **247 rows** take the release date from ruby-lang.org's
  `_data/releases.yml` (blob `cdd8e866a2ca46ad76a41fcda97949fb57ec39a1` at
  ruby/www.ruby-lang.org commit `e910a046bc08`). That file lists releases
  from 1.6.7 onward.
- **942 rows** carry a `?`: the date is the author date of the tag-creation
  commit. That is the svn `add tag ...` commit, or the cvs2svn `This commit was
  manufactured by cvs2svn to create tag ...` commit. The bulk
  `set svn:eol-style` commits from 2007-02-12 are skipped. Most of these rows
  are tag-only patch levels with no announcement. The rest are pre-1.6.7
  releases and `3.0.0-rc2`, which ruby-lang.org does not list. Where both
  sources exist, the tag date matches the announced date in 191 of 247 cases
  and is within 5 days in all but 7.

No row has an empty date.

## Lineage (`predecessor`, `lineage`)

`lineage` records how each `predecessor` was found.

- `line`: the previous release on the same maintenance line. Lines are
  `X.Y.Z` for 1.8.5 through 2.0.0, which had per-teeny branches with patch
  levels, and `X.Y` otherwise.
- `branch-point`: the first release of a line. Its predecessor is the most
  recent earlier release of the nearest lower line whose tagged commit is a
  git ancestor of this release's commit. svn and cvs tag-artifact commits are
  first walked back to the branch commit. For example, `2.1.0-preview1`
  follows `2.0.0-rc1`.
- `git-ancestor`: the 0.x, 1.0 and 1.1 series, which belong to no line. The
  predecessor is the most recent earlier release that is a git ancestor.
- `curated`: early CVS history is a chain of vendor-branch imports, so git
  ancestry does not match the release series for three rows. Each was set by
  hand: `1.0` follows `1.0-971225`, `1.3.1-990126` follows `1.1d1` (the 1.1d
  series became 1.3), and `1.4.0` follows `1.3.7`.
- `first`: `0.49`.

## Gaps

- Untagged releases are not rows, because LANG84 v1 covers tags only. These
  are recorded in the header:
  - `1.9.2-p330` (2014-08-19), listed on ruby-lang.org but never tagged
  - any release before `v0_49`
  - any pre-1.6.7 tarball without a tag. The tarball index
    (`cache.ruby-lang.org/pub/ruby/index.txt`) could not be reached during the
    audit, so this has not been cross-checked.
- Every row is `pending`, stage `none`. No per-release pair exists yet. The
  first ones are the LANG84 phase 3.2 `rb-int-puts` partial pairs.

`python3 code/scripts/release_inventory.py check --matrix` validates this
file and `subsets.csv`.

## Subsets (`subsets.csv`)

`subsets.csv` (LANG84 phase 2b) declares the subsets that `grammar_subset`,
`iir_status` and `backend_status` may name. It has one entry, `rb-int-puts`:
LANG80's integer `puts` subset (phase 5 brings it to stage I and B on the R0
pairs), defined in LANG80. Its corpus goes in `corpus/rb-int-puts/` when a
release first claims stage I. No row names it yet.
