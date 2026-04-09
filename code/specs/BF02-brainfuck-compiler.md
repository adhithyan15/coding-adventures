# BF02 — Brainfuck AOT Compiler

## Overview

This spec introduces two modules that together replace `Brainfuck.Translator`:

- **`Brainfuck.AotCompiler`** — the ahead-of-time pipeline. Runs the lexer, runs the parser, configures the `GenericByteCodeCompiler`, and writes `.bytecode` + `.dbg` to disk. This is the entry point a CLI tool or build system calls.
- **`Brainfuck.CodeGen`** — the code-generation configuration. Builds a `GenericByteCodeCompiler` (spec `04b`) pre-configured with Brainfuck's instruction map and loop handler. This is what the AotCompiler hands the AST to.

The separation matters: `AotCompiler` owns files and pipelines; `CodeGen` owns opcodes and compilation rules. A future JIT compiler for Brainfuck would share `CodeGen` but replace `AotCompiler` entirely.

## Layer Diagram

```
Brainfuck.AotCompiler          (this spec — pipeline + I/O)
    │
    ├─ Brainfuck.Lexer.tokenize/1          (spec BF00)
    ├─ Brainfuck.Parser.parse_tokens/1     (spec BF01)
    └─ Brainfuck.CodeGen.build_compiler/0  (this spec — configures GBC)
           │
           └─ GenericByteCodeCompiler      (spec 04b — code-generation engine)
                  │
                  └─ CodeObject + SidecarWriter
```

## Why Replace the Translator?

The `Translator` handles three concerns at once:
1. Tokenising source characters (implicit lexer)
2. Recognising the loop structure (implicit parser — bespoke bracket-stack)
3. Emitting bytecode (actual compiler work)

After this refactor:
- `Brainfuck.Lexer` handles (1) via a grammar file
- `Brainfuck.Parser` handles (2) via a grammar file — and gives precise error locations for free
- `Brainfuck.CodeGen` handles (3) via `GenericByteCodeCompiler`

The emitted bytecode is **byte-for-byte identical** to `Translator`'s output.

## The Instruction Map

Brainfuck's six simple commands are a 1:1 mapping from token type to opcode. With `GenericByteCodeCompiler.set_instruction_map/2` (spec `04b`), there is no need for a handler function per command — the map replaces them all:

```elixir
|> GBC.set_instruction_map(%{
  "RIGHT"  => Opcodes.right(),    # > → 0x01
  "LEFT"   => Opcodes.left(),     # < → 0x02
  "INC"    => Opcodes.inc(),      # + → 0x03
  "DEC"    => Opcodes.dec(),      # - → 0x04
  "OUTPUT" => Opcodes.output_op(),# . → 0x05
  "INPUT"  => Opcodes.input_op(), # , → 0x06
})
```

Only the `loop` rule needs an explicit handler because it requires two-pass jump patching — the forward jump target is unknown until the body has been compiled.

## CodeGen: Configuring GenericByteCodeCompiler

```elixir
defmodule Brainfuck.CodeGen do
  @moduledoc """
  Configures a GenericByteCodeCompiler for Brainfuck.

  Returns a pre-configured compiler ready to receive an AST.
  The instruction map handles the six simple commands; the loop
  handler manages two-pass jump patching.

  Separating configuration from execution allows the same CodeGen
  to be used by AotCompiler, a future JIT, and test helpers.
  """

  alias CodingAdventures.BytecodeCompiler.GenericByteCodeCompiler, as: GBC
  alias CodingAdventures.Parser.ASTNode
  alias Brainfuck.Opcodes

  @spec build_compiler() :: GBC.t()
  def build_compiler do
    GBC.new()
    |> GBC.set_instruction_map(%{
      "RIGHT"  => Opcodes.right(),
      "LEFT"   => Opcodes.left(),
      "INC"    => Opcodes.inc(),
      "DEC"    => Opcodes.dec(),
      "OUTPUT" => Opcodes.output_op(),
      "INPUT"  => Opcodes.input_op(),
    })
    |> GBC.register_handler("loop", &handle_loop/3)
  end

  # ── Loop handler ─────────────────────────────────────────────────────────
  #
  # The loop rule produces this bytecode:
  #
  #   LOOP_START  <offset after LOOP_END>   ← jump here if cell == 0
  #   ...body instructions...
  #   LOOP_END    <offset of LOOP_START+1>  ← jump here if cell != 0
  #
  # We don't know the "after LOOP_END" offset until the body is compiled,
  # so we emit LOOP_START with a placeholder (0) and patch it afterward.

  defp handle_loop(compiler, node, _code) do
    open_token  = find_token(node, "LOOP_START")
    close_token = find_token_from_end(node, "LOOP_END")

    # Record [ source position in the sidecar
    compiler = record(compiler, open_token)

    # Emit LOOP_START with a placeholder; save its index for patching
    {start_idx, compiler} = GBC.emit_jump(compiler, Opcodes.loop_start())

    # Compile body — GBC's transparent wrapper handling recurses through
    # the intermediate instruction nodes automatically; we just need to
    # compile the ASTNode children of the loop node
    compiler = Enum.reduce(node.children, compiler, fn
      %ASTNode{} = child, c -> GBC.compile_node(c, child)
      _token,              c -> c
    end)

    # Record ] source position
    compiler = record(compiler, close_token)

    # Emit LOOP_END pointing back to the first instruction in the body
    body_start = start_idx + 1
    {_idx, compiler} = GBC.emit(compiler, Opcodes.loop_end(), body_start)

    # Patch LOOP_START to point to the instruction after LOOP_END
    GBC.patch_jump(compiler, start_idx, GBC.current_offset(compiler))
  end

  defp record(compiler, token) do
    writer = SidecarWriter.record_location(compiler.sidecar_writer,
      offset: GBC.current_offset(compiler),
      line:   token.line,
      column: token.column
    )
    %{compiler | sidecar_writer: writer}
  end

  defp find_token(node, type) do
    Enum.find(node.children, &match?(%{type: ^type}, &1))
  end

  defp find_token_from_end(node, type) do
    node.children |> Enum.reverse() |> Enum.find(&match?(%{type: ^type}, &1))
  end
end
```

## AotCompiler: The End-to-End Pipeline

```elixir
defmodule Brainfuck.AotCompiler do
  @moduledoc """
  Ahead-of-time compiler for Brainfuck.

  Orchestrates the full pipeline:
    source string
      → Brainfuck.Lexer
      → Brainfuck.Parser
      → Brainfuck.CodeGen (via GenericByteCodeCompiler)
      → CodeObject + SidecarWriter

  Optionally writes .bytecode and .dbg files to disk.
  """

  alias CodingAdventures.BytecodeCompiler.GenericByteCodeCompiler, as: GBC
  alias Brainfuck.Opcodes

  @doc "Compile source to a CodeObject and sidecar (in memory)."
  @spec compile(String.t()) ::
    {:ok, CodeObject.t(), SidecarWriter.t()} | {:error, String.t()}
  def compile(source) do
    with {:ok, tokens} <- Brainfuck.Lexer.tokenize(source),
         {:ok, ast}    <- Brainfuck.Parser.parse_tokens(tokens) do
      compiler = Brainfuck.CodeGen.build_compiler()
      {code_object, compiler} = GBC.compile(compiler, ast, Opcodes.halt())
      {:ok, code_object, compiler.sidecar_writer}
    end
  end

  @doc "Compile source and write .bytecode + .dbg files to disk."
  @spec compile_to_files(String.t(), output_path :: String.t()) ::
    :ok | {:error, String.t()}
  def compile_to_files(source, output_path) do
    with {:ok, code_object, sidecar} <- compile(source) do
      :ok = CodeObject.write_file(code_object, output_path <> ".bytecode")
      :ok = SidecarWriter.write_file(sidecar,   output_path <> ".dbg")
    end
  end
end
```

## Jump Patching Visualised

For `++[>+<-]`:

```
Step 1: compile ++
  0  INC   nil
  1  INC   nil

Step 2: enter loop — emit LOOP_START with placeholder
  2  LOOP_START  0   ← placeholder, saved as start_idx=2

Step 3: compile >+<-
  3  RIGHT  nil
  4  INC    nil
  5  LEFT   nil
  6  DEC    nil

Step 4: emit LOOP_END pointing back to body_start (start_idx+1 = 3)
  7  LOOP_END  3

Step 5: patch LOOP_START — current_offset() is now 8
  2  LOOP_START  8   ← patched: skip to offset 8 when cell is zero

Step 6: emit HALT
  8  HALT   nil
```

Final instruction stream:
```
Index  Opcode      Operand  Source
─────  ──────────  ───────  ──────────────────
0      INC         nil      line 1, col 1  (+)
1      INC         nil      line 1, col 2  (+)
2      LOOP_START  8        line 1, col 3  ([)
3      RIGHT       nil      line 1, col 4  (>)
4      INC         nil      line 1, col 5  (+)
5      LEFT        nil      line 1, col 6  (<)
6      DEC         nil      line 1, col 7  (-)
7      LOOP_END    3        line 1, col 8  (])
8      HALT        nil      —
```

## Migrating from Translator

| Before | After |
|---|---|
| `Brainfuck.Translator.translate(source)` | `Brainfuck.AotCompiler.compile(source)` |
| Returns `CodeObject` | Returns `{:ok, CodeObject, Sidecar}` |

`Brainfuck.VM.execute_brainfuck/2` is updated internally to call `AotCompiler.compile`. External callers of `execute_brainfuck` see no change.

## Files Changed

| File | Change |
|---|---|
| `code/grammars/brainfuck.tokens` | **New** — lexer grammar (spec BF00) |
| `code/grammars/brainfuck.grammar` | **New** — parser grammar (spec BF01) |
| `lib/brainfuck/lexer.ex` | **New** — `Brainfuck.Lexer` |
| `lib/brainfuck/parser.ex` | **New** — `Brainfuck.Parser` |
| `lib/brainfuck/code_gen.ex` | **New** — `Brainfuck.CodeGen` |
| `lib/brainfuck/aot_compiler.ex` | **New** — `Brainfuck.AotCompiler` |
| `lib/brainfuck/translator.ex` | **Deleted** |
| `lib/brainfuck/vm.ex` | **Updated** — call `AotCompiler` instead of `Translator` |
| `lib/brainfuck.ex` | **Updated** — export `AotCompiler` entry points |
| `test/brainfuck/translator_test.exs` | **Deleted** |
| `test/brainfuck/lexer_test.exs` | **New** |
| `test/brainfuck/parser_test.exs` | **New** |
| `test/brainfuck/aot_compiler_test.exs` | **New** |
| `test/brainfuck/e2e_test.exs` | **Unchanged** — acceptance criterion |
