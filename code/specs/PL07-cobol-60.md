# PL07 — COBOL-60

## Overview

**COBOL** — **CO**mmon **B**usiness-**O**riented **L**anguage — was designed in
1959 by CODASYL (the Conference on Data Systems Languages), convened under the
sponsorship of the U.S. Department of Defense, and first specified in the
**1960 report**. It was the industry's answer to a problem FLOW-MATIC ([PL06])
had only begun to solve: every computer manufacturer had its own languages, so
business programs were locked to specific machines. The DoD — the largest
computer buyer in the world — wanted one language that ran everywhere and that
managers, not just engineers, could read.

COBOL-60 is the direct descendant of **FLOW-MATIC** (Grace Hopper's B-0, our
[PL06]). The CODASYL Short-Range Committee drew from three existing languages —
FLOW-MATIC was the most influential, alongside IBM's **COMTRAN** and the Air
Force's **AIMACO** — and FLOW-MATIC's fingerprints are everywhere in COBOL:
English-keyword verbs, hyphenated data names, and the separation of data
description from procedure. This excavation is the pay-off for building
FLOW-MATIC first: COBOL-60 reuses that machinery and adds the two things
FLOW-MATIC deliberately let us skip — a **fixed-column card format** and the
**PICTURE** clause.

This spec covers the **frontend** (lexing and parsing) of the 1960 language. It
is scoped to a coherent, historically faithful subset — enough to lex and parse
a complete four-division program — with a clear roadmap for the rest.

## Historical Context

### CODASYL and the 1960 report

In April 1959 a group of users, manufacturers, and government representatives met
at the University of Pennsylvania to discuss a common business language. That led
to CODASYL (May 1959) and a Short-Range Committee tasked with designing it. The
committee delivered the **COBOL-60 specification** in 1960; the first compilers
appeared that year, and in December 1960 the *same* COBOL program was
demonstrated running on two different manufacturers' machines (a UNIVAC II and an
RCA 501) — proof that the portability goal was real.

### Why it reads like English

COBOL inherited FLOW-MATIC's radical premise: business logic should be
*self-documenting*. `ADD TAX TO PRICE GIVING TOTAL` is deliberately verbose so a
non-programmer can read it. This is why COBOL has ~300 reserved words and why its
programs are organised as English sentences ending in periods.

### Why the fixed columns

COBOL-60 was written on **80-column punched cards**, and the reference format
reserved specific columns for specific purposes (sequence numbers, a
continuation/comment indicator, two code areas, and a program-identification
tail). This rigid layout — the thing that makes COBOL famously column-sensitive —
is exactly what our lexer's **pre-tokenize column-strip hook** exists to handle
(see "Lexer" below). It is the defining structural difference from FLOW-MATIC's
free-form listings.

## The Reference Format (fixed columns)

Every source line is 80 characters, partitioned by column:

```
 col:  1        6 7 8      11 12                                72 73        80
      ┌──────────┬─┬─────────┬────────────────────────────────────┬──────────┐
      │ sequence │I│ Area A  │ Area B                             │ ident.   │
      │  number  │ │ (8–11)  │ (12–72)                            │ (prog id)│
      └──────────┴─┴─────────┴────────────────────────────────────┴──────────┘
                  ▲
                  └ column 7: indicator
```

| Columns | Name | Purpose |
|---------|------|---------|
| 1–6 | Sequence number | Line ordering on the card deck. **Ignored** by the compiler. |
| 7 | Indicator | `*` = comment line, `/` = comment + page eject, `-` = continuation of the previous line, `D` = debugging line, space = normal. |
| 8–11 | Area A | Division headers, section headers, paragraph names, `FD`/`SD` entries, and level numbers `01` and `77` **must begin here**. |
| 12–72 | Area B | Statements, clauses, and level numbers `02`–`49`/`88`. |
| 73–80 | Identification | Program name / notes. **Ignored** by the compiler. |

The **column-strip** step keeps only Area A + Area B (columns 8–72), drops the
sequence and identification areas, skips `*`/`/` comment lines, and splices `-`
continuation lines onto their predecessor. After stripping, the remaining text is
ordinary free-form-ish COBOL that the grammar tokenizes normally.

> **Area A vs Area B** governs *where* a construct may begin, and is a parser-era
> refinement. The first lexer keeps the stripped code (cols 8–72) and does not
> yet enforce the A/B boundary; see "Scope" and "Future Extensions."

## The Four Divisions

A COBOL program is exactly four divisions, always in this order:

```
IDENTIFICATION DIVISION.        ← what the program is called, who wrote it
ENVIRONMENT   DIVISION.         ← the machine and the files it uses
DATA          DIVISION.         ← the shape of every piece of data
PROCEDURE     DIVISION.         ← what the program does
```

### IDENTIFICATION DIVISION

Names the program and records metadata. `PROGRAM-ID` is required; the rest are
optional commentary paragraphs.

```
IDENTIFICATION DIVISION.
PROGRAM-ID. PAYROLL.
AUTHOR. GRACE HOPPER.
DATE-WRITTEN. 1960.
```

### ENVIRONMENT DIVISION

Describes the hardware and the files — the only machine-dependent part, isolated
here on purpose so the rest of the program stays portable.

```
ENVIRONMENT DIVISION.
CONFIGURATION SECTION.
SOURCE-COMPUTER. UNIVAC-II.
OBJECT-COMPUTER. UNIVAC-II.
INPUT-OUTPUT SECTION.
FILE-CONTROL.
    SELECT EMPLOYEE-FILE ASSIGN TO TAPE.
```

### DATA DIVISION

Describes every data item. Two core sections:

- **FILE SECTION** — record layouts for files, introduced by `FD`.
- **WORKING-STORAGE SECTION** — the program's variables.

```
DATA DIVISION.
WORKING-STORAGE SECTION.
01  EMPLOYEE-RECORD.
    02  EMP-NAME     PICTURE X(20).
    02  EMP-RATE     PICTURE 9(3)V99.
    02  EMP-HOURS    PICTURE 9(3).
77  GROSS-PAY        PICTURE 9(6)V99 VALUE ZERO.
```

### PROCEDURE DIVISION

The logic, organised into optional **sections**, then **paragraphs** (named
groups of sentences), then **sentences** (one or more statements ending in a
period), then **statements** (a verb and its operands).

```
PROCEDURE DIVISION.
MAIN-PARAGRAPH.
    MOVE ZERO TO GROSS-PAY.
    MULTIPLY EMP-RATE BY EMP-HOURS GIVING GROSS-PAY.
    DISPLAY EMP-NAME GROSS-PAY.
    STOP RUN.
```

## Data Description Details

### Level numbers

A two-digit level number gives an item's place in a record hierarchy:

| Level | Meaning |
|-------|---------|
| `01` | Record (top of a hierarchy). Begins in Area A. |
| `02`–`49` | Subordinate group / elementary items (higher number = deeper). |
| `77` | Noncontiguous elementary item (a standalone working-storage variable). Begins in Area A. |
| `88` | Condition-name (a named value or range of the preceding item). |

### PICTURE clauses

`PICTURE` (or `PIC`) gives an elementary item's type and size as a *picture
string* — a tiny language of its own:

| Symbol | Meaning | Example |
|--------|---------|---------|
| `9` | A numeric digit position | `PIC 9(5)` → a 5-digit number |
| `X` | Any character | `PIC X(20)` → 20 characters |
| `A` | An alphabetic character | `PIC A(10)` |
| `V` | Implied decimal point (no stored char) | `PIC 9(3)V99` → 3 int + 2 frac |
| `S` | Operational sign | `PIC S9(4)` |
| `P` | Assumed scaling position | `PIC 9(3)PPP` |
| `(n)` | Repeat the preceding symbol `n` times | `9(5)` = `99999` |

Editing symbols (`Z`, `*`, `$`, `,`, `.`, `+`, `-`, `CR`, `DB`, `B`, `0`, `/`)
are recognised by the full language; the first implementation targets the core
data symbols (`9 X A V S P` and repetition) and treats the picture string as one
opaque token — see the lexer design.

### Figurative constants and literals

- **Figurative constants**: `ZERO`/`ZEROS`/`ZEROES`, `SPACE`/`SPACES`,
  `HIGH-VALUE`, `LOW-VALUE`, `QUOTE`, `ALL` — reserved words standing for values.
- **Numeric literals**: `42`, `-3`, `3.14` (period as decimal point).
- **Nonnumeric literals**: `"..."` or `'...'` (quoted strings).

### Reference modification

Any alphanumeric operand can be **reference-modified** to a substring:
`identifier(start:length)` selects `length` characters starting at 1-based
position `start`; `identifier(start:)` (omitted length) runs to the end of the
item. So for `WS-NAME PIC X(5) VALUE "ABCDE"`, `WS-NAME(2:3)` is `"BCD"` and
`WS-NAME(3:)` is `"CDE"`. The grammar carries it as an optional suffix on a `NAME`
operand (`NAME [ LPAREN operand COLON [ operand ] RPAREN ]`), the `:` being the
new `COLON` token.

Semantics are 1-based, so the byte range is `[start-1, start-1+length)`; an
omitted length runs to the item end (`end = item_width`). Reference modification
is supported in `DISPLAY` and alphanumeric-comparison (`IF`/`EVALUATE`) operands.

**Constant (literal) indices.** When `start` and `length` are both integer
literals, the compiler validates the range at compile time and lowers to a
constant-index `str_slice`, byte-identical to the oracle's slice. An out-of-range
constant reference modification is a compile-time reject (a later rung), never a
run-time trap.

**Computed (data-name) indices.** When `start` and/or `length` is a **data-name**
— `WS(J:K)`, `WS(J:)`, `WS(2:K)` — the index is only known at run time. The
compiler reads each index into an `i64` register (a literal becomes a `const`; a
data-name is copied from its slot) and computes `start0 = start - 1` and
`end = start0 + length` (or `end = item_width` for an omitted length) with
`sub`/`add`, feeding a run-time `str_slice(src, start0, end)`. The index item
must be an **unsigned integer** (`PIC 9…`, no `S`, no `V`); a signed, fractional,
or non-numeric index item is a later rung.

*Out-of-range rule (both engines agree).* A computed reference modification
**traps at run time** exactly when

```
start0 < 0  ||  end < start0  ||  end > item_width
```

(i.e. `start < 1`, a negative length, or the slice running past the item). This
is the *identical* predicate the emitted `str_slice` enforces in the VM/wasm
backends (`start < 0 || end < start || end > s.len()`), and the tree-walk oracle
applies the same predicate in `refmod_string`, returning `RefModOutOfRange`. So an
**in-range** computed refmod slices byte-identically on the compiler and the
oracle, and an **out-of-range** one errors on both — never a silently wrong slice.

Reference modification of a numeric item, and use in a
numeric/arithmetic/`MOVE`-source context, remain later rungs (for both constant
and computed indices).

## Layer Position

```
                  COBOL-60 source (80-column card images)
                          │
                          ▼  pre_tokenize hook: strip_cobol_columns
                  free-form COBOL text (Area A + Area B, cols 8–72)
                          │
          ┌───────────────────────────────────┐
          │  Lexer  (cobol.tokens)          │   ← PR2
          │  KEYWORD, NAME, LEVEL, PIC-STRING, │
          │  NUMBER, STRING, DOT, ( )          │
          └───────────────────────────────────┘
                          ▼ Vec<Token>
          ┌───────────────────────────────────┐
          │  Parser (cobol.grammar)         │   ← PR3+
          │  program → 4 divisions → …         │
          └───────────────────────────────────┘
                          ▼ GrammarASTNode (CST)
          ┌───────────────────────────────────┐
          │  (future) IR / interpreter        │   ← out of scope here
          └───────────────────────────────────┘
```

**Rust crates:** `code/packages/rust/cobol-lexer/` (PR2),
`code/packages/rust/cobol-parser/` (PR3+).
**Grammar files:** `code/grammars/cobol/cobol.tokens` and `cobol.grammar`.

Both crates are thin wrappers over the shared `lexer::GrammarLexer` /
`parser::GrammarParser` — nothing hand-written. The **only** COBOL-specific Rust
logic is the `strip_cobol_columns` pre-tokenize hook, which the shared lexer
already supports (`GrammarLexer::add_pre_tokenize`, a `Fn(String) -> String`).

## Lexer Design (`cobol.tokens` + column-strip hook)

### The column-strip pre-tokenize hook

Registered on the lexer via `add_pre_tokenize`, mirroring the reference
algorithm in `lexer-parser-hooks.md`. For each 80-column line:

1. Lines shorter than 8 chars → emit an empty line (nothing in the code area).
2. Read the **indicator** (column 7, index 6):
   - `*` or `/` → comment line, drop it entirely.
   - `-` → continuation: append this line's code area to the previous line.
   - otherwise → normal.
3. Keep the **code area** (columns 8–72, indices 7..71); drop 1–7 and 73–80.
4. Join with newlines.

This is the one piece of COBOL-specific imperative code. It is a pure
`String -> String` function, unit-tested independently of the grammar.

### Token inventory

| Token | Pattern / source | Examples |
|-------|------------------|----------|
| `LEVEL` | two-digit level number at item start | `01`, `02`, `77`, `88` |
| `NUMBER` | numeric literal | `42`, `-3`, `3.14` |
| `PIC_STRING` | a picture string (after `PIC`/`PICTURE`) | `9(5)`, `X(20)`, `S9(4)V99` |
| `NAME` | hyphenated data/paragraph name | `EMP-NAME`, `GROSS-PAY`, `MAIN-PARAGRAPH` |
| `KEYWORD` | reserved words | `DIVISION`, `SECTION`, `MOVE`, `PICTURE`, `TO`, … |
| `STRING` | nonnumeric literal | `"HELLO"`, `'X'` |
| `DOT` | `.` — sentence / entry terminator | |
| `LPAREN`/`RPAREN` | `(` / `)` | in picture repetition and subscripts |

**Hyphenated names** reuse the FLOW-MATIC pattern
(`[A-Za-z][A-Za-z0-9]*(-[A-Za-z0-9]+)*`). **Case** is treated as insignificant
(`case_sensitive: false` + `@case_insensitive true`), matching the uppercase-only
hardware of 1960 — the same decision made for FLOW-MATIC and Dartmouth BASIC.
Keywords normalize to uppercase; `NAME` values preserve their source case.

### PICTURE strings — the context-sensitive part

A picture string like `X(20)` begins with `X`, which is otherwise a `NAME`, so it
can only be lexed correctly *after* a `PIC`/`PICTURE` keyword. PR2 implements this
with the tokens format's declarative **mode-transition** feature (F10): a
`PIC`/`PICTURE` keyword fires `set-mode picture`, the `picture` group matches one
`PIC_STRING`, and emitting that `PIC_STRING` fires `set-mode default`. Because
`skip:` is global, the space between `PICTURE` and the picture is consumed in
`picture` mode before `PIC_STRING` matches; and because `picture` is an
*inheriting* set-mode (not a push), the default patterns remain reachable, so the
lexer can never get stuck in it.

The core picture pattern is `[9XAVSP()0-9]+` (the data symbols `9 X A V S P` plus
repetition), matched case-insensitively. Excluding the period is deliberate: a
picture therefore terminates at a space **or** at the entry-ending period, so
`PIC X(20).` lexes as `PIC_STRING("X(20)")` then `DOT`. Editing pictures (which
use `. , Z * $ …`) are future work.

An alternative post-tokenize hook (collapsing the token run after `PIC`) was
considered and rejected: reconstructing the picture from already-split tokens is
fragile at the entry-period boundary, whereas the mode transition reads the
picture as one token directly from the source.

### `DOT` vs decimal point

The period is COBOL's sentence terminator (`DOT`) but also the decimal point in a
numeric literal like `3.14`. The lexer resolves this the usual first-match way:
the `NUMBER` pattern (`-?[0-9]+(\.[0-9]+)?`) claims `3.14` as one token, so a
`DOT` is only produced for a period that is not part of a number.

## Parser Design (`cobol.grammar`) — PR3+

A sketch of the top-level productions (PEG, packrat, no left recursion):

```
program          = identification_division environment_division?
                   data_division? procedure_division ;

identification_division = "IDENTIFICATION" "DIVISION" DOT
                          "PROGRAM-ID" DOT NAME DOT { id_paragraph } ;

environment_division    = "ENVIRONMENT" "DIVISION" DOT { env_section } ;

data_division    = "DATA" "DIVISION" DOT { data_section } ;
data_entry       = LEVEL NAME { data_clause } DOT ;
data_clause      = picture_clause | value_clause | ... ;
picture_clause   = ( "PICTURE" | "PIC" ) PIC_STRING ;

procedure_division = "PROCEDURE" "DIVISION" DOT { paragraph } ;
paragraph          = NAME DOT { sentence } ;
sentence           = { statement } DOT ;
statement          = move_stmt | add_stmt | display_stmt | perform_stmt
                   | if_stmt | goto_stmt | evaluate_stmt | string_stmt
                   | stop_stmt | … ;
move_stmt          = "MOVE" operand "TO" NAME { NAME } ;
add_stmt           = "ADD" operand { operand } "TO" NAME [ "GIVING" NAME ] ;
string_stmt        = "STRING" operand { operand } "DELIMITED" "BY" string_delim
                     "INTO" NAME [ "WITH" "POINTER" NAME ]
                     [ "ON" "OVERFLOW" { statement } ]
                     [ "NOT" "ON" "OVERFLOW" { statement } ] [ "END-STRING" ] ;
string_delim       = "SIZE" | operand ;
unstring_stmt      = "UNSTRING" operand "DELIMITED" "BY" operand
                     "INTO" NAME { NAME } [ "WITH" "POINTER" NAME ]
                     [ "ON" "OVERFLOW" { statement } ]
                     [ "NOT" "ON" "OVERFLOW" { statement } ] [ "END-UNSTRING" ] ;
inspect_stmt       = "INSPECT" operand
                     ( inspect_tallying [ inspect_replacing ]
                     | inspect_replacing
                     | inspect_converting )
                     [ "END-INSPECT" ] ;
inspect_tallying   = "TALLYING" tally_for { tally_for } ;
tally_for          = NAME "FOR" tally_item { tally_item } ;
tally_item         = ( "ALL" | "LEADING" ) operand { inspect_region }
                   | "CHARACTERS" { inspect_region } ;
inspect_replacing  = "REPLACING" replace_item { replace_item } ;
replace_item       = "CHARACTERS" "BY" operand { inspect_region }
                   | ( "ALL" | "LEADING" ) operand "BY" operand { inspect_region } ;
inspect_converting = "CONVERTING" operand "TO" operand { inspect_region } ;
inspect_region     = ( "BEFORE" | "AFTER" ) operand ;
stop_stmt          = "STOP" ( "RUN" | NUMBER ) ;
```

### `STRING` (data-name reference note)

`STRING` is a reserved verb *and* the type name of the quoted string-literal
token. They coexist because keyword promotion only rewrites bare `NAME` words,
while a quoted `"…"`/`'…'` always lexes as the literal token regardless of the
keyword list. The **first rung** implements `STRING s… DELIMITED BY SIZE INTO t`:
each sending field (an alphanumeric item or a string/numeric literal) is taken in
FULL (`DELIMITED BY SIZE`), the pieces are concatenated left-to-right, and the
result is stored LEFT-JUSTIFIED into the alphanumeric receiver `t`, truncated at
`t`'s width. Per ANSI-85, STRING writes only what it produced and **does not
space-fill** the untouched tail of `t` (unlike `MOVE`) — the receiver's trailing
bytes keep their prior content. The grammar also *accepts* a real
(identifier/literal) delimiter, `WITH POINTER`, and `ON`/`NOT ON OVERFLOW` so the
reader can reject them as a clean "later rung" error rather than a parse failure.

### `UNSTRING` (first rung)

`UNSTRING` is the inverse of `STRING`: it takes one alphanumeric source apart on a
delimiter into several receivers. The **first rung** implements `UNSTRING source
DELIMITED BY delim INTO r1 [r2 …]`, where `delim` is a **single-character**
delimiter — either a 1-character string literal (`","`, `" "`) or a `PIC X(1)`
item. The source is scanned left-to-right and split into delimited fields; each
field is moved into the next receiver as an ordinary alphanumeric `MOVE`
(left-justified, space-padded, truncated). The exact semantics (oracle = source of
truth, compiler byte-identical):

- Each receiver **including the last** takes the field up to the NEXT delimiter (or
  end-of-source) — the last receiver does *not* absorb the remainder. Fields beyond
  the receiver count are dropped (that would be `ON OVERFLOW`, a later rung).
- Consecutive or leading delimiters bound an EMPTY field → the receiver gets all
  spaces.
- When the source is exhausted (a field ran to end-of-source with no trailing
  delimiter), the remaining receivers are **left unchanged** (they keep their prior
  `VALUE`) — *not* space-filled. A trailing delimiter still yields one final empty
  field.

Worked (delimiter `,`): `"A,B,C" INTO R1 R2 R3` (each `PIC X(3)`) →
`R1="A  " R2="B  " R3="C  "`; `"A,B,C,D" INTO R1 R2 R3` drops `"D"`; `"A,B" INTO
R1 R2 R3` (R3 `VALUE "ZZZ"`) leaves `R3="ZZZ"`; `"A,,C" INTO R1 R2 R3` →
`R2="   "`; `",X" INTO R1 R2` → `R1="   " R2="X  "`.

The grammar takes a single `operand` for the delimiter and `{ NAME }` receivers;
the reader/compiler enforce the 1-character restriction and reject a
multi-character / `ALL` / `OR` delimiter, `WITH POINTER`, `ON`/`NOT ON OVERFLOW`,
and a numeric/group source or receiver as clean "later rung" errors. Because the
delimiter position is data-dependent, the compiler lowers `UNSTRING` to a run-time
**scan loop** (`str_len` + `str_index`/`cmp` to find each delimiter, then a
`str_slice`/`str_concat` reshape into each receiver), whereas `STRING`'s boundaries
were all compile-time constants.

### `INSPECT … TALLYING` (first rung)

`INSPECT` scans an alphanumeric item to either **count** characters (`TALLYING`)
or **substitute** them (`REPLACING`). The **first rung** implements the counting
form `INSPECT source TALLYING counter FOR ALL delim`:

- `source` is an alphanumeric (`PIC X`) data item.
- `counter` is an **unsigned integer** numeric item (`PIC 9(n)`, scale 0). INSPECT
  **ADDs** to the counter — it does **not** clear it first — so the effect is
  `counter := counter + occurrences`. (This is the standard's rule and the common
  source of the "why is my count too high?" bug.)
- `delim` is a **single-character** delimiter — a 1-character string literal
  (`"A"`) or a `PIC X(1)` item. `ALL` means every (non-overlapping, left-to-right)
  occurrence is counted; **`LEADING`** counts only the run of **consecutive**
  delimiters at the START of the source, stopping at the first non-`delim`
  character.
- For `ALL` the count is the number of positions `j` where `source[j] == delim`;
  for `LEADING` it is the length of the leading prefix of positions all equal to
  `delim` (single-byte ASCII compare — the same char/byte assumption
  `STRING`/`UNSTRING` use).

Worked (delimiter `"A"`, `counter` `PIC 9(3)`): `"BANANA"` FOR ALL → three A's →
counter `0 → 3`; a counter starting at `5` over `"MISSISSIPPI"` counting `"S"` →
`5 + 4 = 9` (proving ADD, not replace); `"HELLO"` counting `"Z"` → `0` (unchanged).
FOR LEADING (delimiter `"0"`): `"000123"` → `3` (three leading zeros, stop at
`'1'`); `"120003"` → `0` (first char is `'1'`, so the run is empty — whereas FOR
ALL on the same source is `3`); `"0000"` → `4`.

The count folds into the counter through the **same numeric-store path the
arithmetic verbs use** (COBOL's silent high-order truncation on overflow), so the
oracle (`store_result(counter, counter + count)`) and the compiler
(a `str_len` + `str_index`/`cmp_eq` **count loop**, then `store_scaled`) agree
byte-for-byte. FOR ALL and FOR LEADING share that identical loop — the ONLY
difference is the not-equal branch: FOR ALL skips just the increment and keeps
scanning, FOR LEADING breaks out of the loop (oracle: `filter…count` vs
`take_while…count`). The grammar deliberately accepts the fuller `INSPECT`
surface — a `CHARACTERS` tally, `BEFORE`/`AFTER` regions, several `TALLYING`
counters or `FOR` phrases — so the reader/compiler reject each as a clean
"later rung" error rather than a parse failure. A multi-character / figurative /
numeric / wider-than-one delimiter, and a numeric/group source or a
non-integer/signed/non-numeric counter, are likewise clean later rungs. (`FIRST`
and `INITIAL`, needed only by `REPLACING FIRST` / `BEFORE INITIAL`, are left
unreserved so common data names keep working.)

### `INSPECT … REPLACING` (first rung)

The **substitution** form's first rung implements a LONE
`INSPECT source REPLACING ALL x BY y`:

- `source` is an alphanumeric (`PIC X`) item, modified **in place**. Because
  both `x` and `y` are single characters, the result has the **same width** as
  the source — this is a straight **per-position map**.
- `x` (the search) and `y` (the replacement) are each a **single character** — a
  1-character string literal (`"A"`) or a `PIC X(1)` item — reusing the same
  single-character helpers as `TALLYING`/`UNSTRING`.
- Semantics: `source := source with each x → y`, left to right. Every position
  `j` where `source[j] == x` becomes `y`; all others are unchanged.

Worked (search `"A"`, replacement `"X"`): `"ABABA"` → `"XBXBX"`; a search that
never occurs (`"Z"` in `"HELLO"`) leaves the source unchanged; `"AAAA"` with
`A → X` → `"XXXX"`.

The oracle rebuilds the string (`source.chars().map(|c| if c == x { y } else
{ c }`)) and stores it back through the **same alphanumeric char-store path** a
`MOVE` uses; the compiler **unrolls** the per-position map over the
compile-time-known width `W` (`str_index`/`cmp_eq` per byte, splicing either the
replacement or the original character with `str_slice`/`str_concat`), then copies
the `W`-wide result into the source register. The two agree byte-for-byte.

Deferred as clean later rungs (accepted by the grammar, rejected at read/compile
time): `REPLACING CHARACTERS BY`, `REPLACING LEADING`/`FIRST`, `BEFORE`/`AFTER`
regions, **several** replace items, a multi-character / figurative / wider /
numeric search or replacement, and a numeric/group source.

### Combined `INSPECT … TALLYING … REPLACING` (one statement)

A single `INSPECT` may carry **both** phrases:
`INSPECT source TALLYING counter FOR ALL delim REPLACING ALL x BY y`. Per ISO the
statement executes "**as though an `INSPECT TALLYING` were specified, followed by
an `INSPECT REPLACING`**" — so the order is fixed:

1. **Tally first** — count occurrences of `delim` in the **ORIGINAL** `source` and
   **ADD** them to `counter` (the tally does not modify the source).
2. **Then replace** — substitute every `x` with `y` in the source.

This tally-before-replace order is observable when `delim == x`: the count must
see every occurrence in the pre-replacement bytes. Worked (`delim == x == "S"`):
`"MISSISSIPPI"` TALLYING `S` → `counter += 4`, then `S → Z` → `"MIZZIZZIPPI"`. Had
the count run after the replace it would have seen zero `S` — so the `4` proves
the ordering.

Both the oracle and the compiler compose their two existing single-phrase
lowerings in this exact order on the same source, so the compiled program and the
reference agree byte-for-byte. No grammar change was needed — the grammar already
accepted `inspect_tallying [ inspect_replacing ]`; only the two prior "combined is
a later rung" rejects were removed. Each phrase is still restricted to its
single-character `FOR ALL`/`ALL … BY` form: a combined statement whose `TALLYING`
or `REPLACING` half is itself a deferred sub-form (`LEADING`/`CHARACTERS`, several
counters/FOR/replace items, `BEFORE`/`AFTER`, multi-char/figurative/wider/numeric
operands, a numeric/group source, or a non-integer counter) remains a clean later
rung.

### `INSPECT … CONVERTING` (first rung)

`INSPECT` has a third, distinct form that neither counts nor does a search-and-
replace, but **translates** each character through a table:
`INSPECT source CONVERTING from TO to`.

- `source` is the alphanumeric (`PIC X`) item, **modified in place**.
- `from` and `to` are **string literals of EQUAL length** (1..N characters). This
  first rung supports **string literals only** for `from`/`to`; a `PIC X` item as
  the table (a *variable* table) is a later rung.

Semantics — a per-character **translation table**. For each character of `source`,
if it equals the character of `from` at some index `k`, it is replaced by the
character of `to` at that same index `k`; if it matches no character of `from`, it
is left unchanged. When `from` contains a character more than once the **FIRST
(leftmost) occurrence wins** — that `k` supplies the replacement. The map is
length-preserving (each character maps to exactly one), so the source keeps its
width.

Worked examples:

- `CONVERTING "ABC" TO "XYZ"` maps `A→X`, `B→Y`, `C→Z`; everything else unchanged.
  `"CAB"` → `"ZXY"`.
- `CONVERTING "AEIOU" TO "12345"` maps `A→1, E→2, I→3, O→4, U→5`. `"BEAN"` →
  `B, 2, 1, N` = `"B21N"` (B and N are in no entry, so they pass through).
- `CONVERTING "AAB" TO "XYZ"` — `from` repeats `A`, and the **leftmost** entry
  wins, so `A→X` (not the later `A→Y`) and `B→Z`: `"AAB"` → `"XXZ"`.

The oracle builds the char→char map (first occurrence winning) and maps each source
character through it, storing back through the alphanumeric char path. The compiler
UNROLLS over the compile-time-known source width `W`: at each position it reads the
source byte once and runs a **first-match-wins** chain over the compile-time table
(each `from[k]` a `const` compare byte, each `to[k]` a 1-character `str_const`),
splicing the earliest matching `to[k]` — or the original character — onto a
`str_concat` accumulator, then copies the `W`-wide result back (only after the last
read). The two agree byte-for-byte.

`CONVERTING` is a **standalone** alternative — it is never combined with
`TALLYING`/`REPLACING` in one statement (a combined form does not parse). Later
rungs (clean `Unsupported`): an **unequal-length** (or non-ASCII) `from`/`to` pair,
a `PIC X` **item** / figurative / reference-modified `from`/`to`, a `BEFORE`/`AFTER`
region, and a numeric/group source. The trailing `{ inspect_region }` in the
grammar lets a region-restricted `CONVERTING` parse so it rejects cleanly rather
than failing to parse.

Grammar scope tracks the lexer scope below.

### `MOVE` (cross-category: numeric → alphanumeric — unsigned or signed)

The earlier rungs implement **same-category** `MOVE`: numeric → numeric (rescale
the implied decimal point, truncating), and alphanumeric → alphanumeric
(left-justify, space-pad or truncate on the right). The **first cross-category**
rung is `MOVE numeric-item TO alphanumeric-item`, restricted to an **unsigned
integer** sending item (`PIC 9(n)` — no `S`, no `V`).

COBOL's rule: a numeric sending item moved to an alphanumeric receiver is treated
**as though it were an alphanumeric item holding its digit characters**, then
moved by the alphanumeric rules. The digit characters are the item's `n`-digit,
**zero-padded magnitude** — exactly the image a `DISPLAY` of the same `PIC 9(n)`
prints. So the move is: build that `n`-character digit string, then LEFT-justify
it into the receiver, space-padding the right when the receiver is wider and
truncating on the right when it is narrower.

Worked (`N` is `PIC 9(3)` holding `42`, so its digit image is `"042"`):

- `N → PIC X(3)` (exact fit) → `"042"`.
- `N → PIC X(5)` (wider) → `"042  "` (two trailing spaces).
- `N → PIC X(2)` (narrower) → `"04"` (right-truncated).

The oracle resolves the numeric item to its `Decimal`, takes `Decimal::digits()`
(the `int`+`frac` characters — for an unsigned integer, exactly the `n` zero-padded
digits), and stores it through the **same `move_into_char`** left-justify/pad path
a same-category alphanumeric `MOVE` uses. The compiler builds the `n`-character
digit string at run time from the numeric slot — for each digit position it takes
`(slot / 10^k) % 10` and slices that digit out of a constant `"0123456789"` table
(`str_slice [d, d+1)`), concatenating the `n` one-character pieces — then feeds the
string through the **same `str_slice`/`str_concat` char reshape** an
alphanumeric-item `MOVE` emits. Because both engines run the digit image through
one shared alphanumeric-receiver rule, the stored bytes agree byte-for-byte.

A **scaled** (unsigned `PIC 9(i)V9(d)`, `d > 0`) source is also supported: its
digit image is the `(i+d)`-digit zero-padded magnitude — the integer digits
followed by the fractional digits, concatenated with **no decimal point** (the
scaled slot already holds `value·10^d`, and the oracle's `Decimal::digits()` =
`int + frac` defines it). So `PIC 9(2)V9 = 4.2 → "042"`, `PIC 9(1)V99 = 3.14 →
"314"`; the same left-justify / space-pad / truncate reshape into the receiver
then applies. The mixed **comparison** with an alphanumeric operand uses the same
image (`9(2)V9 = 4.2` compares equal to `"042"`).

A **signed** (`PIC S9(i)V9(d)`, integer or scaled) source is also supported: its
image is the `(i+d)`-digit zero-padded **magnitude** with the operational sign
folded into a **trailing overpunch** on the units (last) digit — the same
zoned-decimal encoding a `DISPLAY` of the same signed field produces. The units
digit `u` maps positive `{ A B C D E F G H I` and negative `} J K L M N O P Q R`.
So `S9(3) = +123 → "12C"`, `= -123 → "12L"`, and `S9V9 = -4.2 → "4K"` (magnitude
`"42"`, units `2` overpunched negative → `'K'`); the same left-justify / space-pad
/ truncate reshape into the receiver then applies (`S9(3) = +123 → X(5)` is
`"12C  "`, `→ X(2)` is `"12"`). The overpunch is driven by the *item* being signed,
not by the value's sign — a signed **positive** source still takes the positive
`{…I` row, which is exactly why an unsigned `PIC 9(3) = 123` (`"123"`) and a signed
`PIC S9(3) = +123` (`"12C"`) differ. The oracle reuses `overpunch_trailing(storage,
neg)` (the very helper `DISPLAY` uses) before the shared `move_into_char`; the
compiler reproduces the identical byte by computing `neg = slot < 0`,
`units = |slot| % 10`, and slicing ONE combined table `"{ABCDEFGHI}JKLMNOPQR"` at
`units + neg*10` (positive row at indices `0..=9`, negative at `10..=19`), then
replacing the magnitude image's last character with it — so both engines still
agree byte-for-byte.

Deferred as clean later rungs (rejected identically on both engines, never wrong
output): an **edited** (`PIC $,ZZ9.99`) numeric source, a **numeric-edited**
receiver, a `SIGN` clause with `SEPARATE`/`LEADING`, an alphanumeric → **signed**
numeric MOVE (the reverse direction), and a **group** item on either side (a group
receiver is rejected as "MOVE into a group item").

### `MOVE` (cross-category: alphanumeric → unsigned numeric — integer or scaled)

The **reverse** cross-category rung is `MOVE alphanumeric-item TO numeric-item`,
for an alphanumeric source (`PIC X(m)`) into an **unsigned** numeric receiver
`PIC 9(i)V9(d)` — no `S`; `d` may be `0` (an integer receiver) or `> 0` (a
**scaled** receiver).

COBOL's rule: the alphanumeric source is treated as an **unsigned integer** `V`
formed from its characters read as digits (`V = V*10 + (char - '0')`,
left-to-right), and **that fold IS the receiver's scaled-slot magnitude
directly** — it fills the receiver's `(i + d)` digit positions **right-justified**,
with the implied point sitting `d` places from the right. So the slot keeps the
**low-order `(i + d)` digits**, left-zero-padded when the source has fewer than
`i + d` digits and high-order-truncated when it has more, i.e.

```text
slot = V mod 10^(i+d)   (rendered with the implied point d places from the right)
```

This is **NOT** the arithmetic decimal-align rule: `V` is *not* multiplied by
`10^d`. The fold already lands at scale `d`.

Worked (integer receiver — `d = 0` — is the special case):

- `PIC X(3)="042"` → `PIC 9(3)` → `42` (`DISPLAY "042"`; the integer special case).
- `PIC X(2)="05"` → `PIC 9(4)` → `0005` (source shorter → left-zero-padded).
- `PIC X(5)="12345"` → `PIC 9(3)` → `345` (source longer → high-order-truncated).
- `PIC X(3)="042"` → `PIC 9(2)V9` → slot `042`, reads **4.2**.
- `PIC X(2)="42"` → `PIC 9(2)V9` → slot `042`, reads **4.2** (left-zero-padded).
- `PIC X(5)="12345"` → `PIC 9(2)V9` → slot `345`, reads **34.5** (high-order trunc).
- `PIC X(1)="5"` → `PIC 9(1)V99` → slot `005`, reads **0.05**.

Both engines compute `V` by the **identical per-character arithmetic**
`V = V*10 + (char_byte - '0')` over the `m` source characters, so they always agree
byte-for-byte. The compiler folds the `m` bytes at run time —
`d = str_index(src,k) - '0'`, `value = value*10 + d` (`mul`/`add`/`sub`/`const`
over `i64`) — then stores `value` through the **same numeric-store helper**
`store_scaled` a numeric `MOVE`/`COMPUTE` uses, handing it the **receiver's own
scale `d`** as the value scale. Because the fold already IS the slot magnitude at
scale `d`, `store_scaled` rescales `d → d` (a no-op — no shift) and keeps the
low-order `(i + d)` digits (`mag mod 10^(i+d)`) = `V mod 10^(i+d)`. (Passing scale
`0` would wrongly up-shift by `10^d`.) The oracle mirrors this exactly: it folds
the identical arithmetic, then builds a `Decimal` that inserts the point `d` places
from the right (`int` = the magnitude's digits above the last `d`, empty → `"0"`;
`frac` = its last `d` digits, left-zero-padded to `d`) and stores it via
`move_into_numeric(int_digits = i, dec_digits = d)`, which keeps the low-order `i`
integer and high-order `d` fractional digits — the same `V mod 10^(i+d)`. For
`d = 0` this is exactly the old integer-receiver behaviour (`int = V_str`,
`frac = ""`).

**All-digit scope / non-digit characters.** This rung scopes to an **all-digit**
source. A non-digit byte is *not* rejected: the same `(byte - '0')` arithmetic runs
on both engines (defined-but-unspecified, but **identical on both by
construction**), and every test uses an all-digit source. This choice — over a
runtime reject — keeps the oracle and compiler provably identical, because the
compiled path has no clean way to raise a runtime error for a non-digit that the
oracle could mirror byte-for-byte. A SPACE source byte (below `'0'`) makes the fold
go negative, but an unsigned `PIC 9` field keeps the **magnitude** on both engines
(the compiler `abs`es before `mod`; the oracle uses `unsigned_abs`) — no stray
`'-'`.

Deferred as clean later rungs (rejected identically on both engines): a **signed**
(`PIC S9`) or **edited** numeric receiver, a **group** item on either side, and a
**source wider than 18 characters** (whose `i64` fold could overflow — an all-digit
source of ≤ 18 chars stays below `10^18 < i64::MAX`).

### Comparison (numeric ↔ alphanumeric)

The earlier condition rungs implement **same-category** comparison: a numeric
relation (`IF A = B`, both numeric) compares the operands by **value** on their
scaled integers, and an alphanumeric relation (both character) compares them by
the **byte rule** — the shorter operand is space-padded on the right to the
longer's length and the two are compared byte-by-byte. This rung adds the
**mixed** relation: a relational condition (in `IF` / `EVALUATE` / any condition
context) comparing a **numeric item** operand (`PIC [S]9(i)V9(d)` — **unsigned or
signed**, integer or scaled) with an **alphanumeric** operand (a `PIC X` item **or**
a string literal), with the numeric operand on either side.

COBOL's rule: **when a numeric and a non-numeric operand are compared, the numeric
operand is treated as though it were moved to an alphanumeric field** — i.e. by
its **digit image**, the `n`-digit zero-padded magnitude (exactly the
numeric→alphanumeric `MOVE` image above, and the same digits a `DISPLAY` of the
same `PIC 9(n)` prints) — and the comparison then proceeds by the **alphanumeric
(byte) rule**: space-pad the shorter operand on the right to the longer's length,
then compare byte-by-byte.

Worked (`NUM` is `PIC 9(3)` holding `42`, so its digit image is `"042"`):

- `IF NUM = "042"` → `"042"` vs `"042"` → **equal** (true).
- `IF NUM = "42"` → `"042"` vs `"42 "` (the shorter literal space-padded) →
  differ at the first byte (`'0'` ≠ `'4'`) → **not equal** (false). A value
  comparison would wrongly call these equal — this pins the byte rule.
- `IF NUM > "040"` → `"042"` vs `"040"` → `'2' > '0'` at the last position →
  **greater** (true).
- `IF "042" = NUM` (numeric on the right) lowers identically → **equal**.
- `IF NUM = W` where `W` is `PIC X(3) = "042"` (the alphanumeric side is an item,
  not a literal) → **equal**.

A **scaled** unsigned operand (`PIC 9(i)V9(d)`, `d > 0`) uses its `(i + d)`-digit
image — integer part then fractional part, no decimal point — so `PIC 9(2)V9 = 4.2`
compares by `"042"` (`IF F = "042"` is true, `IF F > "040"` is true).

A **signed** operand (`PIC S9(i)V9(d)`, integer or scaled) uses that same `(i + d)`-
digit magnitude image with the operational sign folded into a **trailing overpunch**
on the units (last) digit — the *same* image the signed numeric → alphanumeric
`MOVE` produces (`overpunch_trailing`): the units digit `u` maps positive
`{ A B C D E F G H I`, negative `} J K L M N O P Q R`. So `PIC S9(3) = -123` compares
**equal** to `"12L"`, `= +123` equal to `"12C"`, and a scaled `PIC S9V9 = -4.2` equal
to `"4K"`; ordering follows the byte comparison of those images. The overpunch is
driven by the *item* being signed — a signed **positive** operand still takes the
positive `{…I` row, which is why an unsigned `PIC 9(3) = 123` (`"123"`) and a signed
`PIC S9(3) = +123` (`"12C"`) compare differently. A value that truncates to a zero
magnitude stores `neg = false` (COBOL has no negative zero), so its image is `"00{"`
on both engines.

The oracle's `compare_operands` yields the numeric operand's characters — for an
unsigned item its `Decimal::digits()` (fixed-width zero-padded storage), for a
**signed** item its `overpunch_trailing(storage, neg)` — and falls into the same
alphanumeric arm (space-pad to the common length, byte-compare) a character relation
uses. The compiler builds the numeric side's image at run time with the **same
helper** the numeric→alphanumeric `MOVE` uses (`emit_num_digit_string` for the
magnitude, `emit_signed_num_alpha_image` for the signed overpunch), then feeds
**both** operands through the **same `str_cmp` / space-pad path** an alphanumeric
relation emits. Because both engines build the identical image and run the identical
byte comparison, a mixed relation evaluates byte-for-byte the same on both.
`EVALUATE`'s subject-vs-`WHEN` comparison reuses `compare_operands`, so the oracle
applies the same rule there for free. The **compiler** now matches this: each
`EVALUATE` subject-vs-`WHEN`-value comparison is routed through the *same* relation
dispatch an `IF subject <relop> value` uses (`emit_operand_relation`, factored out of
`emit_relation`) — a single value is `cmp_eq`, a `THRU` range is
`and(cmp_ge, cmp_le)`, and the value-list `OR`-folds. So a mixed numeric↔alphanumeric
subject/`WHEN` (numeric subject vs alphanumeric `WHEN`, or the reverse; single value
or `THRU`) compiles and is byte-identical to the oracle, inheriting the full category
dispatch — unsigned, **signed** (overpunch image), and **scaled** numeric sides,
figuratives, and the `ZERO`-numeric routing (`EVALUATE N WHEN ZERO` stays a numeric
comparison) — and the same deferral set as `IF`.

Deferred as clean later rungs (rejected on both engines, so they agree on the
deferral): an **edited** numeric operand in a mixed comparison, a **group** item on
either side, and a **numeric-literal-vs-alphanumeric** pairing (a different pairing,
kept out of scope — the numeric side must be an item on this rung; the compiler
rejects it in `num_digit_str_operand`, and the oracle's `compare_operands` rejects a
numeric-literal operand in a mixed comparison so the two agree). Because `EVALUATE`
now reuses the relation dispatch, this deferral set applies identically to an
`EVALUATE` `WHEN` value: e.g. a numeric-literal `WHEN` against an alphanumeric
subject is a clean reject on both engines.

**Figurative-vs-figurative comparison.** Comparing two figurative constants
(`IF ZERO = ZERO`, `IF ZERO = SPACE`, `IF SPACE < ZERO`) resolves each figurative to
a single fill character, since neither has an operand length to borrow: the oracle's
`src_chars` of a figurative is empty, so both `fill_fig` to `len().max(1)` = 1
(`ZERO` → `"0"`, `SPACE` → `" "`), then byte-compares. The compiler's
`emit_str_condition` takes width 1 for the two-figurative (`None, None`) case instead
of rejecting it, so `ZERO = ZERO` / `SPACE = SPACE` are true, `ZERO ≠ SPACE`, and by
byte value (`'0'` = 0x30 > `' '` = 0x20) `ZERO > SPACE` / `SPACE < ZERO` — byte-for-byte
identical on both engines. (This closed a reject-vs-answer gate divergence: the oracle
already answered these while the compiler rejected them.)

## Scope

This spec's implementation stops at the **frontend** (lex + parse to a CST). No
IR, no interpreter. The goal is to stand up COBOL's structural machinery —
column format, four divisions, level/PICTURE data description, English-verb
procedure — reusing the FLOW-MATIC frontend and the shared hook API.

The first cut targets the **demonstrated language**: a complete four-division
program with `WORKING-STORAGE` items (levels, core PICTUREs, `VALUE`), a handful
of the most common verbs (`MOVE`, `ADD`/`SUBTRACT`/`MULTIPLY`/`DIVIDE …
GIVING`, `COMPUTE … [ROUNDED] = <expr> [ON SIZE ERROR …]`, `DISPLAY`, `PERFORM`,
`GO TO`, `IF`, `STOP RUN`), and the
column-strip/continuation/comment handling. `COMPUTE` brings the first
operator-symbol grammar: a precedence-layered arithmetic expression
(`+ - * / **`, unary sign, parentheses) that the grammar encodes as a
cascade of rules (`arith_expr` → `arith_term` → `arith_factor` → `arith_unary`
→ `arith_primary`), so precedence and grouping live in the parse tree rather
than in later code. The long tail (full ENVIRONMENT/FILE
sections, `REDEFINES`/`OCCURS`, editing PICTUREs, the complete ~300-word reserved
list, `COPY`) is documented as future work.

## Implementation Roadmap (small PRs)

1. **PR1 — this spec (`PL07`).** Specs-first, committed before code.
2. **PR2 — lexer.** `cobol.tokens` + `cobol-lexer` crate: the
   `strip_cobol_columns` pre-tokenize hook, the token inventory above, PICTURE
   handling, and tests over a real fixed-format program.
3. **PR3 — parser (structure).** `cobol.grammar` + `cobol-parser`: the four
   division headers, DATA DIVISION entries, and PROCEDURE paragraphs/sentences.
4. **PR4+ — procedure verbs & clauses.** Flesh out statements and data clauses,
   widen the reserved-word set, and grow the tested program corpus.

## Test Strategy

### Column-strip hook (PR2)
- Sequence numbers (1–6) and identification (73–80) are dropped.
- `*`/`/` comment lines are removed; a `-` continuation line splices onto its
  predecessor.
- A round-trip: a carded program strips to the expected free-form text.

### Lexer (PR2)
- Level numbers `01`/`77` vs `02`–`49`/`88` lex as `LEVEL`.
- Hyphenated names lex as one `NAME`; reserved words as `KEYWORD`.
- `PIC X(20)` / `PIC 9(3)V99` yield one `PIC_STRING` each.
- `3.14` is one `NUMBER`; a sentence-ending `.` is a `DOT`.
- Quoted literals (`"..."`, `'...'`) lex as `STRING`.
- A full four-division program tokenizes without error.

### Parser (PR3+)
- Each division header parses; a `WORKING-STORAGE` hierarchy nests by level.
- A `PROCEDURE` paragraph of sentences parses; each verb's statement shape is
  recognised.
- The full demonstrated program parses end to end.

## Future Extensions

| Feature | Notes |
|---------|-------|
| Area A / Area B enforcement | Column-aware positioning of headers and levels |
| Full ENVIRONMENT / FILE SECTION | `SELECT`/`ASSIGN`, `FD` record descriptions |
| `REDEFINES`, `OCCURS`, `88` conditions | Richer data description |
| Editing PICTUREs | `Z * $ , + - CR DB` and floating insertion |
| Complete reserved-word list | The full ~300 COBOL-60 reserved words |
| `COPY` library text | A pre-tokenize include-style hook |
| `STRING` real delimiters / `WITH POINTER` / `ON OVERFLOW` | Later rungs beyond the first `DELIMITED BY SIZE` cut (need a run-time scan and a receiver pointer) |
| `UNSTRING` multi-char / `ALL` / `OR` delimiters, `WITH POINTER`, `ON OVERFLOW`, multiple `DELIMITED` fields, `COUNT`/`DELIMITER IN`/`TALLYING` | Later rungs beyond the first single-character `DELIMITED BY delim INTO r1 [r2 …]` cut |
| `INSPECT`, other string verbs | The rest of the string-handling verb family |
| IR / interpreter | Run a COBOL program; out of scope for the frontend |

[PL06]: PL06-flow-matic.md
