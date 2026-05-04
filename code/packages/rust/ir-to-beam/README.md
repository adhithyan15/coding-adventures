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
  AtU8  — atom table (module name is always index 1)
  Code  — instruction stream with compact-term encoded operands
  StrT  — string table (empty in v1)
  ImpT  — import table (erlang:+/2, erlang:-/2, erlang:band/2, …)
  ExpT  — export table (run/0 at BEAM label 2)
  LocT  — local function table (empty in v1)
  Attr  — module attributes (BERT nil list)
  CInf  — compiler info   (BERT nil list)
```

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
