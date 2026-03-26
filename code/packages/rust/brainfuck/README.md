# brainfuck

A complete Brainfuck implementation built on the `virtual-machine` crate.

## What is this?

This crate implements the Brainfuck esoteric programming language — a language with only 8 instructions that is nonetheless Turing-complete. It serves as a proof-of-concept that the GenericVM framework can support a real (if minimal) programming language.

## Where does it fit in the stack?

```
Brainfuck Source → **Translator** → CodeObject → **BrainfuckVM** → Output
```

Brainfuck is simple enough that it doesn't need a separate lexer/parser/AST phase. The translator goes directly from source characters to bytecode instructions.

## The Language

| Char | Instruction | Description                                    |
|------|------------|------------------------------------------------|
| `>`  | RIGHT      | Move data pointer right                         |
| `<`  | LEFT       | Move data pointer left                          |
| `+`  | INC        | Increment current cell                          |
| `-`  | DEC        | Decrement current cell                          |
| `.`  | OUTPUT     | Output current cell as ASCII                    |
| `,`  | INPUT      | Read one byte of input                          |
| `[`  | LOOP_START | If cell is 0, jump past matching `]`            |
| `]`  | LOOP_END   | If cell is non-zero, jump back to matching `[`  |

All other characters are comments.

## Usage

```rust
use brainfuck::execute_brainfuck;

// Hello World!
let result = execute_brainfuck(
    "++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.\
     >---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.",
    "",
).unwrap();

assert_eq!(result.output, "Hello World!\n");
```

## API

- **`translate(source) -> CodeObject`** — Parse source into bytecode.
- **`BrainfuckVM::new(input) -> BrainfuckVM`** — Create a VM with input data.
- **`BrainfuckVM::execute(code) -> Vec<VMTrace>`** — Run bytecode, get traces.
- **`execute_brainfuck(source, input) -> BrainfuckResult`** — One-shot execution.
