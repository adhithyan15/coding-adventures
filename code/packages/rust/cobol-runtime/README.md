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

## Scope (v0.1)

A small but **fully correct** slice: unsigned numeric-display (`9`/`V`) and
character (`X`/`A`) pictures; the item tree from level numbers; `VALUE`
initialisation; figurative `ZERO`/`SPACE`; `MOVE` with the exact
justify/pad/truncate rules; `DISPLAY`; `STOP RUN`. Anything not yet modelled
(signed `S`, editing pictures, `COMP`, arithmetic, `PERFORM`, tables, files, and
every other verb) returns a descriptive `RuntimeError` — never wrong output. See
PL08 for the roadmap toward full COBOL and later standards.
