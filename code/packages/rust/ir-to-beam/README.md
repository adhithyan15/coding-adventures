# ir-to-beam

BEAM bytecode backend for the Rust compiler IR.

Lowers an `IrProgram` from the `compiler-ir` crate into a BEAM (Erlang VM)
binary module (`.beam` file format), implementing the LANG20
`CodeGenerator<IrProgram, BEAMModule>` protocol.

## Where it fits

```
IrProgram (compiler-ir)
  │
  ├─ validate_for_beam()          ← pre-flight check
  │
  ├─ lower_ir_to_beam()           ← two-pass IR → BEAMModule lowering
  │       │
  │       ├─ Pass 1: collect LABEL instructions → BEAM label numbers
  │       └─ Pass 2: translate each IR instruction to BEAM bytecode
  │
  └─ encode_beam()                ← serialize BEAMModule → Vec<u8> (.beam)
```

The `BEAMCodeGenerator` type wires all three steps into the LANG20
`CodeGenerator<IrProgram, BEAMModule>` interface.

## Supported IR opcodes (v1)

| IR op     | BEAM instruction                   |
|-----------|------------------------------------|
| LABEL     | `label {u,N}`                      |
| LOAD_IMM  | `move {i,val} {x,r}`               |
| ADD       | `gc_bif2 erlang:+/2`               |
| ADD_IMM   | `move {i,imm} scratch; gc_bif2 +`  |
| SUB       | `gc_bif2 erlang:-/2`               |
| AND       | `gc_bif2 erlang:band/2`            |
| AND_IMM   | `move {i,imm} scratch; gc_bif2 band` |
| JUMP      | `jump {f,label}`                   |
| BRANCH_Z  | `is_ne_exact {f,L} {x,r} {i,0}`   |
| BRANCH_NZ | `is_eq_exact {f,L} {x,r} {i,0}`   |
| CALL      | `call {u,0} {f,label}`             |
| RET/HALT  | `return`                           |
| NOP       | (nothing)                          |
| COMMENT   | (nothing)                          |

Unsupported in v1 (validation errors): `LOAD_BYTE`, `STORE_BYTE`,
`LOAD_WORD`, `STORE_WORD`, `LOAD_ADDR`, `SYSCALL`, `CMP_EQ`, `CMP_NE`,
`CMP_LT`, `CMP_GT`.

## BEAM file structure

Each `.beam` binary is an IFF container with these chunks in order:

```
FOR1 <size> BEAM
  AtU8  — atom table (classic positive-count format; see encoder.rs doc comment)
  Code  — instruction stream with compact-term encoded operands
  StrT  — string table (empty in v1)
  ImpT  — import table (erlang:+/2, erlang:-/2, erlang:band/2, …)
  ExpT  — export table (run/0 at BEAM label 2)
  LocT  — local function table (empty in v1)
  LitT  — literal table (BEAM03; only present when `module.literals` is non-empty
          — e.g. any f64 `const` — zlib-compressed for OTP ≤27 compatibility)
  Attr  — module attributes (ETF nil list, required by OTP 25+)
  CInf  — compiler info   (ETF nil list, required by OTP 25+)
  Meta  — [{enabled_features,[]}] (required by OTP 25+)
```

### AtU8 format

This encoder emits the **classic** atom-table format: a plain positive `u32`
count, then for each atom a single raw length byte followed by its UTF-8
bytes. An earlier version emitted a nibble-packed negative-count form on the
belief OTP 28's loader required it; that was wrong — OTP 28 accepts both, but
**OTP 27 (this repo's pinned CI runtime) rejects the nibble-packed form**
with `corrupt atom table`. The classic form loads on every supported OTP
(20 through 28+) and is a strict superset of what this encoder needs (atoms
are capped at 255 bytes by `validate_for_beam`), so it is now emitted
unconditionally. See `encode_atu8`'s doc comment for the empirical
OTP-27-vs-28 table that motivated this.

### LitT format (BEAM03)

Every float `const` needs a boxed term, so it goes through the module's
literal table rather than an immediate compact-term operand. Like `AtU8`,
this chunk targets the **older, universally-loadable** on-disk format: OTP
28+ can store `LitT` uncompressed, but OTP ≤27 (this repo's pinned CI
runtime) only accepts the zlib-**compressed** form. Rather than add an
external compression dependency, `zlib_store_compress` hand-rolls a valid
RFC 1950/1951 stream using only uncompressed ("stored") DEFLATE blocks — a
small, unambiguous special case any compliant zlib decoder (including the C
zlib real `erl` links against) must accept. See `build_litt_chunk`'s doc
comment for the full format and how it was verified against real `erl`.

### Mandatory OTP 25+ chunks

`Attr`, `CInf`, and `Meta` are required by the OTP 25+ C-loader even for
minimal modules.  Omitting them causes "compiled for an old version of the
runtime system" at load time on OTP 28.

## Quick start

```rust
use compiler_ir::{IrProgram, IrInstruction, IrOp};
use ir_to_beam::{BEAMCodeGenerator, encode_beam};
use codegen_core::codegen::CodeGenerator;

let mut prog = IrProgram::new("_start");
prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 0));

let gen = BEAMCodeGenerator::new("mymod");
assert!(gen.validate(&prog).is_empty());
let module = gen.generate(&prog);
let bytes = encode_beam(&module);
// bytes is a valid .beam binary — load with code:load_binary/3 in Erlang
assert_eq!(&bytes[0..4], b"FOR1");
```

## Running tests

```
cargo test -p ir-to-beam
```
