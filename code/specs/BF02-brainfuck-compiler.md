# BF02 — Brainfuck Bytecode Compiler

## Overview

This spec replaces `Brainfuck.Translator` with a proper bytecode compiler built on the generic `GenericCompiler` from the `bytecode_compiler` package (spec `04-bytecode-compiler.md`). The compiler walks the AST produced by `Brainfuck.Parser` (spec `BF01`) and emits bytecode using the same opcode constants and handler dispatch table already defined in `Brainfuck.Opcodes` and `Brainfuck.Handlers`.

**Nothing in the VM changes.** The opcodes are identical, the handlers are identical, and the `CodeObject` format is identical. Only the front-end — the thing that produces the `CodeObject` — changes from a bespoke character scanner to a generic AST-walking compiler.

This is the first end-to-end exercise of the generic compilation pipeline:

```
source → Lexer → Parser → [THIS: GenericCompiler] → CodeObject → VM
```

Every future language in the toolchain follows the same path.

## Why Replace the Translator?

The `Translator` module handles three concerns at once:
1. Tokenising source characters (implicit lexer)
2. Recognising the loop structure (implicit parser)
3. Emitting bytecode (compiler)

Separating them means:
- Parse errors (unmatched brackets) are caught before compilation and reported with source locations
- The compiler can be written as a simple, stateless AST visitor
- The debug sidecar (spec `05d`) can be emitted as a natural by-product of compilation, because every `emit` call is now associated with a specific `ASTNode` that carries `start_line`/`start_column`

## The Compiler's Job

The `GenericCompiler` works by registering a *handler* for each grammar rule name. When `compile/2` is called with an AST node, the compiler looks up the handler for that node's `rule_name` and calls it.

For Brainfuck, there are four rule names from the grammar:
- `program` — the root; compile each child instruction
- `instruction` — a wrapper node; compile its single child
- `loop` — emit `LOOP_START`, compile body, emit `LOOP_END`, patch jump targets
- `command` — look at the token type and emit the corresponding opcode

## Opcode Mapping

The existing opcodes in `Brainfuck.Opcodes` map directly to grammar rule tokens:

| Token type (from lexer) | Opcode constant | Hex |
|---|---|---|
| `RIGHT` | `Opcodes.right()` | `0x01` |
| `LEFT` | `Opcodes.left()` | `0x02` |
| `INC` | `Opcodes.inc()` | `0x03` |
| `DEC` | `Opcodes.dec()` | `0x04` |
| `OUTPUT` | `Opcodes.output_op()` | `0x05` |
| `INPUT` | `Opcodes.input_op()` | `0x06` |
| Loop open (`[`) | `Opcodes.loop_start()` | `0x07` |
| Loop close (`]`) | `Opcodes.loop_end()` | `0x08` |
| End of program | `Opcodes.halt()` | `0xFF` |

## Jump Patching

The loop instruction requires **two-pass** bytecode emission. When the compiler first sees `[`, it does not yet know where `]` will be (that depends on how many instructions are in the loop body). It must:

1. Emit `LOOP_START` with a placeholder operand (`0`)
2. Remember the bytecode offset of this instruction
3. Compile the entire loop body
4. Emit `LOOP_END` pointing back to the `LOOP_START` offset
5. Go back and patch the `LOOP_START` operand with the offset of the instruction *after* `LOOP_END`

The `GenericCompiler` provides `emit_jump/2` and `patch_jump/3` exactly for this purpose.

```
Before patching:               After patching:

offset  opcode  operand        offset  opcode  operand
──────  ──────  ───────        ──────  ──────  ───────
  0     INC      nil             0     INC      nil
  1     INC      nil             1     INC      nil
  2     LOOP_START  0   ←─┐      2     LOOP_START  8   ──┐
  3     RIGHT    nil        │    3     RIGHT    nil      │
  4     INC      nil        │    4     INC      nil      │
  5     LEFT     nil        │    5     LEFT     nil      │
  6     DEC      nil        │    6     DEC      nil      │
  7     LOOP_END  2  ──────┘     7     LOOP_END  2   ←──┘
  8     HALT     nil             8     HALT     nil
```

`LOOP_START` operand = the offset to jump to when the cell is zero (skip the loop).
`LOOP_END` operand = the offset to jump to when the cell is nonzero (repeat the loop).

## Compiler Implementation

```elixir
defmodule Brainfuck.Compiler do
  @moduledoc """
  Compiles a Brainfuck AST (from Brainfuck.Parser) into a CodeObject
  ready for execution by the Brainfuck VM.

  Uses GenericCompiler from the bytecode_compiler package — the same
  infrastructure all other languages in the toolchain will use.

  Also emits a debug sidecar (.dbg) for debugger and LSP support.
  """

  alias CodingAdventures.BytecodeCompiler.GenericCompiler
  alias CodingAdventures.Parser.ASTNode
  alias Brainfuck.Opcodes

  @doc """
  Compile a Brainfuck AST to a CodeObject.

  Returns {code_object, sidecar_writer} so the caller can serialise
  the sidecar alongside the bytecode.
  """
  @spec compile(ASTNode.t()) :: {CodeObject.t(), SidecarWriter.t()}
  def compile(ast) do
    compiler = GenericCompiler.new()
    writer   = SidecarWriter.new(vm_hint: :stack)

    compiler = compiler
    |> GenericCompiler.register_rule("program",     &handle_program/3)
    |> GenericCompiler.register_rule("instruction", &handle_instruction/3)
    |> GenericCompiler.register_rule("loop",        &handle_loop/3)
    |> GenericCompiler.register_rule("command",     &handle_command/3)

    {code_object, compiler} = GenericCompiler.compile(compiler, ast, Opcodes.halt())
    {code_object, compiler.sidecar_writer}
  end

  # ── Rule handlers ────────────────────────────────────────────────────────

  # program: compile each child instruction in sequence
  defp handle_program(compiler, node, _code) do
    Enum.reduce(node.children, compiler, fn child, c ->
      case child do
        %ASTNode{} -> GenericCompiler.compile_node(c, child)
        _token     -> c   # skip bare tokens (EOF)
      end
    end)
  end

  # instruction: a thin wrapper — just compile its single ASTNode child
  defp handle_instruction(compiler, node, _code) do
    child = Enum.find(node.children, &match?(%ASTNode{}, &1))
    if child, do: GenericCompiler.compile_node(compiler, child), else: compiler
  end

  # command: look at the leaf token type and emit the corresponding opcode
  defp handle_command(compiler, node, _code) do
    token = ASTNode.token(node) || hd(node.children)
    opcode = token_to_opcode(token.type)

    # Record source location in the sidecar before emitting
    writer = SidecarWriter.record_location(compiler.sidecar_writer,
      offset:  GenericCompiler.current_offset(compiler),
      line:    token.line,
      column:  token.column
    )
    compiler = %{compiler | sidecar_writer: writer}

    {_idx, compiler} = GenericCompiler.emit(compiler, opcode)
    compiler
  end

  # loop: emit LOOP_START placeholder, compile body, emit LOOP_END, patch
  defp handle_loop(compiler, node, _code) do
    # Find the LOOP_START token for source position
    open_token = Enum.find(node.children, fn
      %{type: "LOOP_START"} -> true
      _                     -> false
    end)

    # Record [ position in sidecar
    writer = SidecarWriter.record_location(compiler.sidecar_writer,
      offset:  GenericCompiler.current_offset(compiler),
      line:    open_token.line,
      column:  open_token.column
    )
    compiler = %{compiler | sidecar_writer: writer}

    # Emit LOOP_START with a placeholder operand; remember its index
    {loop_start_idx, compiler} = GenericCompiler.emit_jump(compiler, Opcodes.loop_start())

    # Compile the body (all ASTNode children between [ and ])
    compiler = Enum.reduce(node.children, compiler, fn child, c ->
      case child do
        %ASTNode{} -> GenericCompiler.compile_node(c, child)
        _token     -> c
      end
    end)

    # Find the LOOP_END token for source position
    close_token = Enum.find(Enum.reverse(node.children), fn
      %{type: "LOOP_END"} -> true
      _                   -> false
    end)

    # Record ] position in sidecar
    writer = SidecarWriter.record_location(compiler.sidecar_writer,
      offset:  GenericCompiler.current_offset(compiler),
      line:    close_token.line,
      column:  close_token.column
    )
    compiler = %{compiler | sidecar_writer: writer}

    # Emit LOOP_END pointing back to just after LOOP_START
    loop_body_start = loop_start_idx + 1
    {_idx, compiler} = GenericCompiler.emit(compiler, Opcodes.loop_end(), loop_body_start)

    # Patch LOOP_START to point to the instruction after LOOP_END
    after_loop = GenericCompiler.current_offset(compiler)
    GenericCompiler.patch_jump(compiler, loop_start_idx, after_loop)
  end

  # ── Private helpers ──────────────────────────────────────────────────────

  @token_to_opcode %{
    "RIGHT"  => Opcodes.right(),
    "LEFT"   => Opcodes.left(),
    "INC"    => Opcodes.inc(),
    "DEC"    => Opcodes.dec(),
    "OUTPUT" => Opcodes.output_op(),
    "INPUT"  => Opcodes.input_op()
  }

  defp token_to_opcode(type), do: Map.fetch!(@token_to_opcode, type)
end
```

## Convenience Entry Points

```elixir
defmodule Brainfuck.Compiler do
  # ...above...

  @doc "Compile source code to a CodeObject in one step."
  @spec compile_source(String.t()) ::
    {:ok, CodeObject.t(), SidecarWriter.t()} | {:error, String.t()}
  def compile_source(source) do
    with {:ok, ast} <- Brainfuck.Parser.parse(source) do
      {code_object, sidecar} = compile(ast)
      {:ok, code_object, sidecar}
    end
  end

  @doc "Compile source and write both bytecode and sidecar to disk."
  @spec compile_to_files(String.t(), output_path :: String.t()) ::
    :ok | {:error, String.t()}
  def compile_to_files(source, output_path) do
    with {:ok, code_object, sidecar} <- compile_source(source) do
      bytecode_path = output_path <> ".bytecode"
      sidecar_path  = output_path <> ".dbg"

      :ok = CodeObject.write_file(code_object, bytecode_path)
      :ok = SidecarWriter.write_file(sidecar, sidecar_path)
    end
  end
end
```

## What the VM Sees

The `CodeObject` produced is identical to what `Translator` produced. The VM does not know or care whether it came from `Translator` or `Compiler`. For the program `++[>+<-]`:

```
Index  Opcode      Operand  Source
─────  ──────────  ───────  ───────────────────
0      INC         nil      line 1, col 1  (+)
1      INC         nil      line 1, col 2  (+)
2      LOOP_START  8        line 1, col 3  ([)
3      RIGHT       nil      line 1, col 4  (>)
4      INC         nil      line 1, col 5  (+)
5      LEFT        nil      line 1, col 6  (<)
6      DEC         nil      line 1, col 7  (-)
7      LOOP_END    2        line 1, col 8  (])
8      HALT        nil      —
```

The sidecar's line table records a row for each of offsets 0–7. When a debugger stops at offset 4, it looks up offset 4 in the sidecar and reports "line 1, column 5" — the `+` inside the loop.

## Migrating from Translator

The public API that callers use changes minimally. Before:

```elixir
# Old path via Translator
code_object = Brainfuck.Translator.translate(source)
result = Brainfuck.VM.execute_brainfuck(source)
```

After:

```elixir
# New path via Lexer → Parser → Compiler
{:ok, code_object, _sidecar} = Brainfuck.Compiler.compile_source(source)

# The VM convenience function is updated internally — callers unchanged
result = Brainfuck.VM.execute_brainfuck(source)
```

`Brainfuck.VM.execute_brainfuck/2` is updated to call `Compiler.compile_source` instead of `Translator.translate`. External callers of `execute_brainfuck` see no change.

Callers that call `Translator.translate` directly should migrate to `Compiler.compile_source`.

## Files Changed

| File | Change |
|---|---|
| `code/grammars/brainfuck.tokens` | **New** — lexer grammar |
| `code/grammars/brainfuck.grammar` | **New** — parser grammar |
| `lib/brainfuck/lexer.ex` | **New** — `Brainfuck.Lexer` module |
| `lib/brainfuck/parser.ex` | **New** — `Brainfuck.Parser` module |
| `lib/brainfuck/compiler.ex` | **New** — `Brainfuck.Compiler` module |
| `lib/brainfuck/translator.ex` | **Deleted** — superseded by compiler |
| `lib/brainfuck/vm.ex` | **Updated** — call compiler instead of translator |
| `lib/brainfuck.ex` | **Updated** — re-export compiler entry points |
| `test/brainfuck/translator_test.exs` | **Deleted** — superseded |
| `test/brainfuck/lexer_test.exs` | **New** |
| `test/brainfuck/parser_test.exs` | **New** |
| `test/brainfuck/compiler_test.exs` | **New** |
| `test/brainfuck/e2e_test.exs` | **Unchanged** — all existing e2e tests still pass |

The existing end-to-end tests (`e2e_test.exs`) are the acceptance criterion: every program that ran correctly before must produce the same output after this refactor. Since the emitted bytecode is byte-for-byte identical (same opcodes, same operands, same jump targets), the VM behaviour is guaranteed unchanged.
