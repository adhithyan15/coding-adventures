# brainfuck_ir_compiler (Elixir)

Elixir port of the Brainfuck AOT compiler frontend — translates a Brainfuck AST into
target-independent IR.

## What is this?

This is the Brainfuck-specific **frontend** of the AOT compiler pipeline. It knows Brainfuck
semantics (tape, cells, pointer, loops, I/O) and translates them into IR instructions. It does
NOT know about RISC-V, ARM, ELF, or any specific machine target.

## How it fits in the stack

```
Brainfuck source
    ↓ CodingAdventures.Brainfuck.parse/1
AST (ASTNode tree)
    ↓ BrainfuckIrCompiler.compile/3  ← THIS PACKAGE
IrProgram + SourceMapChain
    ↓ (optimiser passes)
    ↓ (machine-code backend)
Binary
```

## Usage

```elixir
alias CodingAdventures.BrainfuckIrCompiler
alias CodingAdventures.BrainfuckIrCompiler.BuildConfig
alias CodingAdventures.CompilerIr.Printer

# Step 1: parse Brainfuck source
{:ok, ast} = CodingAdventures.Brainfuck.parse("+[>+<-].")

# Step 2: compile to IR
config = BuildConfig.release_config()
{:ok, result} = BrainfuckIrCompiler.compile(ast, "hello.bf", config)

# Step 3: inspect the IR
IO.puts Printer.print(result.program)
```

## Build configurations

```elixir
# Debug — all safety checks enabled
debug = BuildConfig.debug_config()

# Release — maximum performance
release = BuildConfig.release_config()

# Custom — e.g. small tape
custom = %{BuildConfig.release_config() | tape_size: 1000}
```

## Brainfuck → IR mapping

| Command | IR Output |
|---------|-----------|
| `>`     | `ADD_IMM v1, v1, 1` |
| `<`     | `ADD_IMM v1, v1, -1` |
| `+`     | `LOAD_BYTE v2, v0, v1; ADD_IMM v2, v2, 1; AND_IMM v2, v2, 255; STORE_BYTE v2, v0, v1` |
| `-`     | `LOAD_BYTE v2, v0, v1; ADD_IMM v2, v2, -1; AND_IMM v2, v2, 255; STORE_BYTE v2, v0, v1` |
| `.`     | `LOAD_BYTE v2, v0, v1; ADD_IMM v4, v2, 0; SYSCALL 1` |
| `,`     | `SYSCALL 2; STORE_BYTE v4, v0, v1` |
| `[`     | `LABEL loop_N_start; LOAD_BYTE v2, v0, v1; BRANCH_Z v2, loop_N_end` |
| `]`     | `JUMP loop_N_start; LABEL loop_N_end` |

## Running tests

```bash
mix deps.get && mix test --cover
```
