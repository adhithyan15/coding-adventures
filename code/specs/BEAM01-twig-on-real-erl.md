# BEAM01 — Twig on real `erl`

## Why this spec exists

Twig already runs on the in-house `vm-core` interpreter (TW01) and
on real `java` via `twig-jvm-compiler` (TW02 + JVM01).  The CLR
backend track is open as CLR01/CLR02.  The third real-runtime
target the user named is the **BEAM VM** — Erlang's runtime —
without going through an Erlang source-compiler step.  We want to
emit `.beam` files directly and have real `erl` execute them.

Today the repo has *almost* the right toolchain:

| Package                       | Role                          | Status |
|-------------------------------|-------------------------------|--------|
| `beam-bytes-decoder`          | low-level chunk parser        | ✅ exists |
| `beam-opcode-metadata`        | version-aware opcode table    | ✅ exists |
| `beam-bytecode-disassembler`  | symbolic `.beam` → IR         | ✅ exists |
| `beam-vm-simulator`           | in-process BEAM VM            | ✅ exists |
| **`beam-bytecode-encoder`**   | **IR → `.beam` bytes**        | ❌ missing |
| **`ir-to-beam`**              | **compiler-ir → BEAM ops**    | ❌ missing |
| **`twig-beam-compiler`**      | **Twig → `.beam` end-to-end** | ❌ missing |

This spec specifies the three missing packages and the validation
strategy: produce `.beam`, hand it to real `erl`, assert on output.

## Sister tracks

| Spec   | Backend | Outcome                                       |
|--------|---------|-----------------------------------------------|
| JVM01  | JVM     | Twig recursion runs on real `java` ✅         |
| CLR01  | CLR     | metadata conformance phase 1 (CLR02 pending) |
| BEAM01 | BEAM    | Twig runs on real `erl` (this spec)           |

All three share the same model: real-runtime correctness is a
first-class requirement, simulators are not enough.

## BEAM file format primer (what the encoder must emit)

A `.beam` file is an IFF container:

```
"FOR1"        — 4-byte magic
<uint32 BE>   — total file length minus 8 (the "FOR1\0\0\0\0" header)
"BEAM"        — 4-byte form type
<chunks...>   — each chunk is:
                 4-byte ASCII tag (e.g. "AtU8", "Code")
                 <uint32 BE> chunk byte length
                 <chunk payload>
                 0..3 bytes padding to next 4-byte boundary
```

For a minimum viable `:hello.fact(5) → 120` execution, we need
these chunks **in this order** (Erlang's loader is order-sensitive
on Atom and Code):

1. **`AtU8`** — UTF-8 atom table.  Index 0 is reserved as "no
   atom".  Index 1 is the module name (mandatory).  Followed by
   exported function names, then any literal atoms.
   ```
   <uint32 BE> n_atoms
   for i in 1..n_atoms:
       <byte> length, <length bytes> utf8 atom name
   ```
2. **`Code`** — the actual instructions.
   ```
   <uint32 BE> sub-header size (16 for current versions)
   <uint32 BE> instruction set version
   <uint32 BE> max opcode used
   <uint32 BE> n_labels
   <uint32 BE> n_functions
   <opcode + operands>...
   ```
   Each instruction is `<opcode_byte><operand1>...<operandN>`
   where each operand uses BEAM's compact encoding (3-bit type tag
   + variable-length value).
3. **`StrT`** — string heap (can be empty, but the chunk must
   exist; loader skips it but its absence trips some loaders).
4. **`ImpT`** — import table.  For zero imports, length-0 entries
   table.  Each row: `{module_atom_idx, function_atom_idx, arity}`.
5. **`ExpT`** — export table.  Each row: `{function_atom_idx,
   arity, label_id}`.  At minimum we need to export `module_info/0`
   and `module_info/1` (auto-injected by `erlc`) plus our own
   functions.
6. **`LocT`** — local function table.  Same shape as `ExpT`.  Can
   be empty if every function is exported.
7. **`FunT`** — fun (closure) table.  Can be omitted if no funs.
8. **`LitT`** — literal table.  Compressed term blob.  Can be
   omitted if no literals.
9. **`Line`** — line number info.  Real `erl` will load `.beam`
   without `Line`, so omit it for v1.
10. **`Attr`** — module attribute proplist (BERT term).  Optional
    for execution but `m:module_info/0` returns an empty list when
    absent, which is fine.
11. **`CInf`** — compiler info proplist (BERT term).  Optional for
    execution.

For the smallest possible Twig program — `(define (main) 42) (main)`
— the encoder must emit:
- `AtU8` with 3 atoms: `<module>`, `main`, `module_info`.
- `Code` with: `func_info`, `label`, `move`, `return`, `int_code_end`.
- `ImpT` with 1 row pointing at `erlang:get_module_info/1` (called
  from the auto-generated `module_info/1`).
- `ExpT` with 3 rows: `main/0`, `module_info/0`, `module_info/1`.

## Compact term encoding (operand encoding)

Every BEAM operand is encoded with a 3-bit type tag in the low
bits of the first byte plus length-prefixed value bits in the
high bits:

| Tag (3 bits) | Type      |
|--------------|-----------|
| 0b000        | literal/u (small unsigned int) |
| 0b001        | integer   |
| 0b010        | atom      |
| 0b011        | x register |
| 0b100        | y register |
| 0b101        | label     |
| 0b110        | character |
| 0b111        | extended (list, fpreg, alloc-list, lit) |

Length encoding (after the 3-bit tag):

- Bit 3 = 0: value is the next 4 bits (range 0..15).
- Bit 3 = 1, bit 4 = 0: value is high 3 bits + next byte (range
  0..2047).
- Bit 3 = 1, bit 4 = 1: value is encoded as a multi-byte
  big-endian integer; the next 3 bits give the length minus 2.

We can ship a single `_encode_operand(tag, value)` that handles
all three cases — it's a 30-line helper.

## Calling convention (mapping our IR to BEAM)

BEAM is register-based (x0..xN, y0..yN), and every function call
is "set xN registers, then `call_only` (tail) or `call`".  This
is a much closer match to our `compiler-ir` than the JVM stack
machine.  Per-function locals live in `y0..yK`, allocated by the
`allocate` opcode at function entry.

Direct mapping:

| `compiler-ir` op             | BEAM ops                          |
|------------------------------|-----------------------------------|
| `LOAD_IMM v, n`              | `move {integer, n}, {x, v}`       |
| `ADD v3, v1, v2`             | `gc_bif2 +/2, {x,v1}, {x,v2}, {x,v3}` |
| `SUB`/`MUL`/`DIV`            | same `gc_bif2` pattern            |
| `CMP_EQ v3, v1, v2`          | `is_eq_exact + move 1/0`          |
| `BRANCH_Z v, label`          | `is_eq_exact label, {x,v}, {integer,0}` |
| `JUMP label`                 | `jump label`                      |
| `LABEL name`                 | `label N`                         |
| `CALL label`                 | `call N {function, arity}`        |
| `RET`                        | `return`                          |
| `SYSCALL 1, v`               | `call_ext erlang:put_chars/1` (after string-of-byte conversion) |
| `HALT`                       | `call_ext erlang:halt/0`          |

The trickiest part is `SYSCALL 1` (write a single byte to stdout)
— the Erlang convention is `io:put_chars/1` taking a binary or
list.  We synthesize `binary:list_to_binary([N])` then `io:put_chars/2`.

## Package decomposition

### 1. `beam-bytecode-encoder`

Pure encoder.  Takes a structured representation of the chunks
(atom list, instruction stream, export list) and produces `.beam`
bytes.  Has zero compiler logic.

```python
@dataclass(frozen=True)
class BEAMModule:
    name: str
    atoms: tuple[str, ...]
    instructions: tuple[BEAMInstruction, ...]
    imports: tuple[BEAMImport, ...]
    exports: tuple[BEAMExport, ...]
    locals_: tuple[BEAMLocal, ...]
    label_count: int

def encode_beam(module: BEAMModule) -> bytes: ...
```

This is the *direct counterpart* to `ir-to-jvm-class-file` /
`cli-assembly-writer` — file-format production, no IR knowledge.

### 2. `ir-to-beam`

Lowers `compiler-ir` `IrProgram` to a `BEAMModule`.  Owns the
calling-convention mapping above.  Same shape as
`ir-to-jvm-class-file`.

```python
def lower_ir_to_beam(ir: IrProgram, config: BEAMBackendConfig) -> BEAMModule: ...
```

### 3. `twig-beam-compiler`

End-to-end Twig source → `.beam` file.  Composes `twig.parse` →
`twig.extract_program` → emit IR → `ir-to-beam` → `beam-bytecode-encoder`.
Direct counterpart to `twig-jvm-compiler`.

```python
def compile_source(src: str) -> PackageResult: ...
def run_source(src: str, *, module_name: str = "twig_main") -> CompletedProcess:
    """Write .beam to a temp dir + invoke real `erl` to run it."""
```

The real-`erl` invocation looks like:

```bash
erl -noshell -pa <tmpdir> -s <module> main -s init stop
```

`erl` looks up `<module>:main/0` and calls it; we capture its
stdout for the assertion (the `(define (main) 42)` program writes
byte 42 then halts).

## Validation strategy

`twig-beam-compiler/tests/test_real_erl.py` mirrors the JVM real-
runtime test exactly:

- `requires_erl` skip mark.
- One test per Twig feature: arithmetic, `if`, `let`, function
  calls, recursion (factorial — same `(fact 5) → 120` test as
  JVM01).
- Each test compiles, drops `.beam` next to a temp dir, invokes
  `erl`, asserts on byte output.

The simulator (`beam-vm-simulator`) loads its own `.beam` files
via `beam-bytes-decoder`, so our encoder also gets exercised by
existing simulator tests for free.

## Implementation chunks

This is large enough to land in three layered PRs:

1. **`beam-bytecode-encoder`** — pure encoder.  Acceptance: round-
   trip through `beam-bytes-decoder` + `beam-bytecode-disassembler`.
2. **`ir-to-beam`** — IR lowering.  Acceptance: `simulator.run(ir)`
   and `simulator.run(decode(encode(lower(ir))))` produce the same
   output.
3. **`twig-beam-compiler`** — Twig front-end + real-`erl` smoke
   tests.  Acceptance: factorial test on real `erl` returns byte
   120.

## Out of scope for v1

- **Tail-call optimisation.**  BEAM's `call_only` does it, but for
  v1 use plain `call`.  Stack-blowing programs are accepted.
- **Pattern matching.**  Twig has none (no `case`/`receive`).
- **Concurrency.**  No `spawn`/`!`/`receive` mapping.  Twig is
  purely functional, no need.
- **OTP.**  No gen_server scaffolding; Twig modules are bare.

## Risk register

- **AtU8 ordering.**  Erlang's loader is sensitive to atom-table
  ordering — module name MUST be at index 1.  Mitigation: writer
  validates this invariant before serialising.
- **Compact-term encoding bugs.**  Off-by-one in operand encoding
  produces silent loader crashes ("bad bytecode").  Mitigation:
  every emit goes through unit tests against the decoder.
- **erl version drift.**  We probe with Erlang/OTP 16.3.1 (system
  default).  Newer OTPs add opcodes; encoder targets a minimum
  instruction-set version and rejects unknown ops at lower-time.
- **Module name collisions in tests.**  Each test must use a
  unique module atom or `erl` caches stale code.  Mitigation: tests
  derive module name from test name + uuid.

## Sister-spec coordination

If/when **CLR02** lands and the Twig CLR backend (currently in
PR limbo) gets refreshed with conformant assemblies, the three
real-runtime tests should look near-identical:

- `twig-jvm-compiler/tests/test_real_jvm.py::test_recursion_factorial`
- `twig-clr-compiler/tests/test_real_dotnet.py::test_recursion_factorial`
- `twig-beam-compiler/tests/test_real_erl.py::test_recursion_factorial`

Same Twig source, three real runtimes, identical asserted output.
That parity is the proof we have a *language*, not three accidents.
