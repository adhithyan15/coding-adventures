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
alphanumeric→alphanumeric) plus both cross-category shapes: a numeric
source — unsigned **or** signed, integer **or** scaled `PIC [S]9(i)V9(d)`, whose
`(i+d)`-digit image concatenates the integer and fractional digits with **no**
decimal point (the oracle's `Decimal::digits()` = `int + frac`) — into an
alphanumeric receiver (that image left-justified, space-padded, or truncated). A
**signed** source additionally carries its operational sign as a **trailing
overpunch** on the units digit — the same zoned-decimal encoding `DISPLAY` of a
`PIC S9…` field produces (positive `{A…I`, negative `}J…R`), so `S9(3)=+123 →
"12C"`, `= -123 → "12L"`, `S9V9=-4.2 → "4K"`; the overpunch is driven by the item
being signed, so a signed *positive* source (`"12C"`) differs from an unsigned one
(`"123"`). And the reverse — an
alphanumeric source into an unsigned numeric receiver, **integer or scaled
`PIC 9(i)V9(d)`**: its `m` chars fold into an unsigned integer `V`, and that fold
**is the receiver's scaled slot magnitude directly** — the `(i+d)` digit positions
right-justified with the implied point `d` places from the right, so
`slot = V mod 10^(i+d)` (`MOVE "042" TO 9(2)V9` → `042` reads `4.2`,
`MOVE "12345" TO 9(2)V9` → `345` reads `34.5`; NOT the arithmetic decimal-align
rule — `V` is not multiplied by `10^d`); an alphanumeric→**signed** numeric MOVE, a
`SIGN` clause with `SEPARATE`/`LEADING`, a group on either side, and a
source wider than 18 chars are later rungs; `DISPLAY`; `STOP RUN`;
fixed-point
decimal `ADD`/`SUBTRACT`/`MULTIPLY`/`DIVIDE` (decimal-point aligned; `ROUNDED`
and `ON SIZE ERROR`; divide-by-zero is a clean error or a size-error condition);
`COMPUTE` with
precedence-correct arithmetic expressions (`+ - * / **`, unary sign,
parentheses), `ROUNDED`, and `ON SIZE ERROR`; `IF … ELSE` with numeric,
alphanumeric, and **mixed numeric ↔ alphanumeric** comparison (**unsigned or
signed**, integer or scaled; the numeric operand treated as its digit image
(`Decimal::digits()`, the item's fixed-width zero-padded storage, `int + frac`
with no point) — then space-padded and byte-compared, so `NUM = "042"` is true but
`NUM = "42"` is false, and `9(2)V9=4.2 = "042"` is true. A **signed** operand
compares by that same magnitude with the operational sign folded into a **trailing
overpunch** on the units digit (`overpunch_trailing`, the same image the
signed→alphanumeric `MOVE` builds), so `S9(3)=-123` equals `"12L"`, `= +123` equals
`"12C"`, `S9V9=-4.2` equals `"4K"`; ordering follows the byte comparison of those
images. A numeric *literal* vs alphanumeric (a different pairing) or a group item in
such a mixed comparison is a clean later rung, matching the compiler; two figurative
constants compared against each other (`IF ZERO = SPACE`) each fill to a single
character — `ZERO` → `"0"`, `SPACE` → `" "` — so `ZERO = ZERO` is true and
`ZERO > SPACE`, and the compiler now compiles the same construct. `EVALUATE`'s
subject-vs-`WHEN` comparison (`subject_in_when`) routes through this **same**
`compare_operands` rule, so a mixed numeric↔alphanumeric subject/`WHEN` — including
signed / scaled numeric sides, figuratives, and the same numeric-literal-vs-alphanumeric
deferral — is compared identically to `IF subject <relop> value`, and the compiler now
reuses its relation dispatch to match byte-for-byte);
**reference modification** `IDENT(start:len)` /
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
paragraphs, so back-edge loops work); and `STRING s… DELIMITED BY {SIZE | delim}
INTO t` (concatenate the sending fields into the alphanumeric receiver, left-
justified, truncated at its width, and, per ANSI-85, **without** space-filling the
untouched tail; `DELIMITED BY SIZE` takes each field in full, while `DELIMITED BY`
a single-character delimiter takes only each field's prefix up to its first
occurrence of the delimiter — `STRING "ab,cd" "ef" DELIMITED BY "," INTO t` →
`"abef"`; optionally with a `WITH POINTER p` phrase — `STRING A B DELIMITED BY
SIZE INTO T WITH POINTER P` — where `p` (a `PIC 9(n)` unsigned integer) gives the
1-based RECEIVER position at which the first transferred character is placed and is
UPDATED afterwards to `p + chars_placed` (one past the last character stored;
`size + 1` when the content fills to or past the receiver end, the excess dropped as
ISO overflow); an initial `p` outside `[1, size]` (0 or `> size`) is ISO overflow,
leaving the receiver and `p` unchanged; and optionally with `ON OVERFLOW imp…` /
`NOT ON OVERFLOW imp…` handlers — after the data movement the `ON OVERFLOW`
imperative runs when the STRING overflowed (the receiver filled before every sending
character was transferred, OR the initial pointer was out of range), else the `NOT ON
OVERFLOW` imperative runs, mirroring `ON SIZE ERROR` structurally); and `UNSTRING source DELIMITED BY delim INTO
r1 [r2 …]` (the inverse — split the alphanumeric source on a single-character
delimiter into successive receivers; each receiver including the last takes the
field up to the next delimiter, extra fields are dropped, empty fields become
spaces, and once the source is exhausted the remaining receivers keep their prior
value; the source may be an alphanumeric item, an alphanumeric string
literal — `UNSTRING "a,b,c" DELIMITED BY "," INTO w1 w2 w3` — **or** a
reference-modified item slice `base(start:len)` — `UNSTRING S(2:3) DELIMITED BY
"," INTO w1 w2 w3` (the sliced characters supply the field text via the shared
reference-modification machinery, so it stays byte-identical to `DISPLAY
S(2:3)`) — with identical splitting, only the character provider differing;
optionally with a `WITH POINTER p` phrase — `UNSTRING S DELIMITED BY "," INTO w1
w2 WITH POINTER P` — where `p` (a `PIC 9(n)` unsigned integer) gives the 1-based
character position at which scanning STARTS and is UPDATED afterwards to one past
the last character examined (`min(final_cursor, len) + 1`); an initial `p` outside
`[1, len]` (0 or `> len`) is ISO overflow, leaving every receiver and `p`
unchanged; and optionally with `ON OVERFLOW imp…` / `NOT ON OVERFLOW imp…`
handlers — after the scan and pointer write-back the `ON OVERFLOW` imperative runs
when the UNSTRING overflows, else the `NOT ON OVERFLOW` one, mirroring `ON SIZE
ERROR` structurally. Overflow here means all receivers are filled but the source is
NOT exhausted — more delimited fields remain (`final_cursor <= len`) — OR the
initial `WITH POINTER` value is out of range; the out-of-range case runs `ON
OVERFLOW` with no data movement); and `INSPECT source TALLYING
counter FOR ALL delim` / `FOR LEADING delim`
(count the occurrences of a single-character delimiter — a 1-char literal or a
`PIC X(1)` item — in the alphanumeric source and **ADD** them to the
unsigned-integer counter; `FOR ALL` counts EVERY occurrence, `FOR LEADING` counts
only the run of consecutive delimiters at the START, stopping at the first
non-match; INSPECT adds, it does not clear the counter first) — and, on the
STANDALONE form, an optional `{BEFORE|AFTER} x` **region** for BOTH `FOR ALL` and
`FOR LEADING` (narrow the count to the sub-slice bounded by the FIRST occurrence of
the single-character delimiter `x`: `BEFORE x` counts left of it — the WHOLE source
if `x` is absent — and `AFTER x` counts right of it — NOTHING if `x` is absent; for
`FOR LEADING` the run is ANCHORED at the window start, so `FOR LEADING "a" AFTER "X"`
over `"aaXaab"` counts the run in the window `"aab"` — 2 — not the `"aa"` before the
`X`); the **`FOR CHARACTERS`** form `INSPECT source TALLYING counter FOR CHARACTERS
[ {BEFORE|AFTER} x ]` (single item, single counter — count NOT a delimiter match but the
NUMBER OF CHARACTER POSITIONS in the region window, ADDed to the counter: with no region
that is `length(source)`, with a region it is the window length of the SAME window
`FOR ALL` uses, inheriting the identical not-found asymmetry — `BEFORE x` absent ⇒ whole
source, `AFTER x` absent ⇒ empty ⇒ 0; a MULTI-item / MULTI-counter `CHARACTERS` and a
`CHARACTERS` half in a combined `TALLYING … REPLACING` stay later rungs); the **multi-item** `INSPECT source TALLYING counter FOR ALL a ALL b [ALL d …]`
(TWO OR MORE `FOR ALL` items sharing ONE counter — ONE left-to-right pass in which, at
each position, the delimiters are tried in WRITTEN ORDER and the FIRST that matches adds
1 to the shared counter, then the scan advances; so DUPLICATE delimiters do NOT
double-count — `FOR ALL "a" ALL "a"` over `"aa"` adds 2, not 4 — and the count is just
the number of positions whose char equals SOME delimiter, each counted once, ADDED to the
counter; this rung's multi path is `ALL`-only, single-char, with no `{BEFORE|AFTER}`
region and no `LEADING`/`CHARACTERS`, under EXACTLY ONE counter — a single tally item's
full capabilities are unchanged); the **multi-counter**
`INSPECT source TALLYING c1 FOR ALL a [ALL b …] c2 FOR ALL d …` (TWO OR MORE `tally_for`
groups, each with its OWN counter and one-or-more single-char `FOR ALL` delimiters — ALL
groups' delimiters form ONE combined priority list scanned in a SINGLE left-to-right pass:
at each position they are tried in WRITTEN ORDER, group-1's items first, and the FIRST that
matches adds 1 to ITS OWN group's counter, then the scan advances — so a position CLAIMED
by an earlier group NEVER reaches a later group, `TALLYING C1 FOR ALL "a" C2 FOR ALL "a"`
over `"aa"` gives `C1 += 2, C2 += 0`; each counter ADDS its own share and the SAME counter
name may appear in two groups, both then adding to that one item; this rung's multi-counter
path is `ALL`-only, single-char, with no `{BEFORE|AFTER}` region and no `LEADING`/
`CHARACTERS`, every counter an unsigned integer `PIC 9(n)`, and the combined form with
several counters is still a later rung); and `INSPECT source REPLACING ALL x BY y` / `REPLACING LEADING x BY y`
(substitute a single character `x` — a 1-char literal or a `PIC X(1)` item — with a
single character `y` in the alphanumeric source, **in place**, a per-position map
that leaves the width unchanged; `ALL` replaces EVERY occurrence, `LEADING` replaces
only the run of consecutive `x` at the START, stopping at the first character that is
not `x` — positions after that first gap are left unchanged even if they equal `x`)
— and, on the STANDALONE form, an optional `{BEFORE|AFTER} z` **region** for BOTH
`REPLACING ALL` and `REPLACING LEADING` (restrict the substitution to the sub-slice
bounded by the FIRST occurrence of the single-character delimiter `z`, using the SAME
window the count uses: `BEFORE z` replaces left of it — the WHOLE source if `z` is
absent — and `AFTER z` replaces right of it — NOTHING if `z` is absent; positions
outside the region keep their original character, and for `REPLACING LEADING` the run
is ANCHORED at the window start, so `REPLACING LEADING "a" BY "*" AFTER "X"` over
`"aaXaab"` rewrites only the in-window run → `"aaX**b"`); the **multi-item**
`INSPECT source REPLACING ALL a BY x [{BEFORE|AFTER} p] ALL b BY y [{BEFORE|AFTER} q] …`
(TWO OR MORE `ALL` replace items in one clause, **each item optionally carrying its OWN
`{BEFORE|AFTER}` region** — ONE left-to-right pass in which each position takes the
FIRST item, in WRITTEN ORDER, that BOTH contains the position in its own window AND whose
single-char search matches the ORIGINAL character; FIRST-MATCH-WINS, and — crucially — NO
RE-CHAINING: a byte a replacement produces is never re-examined by a later item, so
`REPLACING ALL "a" BY "b" ALL "b" BY "z"` over `"ab"` gives `"bz"`, not `"zz"`. Each
item's window is computed over the ORIGINAL source with the SAME `region_window` helper the
lone/single-item forms use — `BEFORE p`→`[0, first_index_of_p)`, `AFTER p`→`(first_index_of_p, len]`,
not-found asymmetry BEFORE→whole / AFTER→empty; an item with no region has the whole source
as its window. So `ALL "a" BY "b" BEFORE "X" ALL "a" BY "c" AFTER "X"` over `"aXaXa"` →
`"bXcXc"`. This rung's multi path stays `ALL`-only, single-char; a `LEADING`/`CHARACTERS`/`FIRST`
item in the list is a later rung — a single replace item keeps all its capabilities);
the **replace-every-position** `INSPECT source REPLACING
CHARACTERS BY x` (no search character — EVERY position of the alphanumeric source is
overwritten with the single replacement char `x`, so with no region the WHOLE field
becomes `x`s, its width unchanged: `"ABABA"` → CHARACTERS BY `"X"` → `"XXXXX"`; the fill
is computed on a BYTE basis — `n = storage.len()` copies stored through `move_into`,
which re-pads/truncates to the picture's CHAR size — so a non-ASCII source stays co-total
with the byte-based compiler: `PIC X(5) VALUE "café"` → CHARACTERS BY `"Z"` → `"ZZZZZ"`
(FIVE `Z`s — the fixed 5-char width caps the padded 6-byte image on both engines); a
`{BEFORE|AFTER}` region on the CHARACTERS item and a single-char but non-ASCII *literal*
replacement are later rungs, a `PIC X(1)` *item* replacement is supported); and the
**combined** `INSPECT source TALLYING counter FOR {ALL|LEADING} delim
REPLACING {ALL|LEADING} x BY y` (one `INSPECT`, both phrases — per ISO it runs as
tally-then-replace: count `delim` in the ORIGINAL source into `counter` FIRST,
then replace `x` with `y`, so a shared `delim == x` is counted before it is
substituted; the `TALLYING` half may be `FOR ALL` or `FOR LEADING` and the
`REPLACING` half may independently be `ALL` or `LEADING`, so either, both, or
neither half may be leading — and each half independently accepts its OWN
`{BEFORE|AFTER} x` **region** when that half is `FOR ALL` / `REPLACING ALL`, both
windows computed over the SAME original source since the tally does not mutate it; a
LEADING half carrying a region is still a later rung in the combined form even though
the STANDALONE `FOR LEADING`/`REPLACING LEADING` + region forms are supported); and
`INSPECT source CONVERTING from TO to` (translate each character
of the alphanumeric source through a per-character table built from the two
equal-length string literals `from`/`to` — a character equal to `from[k]` becomes
`to[k]`, the **first (leftmost) occurrence winning** if `from` repeats a
character, others left unchanged — **in place**, same width), with an optional
`{BEFORE|AFTER} z` **region** (restrict the translation to the sub-slice bounded by
the FIRST occurrence of the single-character delimiter `z`, using the SAME window the
count and replacement use: `BEFORE z` translates left of it — the WHOLE source if `z`
is absent — and `AFTER z` translates right of it — NOTHING if `z` is absent;
positions outside the region keep their original character). Anything not yet
modelled (the explicit
`SIGN` clause with `SEPARATE`/`LEADING`, editing pictures, `COMP`,
`PERFORM … WITH TEST AFTER`/inline, `GO TO … DEPENDING`, `STRING` with a
multi-character or non-ASCII delimiter / a non-ASCII literal sending field under a
delimiter / per-field different delimiters / a signed, fractional, non-numeric or
over-wide (`> 18`-digit) `WITH POINTER` item / `ON OVERFLOW` (a single-character
ASCII `DELIMITED BY` delimiter and a `WITH POINTER p` phrase over a `PIC 9(n)`
pointer ARE supported), `UNSTRING` with a
multi-character
delimiter / a signed, fractional, non-numeric or
over-wide (`> 18`-digit) `WITH POINTER` item / a NUMERIC-literal or FIGURATIVE
source or a NON-ASCII literal source or a NUMERIC-base reference-modified source
(an ASCII alphanumeric string-literal source, an alphanumeric-base
reference-modified source `S(2:3)` — literal or computed index — a `WITH
POINTER p` phrase over a `PIC 9(n)` pointer, and `ON OVERFLOW` / `NOT ON OVERFLOW`
handlers ARE supported),
`INSPECT` with a `CHARACTERS`
tally, a `{BEFORE|AFTER}` region on a `FOR LEADING`/`REPLACING LEADING` phrase
(the region ships for `FOR ALL`/`REPLACING ALL` only — on the lone forms, on
`CONVERTING`, and now on each half of the combined form) or a
MULTI-character region delimiter, a `REPLACING CHARACTERS` item carrying a
`{BEFORE|AFTER}` region or a non-ASCII literal replacement / `REPLACING FIRST` (a lone
`REPLACING CHARACTERS BY x` is now supported), a MULTI-item
`REPLACING` list carrying a `LEADING`/`CHARACTERS`/`FIRST` item or a `{BEFORE|AFTER}`
region (the multi-item list itself is now supported for plain single-char `ALL` items),
a MULTI-item `TALLYING` list carrying a `LEADING`/`CHARACTERS` item or a `{BEFORE|AFTER}`
region (the multi-item tally list itself is now supported for plain single-char `ALL`
items under ONE counter, and several `tally_for` groups each with their own counter are now
supported too), several replace or tally items — or several counters — in the COMBINED `TALLYING … REPLACING` form, or a combined
statement whose `TALLYING`/`REPLACING` half is a
deferred sub-form (a combined `TALLYING … FOR LEADING` and a combined `REPLACING
LEADING`, in any combination, are now supported), `CONVERTING` with
unequal-length
`FROM`/`TO`, or a data-name/figurative/reference-modified `from`/`to`, tables,
files,
and every other verb) returns a descriptive `RuntimeError` — never wrong output.
See PL08 for the roadmap toward full COBOL and later standards.
