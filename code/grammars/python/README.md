# Python grammars

This directory holds three kinds of files:

- `python.tokens` and `python.grammar`: the unversioned pair.
- `python2.7`, `python3.0`, `python3.6`, `python3.8`, `python3.10` and
  `python3.12` pairs: legacy **minor-line** pairs. They serve only the existing
  `parse_python(source, "3.12")` API. LANG84 keeps them until the per-release
  pairs of their line exist, then deletes them. They are never selectable
  through the release API.
- `releases.csv`: the LANG84 release inventory. It has one row per upstream
  tagged CPython release. Per-release pairs (`python<release>.*`) are built
  against it, earliest first. See
  [LANG84](../../specs/LANG84-grammar-driven-versioned-frontends.md).
- Seven Python 3 pairs: `python3.0a1`, `python3.11.17`, `python3.12.3`,
  `python3.13.16` and `python3.16.0a1` (LANG84 phase 7a), and
  `python3.5.10` and `python3.6.0` (phase 7c slice 2). Each is a **partial**
  pair for the `py-int-print` subset, and all but 3.0a1 also lex floats for
  `py-float-print` (phase 7c slice 3). `pair-evidence.csv` cites the
  upstream files each was written from. See [Release pairs](#release-pairs-lang84-phase-7a).
- Four Python 2 pairs (LANG84 phase 7c): `python2.2` and `python2.7.18`
  (slice 1), `python2.4` and `python2.6` (slice 2). They are partial pairs
  for the `py2-int-print` subset: the `print` statement, classic `/`, and
  Python 2's octal, hex and long literals. See
  [Python 2 pairs](#python-2-pairs-lang84-phase-7c).
- `subsets.csv`, `corpus/py-int-print/`, `corpus/py-float-print/` and
  `corpus/py2-int-print/`: the subsets and their oracle-backed test cases. See [Corpus](#corpus-py-int-print-py-float-print-py2-int-print).

The CSV header declares the legacy pairs with
`legacy-minor-line-pairs: ...`, so `release_inventory.py check` does not
mistake them for release pairs.

**Name collision:** `python2.7.*` and `python3.0.*` have the same file names
that releases `2.7` and `3.0` (tags `v2.7` and `v3.0`) will need. Before either
release gets its own pair, the legacy file must be moved or deleted, and its
`legacy-minor-line-pairs` entry dropped. (Releases `2.2`, `2.4`, `2.6`,
`2.7.18`, `3.5.10` and `3.6.0` have their own pairs; their stems do not
collide with `python2.7.*`, `python3.0.*` or `python3.6.*`.)

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

Every row is `pending`, stage `none`, except the five phase-7a rows below.

`python3 code/scripts/release_inventory.py check --matrix` validates this
file and `subsets.csv`.

## Release pairs (LANG84 phase 7a)

`py-int-print` needs `print` as a function and `//`, so it starts with
Python 3. These seven releases have a pair:

| Release | Why this release | Token class | Oracle |
| --- | --- | --- | --- |
| 3.0a1 | earliest Python 3 tag | no `_` in numbers; 33 keywords; 45 operators | pinned source at the tag |
| 3.6.0 | first release with PEP 515 `_` (phase 7c slice 2) | `_` in numbers; 33 keywords; 47 operators (`@=`, and the barry_as_FLUFL `<>`) | pinned source at the tag |
| 3.5.10 | last 3.x final before PEP 515 (phase 7c slice 2) | as 3.6.0 without `_` | pinned source at the tag |
| 3.11.17 | host `python3.11` | `_` (PEP 515); 35 keywords; 47 operators (`@=`, `:=`) | host interpreter |
| 3.12.3 | host `python3.12` | as 3.11.17 plus `!` (PEP 701): 48 operators | host interpreter |
| 3.13.16 | host `python3`; the frontend's `DEFAULT_RELEASE` | as 3.12.3 | host interpreter |
| 3.16.0a1 | newest inventory row | as 3.12.3 | pinned source at the tag |

Every row is `partial`, stage `P`. 3.0a1 has `grammar_subset` and
`iir_status` `py-int-print` and profile `py30`; the other six have
`grammar_subset` `py-float-print`, `iir_status` and `backend_status`
`py-int-print;py-float-print`, and profile `py3` (LANG84 phase 7c slice 3:
3.0a1's `str(float)` is `%.12g`, the others' is `repr`). Since LANG84 phase 7b slice 2
every row also claims `backend_status`. `python-iir-compiler`'s
`tests/release_sweep.rs` groups the seven releases into IIR-identity classes
and writes `release_classes.csv` (`subset,case,release,class`, one row per
positive case and release; `class` is a 16-hex digest of the release's IIR
for the whole corpus, or `rejected` where a `.reject@<release>` override
applies). There are two classes. 3.0a1 and 3.5.10 refuse `int_underscores`;
3.0a1, the first of them in this file, represents them. 3.6.0, 3.12.3,
3.13.16, 3.11.17 and 3.16.0a1 form the other, with 3.6.0 as its
representative (3.12.3 was, until phase 7c slice 2 added 3.6.0). Note that
3.5.10 (2020) comes after 3.6.0 (2016) in this file, which is in date order.
`lang_matrix` runs every case each representative accepts on every backend,
plus the whole corpus on the default 3.13.16. `check --matrix` credits the
3.0a1 rows to 3.5.10 and the 3.6.0 rows to the rest. After a deliberate change,
`PYTHON_RELEASE_SWEEP_WRITE=1 cargo test -p python-iir-compiler --test
release_sweep` rewrites the CSV.

**How each pair was written.** Every pair is `reviewed`: no two releases have
identical syntax blobs, so none is a mechanical copy. `pair-evidence.csv`
records, per release, the git blob IDs (`git ls-tree <commit> <path>` in
`python/cpython`, retrieved 2026-10-20; 2026-10-25 for the slice 2 rows) of:

- the grammar (`Grammar/Grammar` at 3.0a1, `Grammar/python.gram` after);
- `Grammar/Tokens` (absent at 3.0a1, whose operators come from
  `PyToken_OneChar`/`TwoChars`/`ThreeChars` in `Parser/tokenizer.c`);
- the tokenizer (`Parser/tokenizer.c`, `Parser/lexer/lexer.c` from 3.13; from
  3.16 numbers are in `Parser/lexer/number.c`);
- `Lib/keyword.py`, which gives the hard keywords (`kwlist`);
- `Objects/longobject.c` and `Python/bltinmodule.c`, which back the `py3`
  profile (floor `//` and `%`, `ZeroDivisionError`, the `print` builtin).

Each `.tokens`/`.grammar` header repeats the release's `source_ref`,
`source_sha` and syntax blobs. `python-parser`'s `release_probes` test checks
the headers against both CSVs.

**What a pair lexes and parses.** The `.tokens` file lexes the subset's
neighbourhood: decimal integers, names, the release's keywords, and every
operator and delimiter its tokenizer knows. It does not lex strings, floats,
imaginary numbers or non-decimal integers, which are lexer errors. The
`.grammar` file parses `print` calls over `+ - * / // %`, unary `+`/`-` and
parentheses. It keeps `python3.12.grammar`'s rule names (LANG84 C8) and leaves
out the precedence levels the subset never reaches. Both files list their
narrowings in their header comments.

**Differences inside the subset.** The only one is PEP 515's `_` digit
separator. 3.0a1 and 3.5.10 lex `1_000` as an error, as their `tok_get`
rejects it. The other releases accept `1_000`, `0_0` and `00`, and reject
`1__0`, `1_`, `0_1` and `012`, as their tokenizers do. (3.5.10 and 3.6.0 also
lex `<>`, for the `barry_as_FLUFL` easter egg; the grammar refuses it, as
CPython does without the future import.)

**The 3.0 and 2.7 name collision** above still holds. Release `3.0` (final)
and release `2.7` have no pair. Python 2 is in the next section.

## Python 2 pairs (LANG84 phase 7c)

`py2-int-print` is Python 2's integer subset: the `print` statement (`print`
and `print <expr>`), `+ - * / // %`, where `/` is classic division and
floors on ints, and (since slice 2) every integer literal spelling. These
four releases have a pair, one per lexical change the subset can see:

| Release | Why this release | Token class | Oracle |
| --- | --- | --- | --- |
| 2.2 | first final release with `//` (PEP 238) | 29 keywords (`print` among them); 44 operators, with `<>` and the backquote; decimal, octal `010`, hex `0x1F`, `L` suffix | pinned source at the tag |
| 2.4 | PEP 237: hex and octal literals above `LONG_MAX` become positive longs | as 2.2 plus `@` (45 operators) | pinned source at the tag |
| 2.6 | PEP 3127: `0o17` and `0b101` | as 2.4 plus `0o`/`0b` and the keywords `as` and `with` (31) | pinned source at the tag |
| 2.7.18 | last Python 2 release | as 2.6 | pinned source at the tag |

Every row is `partial`, stage `P`, and names `py2-int-print` in
`grammar_subset`, `iir_status` and `backend_status`, with profile `py2`.
There is no host `python2`, so the evidence is the tags' own files, by blob ID
in `pair-evidence.csv`:
- `Grammar/Grammar`, `Parser/tokenizer.c` and `Lib/keyword.py`, which give
  the syntax;
- `Objects/intobject.c`, for classic `/` through `i_divmod`,
  `ZeroDivisionError`, and promotion past the C `long`;
- `Python/ceval.c`, for `PRINT_ITEM` and `PRINT_NEWLINE`;
- `Python/pythonrun.c`, for `Py_FlushLine` before a traceback.

For 2.2 and 2.7.18 those six blob IDs were computed with `git hash-object`
over the files' bytes at each tag, retrieved 2026-10-24; the 2.4 and 2.6
rows are `git ls-tree` in a clone, retrieved 2026-10-25. Each row's note also
names the number parser (`parsenumber` in `Python/compile.c` at 2.2 and 2.4,
`Python/ast.c` from 2.6, and `Python/mystrtoul.c`) behind the literal table.

The grammar is py-int-print's plus one rule, `print_stmt = "print" [
expression { COMMA expression } [ COMMA ] ]`. The compiler refuses
`print a, b` and `print a,` (`Unsupported`): CPython 2 writes a pending
softspace as a newline before an uncaught exception, which only a trap
handler could reproduce.

**Integer literals (slice 2).** Each `INT` is one token in any Python 2
spelling: decimal, octal `0[0-7]*` (`010` is 8, `00` is 0), hex `0x…`, from
2.6 `0o…` and `0b…`, each with an optional `l`/`L`, which prints without the
`L` (`str()` of a long). `08`, `0x` with no digit, `1LL` and `1_000` are
lexer errors. A value above `i64::MAX` is refused (`Unsupported`), and a long
that leaves `i64` traps `Trap.Overflow` like an int, the subset's stated
divergence. On 2.2 and 2.4, `0o17` is a lexer error here; CPython lexes `0`
then the name `o17` and its parser refuses it.

The release sweep puts 2.2 and 2.4 in one IIR-identity class and 2.6 and
2.7.18 in another (only 2.6 and later accept `pep3127_literals`), so
`lang_matrix` runs the corpus on 2.2 and on 2.6, and `check --matrix` credits
those rows to 2.4 and 2.7.18.

## Corpus (`py-int-print`, `py-float-print`, `py2-int-print`)

`corpus/py2-int-print/` has the same layout as the `py-int-print` corpus
below. There is no host `python2`, so each `.cite` names the pinned historical
source as its oracle.
- A rejected case records `host-python2: accepts` or `rejects`.
- A `Trap.Overflow` case records the bigger integer CPython 2 prints.

`python-iir-compiler` cross-checks every accepted case against host
`python3`. It translates `print X` to `print(X)`, classic `/` to `//`, `010`
to `0o10`, and drops an `L` suffix. That is exact on ints because both
languages floor and neither overflows, and Python 3's `int` is Python 2's
`long`.

### `py-int-print`

`corpus/py-int-print/` holds one file set per case:

- `<name>.py`: the program.
- `<name>.expect`: the exact stdout, then `trap:<kind>` when the program stops
  on a LANG83 trap (`Trap.DivZero` or `Trap.Overflow`).
- `<name>.reject`: the C6 error category (`Lex`, `Parse`, `Unsupported`) the
  compile must fail with.
- `<name>.reject@<release>`: a release where an otherwise accepted case must
  be refused, and with which category. `int_underscores.reject@3.0a1` and
  `@3.5.10` say `Lex`, as do `py2-int-print`'s `pep3127_literals.reject@2.2`
  and `@2.4`.
- `<name>.cite`: the oracle record.

The oracle for every accepted case is host CPython 3.11.17, 3.12.3 and 3.13.16
(`python3.11`, `python3.12`, `python3`), with one exception. For the
`Trap.Overflow` cases the expectation comes from the subset definition, not
from CPython: CPython would print a larger `int`, and this subset stops
instead. For those cases the `.cite` records the value CPython prints, and
`python-iir-compiler`'s oracle test checks that the value is outside `i64`.
Each rejected case's `.cite` records whether CPython accepts the program
(`host-python: accepts`) or rejects it too (`host-python: SyntaxError`,
`IndentationError`). A `py-int-print` refusal of a valid Python program is a
subset limit, not a claim about Python.

### `py-float-print`

`corpus/py-float-print/` (LANG84 phase 7c slice 3) has the same layout and
the same three host oracles. It holds the float cases only (literals, `_`
in floats with `.reject@3.5.10`, true division, mixing, signed zeros,
`inf`/`nan`, repr's layout, ZeroDivisionError after output, and one
`Trap.Overflow` case where an overflowing int meets a float, whose `.cite`
records the float CPython prints), plus refusals: float `//` and `%`,
`1e400`, `int / int` above 2^53, imaginary literals, malformed spellings.
The int cases stay in `py-int-print`; a release that claims both subsets
runs both corpora. The release sweep finds two `py-float-print` classes:
3.5.10 alone (no `_`), and 3.6.0 with the other four.

## Subsets (`subsets.csv`)

`subsets.csv` (LANG84 phase 2b) declares the subsets that `grammar_subset`,
`iir_status` and `backend_status` may name:

- `py-int-print` (phase 7a, above), with its corpus in `corpus/py-int-print/`.
- `py-float-print` (phase 7c slice 3, above): `py-int-print` plus floats,
  printed as `repr`, on 3.5.10 and later. The name is LANG79's; LANG79's
  pilot (the legacy `"3.12"` path) is not a claim of it.
