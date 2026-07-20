# PL09 — FLOW-MATIC & COBOL code generation: IIR (execution) and SIR (transpilation)

## Goal

Give **FLOW-MATIC** (PL06) and **COBOL-60** (PL07/PL08) two new lowering paths so
they stop being tree-walk-only interpreters and instead:

1. **Run on every execution backend** by compiling to **IIR**
   (`interpreter-ir`) — the register/label bytecode consumed by NativeAOT, LLVM,
   WASM, JVM, CLR, the VM, the JIT, and the retro-CPU targets.
2. **Transpile to other languages** by compiling to **Semantic IR**
   (`semantic-ir`) — the high-level IR emitted as Python / JavaScript / Go /
   Rust / Ruby / C / TypeScript by the seven existing `semantic-ir-to-*`
   backends.

Neither path requires touching the 17 downstream backends: the work is **one
frontend crate per (language, pipeline)** — four new crates total —
`flow-matic-iir-compiler`, `cobol-iir-compiler`, `flow-matic-to-semantic-ir`,
`cobol-to-semantic-ir`, each exposing
`compile_source(&str, &str) -> Result<Module, Error>`.

## The two pipelines (both already proven for other languages)

```
                      ┌────────────────────────── IIR (run everywhere) ─────────────────────┐
source ─▶ parser CST ─┤ <lang>-iir-compiler ─▶ interpreter_ir::IIRModule ─▶ iir-to-{wasm,   │
                      │   (this spec)             (validate + lang-aot)       llvm,jvm,cil,  │
                      │                                                       beam,riscv,…}  │
                      └─────────────────────────────────────────────────────────────────────┘
                      ┌────────────────────────── SIR (transpile) ──────────────────────────┐
                    ──┤ <lang>-to-semantic-ir ─▶ semantic_ir::Module ─▶ semantic-ir-to-{py,  │
                      │   (this spec)             (validate + manifest)   js,go,rust,rb,c,ts}│
                      └─────────────────────────────────────────────────────────────────────┘
```

Reference frontends to mirror:
* IIR: `dartmouth-basic-iir-compiler`, `algol-iir-compiler` — a stateful
  `Compiler` with an `emit()` helper walking the parser CST by `rule_name`,
  wrapping instructions into `IIRFunction`/`IIRModule`.
* SIR: `c-to-semantic-ir` (typed, imperative — the closest model for COBOL),
  `python-to-semantic-ir` — a `Lowerer` building `Stmt`/`Expr`, declaring a
  `FeatureManifest`, `validate`-ing before returning.

Run-verification: `lang-aot/tests/lang_matrix.rs` executes each
`(program, backend)` cell on its real toolchain across seven columns
(NativeAOT, LLVM, WASM, JVM, CLR, VM, JIT) and asserts stdout / exit code.
Transpile-verification: `sir-conformance` runs the original and each transpiled
program and compares stdout against a written reference.

## Design decisions

### D1. Fixed-point decimal → scaled integers in the frontend

Neither IR has a decimal / packed-decimal / fixed-point type, and adding one
would be a cross-cutting change to all 17 backends. COBOL fixed-point **is** a
scaled integer with an implied point, so we model it that way at lowering time:

* A `PIC 9(i)V9(d)` value lowers to an integer (`i64` in IIR;
  `SirType::Int(IntSpec{width, signed})` in SIR) holding all `i+d` digits, with
  the **scale `d` tracked at compile time** in the symbol table.
* Arithmetic aligns scales by multiplying/dividing by powers of ten (emitted as
  IIR `mul`/`div` or SIR `BuiltinCall`), exactly as the tree-walk runtime's
  `Decimal::to_scaled`/`from_scaled` do. `ROUNDED` adds a half-ulp before the
  truncating divide; `ON SIZE ERROR` is a range check on the integer result.
* We **reuse `cobol-runtime`'s `Picture` and `Decimal`** (already exact and
  tested) in the frontend to compute scales and the digit math — the frontends
  depend on `cobol-runtime` for its picture/value logic, not its interpreter.
* Values wider than `i64` (`PIC 9(19)+`) are out of the first slices; a later
  rung can widen to `i128` or a bignum builtin.

A first-class `SirType::Decimal{digits,scale}` + backend support remains a
possible future (additive, SIR21/22/23-style) but is explicitly **not** in scope
here.

### D2. `GO TO`: full on IIR, structured subset on SIR

* **IIR is unstructured** (labels + conditional jumps), so it represents
  arbitrary `GO TO`, paragraph fall-through, and all five `PERFORM` forms
  natively: each paragraph becomes a `label`; `GO TO P` is `jmp p`; `PERFORM`
  uses a GOSUB-style return-address stack (as BASIC does) or `call`/`ret`.
  **COBOL runs fully on every execution backend.**
* **SIR has no `goto`/label node.** The transpile path targets the *structured*
  subset: `PERFORM para` → `DirectCall`, `PERFORM … UNTIL` → `Stmt::While`,
  `PERFORM … VARYING` → `Stmt::ForRange`/`While`, `IF` → `Expr::If`. Arbitrary
  `GO TO` (and `GO TO … THRU` ranges that aren't call-shaped) is **deferred** —
  a program that uses it lowers to a clean "unsupported for transpilation"
  error, not wrong output. A control-flow relooper to structure arbitrary
  `GO TO` is a possible later rung.

### D4. File I/O over stdin/stdout streams; a real named-file primitive is deferred

Both languages' *sequential* record I/O — the classic batch shape — maps onto the
**existing, backend-portable stdout/stdin builtins**, so **no `file_open`-by-name
native primitive is added to the 17 backends**, and programs stay deterministic
and matrix-verifiable (feed stdin, capture stdout — exactly how Dartmouth BASIC's
`INPUT`/`PRINT` are already verified across all 7 columns):

* `WRITE-ITEM` / COBOL `WRITE` / `DISPLAY` → `print_str` / `print_i64` (the record
  image is the file's fields). **Free today** — every backend implements these.
* `READ-ITEM` / COBOL `READ … AT END` → a stdin read; `END OF DATA` / `AT END`
  is the end-of-input branch.
* `OPEN`/`CLOSE`, `SELECT`/`FD`, `INPUT`/`OUTPUT` file declarations → no-ops.

**The one gap (found while scoping):** `END OF DATA` needs the read to *signal
end-of-input*, and the current `input_i64`/`input_str` builtins return a plain
value with **no portable EOF signal** (the VM registers them as a closure
returning `Value::Int`; the AOT backends call `BasicRuntime.readLong()`). So the
end-of-data loop needs a minimal **EOF-aware read capability** — an
`input_line -> (str, bool_more)` (or a paired `input_eof`) added to the *input*
path across the run columns. This is far smaller than a file-open-by-name
primitive (it extends the existing stdin read, not the filesystem) and is the
prerequisite for `READ-ITEM`/`AT END` loops.

**Multiple simultaneous named files** (the inventory program has four) and COBOL
**indexed/relative random access** are modelled as **in-memory record
arrays/keyed maps** (existing `array_*` ops — deterministic, no primitive), or
eventually as an *optional* real-file capability gated to the filesystem-capable
columns (native/LLVM/JVM/CLR) only. Both are deferred.

### D3. The tree-walk runtimes stay as the reference oracle

`cobol-runtime` (87 tests) — and the FLOW-MATIC semantics defined here — are the
**trusted oracle**. Conformance for both new pipelines checks their output
against the runtime's, so "pivot to IIR" means IIR becomes the *compiled*
cross-backend execution path while the interpreter remains the source of truth,
not deleted.

## FLOW-MATIC execution semantics (defined here for the first time)

FLOW-MATIC (PL06) has a lexer + parser but no runtime, so its dynamic semantics
are defined here as part of the codegen. FLOW-MATIC is a **file/record data-flow
language**: it reads records from input files, compares and moves fields between
records, and writes records to output files, with numbered OPERATIONS as the
unit of control flow.

Mapping to IIR:

| FLOW-MATIC construct | Semantics | IIR lowering |
| --- | --- | --- |
| Operation `(n)` | a labelled point | `label op_n` |
| `GO TO OPERATION n`, `JUMP TO OPERATION n` | unconditional transfer | `jmp op_n` |
| `IF cond GO TO OPERATION n` | branch on the last `COMPARE` | `jmp_if_true` on a saved comparison flag |
| `COMPARE a WITH b` | set a three-way flag (`<`,`=`,`>`) | `cmp_*` into flag vars read by the following `IF`s |
| `OTHERWISE GO TO n` | else-branch of the compare | fall-through `jmp op_n` |
| `MOVE field TO field` | copy one field | `mov` between the field's variables |
| `TRANSFER a TO b` | copy a whole record | field-by-field `mov` |
| `STOP` | halt | `ret` |
| `WRITE-ITEM f` / `HSP` | emit a record / print | `print_str` (record image) |
| `READ-ITEM f` | read next record | file-read builtin (later slice) |

**Field/record model (the core challenge).** A field is a data-name qualified by
its file, `PRODUCT-NO (A)`. The **first slice models each qualified field as a
distinct scalar variable** (an `i64` or `str` register), so control flow
(`COMPARE`/`IF`/`OTHERWISE`/`GO TO`/`JUMP`/`STOP`) and field `MOVE` compile and
run end-to-end with no file runtime. Real **file I/O** (`INPUT`/`OUTPUT`,
`READ-ITEM`/`WRITE-ITEM` over record streams, `END OF DATA` loops) is a later
rung that adds a small record-stream builtin; until then a program that reaches
`READ-ITEM` gets a clean "file I/O not yet lowered" error, never wrong output.

The SIR path for FLOW-MATIC maps the same structured control flow to
`If`/`While`/`DirectCall`; `GO TO OPERATION n` is subject to D2.

## COBOL lowering

Follows PL08's model. Data-division items → typed slots (numeric = scaled `i64`
per D1; `X`/`A` = `str`); the item tree → a flat symbol table.

* **IIR:** each paragraph → a `label`; `GO TO` → `jmp`; the top-level PC / fall
  through → sequential labels; `PERFORM para [n TIMES | UNTIL | VARYING | THRU]`
  → a GOSUB-style return-address stack around the paragraph range; `IF/ELSE` →
  `cmp_*` + `jmp_if_*`; `MOVE`/`ADD`/…/`COMPUTE` → `mov`/`add`/… on scaled
  integers with scale-alignment; `DISPLAY` → `str_const` + `print_str` (numeric
  items via a synthesized digit-print helper, as BASIC does); signed `S` via the
  integer sign; overpunch display via a small formatting helper. **Reference
  modification** `IDENT(start:len)` / `IDENT(start:)` (constant integer indices,
  alphanumeric base) → a constant-index `str_slice` over `[start-1, start-1+len)`
  (an omitted length runs to the item's end), reused in `DISPLAY` and
  alphanumeric-comparison operands; bounds are validated at compile time and an
  out-of-range constant slice is a clean `Unsupported`, not a runtime trap.
* **SIR:** paragraphs → `Function`s; `PERFORM` → `DirectCall`; `PERFORM … UNTIL`
  → `While`; `IF` → `If`; arithmetic → `BuiltinCall` on `Int(IntSpec)` with
  `Convert` for scale/width; `DISPLAY` → `BuiltinCall("puts"/"print")`;
  `GO TO` per D2.

The goal is **parity with the tree-walk runtime's coverage** (MOVE, DISPLAY, all
arithmetic + `ROUNDED`/`ON SIZE ERROR`, `COMPUTE`, `IF/ELSE`, all five `PERFORM`
forms, `GO TO`, signed numerics), grown one construct per PR and checked against
the oracle.

## Phasing (each a small, run-verified PR; specs first)

Starting language: **FLOW-MATIC → IIR** (chosen to de-risk the new-frontend →
IIR → all-backends pipeline on the smaller language before COBOL's bulk).

1. **This spec (PL09).**
2. **`flow-matic-iir-compiler` — minimal slice.** Operations→labels,
   `COMPARE`/`IF`/`OTHERWISE`/`GO TO`/`JUMP`/`STOP`, scalar-field `MOVE`,
   `WRITE-ITEM`→`print_str`. `Language::FlowMatic` in `lang-aot`; `tests/
   backend_compat.rs` (validators) + a `lang_matrix.rs` row proven on VM/JIT and
   at least one AOT column.
3. **FLOW-MATIC → IIR growth:** record/file model, `READ-ITEM`/`END OF DATA`
   loops, the full inventory-pricing program run-verified.
4. **`cobol-iir-compiler` — minimal slice** (`DISPLAY`/`MOVE`/`STOP RUN`, integer
   arithmetic, scaled decimals) → `Language::Cobol60` + matrix rows, then grow
   toward runtime parity (`IF` → `PERFORM` → signed → `COMPUTE` → `GO TO`).
5. **`flow-matic-to-semantic-ir`** (structured subset) → ride the seven SIR
   backends → `sir-conformance` corpus.
6. **`cobol-to-semantic-ir`** (structured subset per D2) → SIR backends →
   conformance, grown toward parity.

Conformance for every rung checks the compiled/transpiled output against the
tree-walk oracle (D3).

## Reference crates

* IIR core: `interpreter-ir` (`IIRModule`/`IIRFunction`/`IIRInstr`/`Operand`).
* IIR frontends to mirror: `dartmouth-basic-iir-compiler`, `algol-iir-compiler`.
* IIR driver + matrix: `lang-aot` (`Language`, `compile_source_to_iir`),
  `lang-aot/tests/lang_matrix.rs`.
* IIR builtins: `iir-builtin-lowering` (`print` → `io_out`, etc.).
* SIR core: `semantic-ir` (`Module`/`Function`/`Stmt`/`Expr`/`SirType`/
  `FeatureManifest`/`validate`).
* SIR frontends to mirror: `c-to-semantic-ir`, `python-to-semantic-ir`.
* SIR backends (unchanged): `semantic-ir-to-{python,javascript,go,rust,ruby,c,typescript}`.
* SIR conformance: `sir-conformance`.
* Oracle + picture/decimal logic: `cobol-runtime` (`picture.rs`, `value.rs`).
