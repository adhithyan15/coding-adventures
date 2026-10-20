# ECMAScript / JavaScript grammars

`es<edition>.tokens` and `es<edition>.grammar` are the pre-LANG84 per-edition
pairs used by the JavaScript frontend: es1, es3, es5 and es2015 through
es2025. `releases.csv` is the LANG84 release inventory for JavaScript. See
[LANG84](../../specs/LANG84-grammar-driven-versioned-frontends.md).

As LANG84 specifies, pair files in this directory take no prefix, because the
release IDs already start with `es` or `js`.

## What the inventory holds

JavaScript has no single tag series, so the release unit is the published
edition or version. There are 29 rows:

- **17 ECMA-262 editions** (channel `edition`): `es1`, `es2`, `es3`, `es5`,
  `es5.1`, and `es2015` through `es2026`. ES4 was abandoned in 2008 and never
  published, so it is excluded.
- **12 Netscape/Mozilla JavaScript versions** (channel `final`): `js1.0`,
  `js1.1`, `js1.2`, `js1.3`, `js1.4`, `js1.5`, `js1.6`, `js1.7`, `js1.8`,
  `js1.8.1`, `js1.8.2` and `js1.8.5`. Firefox 4 (1.8.5) was the last release
  to carry a JavaScript version number.

The tc39/ecma262 repository has 32 tags. Its 11 `esYYYY` tags are rows. The
working-draft tags (`es2016-draft-*`), the candidate tags
(`es20YY-candidate-*`) and `es2026-errata` are excluded, with reasons in the
CSV header.

## Sources and dates

| Rows | `source_ref` / `source_sha` | Date source |
| --- | --- | --- |
| es2016–es2026 | tc39/ecma262 `esYYYY` tag / tagged commit | Ecma General Assembly approval month (June) on the edition's title page; recorded as `YYYY-06` |
| es1, es2, es3, es5, es5.1, es2015 | Ecma archive PDF URL / `-` | edition title page month: June 1997, August 1998, December 1999, December 2009, June 2011, June 2015 |
| js1.1–js1.5 | MDN "New in JavaScript 1.x" page in mdn/archived-content `b0aa15f4` / page blob ID | the release date the page gives: Navigator 3.0 1996-08-19, Navigator 4.0 1997-06-11, Navigator 4.5 1998-10-19, "1999" for server-side 1.4, Navigator 6.0 2000-11-14 |
| js1.6–js1.8.5 | MDN page (for 1.8.2, mdn/content's Firefox 3.6 page `a64fca52`) / page blob ID | the Firefox release that shipped it, dated by Mozilla's Firefox Archive table (wiki.mozilla.org/Firefox_Archive): 1.5 2005-11-29, 2 2006-10-24, 3 2008-06-17, 3.5 2009-06-30, 3.6 2010-01-21, 4 2011-03-22 |
| js1.0 | MDN 1.1 page, whose changelog starts from Navigator 2.0 | `1996-03?`: Navigator 2.0's month, from secondary sources only |

**Date format.** Where a source gives only a month or a year, the date uses
ISO 8601 reduced precision (`1997-06`, `1999`). It sorts as the start of that
period. `?` marks two dates that rest on weaker evidence:

- `js1.0` (`1996-03?`): secondary sources only.
- `es2` (`1998-08?`): the edition document says August 1998, but the General
  Assembly approved it in June 1998.

The Ecma archive (ecma-international.org) could not be reached from the
inventory environment. The PDF URLs follow Ecma's published file naming but
were not fetched, and their SHA-256 values remain `-` until
`release_evidence.py` (LANG84 phase 2c) records them.

## Lineage

The two series form separate chains:

- `js1.0` → `js1.1` → … → `js1.8.5`
- `es1` → `es2` → `es3` → `es5` → `es5.1` → `es2015` → … → `es2026`

`es1` is marked `curated`. Its predecessor is `js1.1`, because ECMA-262 1st
edition standardized Navigator 3.0's JavaScript 1.1.

## Status of the existing pairs

The `existing_pair` column names the pre-LANG84 pair for es1, es3, es5 and
es2015–es2025. These rows stay `pending`, stage `none`:

- The pairs do not cite `source_ref`/`source_sha` in their headers.
- No `release_probes` test runs them.

Under the LANG84 status stages they have therefore not reached stage G. The
LANG84 phase 6 rewrite moves them to their measured stages. es2, es5.1, es2026
and every `js1.x` release have no pair.

`python3 code/scripts/release_inventory.py check` validates this file.
