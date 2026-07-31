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
is supported in `DISPLAY`, alphanumeric-comparison (`IF`/`EVALUATE`) operands, and
as a **MOVE source** into an **alphanumeric** receiver (`MOVE base(start:length) TO
dst`).

**As a MOVE source.** A reference-modified source moved into an alphanumeric
receiver reshapes the slice to the receiver's width by the ordinary alphanumeric
char rule — LEFT-justified, space-padded on the right when the receiver is wider
than the slice, truncated on the right when narrower — exactly as a same-category
alphanumeric MOVE reshapes. The slice comes from the SAME helper `DISPLAY` and
comparison use (the oracle's `refmod_string`, the compiler's `ref_mod_slice`), so a
MOVE of a slice and a `DISPLAY` of the same slice agree byte-for-byte. Constant
`SRC(2:3)`, omitted-length `SRC(3:)`, and computed `SRC(J:K)` indices are all
supported, into one or more receivers (`MOVE SRC(1:3) TO A B`). A **numeric**
receiver (de-editing a slice into a numeric field) remains a later rung, rejected
on both engines. The compiler slices by BYTE offset and the oracle by CHAR index;
on the ASCII-prefix windows this targets they coincide, so accepted programs emit
byte-identical output (a multi-byte char inside or after the window is the
pre-existing refmod byte-vs-char chip, shared with `DISPLAY`/comparison, not new to
the MOVE-source path).

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

Reference modification of a numeric item, and use in a numeric/arithmetic context
(including a `MOVE` into a **numeric** receiver), remain later rungs (for both
constant and computed indices). A `MOVE` of a reference-modified source into an
**alphanumeric** receiver is supported (see "As a MOVE source" above).

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
each sending field (an alphanumeric item, a string/numeric literal, or — see the
figurative rung below — a figurative constant SPACE/ZERO) is taken in
FULL (`DELIMITED BY SIZE`), the pieces are concatenated left-to-right, and the
result is stored LEFT-JUSTIFIED into the alphanumeric receiver `t`, truncated at
`t`'s width. Per ANSI-85, STRING writes only what it produced and **does not
space-fill** the untouched tail of `t` (unlike `MOVE`) — the receiver's trailing
bytes keep their prior content.

**Delimiter rung (now implemented).** `STRING s… DELIMITED BY delim INTO t`, where
`delim` is a **single-character ASCII** delimiter (a 1-char literal, a `PIC X(1)`
item, a **figurative constant** SPACE/ZERO taken as its single ASCII character —
SPACE→`" "` (0x20), ZERO→`"0"` (0x30), reducing to the single-char literal path — or
a **CONSTANT reference-modified** slice `base(start:len)` of length 1 (see the
constant-refmod delimiter/search/replace rung below; a computed `base(J:K)` index
stays a later rung) — all reduced by the same `single_delim_char`/`single_delim_code`
helper `UNSTRING` uses), is now supported. Each sending field contributes only its PREFIX up to (but
not including) the FIRST occurrence of the delimiter char in that field; a field
with no delimiter contributes its whole image, and a field starting with the
delimiter contributes the empty string. The prefixes are concatenated and overlaid
onto `t` EXACTLY as `DELIMITED BY SIZE` does (leftmost `min(len, width)`, no tail
space-fill). Example: `STRING "ab,cd" "ef" "gh,ij" DELIMITED BY "," INTO t` →
`"abefgh"`. `Stmt::String` carries a `delim: Option<Operand>` (`None` = `SIZE`); the
oracle truncates each field by CHARACTER while the compiler emits a byte-based
per-field scan loop, so both must be ASCII: a **non-ASCII** literal delimiter and a
non-ASCII string-LITERAL sending field WHEN a delimiter is active are clean "later
rung" rejects on BOTH engines (byte-vs-char). A non-ASCII `PIC X(1)` delimiter ITEM
is not build-time detectable on the compiler and — as with `UNSTRING` — is left as
the shared byte-vs-char chip rather than a one-sided reject, keeping the accept/
reject sets co-total.

**`WITH POINTER p` rung (now implemented).** `STRING s1 s2 … DELIMITED BY {SIZE |
delim} INTO t WITH POINTER p` — `p` is an **unsigned-integer** item (`PIC 9(n)`,
`n ≤ 18`) holding the **1-based** character position in the RECEIVER at which the
first transferred character is placed. No grammar change was needed (the grammar
already parses `WITH POINTER NAME`); the reader/compiler take the receiver as the
first NAME (`INTO t` precedes the phrase) and the pointer as the first NAME after
the `POINTER` keyword. This directly mirrors the `UNSTRING … WITH POINTER` rung. Two
things change, and the concatenation of the sending fields is UNCHANGED:

- **Overlay offset.** The concatenation is overlaid starting at 0-based index
  `p − 1` (instead of 0), placing `chars_placed = min(concat_len, size − (p−1))`
  characters. Receiver positions BEFORE `p−1` and AFTER `(p−1) + chars_placed` keep
  their prior bytes (STRING overwrites only the run it fills). `p = 1` (start at 0)
  is exactly the no-pointer overlay — the correctness anchor: the same statement with
  `p = 1` fills the SAME receiver as the statement WITHOUT the phrase (verified on
  both engines).
- **Write-back.** After the operation `p` is set to `p + chars_placed`, the 1-based
  position one past the last character stored. When the content does not all fit
  (`concat_len > size − (p−1)`) the excess is **dropped** — this is ISO's overflow
  (it now sets the overflow flag; see the `ON OVERFLOW` rung below) — and
  `chars_placed = size − (p−1)`, so `p` becomes `size + 1`. Worked: `"WXYZ"` into a
  5-wide receiver with `p = 3` → `"..WXY"` (the `Z` dropped), and `p` becomes 6.

**Out-of-range initial pointer.** Because `p` is a **run-time** value, neither engine
can range-check it at build time — the compiler emits a run-time `p < 1 || p > size`
guard (jumping past the overlay and the write-back to a trailing `st_end` label), and
the oracle returns early. When the initial `p` is outside `[1, size]` — either
`p == 0` (a 0-based start of −1) or `p > size` (past the receiver end) — this is
ISO's **overflow** condition. Both engines apply the ISO "overflow ⇒ data movement
does not occur" rule DETERMINISTICALLY: **no character is transferred (receiver
unchanged) and `p` is left unchanged** — and, with the `ON OVERFLOW` rung below, the
`ON OVERFLOW` imperative now runs. Both engines produce byte-identical receiver AND
final `p` for every initial value — fuzz-proven across `[0, size+2]`.

**Pointer picture validation** (co-total): `p` must be an unsigned integer `PIC
9(n)`, `n ≤ 18` (the same class the `INSPECT` counter demands). A **signed** (`S9`),
**fractional** (`9V9`), **non-numeric** (`PIC X`), **group**, or **over-wide**
(`> 18`-digit) pointer is a clean later rung, rejected on BOTH engines (the compiler
validates the picture at build time, the oracle at exec time — with matching
messages).

**`ON OVERFLOW` / `NOT ON OVERFLOW` rung (now implemented).** `STRING s1 s2 …
DELIMITED BY {SIZE | delim} INTO t [WITH POINTER p] [ON OVERFLOW imp…] [NOT ON
OVERFLOW imp…]` — the two optional imperative statement lists now run conditionally,
mirroring `ON SIZE ERROR` structurally. No grammar change was needed (the grammar
already parses `[ "ON" "OVERFLOW" { statement } ]` and `[ "NOT" "ON" "OVERFLOW" {
statement } ]` as inline optional sequences). The `overflow` boolean is defined
exactly as ISO requires and computed with the **identical** comparison on both
engines:

- **No `WITH POINTER`:** `overflow = concat_len > size` (compile-time-known; the
  compiler materialises it as a `const`). `concat_len == size` fills the receiver
  exactly, dropping nothing, so it is NOT overflow.
- **`WITH POINTER p`, in range:** `overflow = concat_len > avail` where `avail = size
  − (p−1)` (a run-time drop test).
- **`WITH POINTER p`, out of range** (`p == 0 || p > size`): `overflow = true`, with
  no data movement / pointer write-back.

After the (unchanged) data movement, the `ON OVERFLOW` imperative runs when `overflow`
is true, else the `NOT ON OVERFLOW` imperative; either list may be empty (clause
absent). The oracle runs the selected list through the same `run_stmts` path `COMPUTE
… ON SIZE ERROR` uses (so a `STOP RUN`/`GO TO` inside propagates its `Flow`);
`exec_string` now returns `Result<Flow, RuntimeError>`. The compiler splits the
`statement` children at the `NOT` keyword (as the `IF` reader splits at `ELSE`) and
emits the usual `jmp_if_false`/branch/`label` skeleton guarding on the `overflow`
register. **Behaviour change:** the out-of-range `WITH POINTER` case previously
returned with no imperative; it now runs `ON OVERFLOW`.

**Reference-modification sending-field rung (now implemented).** A STRING sending
field may itself be a **reference modification** — `STRING WS(2:3) DELIMITED BY {SIZE
| delim} INTO t` — with **CONSTANT (literal) indices**. No grammar change was needed
(the grammar's `operand` already carries an optional refmod suffix). The sliced
substring is produced by the SAME shared refmod-substring evaluators every other
context uses — the oracle's `refmod_string`, the compiler's `ref_mod_slice` — so a
STRING of `WS(2:3)` is byte-identical to `DISPLAY WS(2:3)` and to a `MOVE WS(2:3)`
source. Once produced, the substring is just another char image: it drops into the
concatenation, the delimiter prefix-scan, the receiver overlay, and `WITH POINTER`
UNCHANGED — no downstream logic special-cases it. Constant indices `WS(2:3)` and an
omitted length `WS(3:)` are supported.

The boundary is the **index kind**, and it is co-total: a **computed (data-name)
index** — `STRING WS(J:K) …` — has a length known only at run time, which the STRING
image contract (a compile-time `(register, length)` pair on the compiler; a produced
`String` on the oracle) cannot carry, so it stays a clean "later rung" reject on BOTH
engines. The compiler rejects it UP FRONT — before emitting any slice instructions —
so no dead code is produced, and the oracle applies the identical literal-index test,
keeping the accept/reject sets exactly aligned. Byte-vs-char: `ref_mod_slice` is
byte-based and `refmod_string` char-based; they coincide on the ASCII-clean windows
this rung targets (accepted programs emit byte-identical output). A multi-byte char
inside or after the window is the PRE-EXISTING refmod byte-vs-char chip, shared with
`DISPLAY`/`MOVE`-source and not new to STRING; positive tests keep any multi-byte char
strictly OUTSIDE the window.

**Figurative-constant sending-field rung (now implemented).** A STRING sending field
may be a **figurative constant** SPACE or ZERO — `STRING SPACE "X" DELIMITED BY SIZE
INTO t` → `" X"`. It is taken as its **single-character image**: SPACE→`" "` (0x20),
ZERO→`"0"` (0x30), reducing to the existing string-literal sending-field path (the
oracle's `string_source_chars` maps `Fig::Space`/`Fig::Zero` to the 1-char string; the
compiler's `string_source` emits a 1-char `str_const` exactly like the `Src::Str` arm).
Every spelling folds identically — SPACE/SPACES, ZERO/ZEROS/ZEROES. Once produced, the
1-char image drops into the concatenation, the delimiter prefix-scan, the receiver
overlay, and `WITH POINTER` UNCHANGED. Both images are ASCII, and `Fig` is closed at
{Space, Zero}, so no non-ASCII figurative can reach the path — the non-ASCII
sending-field-under-a-delimiter guard passes trivially. This is symmetric to the
`INSPECT … CONVERTING` figurative rung. Still deferred on the figurative side: a
**numeric item**, a **group item**, and a **computed (data-name) index** reference
modification as a sending field remain later rungs, rejected identically on both engines.

Still deferred as clean "later rung" errors: a **multi-character** delimiter, a
non-ASCII delimiter, a non-ASCII literal sending field under a delimiter, a
**computed (data-name) index** reference-modification sending field, and
**per-field different delimiters** (all still *accepted* by the grammar so the reader
can reject them cleanly rather than as a parse failure).

### `UNSTRING` (first rung)

`UNSTRING` is the inverse of `STRING`: it takes one alphanumeric source apart on a
delimiter into several receivers. The **first rung** implements `UNSTRING source
DELIMITED BY delim INTO r1 [r2 …]`, where `delim` is a **single-character**
delimiter — a 1-character string literal (`","`, `" "`), a `PIC X(1)`
item, or a **figurative constant** SPACE/ZERO taken as its single ASCII character
(SPACE→`" "`, ZERO→`"0"`, reducing to the single-char literal path). The source is
scanned left-to-right and split into delimited fields; each
field is moved into the next receiver as an ordinary alphanumeric `MOVE`
(left-justified, space-padded, truncated). The exact semantics (oracle = source of
truth, compiler byte-identical):

- Each receiver **including the last** takes the field up to the NEXT delimiter (or
  end-of-source) — the last receiver does *not* absorb the remainder. Fields beyond
  the receiver count are dropped (this is ISO's `ON OVERFLOW` condition — now
  implemented; see the `ON OVERFLOW` rung below).
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
multi-character / `ALL` / `OR` delimiter
and a numeric/group source or receiver as clean "later rung" errors (`WITH POINTER`
and `ON OVERFLOW` / `NOT ON OVERFLOW` are now supported — see below). Because the
delimiter position is data-dependent, the compiler lowers `UNSTRING` to a run-time
**scan loop** (`str_len` + `str_index`/`cmp` to find each delimiter, then a
`str_slice`/`str_concat` reshape into each receiver), whereas `STRING`'s boundaries
were all compile-time constants.

**Literal source (later rung, now implemented).** The source may be an
alphanumeric **string literal** as well as an item — `UNSTRING "a,b,c" DELIMITED
BY "," INTO w1 w2 w3` → `w1="a" w2="b" w3="c"` (each field reshaped to its
receiver's width). No grammar change was needed: the grammar already parses a
literal operand in the source position, so this was a read-time acceptance only.
Only the character **provider** differs — an identifier source reads the item's
storage (numeric/group items still rejected); a string-literal source scans the
literal's own bytes (no item lookup, no picture check — a string literal is
inherently alphanumeric); in the compiler the literal is materialised into a
`str_const` register that the SAME scan loop reads. The delimiter scan and
per-receiver reshape are entirely shared. Only an **ASCII** literal is accepted:
the oracle scans a literal by CHARACTER while the compiler's IIR string ops are
BYTE-based, so the two agree only when each character is one byte; a **non-ASCII**
literal source (e.g. `UNSTRING "café" …`) is a clean later-rung reject at read
time on both engines, keeping them co-total.

**Reference-modified source (later rung, now implemented).** The source may also
be a **reference-modified item slice** `base(start:len)` — `UNSTRING S(2:3)
DELIMITED BY "," INTO w1 w2 w3` takes the 1-based 3-character slice of `S` and
splits it exactly as an item source would. This is a direct mirror of the
literal-source rung: the ONLY thing that changes is the character **provider** —
the field text is the ref-mod slice, obtained through the SAME slice machinery
`DISPLAY` / comparisons already use (the oracle's `refmod_string`, the compiler's
`ref_mod_slice`). No grammar change was needed (the grammar already parses a
ref-mod operand in the source position) and everything downstream — the delimiter
scan and per-receiver reshape — is unchanged. Because the slice register is
byte-for-byte what the two engines already agree on for `DISPLAY S(2:3)`, the
split matches on both. Both a literal start index (`S(2:3)`) and a **computed**
data-name index (`S(J:3)`) are supported. A **NUMERIC-base** reference-modified
source is **still deferred** — the shared slice helper rejects a numeric base on
both engines, so `UNSTRING N(2:3) …` errors identically; a GROUP base,
out-of-range indices, and a signed/fractional index behave exactly as the existing
reference-modification machinery does (this rung only routes the source through it).

**Figurative-constant source (later rung, now implemented).** The source may also
be a **figurative constant** SPACE or ZERO, mapped to its **single-character**
image — SPACE→`" "` (0x20), ZERO→`"0"` (0x30) — `UNSTRING SPACE DELIMITED BY ","
INTO w1 w2` → the 1-character source `" "` has no comma, so the whole source is one
field landing in `w1` (space-padded to its width) and `w2` keeps its prior value.
This reduces to the **single-char literal-source scan** already implemented above:
the oracle maps `Fig::Space`/`Fig::Zero` to `Lit::Str(" ")`/`Lit::Str("0")` at read
time; the compiler emits the same 1-char `str_const` source register the string
literal uses. No grammar change was needed. Both images are ASCII, so the non-ASCII
literal-source guard is never tripped, and `Fig` = {Space, Zero} is closed, so no
non-ASCII figurative can reach the path. Every spelling folds identically
(SPACE/SPACES, ZERO/ZEROS/ZEROES). The delimiter scan, WITH POINTER write-back,
multi-receiver reshape, and ON OVERFLOW paths are entirely unchanged. A **numeric**
literal source and a **computed** reference-modified source remain later rungs. This
is symmetric to the `STRING` figurative sending-field and `INSPECT CONVERTING`
figurative rungs.

**`WITH POINTER p` (later rung, now implemented).** `UNSTRING source DELIMITED BY
delim INTO r1 [r2 …] WITH POINTER p` — `p` is an **unsigned-integer** item (`PIC
9(n)`, `n ≤ 18`) holding a **1-based** character position. No grammar change was
needed (the grammar already parses `WITH POINTER NAME`); the reader/compiler split
the receiver NAMEs from the pointer NAME at the `POINTER` keyword (the grammar is
flat). Two things change, and everything else — field extraction, receiver
reshape, exhaustion, empty fields — is UNCHANGED:

- **Start offset.** Scanning starts at 0-based index `p_value − 1` instead of 0. So
  `p = 1` is exactly the no-pointer behaviour, and is the correctness anchor: the
  same statement with `p = 1` fills the SAME receivers as the statement WITHOUT the
  phrase (verified on both engines).
- **Write-back.** After the operation `p` is set to the 1-based position of the
  character immediately following the last one examined: `min(final_cursor, len) +
  1`. The scan's final 0-based cursor sits one past the terminating delimiter; for a
  field that ran to end-of-source that step is a phantom one past the end, which the
  clamp to `len` removes. Worked: source `"a,b,c"` (len 5), `p = 3` → start at index
  2 ("b,c"), `r1="b"`, `r2="c"`, and `p` is updated to 6.

**Out-of-range initial pointer.** Because `p` is a **run-time** value, neither
engine can range-check it at build time — the compiler emits a run-time guard, and
the oracle checks at exec time. When the initial `p` is outside the valid range
`[1, len]` — either `p == 0` (a 0-based start of −1) or `p > len` (past the source)
— this is ISO's **overflow** condition. We apply the ISO "overflow ⇒ data movement
does not occur" rule DETERMINISTICALLY: **no receiver is modified and `p` is left
unchanged** — and, with the `ON OVERFLOW` rung below, the out-of-range case now sets
the overflow flag and runs the `ON OVERFLOW` imperative. Both engines produce
byte-identical receivers AND a byte-identical final `p` for every initial value (0,
1, mid, len, len+1, huge) — fuzz-proven across `[0, len+2]`.

**Pointer picture validation** (co-total): `p` must be an unsigned integer `PIC
9(n)` (the same class the `INSPECT` counter demands). A **signed** (`S9`),
**fractional** (`9V9`), **non-numeric** (`PIC X`), **group**, or **over-wide**
(`> 18`-digit) pointer is a clean later rung, rejected on BOTH engines (the compiler
validates the picture at build time, the oracle at exec time — with matching
messages).

Still deferred on both engines: a
**NUMERIC**-literal source (`UNSTRING 123 …`), a **non-ASCII** string-literal
source — only an ASCII alphanumeric string literal is supported — and a
**NUMERIC-base** reference-modified source (an alphanumeric-base
reference-modified source is now supported, and a **FIGURATIVE** source SPACE/ZERO
is now accepted as its single-character literal image — see above). A
signed/fractional/non-numeric `WITH POINTER` item is also deferred, but the `WITH
POINTER` phrase itself over a `PIC 9(n)` pointer and the `ON OVERFLOW` / `NOT ON
OVERFLOW` handlers (see the rung below) are now supported.

**`ON OVERFLOW` / `NOT ON OVERFLOW` rung (now implemented).** `UNSTRING source
DELIMITED BY delim INTO r1 [r2 …] [WITH POINTER p] [ON OVERFLOW imp…] [NOT ON
OVERFLOW imp…]` — the DIRECT sibling of the `STRING` overflow rung above. No grammar
change was needed; the grammar already parses `[ "ON" "OVERFLOW" { statement } ]`
and `[ "NOT" "ON" "OVERFLOW" { statement } ]`. The reader/compiler split the two
`statement` lists at the `NOT` keyword (exactly as the `IF` reader splits then/else
at `ELSE`, and as `STRING` now does); receiver/pointer NAMEs are direct token
children, never `statement` nodes, so the split never sees them.

The **overflow condition** — computed IDENTICALLY on both engines (a mismatch would
diverge) — is: all receivers are filled but the source is NOT exhausted (more
delimited fields remain), OR the initial `WITH POINTER` value is out of range.
Concretely:

- **Scan path (in-range or no pointer):** `overflow = (final_cursor <= len)`, where
  `final_cursor` is the scan's final 0-based cursor `p`. This ONE comparison is
  correct for every case: the loop broke early because the source was exhausted first
  (`p > len`) → not overflow; every receiver filled with the last field ending AT a
  delimiter (`p ≤ len`, more source remains) → overflow; the last field ran to
  end-of-source (`p = len+1 > len`) → not overflow; a trailing delimiter as the last
  consumed char (`p == len`, an empty field still remains) → overflow.
- **Out-of-range pointer (`p == 0 || p > len`):** `overflow = true`, with NO data
  movement and `p` left unchanged (the write-back is also skipped).

After the (unchanged) scan and pointer write-back, the `ON OVERFLOW` imperative runs
when `overflow` is true, else the `NOT ON OVERFLOW` imperative; either list may be
empty (clause absent), and control flow (`GO TO` / `STOP RUN`) inside the chosen list
propagates. The compiler emits the `overflow` register (pre-seeded to `1`, the
out-of-range guards fall through with it set, the in-range path overwrites it with
`cmp_le(p, len)`) and the `jmp_if_false`/branch/`label` skeleton ONLY when a clause
is present — a plain `UNSTRING` lowers exactly as before. **Behaviour change:** the
out-of-range `WITH POINTER` case previously returned with no imperative; it now runs
`ON OVERFLOW`.

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
  (`"A"`), a `PIC X(1)` item, or a **figurative constant** SPACE/ZERO taken as its
  single ASCII character (SPACE→`" "`, ZERO→`"0"`, reducing to the single-char
  literal path). `ALL` means every (non-overlapping, left-to-right)
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
`take_while…count`).

**`BEFORE`/`AFTER` region (FOR ALL follow-up rung).** The `FOR ALL` form now
accepts an optional `{BEFORE|AFTER} x` **region** that narrows the count to a
sub-slice of the source, bounded by the **FIRST (leftmost) occurrence** of the
single-character region delimiter `x`:

- `BEFORE x` counts `delim` only in `source[0 .. first_index_of(x)]`; if `x` is
  **absent** the region is the **ENTIRE** source.
- `AFTER x` counts `delim` only in `source[first_index_of(x)+1 .. end]`; if `x` is
  **absent** the region is **EMPTY** (count `0`).

This not-found **asymmetry** (BEFORE→whole, AFTER→empty) is the ISO rule and the
crux of the rung. Worked (`counter` `PIC 9(3)`, source `"AB0CD0"`, `delim = "0"`):
`BEFORE "C"` → region `"AB0"` → `1`; `AFTER "C"` → region `"D0"` → `1`; `BEFORE
"Z"` (absent) → whole source → `2`; `AFTER "Z"` (absent) → empty → `0`; `AFTER "A"`
in `"ABABA"` counting `"A"` (region delimiter equals tally delimiter) → the first
`A` bounds the region to `"BABA"` → `2`. The oracle computes the window `[start,
end)` over the source's chars and counts within it; the compiler emits a one-shot
scan for the first occurrence of `x` (a `found` flag + first index), derives
`[start, end)` with the same asymmetry, and bounds its count loop with `j < start
|| j >= end → skip` — byte-identical, and with no region nothing extra is emitted.
This `FOR ALL` region rung is scoped SMALL: a **single-character** region delimiter
only.

**`BEFORE`/`AFTER` region on the STANDALONE `FOR LEADING` (follow-up rung).** The
lone `FOR LEADING delim {BEFORE|AFTER} x` is now supported too, with the leading run
**anchored at the window start**: `FOR LEADING` counts only the maximal run of
`delim` that begins **at the window's start index** and stops at the first
non-matching character INSIDE the window (or the window end) — a leading run that
would start at position 0 but whose window starts at `first+1` (AFTER) begins
mid-string. Worked (`delim = "a"`): `FOR LEADING "a" AFTER "X"` over `"aaXaab"` →
window `"aab"` (indices 3..6) → `2` (the `"aa"` before the `X` does NOT contribute);
`AFTER "X"` over `"aaXbb"` → window `"bb"` → `0` (the window opens on a mismatch);
`BEFORE "X"` over `"aaXaa"` → window `"aa"` → `2`; `AFTER "Z"` absent → empty window
→ `0`; `BEFORE "Z"` absent → whole source → `2`. The oracle's window `take_while`
was already anchored at the window start (it slices `[start, end)` first); the
compiler seats its count-loop counter at `start` and bounds it by the window `end`
(instead of `0..len`), so the existing stop-at-first-mismatch break yields the
window-anchored run — byte-identical, and the `FOR ALL` lowering is untouched.

**`FOR CHARACTERS` (follow-up rung).** `INSPECT source TALLYING counter FOR
CHARACTERS [ {BEFORE|AFTER} x ]` is the **count-every-position** form: it does NOT
match a delimiter (the grammar's `CHARACTERS` branch of `tally_item` carries no
operand). Instead the count is the **number of character positions in the region
window** — exactly the window LENGTH — ADDed to the counter (INSPECT adds, it does
not clear). With **no region** that is `length(source)`; with a `{BEFORE|AFTER} x`
region it is `end - start` of the SAME window `FOR ALL` uses, so it inherits the
identical not-found asymmetry: `BEFORE x` with `x` absent ⇒ WHOLE source, `AFTER x`
with `x` absent ⇒ EMPTY window ⇒ `0`. Worked (`counter` `PIC 9(3)`): `"BANANA"` FOR
CHARACTERS → `6`; `"AB0CD0"` FOR CHARACTERS BEFORE `"C"` → window `"AB0"` → `3`;
FOR CHARACTERS AFTER `"C"` → window `"D0"` → `2`; `"HELLO"` FOR CHARACTERS BEFORE
`"Z"` (absent) → `5`; AFTER `"Z"` (absent) → `0`.

The count is byte-identical on both engines because both derive the window from the
SAME shared helper: the oracle's `inspect_tally` sets `count = window.len()` (skipping
`single_delim_char` entirely — there is no delimiter), and the compiler emits
`cnt = end - start` (a `sub`) when a region is present or `cnt = str_len(S)` when it
is not, skipping the per-character match loop. This rung enables the **single-item
single-counter** CHARACTERS phrase; a MULTI-item `CHARACTERS` (one CHARACTERS item
ALONGSIDE other items under ONE counter) is enabled by a LATER rung — see "A
`CHARACTERS` item in a multi-item list" below. A `CHARACTERS` TALLYING half inside a
combined `TALLYING … REPLACING` is enabled by a later rung too — see "A `CHARACTERS`
TALLYING half in the combined form" below. A MULTI-counter `CHARACTERS` and the
combined REPLACING half's OWN `CHARACTERS` form stay later rungs rejected identically
on both engines.

Now supported (identically on both engines): a **combined** `TALLYING … REPLACING`
whose LEADING half (tally and/or replace) carries a region — the combined exec/emit
compose the SAME standalone LEADING+region routines in ISO tally-then-replace order,
so the combination is byte-identical to the oracle. Still deferred: a
multi-character / non-ASCII region delimiter and the REPLACING half's OWN
`CHARACTERS` form in the combined form (the TALLYING half's `FOR CHARACTERS` is now
supported — see "A `CHARACTERS` TALLYING half in the combined form" below).

**A `CHARACTERS` TALLYING half in the combined form (follow-up rung).** The
combined `TALLYING … REPLACING` form now admits a `FOR CHARACTERS` TALLYING half:
`INSPECT S TALLYING C FOR CHARACTERS [ {BEFORE|AFTER} x ] REPLACING …`. The former
read-time reject ("INSPECT TALLYING … FOR CHARACTERS in a combined TALLYING/REPLACING
is a later rung") is lifted for the TALLYING half only. It executes in the same ISO
tally-then-replace order: the CHARACTERS half counts EVERY position in its (optional)
window into the counter — exactly the STANDALONE `FOR CHARACTERS` count above
(`window.len()` on the oracle, `str_len(S)` or `end - start` on the compiler,
inheriting the same `BEFORE`→whole / `AFTER`→empty asymmetry) — over the ORIGINAL
source bytes, THEN the REPLACING half rewrites. The REPLACING half keeps its full
existing `ALL`/`LEADING` (+region) support unchanged. Threading: the oracle adds a
`tally_characters: bool` field to `Stmt::InspectTallyReplace` (populated from
`read_inspect_tally_all`'s CHARACTERS flag) and passes it into `inspect_tally`
instead of the hardcoded `false`; the compiler flips the combined call site's
`allow_characters` argument to `emit_inspect_tallying` from `false` to `true`, routing
the count through the SAME CHARACTERS lowering the standalone tally uses. Worked
(`C` `PIC 9(3)`): `"XAYAZ"` `TALLYING C FOR CHARACTERS REPLACING ALL "A" BY "B"` →
`C = 5` (the full length, NOT the two `"A"`s) and `S = "XBYBZ"`; `"AAXAA"`
`TALLYING C FOR CHARACTERS BEFORE "X" REPLACING ALL "A" BY "B"` → `C = 2` (the window
`"AA"` before `"X"`) while the un-regioned replace rewrites EVERY `"A"` →
`S = "BBXBB"`.

Still deferred (co-total): the combined REPLACING half's OWN `CHARACTERS` form
(`REPLACING CHARACTERS BY x`) — a DIFFERENT node (`InspectReplacingCharacters` /
`emit_inspect_replacing_characters`) read by the single-item `read_inspect_replacing_all`
in the combined arm, which still rejects a `CHARACTERS` REPLACING item on both engines.

**Byte-vs-char count chip (unchanged).** The CHARACTERS count is position-based; the
compiler counts BYTE positions (`str_len`) while the oracle counts CHAR positions
(`chars.len()`), coinciding on ASCII. A non-ASCII source is the PRE-EXISTING
byte-vs-char count chip (task_396ba6f6), identical to the standalone `FOR CHARACTERS`
and the multi-item `CHARACTERS`; moreover the combined REPLACING half reconstructs the
field per byte position and traps on a multi-byte char anyway (the shared
reconstruction chip). Neither is fixed here.

The grammar deliberately accepts the fuller `INSPECT` surface — a MULTI-item or
MULTI-counter `CHARACTERS` tally (the single-item single-counter `CHARACTERS` form is
now supported — see "`FOR CHARACTERS`" above; a region on the LEADING half of the
**combined** form is now supported too, alongside the STANDALONE
`FOR LEADING`/`REPLACING LEADING` regions, the lone `REPLACING ALL` and `CONVERTING`
regions, and a region on each `ALL` half of the combined form — see those sections
below), and a
multi-character region
delimiter — so the reader/compiler reject each as a clean "later rung" error
rather than a parse failure. A **figurative constant** SPACE/ZERO delimiter (tally
OR region) is now accepted, reduced to its single ASCII character through the shared
`single_delim_char`/`single_delim_code` helper. A multi-character / numeric /
wider-than-one delimiter (tally OR region), and a numeric/group source or a
non-integer/signed/non-numeric counter, are likewise clean later rungs. (`FIRST`
and `INITIAL`, needed only by `REPLACING FIRST` / `BEFORE INITIAL`, are left
unreserved so common data names keep working.)

### `INSPECT … REPLACING` (first rung + `LEADING`)

The **substitution** form implements a LONE
`INSPECT source REPLACING ALL x BY y` (and, as a follow-up rung,
`INSPECT source REPLACING LEADING x BY y`):

- `source` is an alphanumeric (`PIC X`) item, modified **in place**. Because
  both `x` and `y` are single characters, the result has the **same width** as
  the source — this is a straight **per-position map**.
- `x` (the search) and `y` (the replacement) are each a **single character** — a
  1-character string literal (`"A"`), a `PIC X(1)` item, or a **figurative constant**
  SPACE/ZERO taken as its single ASCII character (SPACE→`" "`, ZERO→`"0"`, reducing
  to the single-char literal path) — reusing the same single-character helpers as
  `TALLYING`/`UNSTRING` (`single_delim_code` for the search byte scan,
  `single_delim_str` for the 1-char replacement string).
- `ALL` semantics: `source := source with each x → y`, left to right. Every
  position `j` where `source[j] == x` becomes `y`; all others are unchanged.
- `LEADING` semantics: replace only the run of **consecutive** `x` characters at
  the **START** of the source, stopping at the first character that is not `x`.
  A position `j` is replaced iff **every** character at `0..=j` equals `x`;
  positions after that first gap are left unchanged **even if they equal `x`**.

Worked, `ALL` (search `"A"`, replacement `"X"`): `"ABABA"` → `"XBXBX"`; a search
that never occurs (`"Z"` in `"HELLO"`) leaves the source unchanged; `"AAAA"` with
`A → X` → `"XXXX"`. Worked, `LEADING` (search `"0"`, replacement `"*"`):
`"000123"` → `"***123"`; `"00X00"` → `"**X00"` (stops at `X`; contrast `ALL`,
which gives `"**X**"`); `"120003"` → `"120003"` (no leading run); `"0000"` →
`"****"`; a blank source is unchanged.

The oracle rebuilds the string and stores it back through the **same alphanumeric
char-store path** a `MOVE` uses. For `ALL` it is a stateless map
(`source.chars().map(|c| if c == x { y } else { c })`); for `LEADING` it is a
**stateful** map that keeps an `in_run` flag, replacing while `in_run && c == x`
and flipping `in_run` off (permanently) at the first non-`x`. The compiler
**unrolls** the per-position map over the compile-time-known width `W`
(`str_index`/`cmp_eq` per byte, splicing either the replacement or the original
character with `str_slice`/`str_concat`), then copies the `W`-wide result into the
source register. For `LEADING` it threads a runtime `active` flag (i64, init 1)
through the unroll: position `j` is replaced iff `active AND (s[j] == x)`, and
`active := active AND (s[j] == x)` sticks at 0 after the first non-match — the
extra `and` is the ONLY difference from `ALL`, and it folds away for `ALL`. The
two engines agree byte-for-byte.

**`BEFORE`/`AFTER` region (REPLACING ALL follow-up rung).** The `REPLACING ALL`
form now accepts an optional `{BEFORE|AFTER} z` **region** that restricts the
substitution to a sub-slice of the source, bounded by the FIRST (leftmost)
occurrence of the single-character region delimiter `z` — the exact analogue of
the `TALLYING FOR ALL` region applied to the replace instead of the count:

- `BEFORE z` replaces `x`→`y` only in `source[0 .. first_index_of(z)]`; if `z` is
  **absent** the region is the **ENTIRE** source (whole-source replace).
- `AFTER z` replaces only in `source[first_index_of(z)+1 .. end]`; if `z` is
  **absent** the region is **EMPTY** (no replacement).

Positions **outside** the region keep their **original** character. The window is
computed over the **ORIGINAL** source and is the SAME `[start, end)` the count
uses — the oracle factors a shared `region_window` helper called by both
`inspect_tally` and `inspect_replace`, and the compiler REUSES
`emit_inspect_region_window` and guards its per-position unroll with `start <= j <
end` — so the BEFORE→whole / AFTER→empty asymmetry is byte-identical across the two
INSPECT operations. Worked (search `"0"`, replacement `"*"`, source `"0A0B0"`):
`BEFORE "B"` → region `"0A0"` → `"*A*B0"`; `AFTER "B"` → region `"0"` (trailing) →
`"0A0B*"`; `BEFORE "Z"` (absent) → whole source → `"*A*B*"`; `AFTER "Z"` (absent) →
empty → `"0A0B0"` unchanged; `AFTER "0"` (region delimiter equals search) → the
first `0` bounds the region to `"A0B0"` → `"0A*B*"` (the leading `0` is left of the
region and kept). This `REPLACING ALL` region rung is scoped SMALL: a
**single-character** region delimiter. With no region the lowering is unchanged.

**`BEFORE`/`AFTER` region on the STANDALONE `REPLACING LEADING` (follow-up rung).**
The lone `REPLACING LEADING x BY y {BEFORE|AFTER} z` is now supported too, the exact
analogue of `FOR LEADING` + region on the count side, with the substitution run
**anchored at the window start**: characters before the window are copied through
unchanged and neither begin nor break the run, the run begins at the window start,
and it stops at the first non-`x` INSIDE the window (or the window end). Worked
(search `"a"`, replacement `"*"`): `AFTER "X"` over `"aaXaab"` → window `"aab"` →
`"aaX**b"` (only the two leading `a`s after the `X`, NOT the `"aa"` before it);
`AFTER "X"` over `"aaXbb"` → window `"bb"` → unchanged (window opens on a mismatch);
`BEFORE "X"` over `"aaXaa"` → window `"aa"` → `"**Xaa"`; `AFTER "Z"` absent → empty
window → unchanged; `BEFORE "Z"` absent → whole source → `"**Xaa"`. The oracle
copies positions outside `[start, end)` through unchanged and leaves the run state
untouched; the compiler threads `use_repl = active AND (s[j]==x) AND in_region` and
decays the run only on an IN-WINDOW mismatch (`active := active AND ((s[j]==x) OR
NOT in_region)`) — byte-identical, and the `ALL` and no-region `LEADING` lowerings
are untouched.

**Multiple `TALLYING` items under one counter (follow-up rung).**
`INSPECT source TALLYING counter FOR ALL a ALL b [ALL d …]` — TWO OR MORE `FOR ALL`
tally items sharing ONE counter — is now supported (previously rejected at read time as
"several FOR phrases is a later rung"). This is the count-side analogue of the multi-item
`REPLACING` rung below. Per ISO the delimiters form an ordered priority list: ONE
left-to-right pass over the source, and at each position the delimiters are tried **in
written order** and the **first** that matches adds 1 to the shared counter, then the scan
advances past the match (a single-char match is a normal one-position step). A position
matching no delimiter advances with no increment. `INSPECT` **adds** the count to the
counter; it does not clear it first (`counter := counter + count`).

The crux is that **duplicate delimiters do NOT double-count**: `FOR ALL "a" ALL "a"` over
`"aa"` adds 2 — each `a` position is counted ONCE by the first item, the second never
fires there. Net, the count is the number of source positions whose character equals SOME
delimiter, each counted exactly once. Worked: `"abcab"` `FOR ALL "a" ALL "b"` → `4`;
`"aQbQa"` `FOR ALL "a" ALL "b"` → `3` (the `Q`s match nothing).

The oracle adds a `Stmt::InspectTallyMulti { source, counter, delims }` variant read via
`read_inspect_tally_multi`; `read_statement` dispatches on the number of `tally_item`
children under the SOLE `tally_for` (exactly one → the single-item path with all its
capabilities; two or more → the multi path). `exec_inspect_tally_multi` resolves every
delimiter char FIRST (shared `single_delim_char`, so an invalid delimiter aborts before
touching the counter), validates the counter as an unsigned `PIC 9(n)` integer, counts in
one pass, and folds via the same `store_result` path (silent high-order truncation on
overflow) the single-item tally uses. The compiler mirrors this with
`emit_inspect_tally_multi` (a genuine runtime `str_len`-bounded loop — the tally builds no
fixed-width string — with an ordered `cmp_eq` chain per position that bumps and jumps past
the rest on the first match) and an `inspect_tally_multi` CST reader counting the SAME
`tally_item` children, so the two engines' accept/reject sets are co-total.

This multi-item path started scoped SMALL (only `ALL` items, single-char delimiters, no
region, no `LEADING`/`CHARACTERS`, under ONE counter), but follow-up rungs have since lifted
that scope IN PLACE: a per-item `{BEFORE|AFTER}` region, a `LEADING` item, and — this rung —
a `CHARACTERS` item are all now admitted (see "A `CHARACTERS` item in a multi-item list"
below). What REMAINS deferred: the combined `TALLYING … REPLACING` form with several tally
items stays a later rung (rejected identically on both engines). A single tally item keeps
the full single-item path unchanged. **Several counters** (more than one `tally_for`) is
supported by the next rung — that MULTI-counter path stays `ALL`-only and keeps rejecting
`LEADING`/`CHARACTERS`.

**A `CHARACTERS` item in a multi-item list (follow-up rung).** A `CHARACTERS` item may now
appear ALONGSIDE other items in a single-counter multi-item `TALLYING` list — e.g.
`TALLYING C FOR ALL "A" CHARACTERS`, `TALLYING C FOR CHARACTERS ALL "," BEFORE "X"`. The
former reject in `read_inspect_tally_multi` / `inspect_tally_multi` ("INSPECT TALLYING …
FOR CHARACTERS is a later rung") is lifted for THIS path only. In the same ordered
first-match-per-position pass (the one #65 extended to mix `ALL` and `LEADING`), a
`CHARACTERS` item is the **always-eligible catch-all**: at each source position it is
eligible iff the position is in its window (`in_win`) — NO delimiter compare, NO
active-run tracking, and it carries NO delimiter operand. So it contributes 1 for every
in-window position not already claimed by an EARLIER item in written order. Written order
is honoured: `FOR ALL "A" CHARACTERS` over a mixed ASCII string totals the source length
(ALL "A" claims its positions, CHARACTERS the rest), while `FOR CHARACTERS ALL "A"`
(CHARACTERS first) claims every position so the ALL item never fires. An optional
`{BEFORE|AFTER} z` region narrows the catch-all's window exactly like any other item, via
the SAME `region_window` / `emit_inspect_region_window` machinery.

The item type gains an explicit kind so the illegal "LEADING and also CHARACTERS" state is
unrepresentable: the oracle's `TallyMultiLeadingItem` becomes
`(Option<Operand>, TallyMultiKind, Option<Region>)` (`TallyMultiKind` = `All`/`Leading`/
`Characters`; the delimiter is `None` for `Characters`), and the compiler mirrors it with a
local `TallyKind`. `exec_inspect_tally_multi` / `emit_inspect_tally_multi` resolve a
`CHARACTERS` item's window only (no `single_delim_char` / `single_delim_code`), gate its
eligibility on `in_win` alone, and skip it in the leading active-run update. The
MULTI-counter path and the combined `TALLYING … REPLACING` form KEEP rejecting `CHARACTERS`
(those gates live in `read_inspect_tally_counters` / the combined caller). **Byte-vs-char
count chip (task_396ba6f6, unchanged):** a `CHARACTERS` item counts POSITIONS, and the
oracle iterates CHAR positions while the compiler iterates BYTE positions, so a non-ASCII
source INSIDE a CHARACTERS window diverges — the SAME chip as the single `FOR CHARACTERS`
form, deliberately NOT fixed here (positive tests are ASCII; a non-ASCII test is kept
co-total by placing the multi-byte char outside every window).

**Multiple `TALLYING` counters (follow-up rung).**
`INSPECT source TALLYING c1 FOR ALL a [ALL b …] c2 FOR ALL d [ALL e …] …` — TWO OR MORE
`tally_for` groups, each with its OWN counter and one-or-more single-char `FOR ALL`
delimiters — is now supported (previously rejected at read time as "several counters is a
later rung"). This GENERALISES the multi-item single-counter rung above to a list of
`(counter, delimiter)` pairs where the matched pair's OWN counter is bumped.

Per ISO, ALL the tally items across ALL groups form ONE **combined ordered priority list**,
scanned in a SINGLE left-to-right pass over the source. At each position the items are tried
**in written order** (group 1's items first, then group 2's, …) and the **first** that
matches increments ITS OWN group's counter by 1, then the scan advances past the match. A
position matching no item advances with no increment. The decisive consequence: a character
CLAIMED by an earlier group's item NEVER reaches a later group's item — so the groups are
NOT independent counts. Worked:

- `"aa"  TALLYING C1 FOR ALL "a"  C2 FOR ALL "a"`  → `C1 += 2, C2 += 0` (C1's "a" wins both)
- `"ab"  TALLYING C1 FOR ALL "a"  C2 FOR ALL "b"`  → `C1 += 1, C2 += 1`
- `"aba" TALLYING C1 FOR ALL "a" ALL "b"  C2 FOR ALL "a"`  → `C1 += 3, C2 += 0`

Each counter **adds** its own share (`INSPECT` never clears), and each truncates
independently through the same store path. The SAME counter name may legally appear in two
groups — both groups' matches then add to that one item (the counter is resolved by name at
each add, so this stays correct).

The oracle adds a `Stmt::InspectTallyCounters { source, groups }` variant read via
`read_inspect_tally_counters`; `read_statement` dispatches PURELY on the number of
`tally_for` groups (`>= 2` → this variant; exactly one keeps the unchanged single-counter
`Inspect`/`InspectTallyMulti` paths). `exec_inspect_tally_counters` validates every counter
as an unsigned `PIC 9(n)` integer FIRST, resolves every delimiter to `(group, char)` FIRST
(shared `single_delim_char`), runs one first-match pass into per-group accumulators, then
folds each accumulator into its counter via the same `store_result` path. The compiler
mirrors this with `emit_inspect_tally_counters` (one accumulator register per group through
a single `str_len`-bounded loop, an ordered `cmp_eq` chain over the flattened
`(group, delimiter)` list per position, then a per-group `counter := counter + accumulator`
via the same `store_scaled`, re-reading each counter's register so a shared counter
accumulates both shares) and an `inspect_tally_counters` CST reader walking the SAME
`tally_for`/`tally_item` children, so the two engines' accept/reject sets are co-total.

This multi-counter path is scoped SMALL: only `ALL` items, each a single-char delimiter,
with **no** `{BEFORE|AFTER}` region and **no** `LEADING`/`CHARACTERS`, every counter an
unsigned `PIC 9(n)` integer. A group carrying any of those, and the combined
`TALLYING … REPLACING` form with several counters (still routed through the several-counters
reject in `read_inspect_tally_all`), remain later rungs (rejected identically on both
engines). Exactly ONE `tally_for` keeps the single-counter paths unchanged.

**Multiple `REPLACING` items in one clause (follow-up rung).**
`INSPECT source REPLACING ALL a BY x ALL b BY y [ALL c BY z …]` — TWO OR MORE `ALL`
replace items in a single `REPLACING` clause — is now supported (previously rejected at
read time as "several replace items is a later rung"). Per ISO this is ONE left-to-right
pass over the source: at each position the items are considered **in written order** and
the **first** item whose single-char search matches the ORIGINAL character is applied,
then the position advances. Two properties follow:

- **First-match-wins** — only the earliest-written matching item fires at a position
  (`ALL "a" BY "x" ALL "a" BY "y"` maps every `a` to `x`, never `y`).
- **No re-chaining** — the byte a replacement produces is NEVER re-examined by a later
  item. `REPLACING ALL "a" BY "b" ALL "b" BY "z"` over `"ab"` → `"bz"`, NOT `"zz"`:
  position 0's original `a`→`b` stops (the produced `b` is not turned into `z`), and
  position 1's ORIGINAL `b`→`z`. A naive sequential two-pass replace would give `"zz"`;
  the single-pass reading-only-the-original semantics is what makes `"bz"` correct.

The oracle adds a `Stmt::InspectReplacingMulti { source, items }` variant read via
`read_inspect_replacing_multi`; `read_statement` dispatches on the number of
`replace_item` children (exactly one → the single-item path with all its capabilities;
two or more → the multi path). `exec_inspect_replacing_multi` resolves every
`(search, replace)` char pair FIRST (shared `single_delim_char`, so an invalid operand
aborts before mutating) then rebuilds in one pass over the original characters. The
compiler mirrors this with `emit_inspect_replacing_multi` and an `inspect_replacing_multi`
CST reader that counts the SAME children, so the two engines' accept/reject sets are
co-total; the per-position lowering is an ordered if-else chain that appends the first
matching item's replacement and jumps to the position's done label (first-match-wins),
always comparing against the original `s[j]` (no re-chaining).

This multi-item path is scoped SMALL: only `ALL` items, each a single-char search BY
single-char replacement, with **no** `{BEFORE|AFTER}` region and **no**
`LEADING`/`CHARACTERS`/`FIRST`. A multi-item list carrying any of those, and the
combined `TALLYING … REPLACING` form with several items, remain later rungs (rejected
identically on both engines). A single replace item keeps the full single-item path
(LEADING, region) unchanged.

**`REPLACING CHARACTERS BY x [ {BEFORE|AFTER} z ]` (follow-up rung).** `INSPECT source
REPLACING CHARACTERS BY x` is the **replace-every-position** form: unlike `REPLACING
ALL …` there is no search character — with no region EVERY position of the
alphanumeric `source` is overwritten with the single replacement char `x`, so the
WHOLE field becomes `x`s. Its width is unchanged (a field of char-width N becomes N
copies of `x`). Worked (`PIC X(5)`): `"ABABA"` REPLACING CHARACTERS BY `"X"` →
`"XXXXX"`; `"A B C"` BY `"-"` → `"-----"` (even embedded spaces are overwritten); the
replacement may be a `PIC X(1)` **item** as well as a literal.

The whole subtlety of the no-region path is the **byte basis**. The compiled
`cobol-iir-compiler` models storage as a BYTE buffer (`str_len` is a byte length;
`PIC X` positions ARE bytes) while the oracle models it as a Rust `String`. To agree
for ANY source we compute the fill on the byte basis: the oracle builds
`n = storage.len()` (byte-length) copies of `x` then stores through `move_into`, which
re-pads/truncates to the picture's fixed CHAR size; the compiler appends `x` exactly
`width` (picture char width) times. Because `x` is a single ASCII byte, both converge
on `width` copies of `x`. Worked non-ASCII regression: `PIC X(5) VALUE "café"` stores
`"café "` (padded to 5 CHARS = 6 BYTES); REPLACING CHARACTERS BY `"Z"` fills the
oracle's `n = 6` copies, capped by `move_into` to the picture's 5 chars → `"ZZZZZ"`
(FIVE `Z`s, not six — the fixed width caps the padded byte image on BOTH engines),
exactly the compiler's `width = 5` fill.

**Optional `{BEFORE|AFTER} z` region (THIS rung).** The former deferral of a region on
the CHARACTERS item is now **lifted** on both engines. The `InspectReplacingCharacters`
statement gains an `Option<Region>` field (the SAME `Region` the ALL/region path uses),
read with the SAME `read_inspect_region` helper. When a region is present, only the
window positions become `x` and positions OUTSIDE the window keep their original char,
using the SAME window machinery as the ALL/region and TALLYING CHARACTERS forms: the
oracle computes the CHAR window `[start, end)` over the source's current storage via
`region_window` and rebuilds `chars[..start]` ++ `repeat(x, end - start)` ++
`chars[end..]`; the compiler derives the BYTE window `[start, end)` via
`emit_inspect_region_window` over the ORIGINAL source and, unrolling `0..width`, appends
`x` iff `start <= j < end` else the original `str_slice(s, j, j+1)` (the ALL-with-region
idiom MINUS the `s[j] == x` compare, since CHARACTERS replaces EVERY in-window position).
Both honour the ISO not-found asymmetry (`BEFORE z` absent → whole field; `AFTER z`
absent → empty → source unchanged). Worked (`PIC X(5) "AB,CD"`): BEFORE `","` → window
`"AB"` → `"**,CD"`; AFTER `","` → window `"CD"` → `"AB,**"`; BEFORE `"Z"` (absent) →
`"*****"`; AFTER `"Z"` (absent) → `"AB,CD"`. The no-region path is byte-identical to
before this rung.

**Byte-vs-char chip (pre-existing).** The oracle's window is a CHAR span; the compiler's
is a BYTE span. They coincide on an ASCII source. A **non-ASCII** source (a byte window
splitting a multi-byte char, or the compiler's per-position `str_slice` reconstruction
trapping on a multi-byte char while the oracle succeeds char-based) is the PRE-EXISTING
byte-vs-char chip (task_396ba6f6) shared with every REPLACING-with-region lowering — NOT
newly guarded here; positive tests use ASCII, and one characterization test documents the
divergence.

Scoped SMALL and co-total, this rung applies these guards **identically** on both
engines: (1) `x` must be a SINGLE character (the shared single-char check REPLACING
ALL uses); (2) a single-char but **non-ASCII literal** `x` (e.g. `"é"`, one char /
two bytes) is a later rung — mirroring the byte-based compiler validator; the oracle
adds an `is_ascii()` check on the resolved literal char to match. A `PIC X(1)` item
replacement is NOT ASCII-gated: the byte-fill above is co-total for a multi-byte item
too (both engines emit `width` copies of the item's char). (3) The optional region
delimiter `z` is validated single-char by the shared helpers (a multi-char/numeric/
reference-modified region delimiter stays a later rung, co-total). (4) The
source-category guard is unchanged (a numeric/group/reference-modified/literal source
stays a later rung). `REPLACING CHARACTERS` inside a MULTI-item list is now supported (see
"Multi-item `REPLACING` list with a `CHARACTERS` item" below); inside the COMBINED
`TALLYING … REPLACING` form it remains a later rung.

Deferred as clean later rungs (accepted by the grammar, rejected at read/compile
time): `REPLACING CHARACTERS BY x` with a non-ASCII
literal replacement (see above), `REPLACING FIRST` (`FIRST` does not parse as a
replace keyword — it is deferred at parse time),
a multi-character region delimiter, a multi-item `REPLACING` list carrying a
`FIRST` item (a `{BEFORE|AFTER}` region on each item of a multi-item list, a `LEADING`
item, and a `CHARACTERS BY x` item are all now supported — see the sections below),
**several** replace items in the **combined** `TALLYING … REPLACING` form, a multi-character /
wider / numeric search or replacement, and a numeric/group source (a **figurative
constant** SPACE/ZERO search or replacement is now accepted, reduced to its single
ASCII character through the shared `single_delim_code` / `single_delim_str` helpers).
(A single-clause multi-item `REPLACING ALL` list is now supported — see just above.
A `REPLACING LEADING` inside the combined `TALLYING … REPLACING` form, and a
`{BEFORE|AFTER}` region on each half — `ALL` **or** `LEADING` — of the combined form,
are supported — see the combined section below; the STANDALONE
`REPLACING LEADING … {BEFORE|AFTER}` is supported as described just above.)

### Multi-item `REPLACING` with a per-item `{BEFORE|AFTER}` region (follow-up rung, v0.61.0 / v0.57.0)

The multi-item `REPLACING` rung above and the single-item `REPLACING ALL … {BEFORE|AFTER}`
region rung now **compose**: each item of a multi-item `REPLACING` list may carry its OWN
optional `{BEFORE|AFTER}` region. Previously any region on a multi-item list was a later rung
(rejected on both engines with "INSPECT REPLACING with several items and a BEFORE/AFTER region
is a later rung"); that reject is now **lifted**. No grammar change is needed —
`replace_item = (ALL|LEADING) operand BY operand inspect_region*` already parses per-item
regions.

**Semantics (ISO, exact composition of two shipped features).** ONE left-to-right pass over the
ORIGINAL source. Each item's window is computed over the original via the SAME `region_window`
helper the lone/single-item forms use: `BEFORE p`→`[0, first_index_of_p)`,
`AFTER p`→`(first_index_of_p, len]`, with the not-found asymmetry BEFORE→whole / AFTER→empty; an
item with NO region has the whole source as its window. At each position the items are tried IN
WRITTEN ORDER and the FIRST item that BOTH (i) contains the position in its window AND (ii) whose
single-char search equals the current ORIGINAL character WINS — the composition of multi-item
first-match-wins with the per-item region gate. First-match-wins and no-re-chaining are unchanged
(the scan always reads the original, never the produced char).

Worked (`PIC X(5)`):
- `"a0b0a"` with `ALL "a" BY "x" ALL "0" BY "*" BEFORE "b"` → `"x*b0x"` (both `a`s → `x`; only the
  `0` at index 1, inside `[0,2)`, → `*`; the `0` at index 3 is outside its window and stays).
- `"aXaXa"` with `ALL "a" BY "b" BEFORE "X" ALL "a" BY "c" AFTER "X"` → `"bXcXc"` (index 0 claimed
  by the earlier `BEFORE` item; indices 2,4 by the `AFTER` item).
- `"abab"` with `ALL "a" BY "*" AFTER "Z" ALL "b" BY "y"` → `"ayay"` (`AFTER "Z"` with `Z` absent
  is an EMPTY window, so that item never fires; the region-less item still rewrites the `b`s).

**Implementation.** The oracle's `Stmt::InspectReplacingMulti.items` becomes
`Vec<(Operand, Operand, Option<Region>)>` (the third slot is the per-item region);
`read_inspect_replacing_multi` reads each item's region with the same `read_inspect_region` the
single-item reader uses, keeping the LEADING/CHARACTERS/FIRST rejects. `exec_inspect_replacing_multi`
resolves every `(search, replace)` char pair AND its `[start, end)` window over the original
source BEFORE mutating storage, then rebuilds in one pass gating each item's compare by
`start <= i < end`. The compiler mirrors this: `ReplaceItem` carries the region node,
`inspect_replacing_multi` parses it with the same extraction the single-item `inspect_replacing_all`
uses, and `emit_inspect_replacing_multi` derives each item's `[start, end)` ONCE (before the
unrolled `0..W` pass) with the SAME `emit_inspect_region_window` the single-item region emitter
uses, then ANDs the position-in-window test into that item's `cmp_eq` link. The two engines' CST
readers count the same `replace_item` children, so their accept/reject sets stay co-total.

**Byte-safety.** The match only fires on a single-char ASCII search, so a multi-byte source char
never equals a search byte (never falsely matched), and each window is content-defined (bounded by
the first occurrence of an ASCII region delimiter), so the oracle (char indices) and the byte-based
compiler (byte indices) agree on which positions are inside. The MATCH side is byte-safe. The
RECONSTRUCTION of a source that itself contains a multi-byte char remains the PRE-EXISTING
byte-vs-char chip shared by EVERY `REPLACING` lowering: the compiler rebuilds the field with
per-position `str_slice`, which cannot slice a multi-byte char and traps — EXACTLY as the merged
single-item `REPLACING ALL` does on the same source (the oracle iterates `char`s and succeeds; this
divergence is not introduced here). This rung adds no new non-ASCII behavior.

**Scope kept for a later rung** (at the time of this rung): a `LEADING`, `CHARACTERS`, or `FIRST`
item in a multi-item list, and the combined `TALLYING … REPLACING` form with several items (the
`LEADING` and `CHARACTERS` items were lifted in later rungs — see below). The single-item
`REPLACING ALL … {BEFORE|AFTER}` path is untouched.

### Multi-item `TALLYING` with a per-item `{BEFORE|AFTER}` region (follow-up rung, v0.62.0 / v0.58.0)

The count-side analogue of the multi-item `REPLACING`-region rung above. A single-counter
multi-item `TALLYING` (`TALLYING C FOR ALL a ALL b …`) and the single-item
`TALLYING FOR ALL … {BEFORE|AFTER}` region rung now **compose**: each `ALL` delimiter item of a
multi-item `TALLYING` list may carry its OWN optional `{BEFORE|AFTER}` window. Previously any region
on a multi-item tally list was a later rung (rejected on both engines with "INSPECT TALLYING with
several items and a BEFORE/AFTER region is a later rung"); that reject is now **lifted**. No grammar
change is needed — `tally_item = (ALL|LEADING) operand inspect_region*` already parses per-item
regions.

**Semantics (ISO, exact composition of two shipped features).** ONE left-to-right pass over the
source. There is ONE counter (this variant is exactly one `tally_for` with ≥ 2 items; several
counters stays a later rung). Each item's window is computed over the source via the SAME
`region_window` helper the lone/single-item forms use: `BEFORE p`→`[0, first_index_of_p)`,
`AFTER p`→`(first_index_of_p, len]`, with the not-found asymmetry BEFORE→whole / AFTER→empty; a
region-less item has the whole source as its window. A position contributes 1 to the counter iff
SOME item in WRITTEN ORDER BOTH (i) contains the position in its window AND (ii) whose single-char
delimiter equals the current char — and the FIRST such item's match is enough (first-match-per-
position `break`, so duplicate/overlapping items never double-count). `INSPECT` **adds** to the
counter; it does not clear it.

Worked:
- `"aXaXa"` (`PIC X(5)`, `X` at 1,3) with `ALL "a" BEFORE "X" ALL "a" AFTER "X"` → `counter += 3`
  (index 0 by the `BEFORE` item's window `[0,1)`; indices 2,4 by the `AFTER` item's window `[2,5)`).
- `"0a0a0"` with `ALL "0" AFTER "a" ALL "a"` → `counter += 4` (the region-less `"a"` item counts the
  two `a`s; the `AFTER "a"` item's window `[2,5)` counts the `0`s at 2,4; the `0` at index 0 is
  outside its window and matches no item).
- `"abab"` with `ALL "a" AFTER "Z" ALL "b"` → `counter += 2` (`AFTER "Z"` with `Z` absent is an
  EMPTY window, so that item contributes 0; the region-less `"b"` item still counts the two `b`s).
- `"aabaa"` with `ALL "a" BEFORE "b" ALL "a"` → `counter += 4` (indices 0,1 counted ONCE by the
  earlier `BEFORE` item; indices 3,4 by the region-less item — a naive per-item sum 2+4=6 would
  wrongly double the shared positions 0,1).

**Implementation.** The oracle's `Stmt::InspectTallyMulti` field `delims: Vec<Operand>` becomes
`items: Vec<(Operand, Option<Region>)>` (the second slot is the per-item region);
`read_inspect_tally_multi` reads each item's region with the same `read_inspect_region` the
single-item reader uses, keeping the LEADING/CHARACTERS rejects. `exec_inspect_tally_multi` resolves
every `(delimiter char, [start, end) window)` over the source chars BEFORE touching the counter, then
counts positions matched by SOME in-window item (`any` realises first-match-per-position for a pure
count), and folds the count through the same numeric `store_result` path the single-item tally uses.
The compiler mirrors this: `TallyItem` carries the region node, `inspect_tally_multi` parses it with
the same extraction the single-item `inspect_tally_all` uses, and `emit_inspect_tally_multi`
materialises `str_len` ONCE, derives each item's `[start, end)` with the SAME
`emit_inspect_region_window` the single-item region emitter uses, then — in its runtime
`str_len`-bounded loop — gates each delimiter's `cmp_eq` link by `start ≤ j < end` against the
RUNTIME position register `j`. The two engines' CST readers count the same `tally_item` children, so
their accept/reject sets stay co-total.

**Non-ASCII-clean (a POSITIVE parity, NOT a trap).** Unlike `REPLACING`, `TALLYING` only **counts** —
it never reconstructs the source via `str_slice` — so there is NO UTF-8-boundary trap. Match-based
counting of ASCII delimiters is byte-robust (a multi-byte source char's continuation bytes never
equal an ASCII delimiter byte), and each window is content-defined (bounded by the first occurrence
of the ASCII region delimiter), so the char-based oracle and the byte-based compiler scan the SAME
substring and count the SAME matches EVEN ON A NON-ASCII SOURCE. Worked positive parity:
`"aé0b0"` with `ALL "0" BEFORE "b" ALL "0" AFTER "b"` → `counter += 2` on BOTH engines (the `é` and
its continuation byte match no ASCII delimiter; the two `0`s are counted, one per window). A
non-ASCII item/region delimiter *operand* stays the pre-existing `single_delim_char` vs
`single_delim_code` chip, identical across single- and multi-item tallying — no new one-sided guard.

**Scope kept for a later rung** (unchanged, identical messages on both engines): a
`CHARACTERS` item in a multi-item list; SEVERAL counters (more than one `tally_for`); and the
combined `TALLYING … REPLACING` form with several tally items. The single-item
`TALLYING FOR ALL … {BEFORE|AFTER}` path is untouched. (A `LEADING` item in a multi-item list —
which this rung still deferred — is now **lifted** by the follow-up rung "multi-item `TALLYING`
list with a `LEADING` item" below.)

### Several-counters `TALLYING` where each item carries a per-item `{BEFORE|AFTER}` region (follow-up rung, v0.63.0 / v0.59.0)

The multi-**counter** analogue of the multi-item `TALLYING`-region rung above. The several-counters
`TALLYING` form (`TALLYING c1 FOR ALL a [ALL b …] c2 FOR ALL d …`, two or more `tally_for` groups)
and the per-item `{BEFORE|AFTER}` region now **compose**: each `ALL` delimiter item of ANY counter
group may carry its OWN optional `{BEFORE|AFTER}` window. Previously any region in the multi-counter
path was a later rung (rejected on both engines with "INSPECT TALLYING with several counters and a
BEFORE/AFTER region is a later rung"); that reject is now **lifted**. No grammar change is needed —
`tally_item = (ALL|LEADING) operand inspect_region*` already parses per-item regions.

**Semantics (ISO COMBINED priority list ACROSS counters, now windowed).** ONE left-to-right pass. All
delimiters of all groups, flattened in WRITTEN ORDER (group 1's items first, then group 2's, …), form
ONE ordered priority list, each entry carrying its item's `[start, end)` window (computed over the
source via the SAME `region_window` helper the lone/single-item forms use; a region-less item = the
whole source, with the not-found asymmetry BEFORE→whole / AFTER→empty). At each position the flat
list is walked in order and the FIRST entry that is BOTH in-window AND whose single-char delimiter
equals the current char increments ITS OWN GROUP's accumulator, then `break`s (first-match-wins across
counters). A per-GROUP accumulator (not per counter NAME) keeps counts separate even when two groups
share a counter name; each group's accumulator is ADDED to its counter at the end. `INSPECT` **adds**;
it does not clear. Every counter must be an unsigned integer `PIC 9(n)` (validated first, before the
scan, so an invalid group aborts with every counter untouched).

Worked:
- `"aXaXa"` (`X` at 1,3) with `C1 FOR ALL "a" BEFORE "X"  C2 FOR ALL "a" AFTER "X"` → `C1 += 1`
  (index 0 in `[0,1)`), `C2 += 2` (indices 2,4 in `[2,5)`).
- `"0a0a0"` with `C1 FOR ALL "0" AFTER "a"  C2 FOR ALL "a"` → `C1 += 2` (the `0`s at 2,4 in `[2,5)`),
  `C2 += 2` (the two `a`s); index 0's `0` is outside C1's window and C2 does not match it.
- `"abab"` with `C1 FOR ALL "a" AFTER "Z"  C2 FOR ALL "b"` → `C1 += 0` (`AFTER "Z"` with `Z` absent is
  an EMPTY window), `C2 += 2`.
- `"aZa"` (`Z` at 1) with `C1 FOR ALL "a" BEFORE "Z"  C2 FOR ALL "a"` → `C1 += 1` (index 0), `C2 += 1`
  (index 2) — index 0 is STARVED from C2 by C1's in-window delimiter (else C2 would be 2).
- `"0b0"` with `C FOR ALL "0" BEFORE "b"  C FOR ALL "0" AFTER "b"` → the SAME counter `C += 2` (each
  group contributes 1 through its own window; both add to the one item).

**Implementation.** The oracle's `Stmt::InspectTallyCounters` groups type becomes
`Vec<TallyCounterGroup>` where `TallyCounterGroup = (String, Vec<TallyMultiItem>)` and
`TallyMultiItem = (Operand, Option<Region>)` (each group's items carry their own optional region);
`read_inspect_tally_counters` reads each item's region with the same `read_inspect_region` the
single-item reader uses, keeping the LEADING/CHARACTERS rejects. `exec_inspect_tally_counters` resolves
every `(group_index, delimiter char, [start, end) window)` over the source chars BEFORE the scan, then
per position walks the flat list and the first in-window match bumps `accs[group_index]` and breaks;
each per-group accumulator is added to its counter via the same numeric store path. The compiler
mirrors this: `inspect_tally_counters` returns `Vec<TallyCounterGroup>` (`= (String, Vec<TallyItem>)`)
parsing each item's region with the same extraction the single-item `inspect_tally_all` uses, and
`emit_inspect_tally_counters` materialises `str_len` ONCE, derives each flat entry's `[start, end)`
with the SAME `emit_inspect_region_window` the single-item region emitter uses, then — in its runtime
`str_len`-bounded loop — gates each delimiter's `cmp_eq` link by `start ≤ j < end` against the RUNTIME
position register `j` (a region-less item folds to `eq` alone). The two engines' CST readers count the
same `tally_for`/`tally_item` children, so their accept/reject sets stay co-total.

**Non-ASCII-clean (a POSITIVE parity, NOT a trap).** As for the multi-item rung: `TALLYING` only
**counts** — it never reconstructs the source via `str_slice` — so there is NO UTF-8-boundary trap.
ASCII delimiters never equal a multi-byte continuation byte, and each window is content-defined, so
the char-based oracle and the byte-based compiler count identically EVEN ON A NON-ASCII SOURCE. Worked
positive parity: `"aé0b0"` with `C1 FOR ALL "0" BEFORE "b"  C2 FOR ALL "0" AFTER "b"` → `C1 += 1`,
`C2 += 1` on BOTH engines (the `é` and its continuation byte match no ASCII delimiter). A non-ASCII
item/region delimiter *operand* stays the pre-existing `single_delim_char` vs `single_delim_code`
chip — no new one-sided guard.

**Scope kept for a later rung** (unchanged, identical messages on both engines): a `LEADING` or
`CHARACTERS` item in ANY group of the multi-counter path; and the combined `TALLYING … REPLACING`
form with several counters. This variant fires only for two or more `tally_for` groups; exactly one
group keeps the single-counter paths (`Inspect` / `InspectTallyMulti`) unchanged.

### Multi-item `TALLYING` list with a `LEADING` item (follow-up rung, v0.64.0 / v0.60.0)

The single-counter multi-item `TALLYING` list (`TALLYING C FOR … a … b …`, two or more `tally_item`s
under one `tally_for`) now accepts a `LEADING` item alongside `ALL` items — a MIX of `ALL` and
`LEADING`, each with its own optional `{BEFORE|AFTER}` region. Previously any `LEADING` item in a
multi-item list was a later rung (rejected on both engines with "INSPECT TALLYING with several items
and a LEADING item is a later rung"); that reject is now **lifted**. Only a `CHARACTERS` item in a
multi-item list, SEVERAL counters, and the combined `TALLYING … REPLACING` form with several items
remain later rungs. No grammar change is needed — `tally_item = (ALL|LEADING) operand inspect_region*`
already parses a per-item `LEADING` keyword.

**Semantics (single counter, per-item `active` run flags).** Resolve each item to
`(delim_char, leading, start, end)` where `[start, end)` is its window over the source
(`region_window`; a region-less item = the whole source `(0, len)`). ONE left-to-right pass over the
source positions carries a per-item `active` flag (only consulted for `LEADING` items, all init
`true`):

```text
count = 0;  active = [true; N]
for i in 0..len:
    c = chars[i]
    # tally decision: first ELIGIBLE item in WRITTEN ORDER, count once, then stop
    for k in 0..N:
        (d, leading, start, end) = resolved[k]
        in_win = start <= i && i < end
        if in_win && c == d && (!leading || active[k]): count += 1; break
    # then update EVERY LEADING item's run flag — INDEPENDENT of which item tallied:
    for k in 0..N:
        (d, leading, start, end) = resolved[k]
        if leading && start <= i && i < end && c != d: active[k] = false
counter := counter + count            # INSPECT ADDS; does not clear
```

The decisive subtleties (identical on both engines, or they diverge): (1) the `active` update is a
SEPARATE pass over ALL leading items AFTER the tally decision, breaking a run ONLY on an IN-WINDOW
`c != d` — **not** when another item claimed the position; a matching char keeps the run alive even
if a higher-priority item tallied it. (2) A `LEADING` item is eligible only while its `active` flag is
STILL `true` at position `i` (the char equals `d` AND every prior in-window position also equalled
`d`). (3) First-match-per-position: a position tallies at most once (the first eligible item), but the
active-update still runs for all leading items. (4) A region-less `LEADING` item anchors its run at
source position 0; a `LEADING` item WITH a region anchors it at its window start (composing with the
per-item regions of the rung above). Worked: `"aabab"` `FOR LEADING "a" ALL "b"` → the leading run
`"aa"` (breaks at the first `b`) gives 2, `ALL "b"` gives 2, and the `a` at index 3 is not counted
(run dead) → `4`. Worked (run anchored at window start): `"aaXaab"` `FOR LEADING "a" AFTER "X" ALL "b"`
→ leading window `"aab"` counts the two a's after the `X` (the two before are ignored) plus one `b` →
`3`. Worked (run survives a claim): `"aab"` `FOR ALL "a" LEADING "a"` → `ALL "a"` claims both a's
(count 2), the leading run stays alive through them and decays only at the `b`; the leading item never
tallies, count `2`.

**Compiler lowering.** The runtime scan loop of `emit_inspect_tally_multi` gains a per-`LEADING`-item
`active` register (i64, init `1`, allocated before the loop — the runtime-loop analogue of the
compile-time-unrolled `active` flag in the single-item `emit_inspect_replacing` LEADING lowering). In
the tally-decision chain a `LEADING` item's eligibility AND-gates on its `active` register; at the
per-position convergence label (reached by both a tally match and a no-match fall-through) EVERY
leading item's run is updated — `active := active AND eq` (region-less) or
`active := active AND (eq OR NOT in_win)` (windowed, so out-of-window positions never touch the run),
recomputing `eq`/`in_win` there because an early match `jmp` skips the later chain registers. This
mirrors the oracle's separate active-update pass exactly, so the compiled program matches the
tree-walk reference byte-for-byte.

**Non-ASCII-clean (a POSITIVE parity, NOT a trap).** As for the sibling tally rungs, `TALLYING` only
**counts** — it never reconstructs the source via `str_slice` — so there is NO UTF-8-boundary trap.
An ASCII delimiter never equals a multi-byte continuation byte, and a `LEADING` run breaks at the SAME
logical position on both engines: the char-based oracle breaks at the multi-byte char, the byte-based
compiler breaks at that char's FIRST byte (and its continuation bytes match nothing). Worked positive
parity: `"aaébb"` `FOR LEADING "a" ALL "b"` → leading `"aa"` (breaks at `é`) `+` `ALL "b"` `= 4` on
BOTH engines. A non-ASCII item/region delimiter *operand* stays the pre-existing `single_delim_char`
vs `single_delim_code` chip — no new one-sided guard.

**Scope kept for a later rung** (unchanged, identical messages on both engines): a `CHARACTERS` item
in a multi-item list; SEVERAL counters (more than one `tally_for`); and the combined
`TALLYING … REPLACING` form with several tally items. The single-item path (which already supports a
lone `FOR LEADING`, a region, and `CHARACTERS`) is untouched, and the several-counters path stays
`ALL`-only. The counter must remain an unsigned-integer `PIC 9(n)`.

### Multi-item `REPLACING` list with a `LEADING` item (follow-up rung, v0.66.0 / v0.70.0)

The exact **replace-side twin** of the multi-item `TALLYING`-with-`LEADING` rung above. The multi-item
`REPLACING` list (`REPLACING … a BY x … b BY y …`, two or more `replace_item`s in one clause) now
accepts a `LEADING` item alongside `ALL` items — a MIX of `ALL` and `LEADING`, each with its own
optional `{BEFORE|AFTER}` region. Previously any `LEADING` item in a multi-item REPLACING list was a
later rung (rejected on both engines with "INSPECT REPLACING with several items and a LEADING item is a
later rung"); that reject is now **lifted**. Only a `CHARACTERS`/`FIRST` item in a multi-item list, and
the combined `TALLYING … REPLACING` form with several items, remain later rungs. No grammar change is
needed — `replace_item = (ALL|LEADING) operand BY operand inspect_region*` already parses a per-item
`LEADING` keyword.

**Semantics (per-item `active` run flags — the SAME machine as the tally twin, but the decision loop
EMITS instead of counts).** Resolve each item to `(search_char, replace_char, leading, start, end)`
where `[start, end)` is its window over the ORIGINAL source (`region_window`; a region-less item = the
whole source `(0, len)`). ONE left-to-right pass over the original positions carries a per-item
`active` flag (only consulted for `LEADING` items, all init `true`):

```text
active = [true; N]                    # one per item; consulted for LEADING only
for i in 0..len:
    c = chars[i]
    out[i] = c                        # default: unchanged
    # decision: first ELIGIBLE item in WRITTEN ORDER, emit its replacement, then stop
    for k in 0..N:
        (search, replace, leading, start, end) = resolved[k]
        in_win = start <= i && i < end
        if in_win && c == search && (!leading || active[k]): out[i] = replace; break
    # then update EVERY LEADING item's run flag — INDEPENDENT of which item won:
    for k in 0..N:
        (search, _, leading, start, end) = resolved[k]
        if leading && start <= i && i < end && c != search: active[k] = false
source := out                         # exactly len chars, width unchanged
```

The ONLY difference from the tally twin is the decision line: instead of `count += 1` it produces the
replacement char at position `i` (keeping the original on no match). The run-update pass is IDENTICAL.
The scan reads the ORIGINAL `chars` (never the output) — the no-re-chaining property, exactly like the
existing multi-item REPLACING. The decisive subtleties (identical on both engines): (1) the `active`
update is a SEPARATE pass over ALL leading items AFTER the decision, breaking a run ONLY on an
IN-WINDOW `c != search` — **not** when another item claimed the position; a matching char keeps the run
alive even if a higher-priority item won it. (2) A `LEADING` item is eligible only while its `active`
flag is STILL `true`. (3) First-match-wins: a position is replaced by at most one item, but the
active-update still runs for all leading items. (4) A region-less `LEADING` item anchors its run at
source position 0; a `LEADING` item WITH a region anchors it at its window start. Worked (`PIC X`):
- `"aabaa"` `REPLACING LEADING "a" BY "X" ALL "b" BY "Y"` → `"XXYaa"` (leading run of `a` at 0,1 → `X`;
  the `b` → `Y`; the `a`s at 3,4 are past the dead run and stay).
- `"aab"` `REPLACING LEADING "a" BY "X" ALL "a" BY "Y"` → `"XXb"` (the LEADING item claims 0,1;
  first-match-wins means the duplicate `ALL "a"` never sees them).
- `"Xaa"` `REPLACING ALL "X" BY "Q" LEADING "a" BY "Z"` → `"Qaa"` (the higher-priority `ALL "X"` wins
  index 0, whose char breaks the LEADING `a` run in the SEPARATE update pass, so the `a`s at 1,2 are
  NOT replaced — a fold-into-decision bug would give `"QZZ"`).
- `"aaZbb"` `REPLACING LEADING "a" BY "X" BEFORE "Z" LEADING "b" BY "Y" AFTER "Z"` → `"XXZYY"` (two
  independent per-item run flags, each anchored at its own window start).

**Implementation.** The oracle's `Stmt::InspectReplacingMulti.items` becomes
`Vec<ReplaceMultiLeadingItem>` (`= (Operand, Operand, bool, Option<Region>)`, the replace twin of
`TallyMultiLeadingItem`); `read_inspect_replacing_multi` reads each item's `LEADING` keyword the same
way `read_inspect_tally_multi` does, keeping the CHARACTERS/FIRST rejects. `exec_inspect_replacing_multi`
resolves every `(search, replace, leading, window)` over the original source BEFORE mutating storage,
then rebuilds in one pass with the two-loop active-flag machine above. The compiler mirrors this:
`ReplaceItem` gains a `leading` bool, a new `ResolvedReplaceLeadingItem` alias carries the
search/replace registers plus `leading`/`active`/`window`, and `emit_inspect_replacing_multi` allocates
a per-`LEADING`-item `active` register (i64, init 1) before the compile-time-unrolled `0..W` pass. In
the decision chain a `LEADING` link AND-gates on `active`; at the per-position `done` convergence label
(reached by both a match `jmp` and the no-match fall-through) EVERY leading item's run is updated —
`active := active AND eq` (region-less) or `active := active AND (eq OR NOT in_win)` (windowed) —
recomputing `eq`/`in_win` there because an early match `jmp` skips the later chain registers, exactly
as the tally side's `cont` section does. The two engines' CST readers count the same `replace_item`
children, so their accept/reject sets stay co-total.

**Byte-safety.** Identical to the multi-item REPLACING-region rung: the match fires only on a
single-char ASCII search (a multi-byte source char is never falsely matched), and each window is
content-defined. But because REPLACING RECONSTRUCTS the source, a source that itself contains a
multi-byte char remains the PRE-EXISTING byte-vs-char chip (`task_396ba6f6`) shared by EVERY `REPLACING`
lowering: the byte-based compiler rebuilds with per-position `str_slice` and traps, while the char-based
oracle succeeds. This rung adds NO new non-ASCII divergence (its non-ASCII test is a characterization
test pinning both engines, not `assert_matches_oracle`).

**Scope kept for a later rung** (unchanged, identical messages on both engines): a `FIRST`
item in a multi-item list, and the combined `TALLYING … REPLACING` form with several items. The
single-item path (which already supports a lone `REPLACING LEADING`, a region, and `CHARACTERS`) is
untouched.

### Multi-item `REPLACING` list with a `CHARACTERS` item (follow-up rung, v0.81.0 / v0.77.0)

The REPLACE twin of "A `CHARACTERS` item in a multi-item list" on the tally side. A
`CHARACTERS BY x` item may now appear ALONGSIDE other items in a multi-item `REPLACING`
list — e.g. `REPLACING ALL "A" BY "B" CHARACTERS BY "*"`,
`REPLACING CHARACTERS BY "*" ALL "A" BY "B"`,
`REPLACING ALL "A" BY "B" CHARACTERS BY "*" BEFORE "X"`. The former reject in
`read_inspect_replacing_multi` / `inspect_replacing_multi` ("INSPECT REPLACING CHARACTERS is
a later rung") is lifted for THIS path only. In the same ordered first-match-per-position
rebuild (the one #71 extended to mix `ALL` and `LEADING`), a `CHARACTERS` item is the
**always-eligible catch-all**: at each position it is eligible iff the position is in its
window (`in_win`) — NO search compare, NO active-run tracking, and it carries NO search
operand. So it EMITS its replacement char at every in-window position not already claimed by
an EARLIER item in written order. Written order is honoured: a region-less `CHARACTERS` item
shadows every item written after it (`REPLACING CHARACTERS BY "*" ALL "A" BY "B"` over
`"AABB"` → `"****"`, the ALL never fires), while `REPLACING ALL "A" BY "B" CHARACTERS BY "*"`
over `"AXAY"` → `"B*B*"` (ALL "A" claims its positions, CHARACTERS the rest). An optional
`{BEFORE|AFTER} z` region narrows the catch-all's window exactly like any other item, via the
SAME `region_window` / `emit_inspect_region_window` machinery — and a char PAST that window is
still claimed by a trailing `ALL` item.

The item type gains an explicit kind so the illegal "LEADING and also CHARACTERS" state is
unrepresentable: the oracle's `ReplaceMultiLeadingItem` becomes
`(Option<Operand>, Operand, ReplaceMultiKind, Option<Region>)` (`ReplaceMultiKind` =
`All`/`Leading`/`Characters`; the SEARCH operand is `None` for `Characters`, the REPLACE
operand always present), and the compiler mirrors it with a local `ReplaceKind`.
`exec_inspect_replacing_multi` / `emit_inspect_replacing_multi` resolve a `CHARACTERS` item's
replacement + window only (no `single_delim_char` / `single_delim_code` for a search), gate
its eligibility on `in_win` alone — a region-less `CHARACTERS` item emits an unconditional
"append replacement + jump to the position's done label" with no predicate, shadowing later
links — and skip it in the leading active-run update. The combined `TALLYING … REPLACING`
form KEEPS rejecting a `CHARACTERS` REPLACING half (that gate lives in the single-item
`read_inspect_replacing_all`, reached via the combined caller). **Byte-vs-char reconstruct
chip (task_396ba6f6, unchanged):** a `CHARACTERS` item rebuilds POSITIONS, and — like EVERY
`REPLACING` lowering — the byte-based compiler rebuilds with per-position `str_slice` while
the char-based oracle iterates CHARs, so a non-ASCII source that keeps a multi-byte char
diverges (compiler traps, oracle succeeds), deliberately NOT fixed here (positive tests are
ASCII; one characterization test pins the shared-chip divergence).

### Combined `INSPECT … TALLYING … REPLACING` (one statement)

A single `INSPECT` may carry **both** phrases:
`INSPECT source TALLYING counter FOR {ALL|LEADING} delim REPLACING {ALL|LEADING} x
BY y`. Per ISO the statement executes "**as though an `INSPECT TALLYING` were specified,
followed by an `INSPECT REPLACING`**" — so the order is fixed:

1. **Tally first** — count occurrences of `delim` in the **ORIGINAL** `source` and
   **ADD** them to `counter` (the tally does not modify the source). The tally half
   may be `FOR ALL` (count every occurrence) or `FOR LEADING` (count only the
   consecutive run of `delim` at the start, stopping at the first non-match).
2. **Then replace** — substitute `x` with `y` in the source. The replace half may be
   `REPLACING ALL` (substitute every `x`) or `REPLACING LEADING` (substitute only the
   consecutive run of `x` at the start, stopping at the first non-`x`). The two
   halves' leading flags are **independent** — either, both, or neither may be
   `LEADING`.

This tally-before-replace order is observable when `delim == x`: the count must
see every occurrence in the pre-replacement bytes. Worked (`delim == x == "S"`):
`"MISSISSIPPI"` TALLYING `S` → `counter += 4`, then `S → Z` → `"MIZZIZZIPPI"`. Had
the count run after the replace it would have seen zero `S` — so the `4` proves
the ordering. Worked `FOR LEADING` (`delim == x == "0"`): `"000X0"` TALLYING
`FOR LEADING "0"` → `counter += 3` (the leading run stops at `X`, so the trailing
`0` is **not** counted — `FOR ALL` would give 4), then `REPLACING ALL "0" BY "*"` →
`"***X*"` (the replace still rewrites every `0`). Worked `REPLACING LEADING`
(`delim == x == "0"`): `"00X00"` TALLYING `FOR ALL "0"` → `counter += 4` (all four
zeros), then `REPLACING LEADING "0" BY "*"` → `"**X00"` (only the leading run of two
is rewritten; the count still saw all four). Worked **both halves leading**:
`"00X00"` TALLYING `FOR LEADING "0"` → `counter += 2`, then `REPLACING LEADING` →
`"**X00"`.

Both the oracle and the compiler compose their two existing single-phrase
lowerings in this exact order on the same source, so the compiled program and the
reference agree byte-for-byte. No grammar change was needed — the grammar already
accepted `inspect_tallying [ inspect_replacing ]`. The `TALLYING` half accepts
`FOR ALL`/`FOR LEADING` (reusing the lone-TALLYING leading-run count/break) and the
`REPLACING` half independently accepts `ALL`/`LEADING` (reusing the lone-REPLACING
`active` run flag): a combined statement whose either half is a deferred sub-form
(`CHARACTERS`, several counters/FOR/replace items, multi-char/
wider/numeric operands — a **figurative constant** SPACE/ZERO operand is now
accepted through the shared single-character helpers, reduced to its single ASCII
character — a numeric/group source, or a non-integer
counter) remains a clean later rung. (A lone `FOR LEADING` tally, a lone
`REPLACING LEADING`, and every combination of leading/`ALL` on the two combined
halves are each supported.)

**`BEFORE`/`AFTER` region per half (combined follow-up rung).** Each half of the
combined form independently accepts its OWN `{BEFORE|AFTER}` **region** — the region
that shipped for the lone `TALLYING FOR ALL` and `REPLACING ALL` phrases, now allowed
on the combined `FOR ALL` count half and the combined `REPLACING ALL` replace half at
once. The two regions are fully INDEPENDENT: the `TALLYING` half may carry its own
`{BEFORE|AFTER} x1`, the `REPLACING` half its own `{BEFORE|AFTER} x2` — either, both,
or neither, with their own kind and delimiter. Each half's window `[start, end)` is
computed by the SAME shared helper (`region_window` in the oracle,
`emit_inspect_region_window` in the compiler) over the ORIGINAL source, with the same
leftmost-first-index and BEFORE→whole / AFTER→empty not-found asymmetry the lone forms
use; only positions inside a half's window are counted / substituted, and positions
outside are untouched. Because the tally does NOT mutate the source, BOTH windows are
derived over the SAME original bytes — the ISO tally-then-replace order is preserved,
and a shared `delim == x` (even with a shared region delimiter) is still counted
before it is substituted. Worked (tally region only): `"AB0CD0"` TALLYING
`FOR ALL "0" BEFORE "C"` → region `"AB0"` → `counter += 1`, then the region-less
`REPLACING ALL "0" BY "*"` → `"AB*CD*"`. Worked (different kinds per half):
`"0A0B0"` TALLYING `FOR ALL "0" BEFORE "B"` → region `"0A0"` → `2`, then
`REPLACING ALL "0" BY "*" AFTER "B"` → region `"0"` (trailing) → `"0A0B*"`. Worked
(both not-found): `"0A0"` TALLYING `FOR ALL "0" AFTER "Z"` → empty → `0`, then
`REPLACING ALL "0" BY "*" BEFORE "Z"` → whole source → `"*A*"`. That rung was scoped
SMALL: `FOR ALL` / `REPLACING ALL` only.

**`BEFORE`/`AFTER` region on a combined LEADING half (this rung).** The last
combined-form deferral is now LIFTED: a **LEADING** half (tally and/or replace) may
ALSO carry its own `{BEFORE|AFTER}` region. This is a pure reject-lift — BOTH engines
already own the full LEADING+region machinery per half (the standalone
`FOR LEADING`/`REPLACING LEADING … {BEFORE|AFTER}` routines), and the combined
exec/emit already COMPOSE those same routines in ISO order (tally FIRST over the
ORIGINAL bytes, THEN replace). Only the combined caller's re-imposed guards deferred
the combination: the oracle's combined `read_statement` arm dropped its two
`leading && *_region.is_some()` rejects, and the compiler now passes
`allow_leading_region = true` to both `emit_inspect_tallying` and
`emit_inspect_replacing` (previously `false`). A LEADING half with a region anchors
its run at the window start, exactly as the standalone form does. Worked (LEADING
tally + region): `"00A0B"` TALLYING `FOR LEADING "0" BEFORE "A"` → window `"00"` →
`counter += 2`, then region-less `REPLACING ALL "0" BY "*"` → `"**A*B"`. Worked
(LEADING replace + region): `"00A0B"` TALLYING `FOR ALL "0"` → `3`, then
`REPLACING LEADING "0" BY "*" BEFORE "A"` → window `"00"` → `"**A0B"`. A
multi-character region delimiter and a `CHARACTERS` half remain clean later rungs
rejected identically on both engines. The shared single-delimiter check still rejects
a wider-than-one region delimiter. No grammar change was needed — the grammar already
accepts a region on each phrase.

### `INSPECT … CONVERTING` (first rung)

`INSPECT` has a third, distinct form that neither counts nor does a search-and-
replace, but **translates** each character through a table:
`INSPECT source CONVERTING from TO to`.

- `source` is the alphanumeric (`PIC X`) item, **modified in place**.
- `from` and `to` are each a **string literal**, a **data-name** (`PIC X` item), a
  **CONSTANT reference modification** `base(start:len)` / `base(start:)` (both
  indices LITERALS) — a *variable* table — or a **figurative constant** SPACE / ZERO,
  of **EQUAL length**. A figurative reduces to the single-character literal `" "`
  (0x20, for SPACE / SPACES) or `"0"` (0x30, for ZERO / ZEROS / ZEROES) — a length-1
  ASCII literal — so it takes the **entire** string-literal path unchanged (the
  equal-length check and the ASCII-literal guard both already handle it), converting
  when paired with a length-1 operand and deferring via the existing equal-length
  reject otherwise. A data-name's set is
  its CURRENT storage (its declared width in characters); a const refmod's set is its
  slice, whose length is static (the const `len`, or `base_width - start + 1` when
  omitted), so the equal-length requirement is checked at compile time whichever mix
  of literal / item / const-slice / figurative the two sides are. Any side may alias the source; a
  `from`/`to` that ALIASES the source is read BEFORE the rewrite, so it sees the
  source's ORIGINAL bytes (the oracle resolves the operand — reading item storage or
  slicing the refmod via the shared `refmod_string` — into the table up front; the
  compiler hoists the loop-invariant `str_index`/`str_slice` reads, and for a const
  refmod materialises the slice via the shared `ref_mod_slice`, out of the
  per-position loop). A CONST refmod reduces EXACTLY to the data-name case — its slice
  register is a fixed-width alphanumeric string register. A numeric/group item as
  `from`/`to`, a **numeric literal** `from`/`to`, and a **COMPUTED** (data-name index)
  reference-modified `from`/`to` remain later rungs — the computed refmod deferred
  co-totally on both engines by the same const-index predicate the MOVE/STRING refmod
  rungs use (`matches!(start, Lit) && len.is_none_or(|l| matches!(l, Lit))`); a
  numeric refmod base is rejected identically by `refmod_string`/`ref_mod_slice`.

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

**`BEFORE`/`AFTER` region (CONVERTING follow-up rung).** `CONVERTING` now accepts an
optional `{BEFORE|AFTER} z` **region** that restricts the translation to the sub-slice
of the source bounded by the FIRST (leftmost) occurrence of the single-character
region delimiter `z` — the exact analogue of the `TALLYING FOR ALL` and `REPLACING
ALL` regions applied to the translation:

- `BEFORE z` translates through the table only in `source[0 .. first_index_of(z)]`;
  if `z` is **absent** the region is the **ENTIRE** source (whole-source translate).
- `AFTER z` translates only in `source[first_index_of(z)+1 .. end]`; if `z` is
  **absent** the region is **EMPTY** (nothing converted).

Positions **outside** the region keep their **original** character, even if that
character appears in the `from` set. The window is computed over the ORIGINAL source
and is byte-identical to the one the count and replacement use — the oracle calls the
same shared `region_window` helper, and the compiler reuses `emit_inspect_region_window`
and, at each position outside the window, jumps past the table chain to keep the
original char (translating only when `start <= j < end`). Worked: table `A→0`;
`BEFORE "Y"` in `"AXAYA"` → region `"AXA"` → `"0X0YA"`; `AFTER "Y"` → region `"A"`
(trailing) → `"AXAY0"`; `BEFORE "Z"` (absent) → whole source → `"0X0Y0"`; `AFTER "Z"`
(absent) → empty → `"AXAYA"` unchanged; `AFTER "A"` (region delimiter also in the
`from` set) → the first `A` bounds the region to `"XAYA"` → `"AX0Y0"` (the leading `A`
is left of the region and kept). This rung is scoped SMALL: a **single-character**
region delimiter. With no region the lowering is unchanged.

`CONVERTING` is a **standalone** alternative — it is never combined with
`TALLYING`/`REPLACING` in one statement (a combined form does not parse). Later
rungs (clean `Unsupported`): an **unequal-length** `from`/`to` pair (now including
item widths and const-slice lengths), a non-ASCII **literal** `from`/`to`, a
**numeric/group item** as `from`/`to`, a **numeric-literal** `from`/`to`, a **COMPUTED**
(data-name index) reference-modified `from`/`to`, a **multi-character** region
delimiter, and a numeric/group source. A `PIC X` **item** `from`/`to`, a
**CONSTANT** reference-modified `from`/`to` (`S(2:3)` / `S(2:)`), and a **figurative
constant** `from`/`to` (SPACE / ZERO, mapped to `" "` / `"0"`) are now supported; a
non-ASCII byte in an item's or slice's runtime storage is the pre-existing
byte-vs-char operand chip (the ASCII case is byte-identical on both engines).
SPACE and ZERO are the only figuratives in the model — there is no
QUOTE/LOW-VALUE/HIGH-VALUE — so the figurative case has nothing further to defer.

Grammar scope tracks the lexer scope below.

### Constant reference-modified single-character delimiter / search / replace operand (follow-up rung, v0.78.0 / v0.74.0)

Wherever a **single-character** operand is taken through the shared delimiter
helpers — a `DELIMITED BY` delimiter (STRING, UNSTRING), an `INSPECT TALLYING
FOR ALL` delimiter, and an `INSPECT REPLACING ALL/LEADING x BY y` search char
*and* replace char — the operand may now be a **CONSTANT reference modification**
`base(start:len)` whose **literal** indices carve a slice of length exactly 1.
That length-1 slice IS a single ASCII character, so it reduces to the same
single-char path the 1-char literal, `PIC X(1)` item, and figurative-constant
(SPACE/ZERO, the prior v0.77.0 / v0.73.0 rung) operands already take. This
completes the delimiter/search/replace **operand-class arc** (literal, item,
figurative, refmod). No grammar change is needed — `operand` already carries an
optional `(start:len)` suffix.

**One helper family, lifted co-total.** The oracle uses ONE shared helper
(`single_delim_char`); the compiler splits by use (`single_delim_code` yields the
i64 scan byte, `single_delim_str` yields the 1-char replace string). All three had
their `RefMod` arm lifted by reusing the SAME machinery every other refmod context
uses: the oracle's `refmod_string`, the compiler's `ref_mod_slice`. The oracle
reconstructs the slice through `refmod_string` and matches its single char (`[c]`);
`single_delim_code` takes `str_index(slice, 0)` of the length-1 `SliceLen::Const(1)`
slice; `single_delim_str` hands back the length-1 slice register directly (it IS the
1-char string). The `Const`/`Runtime` split of `ref_mod_slice` is co-total with the
oracle's `const_ix` predicate (`start` literal, `len` literal-or-omitted) — the same
split #74's CONVERTING refmod established — so both engines accept and reject the
identical set of programs.

**Deferred as clean later rungs (co-total on both engines):** a **computed**
(data-name index) reference modification `base(J:K)` — a run-time slice length the
compile-time contract cannot carry (`SliceLen::Runtime` in the compiler; the
`const_ix` predicate is false in the oracle) → "a computed reference-modified
delimiter is a later rung"; a constant refmod of slice-length **≠ 1** → a
multi-character delimiter; a **numeric base** under refmod → the pre-existing
`refmod_string` / `ref_mod_slice` numeric-base reject; a **group base** under refmod
→ rejected on both engines (the compiler rejects it via `item_index`; the oracle's
`single_delim_char` rejects a group base up front rather than slicing `group_image`,
so the site stays co-total — an UNDECLARED name still falls through to the shared
`UndefinedName`); the `RefModOutOfRange` bounds trap for an out-of-range constant
slice is inherited unchanged.

**Byte-vs-char.** The compiler reconstructs the slice from BYTES, the oracle from
CHARACTERS; they coincide on an **ASCII** base. A **non-ASCII** base is the
PRE-EXISTING refmod byte-vs-char chip (shared with `DISPLAY` / comparison / MOVE
source), not new to this rung — the positive parity tests use ASCII bases, and a
characterization test keeps the multi-byte char strictly OUTSIDE the length-1 window
so both engines pick the same ASCII byte/char.

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

### `MOVE` (cross-category: alphanumeric → numeric — signed or unsigned, integer or scaled)

The **reverse** cross-category rung is `MOVE alphanumeric-item TO numeric-item`,
for an alphanumeric source (`PIC X(m)`) into a numeric receiver
`PIC [S]9(i)V9(d)` — **signed or unsigned**; `d` may be `0` (an integer receiver)
or `> 0` (a **scaled** receiver). With the signed receiver handled (v0.65.0 /
v0.69.0), the **Char↔Numeric MOVE matrix is now complete** — both directions and
both signednesses.

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
go negative, but the source has **no operational sign**, so both engines keep the
**magnitude** (the compiler `abs`es before `mod`; the oracle uses `unsigned_abs`) —
no stray `'-'` — for a signed *or* unsigned receiver alike.

**Signed receiver (positive magnitude; DISPLAY overpunch).** A signed receiver
(`PIC S9(i)V9(d)`) is handled by the **same** fold and store path — the guard is
merely relaxed to accept any numeric receiver. Because an alphanumeric source has
no operational sign, the stored value is **always positive**: the compiler
`emit_abs`es the fold *before* `store_scaled` (so `reapply_sign`, which would
otherwise re-apply the negative sign of a SPACE-driven fold, becomes a genuine
no-op), and the oracle builds `Decimal { neg: false }`. `DISPLAY` of the signed
field then overpunches the units digit on its **positive** row (`{A…I` for units
0-9), via the same `overpunch_trailing`/`item_image` path signed `DISPLAY` and the
signed→alphanumeric `MOVE` already use. Worked:

- `PIC X(3)="123"` → `PIC S9(3)` → magnitude `123`, `DISPLAY` **`12C`** (units 3 → `C`).
- `PIC X(3)="120"` → `PIC S9(3)` → `DISPLAY` **`12{`** (units 0 → `{`).
- `PIC X(1)=" "` (SPACE) → `PIC S9(3)` → magnitude `016`, positive, `DISPLAY` **`01F`** (no stray sign).
- `PIC X(3)="042"` → `PIC S9(2)V9` → slot `042`, `DISPLAY` **`04B`** (units 2 → `B`).

**Non-ASCII byte-vs-char.** The fold is byte-based on both engines
(`chars.bytes()` in the oracle, `emit_str_to_int` over bytes in the compiler), but
the compiler folds the item's `width()` = **char-count** leading bytes while the
oracle folds the **full byte storage** of the char-padded value. For a source with
a multi-byte character these read a different number of bytes, so the two engines
already disagree — **identically on the unsigned and signed paths** (the signed
relaxation does not touch this). This is the pre-existing byte-vs-char chip,
deferred; a dedicated `jit_e2e` test pins both engines' outputs to document it.

Deferred as clean later rungs (rejected identically on both engines): an **edited**
numeric receiver, a **group** item on either side, and a **source wider than 18
characters** (whose `i64` fold could overflow — an all-digit source of ≤ 18 chars
stays below `10^18 < i64::MAX`).

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

### Level-88 condition-name on an alphanumeric item (read + `SET … TO TRUE`, v0.68.0 / v0.64.0)

A `88` condition-name registers a boolean over the immediately preceding item — its
**conditional variable** — that holds when the variable equals any listed `VALUE`
or falls within any inclusive `THRU` range. The earlier rung implemented this for a
**numeric** conditional variable in both directions: reading it (`IF IS-DONE`) and
setting it (`SET IS-DONE TO TRUE`, which assigns the first value / a range's low
bound). This rung lifts both directions for an **alphanumeric** (`PIC X`) conditional
variable, for **string VALUEs** — discrete strings AND inclusive `THRU` **ranges whose
bounds are string literals**:

```
01  GRADE  PIC X VALUE "C".
    88  PASSING  VALUE "A" THRU "D".
01  FLAG  PIC X VALUE "N".
    88  IS-YES  VALUE "Y".
    88  IS-NO   VALUE "N".
```

`IF IS-NO` is true (`FLAG` = `"N"`), `SET IS-YES TO TRUE` stores `"Y"`, and `IF IS-YES`
is then true. Multiple discrete values OR-fold: `88 VOWEL VALUE "A" "E" "I"` holds when
`FLAG` is any of them, and `SET VOWEL TO TRUE` assigns the first (`"A"`). A string range
`88 PASSING VALUE "A" THRU "D"` holds inclusively for `GRADE` in `"A".."D"`, and
`SET PASSING TO TRUE` assigns the range's LOW bound `"A"`. Ranges and discrete strings
mix and OR-fold: `88 OK VALUE "A" THRU "C" "Z"` holds for the range or the discrete `"Z"`.

- **Read.** When every VALUE item is a string literal or a string range, the name holds
  when the variable matches ANY of them under COBOL's **alphanumeric (byte) comparison** —
  the *same* space-padded byte compare an `IF var = "…"` / `IF var >= "…"` relation runs —
  OR-folded over the values. A discrete string `s` holds when `var == s`; an inclusive
  range `lo THRU hi` holds when `var >= lo` AND `var <= hi`. A VALUE shorter than the
  field is space-padded to the field width, so `88 IS-Y VALUE "Y"` matches
  `FLAG PIC X(3)` holding `"Y  "`. The oracle routes the variable and each bound through
  `compare_operands` (its alphanumeric arm); the compiler emits a `cmp_eq` (discrete) or
  `and(cmp_ge, cmp_le)` (range) over the same `str_cmp` / space-pad path
  (`emit_str_condition`) an alphanumeric `IF` uses, OR-folded with `or`. Reusing that
  shared machinery is what makes the read byte-identical between the engines.
- **Set.** `SET cond-name TO TRUE` stores the FIRST value's string into the slot exactly
  as `MOVE "…" TO item` — a discrete string, or a range's LOW bound: the oracle's
  `move_into` (via `src_from_lit` → `Src::Chars`) and the compiler's `format_into_picture`
  → slot `str_const`, both fitting the string to the receiver width by the ordinary
  alphanumeric rule.
- **Accept predicate (co-total).** Accepted **iff** the conditional variable is
  alphanumeric AND every VALUE item is a string `Single(Str)` OR a `Range(Str, Str)`
  (both bounds string literals) — the same `all_str_values` check on both engines, so
  they accept and reject the very same programs.

Deferred as clean later rungs (rejected identically on both engines): a `THRU` range
with a **non-string bound** (`88 X VALUE "A" THRU 5`), a **numeric or figurative** VALUE
on an alphanumeric 88 (`88 X VALUE 5`, `88 X VALUE SPACES`), a **mixed string/numeric**
list, plus a `88` over a **group** conditional variable. A `88` over an **unnamed** (`FILLER`) conditional variable
(`01 FILLER PIC X. 88 IS-B VALUE "B".`) is likewise a later rung, rejected at
build/collect time on both engines: the compiler does not model FILLERs in its item
table (so the 88 would bind to the wrong, last-named item), while the oracle does, so
this reject closes the divergence for both the alphanumeric and the (pre-existing
latent) numeric FILLER-88 case. A `88` that follows a FILLER *and then a named item*
binds to the named item and is accepted. The numeric level-88 paths are unchanged. The comparison and
store are ASCII-clean (byte = char); a non-ASCII string VALUE or runtime value is the
pre-existing alphanumeric byte-vs-char behavior inherited from the IF-alphanumeric
path, not introduced here.

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
| `UNSTRING` multi-char / `ALL` / `OR` delimiters, a signed/fractional/non-numeric/over-wide `WITH POINTER` item, multiple `DELIMITED` fields, `COUNT`/`DELIMITER IN`/`TALLYING`, a NUMERIC-literal / NON-ASCII source, a NUMERIC-base reference-modified source | Later rungs beyond the single-character `DELIMITED BY delim INTO r1 [r2 …]` cut (an ASCII alphanumeric string-literal source, a FIGURATIVE SPACE/ZERO source mapped to its single-character image, an alphanumeric-base reference-modified source `S(2:3)`, literal or computed index, a `WITH POINTER p` phrase over a `PIC 9(n)` unsigned-integer pointer, AND `ON OVERFLOW` / `NOT ON OVERFLOW` handlers ARE now supported) |
| `INSPECT`, other string verbs | The rest of the string-handling verb family |
| IR / interpreter | Run a COBOL program; out of scope for the frontend |

[PL06]: PL06-flow-matic.md
