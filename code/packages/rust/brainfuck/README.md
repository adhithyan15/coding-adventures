# brainfuck

A complete Brainfuck implementation built on the `virtual-machine` crate.

## What is this?

This crate implements the Brainfuck esoteric programming language — a language with only 8 instructions that is nonetheless Turing-complete. It serves as a proof-of-concept that the GenericVM framework can support a real (if minimal) programming language.

## Where does it fit in the stack?

```
Brainfuck Source → **Lexer** → Token Stream → **Parser** → AST → **Translator** → CodeObject → **BrainfuckVM** → Output
```

The package now ships a full pipeline: grammar-driven tokenizer and parser for source-level analysis, plus the existing translator and VM for execution.

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

## Package Structure

| File         | Purpose                                          |
|--------------|--------------------------------------------------|
| `lexer.rs`   | Grammar-driven tokenizer (`tokenize`)             |
| `parser.rs`  | Grammar-driven parser (`parse`), returns AST      |
| `lib.rs`     | Translator, BrainfuckVM, BrainfuckResult, re-exports |

## Usage

### Lexer

```rust
use brainfuck::tokenize;

let tokens = tokenize("++[>+<-].");
for tok in &tokens {
    println!("{} {:?} at {}:{}", tok.token_type, tok.value, tok.line, tok.column);
}
// COMMAND "+" at 1:1
// COMMAND "+" at 1:2
// LOOP_START "[" at 1:3
// ...
```

### Parser

```rust
use brainfuck::parse;

let ast = parse("++[>+<-].").unwrap();
// Returns AstNode::Program { children: [
//   AstNode::Instruction(AstNode::Command('+')),
//   AstNode::Instruction(AstNode::Command('+')),
//   AstNode::Loop { children: [...] },
//   AstNode::Instruction(AstNode::Command('.')),
// ]}
println!("{:?}", ast);
```

### VM Execution

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

- **`tokenize(source: &str) -> Vec<Token>`** — Tokenize source into a token stream.
- **`parse(source: &str) -> Result<AstNode, ParseError>`** — Parse source into an AST.
- **`translate(source) -> CodeObject`** — Compile source into bytecode.
- **`BrainfuckVM::new(input) -> BrainfuckVM`** — Create a VM with input data.
- **`BrainfuckVM::execute(code) -> Vec<VMTrace>`** — Run bytecode, get traces.
- **`execute_brainfuck(source, input) -> BrainfuckResult`** — One-shot execution.
