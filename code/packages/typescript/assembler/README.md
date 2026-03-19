# Assembler

**Layer 5 of the computing stack** — translates human-readable ARM assembly language into binary machine code.

## What this package does

Implements a two-pass assembler that converts ARM assembly source into executable machine code:

| Feature | Description |
|---------|-------------|
| Two-pass assembly | First pass collects labels, second pass emits binary |
| Labels | Named locations in code for branch targets and data |
| Symbol tables | Maps label names to memory addresses |
| Instruction encoding | Translates mnemonics and operands to binary machine code |
| Condition codes | Full ARM conditional execution support (EQ, NE, GT, LT, etc.) |
| Source maps | Maps binary addresses back to source line numbers |
| Error reporting | Collects all errors in a single pass (doesn't stop at the first) |

Connects the software layers (lexer, parser, compiler) to the hardware layers (CPU, arithmetic, logic gates).

## Where it fits

```
Logic Gates -> Arithmetic -> CPU -> ARM -> [Assembler] -> Lexer -> Parser -> Compiler -> VM
```

This package sits between the **ARM simulator** (which defines the binary encoding format) and the **compiler** (which can generate assembly source as a compilation target).

## Installation

```bash
npm install @coding-adventures/assembler
```

## Usage

### Assemble source text

```typescript
import { assemble } from "@coding-adventures/assembler";

const result = assemble(`
  MOV R0, #1       ; load 1
  MOV R1, #2       ; load 2
  ADD R2, R0, R1   ; R2 = R0 + R1 = 3
  HLT              ; stop
`);

console.log(result.machineCode);   // Uint8Array of 16 bytes
console.log(result.symbolTable);   // Map (empty — no labels)
console.log(result.sourceMap);     // Map { 0 => 2, 4 => 3, 8 => 4, 12 => 5 }
console.log(result.errors);        // [] (no errors)
```

### Use labels and branches

```typescript
const result = assemble(`
  MOV R0, #10
  loop:
  SUB R0, R0, #1
  CMP R0, #0
  BNE loop
  HLT
`);

console.log(result.symbolTable);  // Map { "loop" => 4 }
```

### Encode individual instructions

```typescript
import { encodeMovImm, encodeAdd, encodeHlt, instructionsToBytes } from "@coding-adventures/assembler";

const program = instructionsToBytes([
  encodeMovImm(0, 1),     // MOV R0, #1
  encodeMovImm(1, 2),     // MOV R1, #2
  encodeAdd(2, 0, 1),     // ADD R2, R0, R1
  encodeHlt(),             // HLT
]);
```

### Use the Assembler class directly

```typescript
import { Assembler } from "@coding-adventures/assembler";

const asm = new Assembler();
const result = asm.assemble("MOV R0, #42\nHLT\n");
```

## Supported instructions

### Data processing

| Mnemonic | Description | Example |
|----------|-------------|---------|
| MOV | Move value into register | `MOV R0, #1` |
| MVN | Move NOT (bitwise complement) | `MVN R0, R1` |
| ADD | Addition | `ADD R2, R0, R1` |
| SUB | Subtraction | `SUB R2, R0, R1` |
| AND | Bitwise AND | `AND R0, R1, R2` |
| ORR | Bitwise OR | `ORR R0, R1, R2` |
| EOR | Bitwise XOR | `EOR R0, R1, R2` |
| CMP | Compare (sets flags only) | `CMP R0, #10` |
| TST | Test bits (AND, flags only) | `TST R0, #0xFF` |

### Branch

| Mnemonic | Description | Example |
|----------|-------------|---------|
| B | Branch (jump) | `B loop` |
| BL | Branch with link (function call) | `BL func` |
| BEQ | Branch if equal | `BEQ done` |
| BNE | Branch if not equal | `BNE loop` |

### Memory

| Mnemonic | Description | Example |
|----------|-------------|---------|
| LDR | Load from memory | `LDR R0, [R1, #4]` |
| STR | Store to memory | `STR R0, [R1]` |

### Special

| Mnemonic | Description |
|----------|-------------|
| HLT | Halt execution |
| NOP | No operation |

## Spec

See [06-assembler.md](../../../specs/06-assembler.md) for the full specification.
