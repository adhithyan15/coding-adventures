# Assembler (Rust)

An ARM assembly parser and binary encoder. Translates ARM assembly mnemonics into 32-bit machine code words.

## How it fits in the stack

```
Assembly source text
    |
    v
[Assembler: Parse] ----> Structured instructions (ArmInstruction)
    |
    v
[Assembler: Encode] ---> 32-bit binary words (Vec<u32>)
    |
    v
[CPU Simulator] ---------> Execution
```

The assembler bridges human-readable assembly language and machine-executable binary code. It sits between the programmer (who writes assembly) and the CPU simulator (which executes binary).

## Supported instructions

| Mnemonic | Example            | Description                     |
|----------|--------------------|---------------------------------|
| MOV      | `MOV R0, #42`      | Move immediate to register      |
| ADD      | `ADD R2, R0, R1`   | Add two registers               |
| SUB      | `SUB R2, R0, R1`   | Subtract two registers          |
| AND      | `AND R2, R0, R1`   | Bitwise AND                     |
| ORR      | `ORR R2, R0, R1`   | Bitwise OR                      |
| EOR      | `EOR R2, R0, R1`   | Bitwise XOR                     |
| CMP      | `CMP R0, R1`       | Compare (sets flags)            |
| LDR      | `LDR R0, [R1]`     | Load from memory                |
| STR      | `STR R0, [R1]`     | Store to memory                 |
| NOP      | `NOP`              | No operation                    |

## ARM instruction encoding

ARM instructions are 32 bits wide with a regular structure:

```
31-28  27-26  25    24-21   20    19-16  15-12  11-0
[cond] [00]   [I]   [opcode][S]   [Rn]   [Rd]   [operand2]
```

## Usage

```rust
use assembler::Assembler;

let mut asm = Assembler::new();
let instructions = asm.parse("MOV R0, #42\nADD R2, R0, R1").unwrap();
let binary = asm.encode(&instructions).unwrap();
assert_eq!(binary.len(), 2); // Two 32-bit instruction words
```
