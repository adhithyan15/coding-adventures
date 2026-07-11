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
                   | if_stmt | goto_stmt | stop_stmt | … ;
move_stmt          = "MOVE" operand "TO" NAME { NAME } ;
add_stmt           = "ADD" operand { operand } "TO" NAME [ "GIVING" NAME ] ;
stop_stmt          = "STOP" ( "RUN" | NUMBER ) ;
```

Grammar scope tracks the lexer scope below.

## Scope

This spec's implementation stops at the **frontend** (lex + parse to a CST). No
IR, no interpreter. The goal is to stand up COBOL's structural machinery —
column format, four divisions, level/PICTURE data description, English-verb
procedure — reusing the FLOW-MATIC frontend and the shared hook API.

The first cut targets the **demonstrated language**: a complete four-division
program with `WORKING-STORAGE` items (levels, core PICTUREs, `VALUE`), a handful
of the most common verbs (`MOVE`, `ADD`/`SUBTRACT`/`MULTIPLY`/`DIVIDE …
GIVING`, `DISPLAY`, `PERFORM`, `GO TO`, `IF`, `STOP RUN`), and the
column-strip/continuation/comment handling. The long tail (full ENVIRONMENT/FILE
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
| IR / interpreter | Run a COBOL program; out of scope for the frontend |

[PL06]: PL06-flow-matic.md
