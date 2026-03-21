# Brainfuck Interpreter

A Brainfuck interpreter built on the **pluggable GenericVM** framework from `@coding-adventures/virtual-machine`.

## Why Brainfuck?

This package proves that the GenericVM architecture works for radically different languages. Starlark has 50+ opcodes, variables, functions, and collections. Brainfuck has 8 opcodes and a tape. Both run on the same GenericVM chassis -- different engines, same car.

## Usage

```typescript
import { executeBrainfuck, translate, createBrainfuckVm } from "@coding-adventures/brainfuck";

// Hello World
const result = executeBrainfuck(
  "++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]" +
  ">>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++."
);
console.log(result.output); // "Hello World!\n"

// Addition: 2 + 5 = 7
const result2 = executeBrainfuck("++>+++++[<+>-]");
console.log(result2.tape[0]); // 7

// Cat program (echo input)
const result3 = executeBrainfuck(",[.,]", "Hi!");
console.log(result3.output); // "Hi!"

// Step-by-step: translate, create VM, execute
const code = translate("+++.");
const vm = createBrainfuckVm();
const traces = vm.execute(code);
console.log(vm.output.join("")); // "\x03"
console.log(traces.length);      // 5 (3 INCs + 1 OUTPUT + 1 HALT)
```

## Architecture

```
Source code ("++[>+<-]")
       |
       v
   Translator  -->  CodeObject (instructions, no constants/names)
       |
       v
   GenericVM   -->  BrainfuckResult (output, tape, traces)
   (with BF
    handlers)
```

- **Translator** (`translator.ts`): Converts BF source to bytecode. Each character maps to one instruction. Bracket matching resolves jump targets.
- **Handlers** (`handlers.ts`): 9 handler functions registered with GenericVM via `registerOpcode()`.
- **VM Factory** (`vm.ts`): `createBrainfuckVm()` creates a GenericVM with BF handlers and tape state.

## The 8 Commands

| Command | Opcode | Description |
|---------|--------|-------------|
| `>` | RIGHT | Move data pointer right |
| `<` | LEFT | Move data pointer left |
| `+` | INC | Increment cell (wraps 255->0) |
| `-` | DEC | Decrement cell (wraps 0->255) |
| `.` | OUTPUT | Print cell as ASCII |
| `,` | INPUT | Read byte into cell |
| `[` | LOOP_START | Jump past `]` if cell is 0 |
| `]` | LOOP_END | Jump back to `[` if cell is not 0 |

Everything else is a comment.

## How It Fits in the Stack

This is a **Layer 5** package (Virtual Machine layer), sitting alongside `starlark-vm` as a second language plugin for the GenericVM framework.

```
Layer 5: Language VMs    [starlark-vm] [brainfuck]  <-- YOU ARE HERE
Layer 5: Generic VM      [virtual-machine (GenericVM)]
Layer 4: Compiler        [bytecode-compiler (GenericCompiler)]
Layer 3: Parser          [parser]
Layer 2: Lexer           [lexer]
Layer 1: Grammar Tools   [grammar-tools]
```
