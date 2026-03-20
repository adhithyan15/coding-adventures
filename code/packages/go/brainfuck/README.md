# Brainfuck Interpreter (Go)

A complete Brainfuck interpreter implemented in Go, built on top of the `virtual-machine` package's data types (`OpCode`, `Instruction`, `CodeObject`, `VMTrace`).

## Where It Fits in the Stack

This package is Layer 6 in the coding-adventures computing stack:

```
Layer 7: Programs (build tool, etc.)
Layer 6: Language Implementations ← brainfuck (this package)
Layer 5: Virtual Machine           ← virtual-machine (types reused here)
Layer 4: Compiler
Layer 3: Parser / AST
Layer 2: Lexer / Tokens
Layer 1: Logic Gates
```

The Brainfuck interpreter reuses the virtual-machine package's types (`CodeObject`, `Instruction`, `VMTrace`, `OpCode`) but implements its own execution loop, since Brainfuck's tape-based memory model is fundamentally different from the stack-based `VirtualMachine`.

## Package Structure

| File               | Purpose                                      |
|--------------------|----------------------------------------------|
| `opcodes.go`       | Opcode constants and character-to-opcode map  |
| `translator.go`    | Source code to bytecode translation            |
| `handlers.go`      | BrainfuckVM struct and execution logic         |
| `vm.go`            | BrainfuckResult, factory, convenience executor |
| `translator_test.go` | Translator unit tests                       |
| `handlers_test.go` | Individual opcode handler tests                |
| `vm_test.go`       | End-to-end integration tests                   |

## Usage

### Quick Execution

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/brainfuck"

result := brainfuck.ExecuteBrainfuck("++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.", "")
fmt.Println(result.Output) // "Hello World!\n"
```

### Step-by-Step Execution

```go
code := brainfuck.Translate("+++.")
bvm := brainfuck.CreateBrainfuckVM("")
traces := bvm.Execute(code)
fmt.Println(strings.Join(bvm.Output, "")) // "\x03"
```

### With Input

```go
result := brainfuck.ExecuteBrainfuck(",.", "A")
fmt.Println(result.Output) // "A"
```

## How It Works

### Translation

The `Translate()` function converts Brainfuck source into a `CodeObject`:

1. Each command character (`> < + - . , [ ]`) becomes one `Instruction`.
2. Non-command characters are ignored (they're comments).
3. Brackets are matched using a stack, with `[` and `]` patched to jump to each other.
4. A `HALT` instruction is appended at the end.

### Execution

The `BrainfuckVM` maintains:

- **Tape**: 30,000 byte cells (0-255), initialized to 0
- **Data Pointer (DP)**: Index into the tape, starts at 0
- **Program Counter (PC)**: Current instruction index
- **Input Buffer**: Simulated stdin for `,` commands

Each instruction is dispatched via a switch statement in the `Step()` method.

### Cell Wrapping

Cells are unsigned bytes. Incrementing 255 wraps to 0; decrementing 0 wraps to 255.

### EOF Behavior

When input is exhausted, `,` commands set the cell to 0.

## Running Tests

```bash
go test ./... -v -cover
```

## Parallel Implementations

This package has equivalent implementations in:

- **Python**: `code/packages/python/brainfuck/`
- **Ruby**: `code/packages/ruby/brainfuck/`

All three implementations share the same test cases and produce identical results.
