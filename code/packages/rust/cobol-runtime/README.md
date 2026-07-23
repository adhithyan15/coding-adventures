# cobol-runtime (coding-adventures-cobol-runtime)

The **execution** layer of the COBOL stack — a tree-walking interpreter for
COBOL-60 built on [`cobol-parser`](../cobol-parser). It turns WORKING-STORAGE
into a PICTURE-typed data model and runs the PROCEDURE DIVISION, capturing
everything `DISPLAY`ed. Implements [PL08](../../../specs/PL08-cobol-runtime.md).

This is the spine of the long-term goal — a *full, faithful* COBOL — because
COBOL's quirks are runtime behaviours (fixed-point decimal, PICTURE editing on
`MOVE`, `USAGE` storage, level-88, `PERFORM … THRU`, …), not syntax.

## API

```rust
use coding_adventures_cobol_runtime::run_cobol;
let out = run_cobol(source)?; // everything the program DISPLAYed
```

## Scope

A small but **fully correct** slice, growing one quirk at a time: numeric-display
(`9`/`V`, and signed `S9…` with trailing-overpunch `DISPLAY`) and character
(`X`/`A`) pictures; the item tree from
level numbers; `VALUE` initialisation; figurative `ZERO`/`SPACE`; `MOVE` with
the exact justify/pad/truncate rules — same-category (numeric→numeric,
alphanumeric→alphanumeric) plus both cross-category shapes: an unsigned numeric
source — integer **or** scaled `PIC 9(i)V9(d)`, whose `(i+d)`-digit image
concatenates the integer and fractional digits with **no** decimal point (the
oracle's `Decimal::digits()` = `int + frac`) — into an alphanumeric receiver (that
image left-justified, space-padded, or truncated) and the reverse — an
alphanumeric source into an unsigned numeric receiver, **integer or scaled
`PIC 9(i)V9(d)`**: its `m` chars fold into an unsigned integer `V`, and that fold
**is the receiver's scaled slot magnitude directly** — the `(i+d)` digit positions
right-justified with the implied point `d` places from the right, so
`slot = V mod 10^(i+d)` (`MOVE "042" TO 9(2)V9` → `042` reads `4.2`,
`MOVE "12345" TO 9(2)V9` → `345` reads `34.5`; NOT the arithmetic decimal-align
rule — `V` is not multiplied by `10^d`); a signed
numeric item on either side, a group, and a
source wider than 18 chars are later rungs; `DISPLAY`; `STOP RUN`;
fixed-point
decimal `ADD`/`SUBTRACT`/`MULTIPLY`/`DIVIDE` (decimal-point aligned; `ROUNDED`
and `ON SIZE ERROR`; divide-by-zero is a clean error or a size-error condition);
`COMPUTE` with
precedence-correct arithmetic expressions (`+ - * / **`, unary sign,
parentheses), `ROUNDED`, and `ON SIZE ERROR`; `IF … ELSE` with numeric,
alphanumeric, and **mixed unsigned-numeric ↔ alphanumeric** comparison (the
numeric operand — integer or scaled — treated as its digit image
(`Decimal::digits()`, the item's fixed-width zero-padded storage, `int + frac`
with no point) — then space-padded and byte-compared, so `NUM = "042"` is true but
`NUM = "42"` is false, and `9(2)V9=4.2 = "042"` is true; a signed numeric operand
or a group item in such a mixed comparison is a clean later rung, matching the
compiler); **reference modification** `IDENT(start:len)` /
`IDENT(start:)` (a 1-based substring of an alphanumeric item, in `DISPLAY` and
alphanumeric-comparison operands — with **constant** integer indices *or*
**computed** data-name indices like `WS(J:K)`, the index item an unsigned integer;
an out-of-range computed refmod is a `RefModOutOfRange` trap under the same
predicate the compiled `str_slice` enforces, so both engines error identically);
`PERFORM para
[THRU para2] [n TIMES | UNTIL cond |
VARYING id FROM x BY y UNTIL cond]` (out-of-line paragraph or paragraph-range
invocation — fixed-count, conditional, and counted loops, with a recursion
guard); `GO TO para` (unconditional transfer, run as a program counter over
paragraphs, so back-edge loops work); and `STRING s… DELIMITED BY SIZE INTO t`
(concatenate the sending fields — each taken in full — into the alphanumeric
receiver, left-justified, truncated at its width, and, per ANSI-85, **without**
space-filling the untouched tail); and `UNSTRING source DELIMITED BY delim INTO
r1 [r2 …]` (the inverse — split the alphanumeric source on a single-character
delimiter into successive receivers; each receiver including the last takes the
field up to the next delimiter, extra fields are dropped, empty fields become
spaces, and once the source is exhausted the remaining receivers keep their prior
value); and `INSPECT source TALLYING counter FOR ALL delim` (count the
occurrences of a single-character delimiter — a 1-char literal or a `PIC X(1)`
item — in the alphanumeric source and **ADD** them to the unsigned-integer
counter; INSPECT adds, it does not clear the counter first); and `INSPECT source
REPLACING ALL x BY y` (replace every occurrence of a single character `x` — a
1-char literal or a `PIC X(1)` item — with a single character `y` in the
alphanumeric source, **in place**, a per-position map that leaves the width
unchanged); and the **combined** `INSPECT source TALLYING counter FOR ALL delim
REPLACING ALL x BY y` (one `INSPECT`, both phrases — per ISO it runs as
tally-then-replace: count `delim` in the ORIGINAL source into `counter` FIRST,
then replace `x` with `y`, so a shared `delim == x` is counted before it is
substituted); and `INSPECT source CONVERTING from TO to` (translate each character
of the alphanumeric source through a per-character table built from the two
equal-length string literals `from`/`to` — a character equal to `from[k]` becomes
`to[k]`, the **first (leftmost) occurrence winning** if `from` repeats a
character, others left unchanged — **in place**, same width). Anything not yet
modelled (the explicit
`SIGN` clause with `SEPARATE`/`LEADING`, editing pictures, `COMP`,
`PERFORM … WITH TEST AFTER`/inline, `GO TO … DEPENDING`, `STRING` with a real
delimiter / `WITH POINTER` / `ON OVERFLOW`, `UNSTRING` with a multi-character
delimiter / `WITH POINTER` / `ON OVERFLOW`, `INSPECT` with a `LEADING`/
`CHARACTERS` tally, `BEFORE`/`AFTER` phrases, `REPLACING CHARACTERS`/`LEADING`/
`FIRST`, several replace items, or a combined statement whose `TALLYING` or
`REPLACING` half is itself a deferred sub-form, `CONVERTING` with unequal-length
`FROM`/`TO`, a data-name/figurative/reference-modified `from`/`to`, or a
`BEFORE`/`AFTER` region, tables,
files,
and every other verb) returns a descriptive `RuntimeError` — never wrong output.
See PL08 for the roadmap toward full COBOL and later standards.
