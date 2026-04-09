# 04b — Generic Bytecode Compiler

## Overview

The `GenericByteCodeCompiler` is the code-generation engine at the heart of every language compiler in the toolchain. It accepts an AST (from any parser) and emits a flat bytecode instruction stream (for any VM).

This spec renames and extends the existing `GenericCompiler` module (from the `bytecode_compiler` package) with a key design improvement: **a declarative instruction map**. For languages like Brainfuck where many AST nodes translate 1:1 to a single opcode, there should be no need to write an imperative handler function — you configure a table and the engine handles it.

## The Two-Layer Separation

Before this spec existed, the term "compiler" was used for both things. They are distinct:

```
┌────────────────────────────────────────────────────────┐
│  GenericAotCompiler  (language-specific outer shell)   │
│  • runs the lexer                                      │
│  • runs the parser                                     │
│  • configures GenericByteCodeCompiler                  │
│  • writes .bytecode + .dbg to disk                     │
└───────────────────────────┬────────────────────────────┘
                            │  feeds AST to ↓
┌───────────────────────────▼────────────────────────────┐
│  GenericByteCodeCompiler  (this spec)                  │
│  • walks the AST                                       │
│  • consults instruction map  (declarative 1:1 rules)   │
│  • calls handler functions   (imperative complex rules) │
│  • manages constant pool, name table, jump patching    │
│  • emits CodeObject                                    │
└────────────────────────────────────────────────────────┘
```

`GenericByteCodeCompiler` knows nothing about files, lexers, or parsers. It only takes an AST and produces a `CodeObject`. `GenericAotCompiler` (spec `04c`) is the pipeline wrapper that owns the end-to-end flow.

## The Instruction Map

The most important addition over `GenericCompiler` is the **instruction map**: a declarative table that says "when you see an AST node of this rule name, or a leaf token of this type, emit this opcode."

This eliminates boilerplate handler functions for simple 1:1 cases. Compare:

**Before (handler for every command):**
```elixir
compiler
|> register_handler("right_cmd",  fn c, _node, _ -> {_, c} = emit(c, 0x01); c end)
|> register_handler("left_cmd",   fn c, _node, _ -> {_, c} = emit(c, 0x02); c end)
|> register_handler("inc_cmd",    fn c, _node, _ -> {_, c} = emit(c, 0x03); c end)
|> register_handler("dec_cmd",    fn c, _node, _ -> {_, c} = emit(c, 0x04); c end)
|> register_handler("output_cmd", fn c, _node, _ -> {_, c} = emit(c, 0x05); c end)
|> register_handler("input_cmd",  fn c, _node, _ -> {_, c} = emit(c, 0x06); c end)
```

**After (one table):**
```elixir
compiler
|> set_instruction_map(%{
  "RIGHT"  => 0x01,
  "LEFT"   => 0x02,
  "INC"    => 0x03,
  "DEC"    => 0x04,
  "OUTPUT" => 0x05,
  "INPUT"  => 0x06,
})
```

The keys in the instruction map are **token type names** (uppercase, as produced by the lexer). When the compiler visits a `command` rule node, it looks at the leaf token, checks the map, and emits the corresponding opcode automatically. No handler function required.

For nodes that are *not* in the instruction map — like `loop`, which requires two-pass jump patching — a handler function is still registered with `register_handler/3`.

## Lookup Priority

When the compiler visits an AST node, it resolves what to do in this order:

```
1. Is this node's rule_name in the handler registry?
      → call the registered handler function

2. Is this a leaf node (wraps a single token)?
   And is that token's type in the instruction_map?
      → emit instruction_map[token.type]

3. Is this node's rule_name a "transparent wrapper"?
   (a rule with exactly one child that should be compiled through)
      → recurse into the single child

4. Does this node have children?
      → compile each child in order (default sequential behaviour)

5. None of the above
      → no-op (the node emits nothing, e.g. EOF, punctuation tokens)
```

Step 3 is important. Rules like `instruction` in the Brainfuck grammar are pure wrappers — they always have exactly one child. Without transparent wrapper handling, every language would need to register a pass-through handler for every intermediate rule. The compiler detects this automatically when a rule node has exactly one `ASTNode` child and no instruction_map or handler entry.

## Public API

```elixir
defmodule CodingAdventures.BytecodeCompiler.GenericByteCodeCompiler do

  @type opcode    :: non_neg_integer()
  @type rule_name :: String.t()
  @type token_type :: String.t()

  # ── Construction ────────────────────────────────────────────────────────

  @doc "Create a new empty compiler."
  @spec new() :: t()
  def new()

  # ── Instruction map (declarative 1:1 rules) ───────────────────────────

  @doc """
  Set the instruction map: a table from token type → opcode.

  When the compiler encounters a leaf token whose type appears in this
  map, it emits the corresponding opcode with no operand.

  Calling this replaces the entire map. Call merge_instruction_map/2
  to extend an existing map incrementally.
  """
  @spec set_instruction_map(t(), %{token_type() => opcode()}) :: t()
  def set_instruction_map(compiler, map)

  @doc "Merge additional entries into the instruction map."
  @spec merge_instruction_map(t(), %{token_type() => opcode()}) :: t()
  def merge_instruction_map(compiler, additions)

  # ── Handler functions (imperative complex rules) ──────────────────────

  @doc """
  Register a handler for a grammar rule name.

  The handler is called when the compiler visits a node whose rule_name
  matches. Handlers take precedence over the instruction map.

  Handler signature: (compiler, ast_node, code_object_so_far) → compiler
  """
  @spec register_handler(t(), rule_name(), handler_fn()) :: t()
  def register_handler(compiler, rule_name, handler_fn)

  # ── Compilation ──────────────────────────────────────────────────────

  @doc "Compile an AST node and all its descendants. Returns the final CodeObject."
  @spec compile(t(), ASTNode.t(), halt_opcode :: opcode() | nil) ::
    {CodeObject.t(), t()}
  def compile(compiler, ast, halt_opcode \\ nil)

  @doc "Compile a single node (used inside handler functions to recurse)."
  @spec compile_node(t(), ASTNode.t()) :: t()
  def compile_node(compiler, node)

  # ── Emission ─────────────────────────────────────────────────────────

  @doc "Emit a single instruction. Returns {instruction_index, updated_compiler}."
  @spec emit(t(), opcode(), operand :: integer() | nil) :: {integer(), t()}
  def emit(compiler, opcode, operand \\ nil)

  @doc "Emit a jump instruction with a placeholder operand. Returns {index, compiler}."
  @spec emit_jump(t(), opcode()) :: {integer(), t()}
  def emit_jump(compiler, opcode)

  @doc "Patch a previously emitted jump's operand. Uses current offset if target is nil."
  @spec patch_jump(t(), index :: integer(), target :: integer() | nil) :: t()
  def patch_jump(compiler, index, target \\ nil)

  # ── Constant and name pools ──────────────────────────────────────────

  @doc "Add a constant value, deduplicating. Returns {index, compiler}."
  @spec add_constant(t(), value :: any()) :: {integer(), t()}
  def add_constant(compiler, value)

  @doc "Add a variable name, deduplicating. Returns {index, compiler}."
  @spec add_name(t(), name :: String.t()) :: {integer(), t()}
  def add_name(compiler, name)

  @doc "Return the bytecode offset of the next instruction to be emitted."
  @spec current_offset(t()) :: integer()
  def current_offset(compiler)
end
```

## Example: Brainfuck in ~15 Lines

With the instruction map, configuring the Brainfuck compiler requires almost no code for the six simple commands:

```elixir
alias CodingAdventures.BytecodeCompiler.GenericByteCodeCompiler, as: GBC
alias Brainfuck.Opcodes

compiler =
  GBC.new()
  |> GBC.set_instruction_map(%{
    "RIGHT"  => Opcodes.right(),
    "LEFT"   => Opcodes.left(),
    "INC"    => Opcodes.inc(),
    "DEC"    => Opcodes.dec(),
    "OUTPUT" => Opcodes.output_op(),
    "INPUT"  => Opcodes.input_op(),
  })
  |> GBC.register_handler("loop", fn compiler, node, _code ->
    # Two-pass: emit placeholder, compile body, emit back-jump, patch forward-jump
    {start_idx, compiler} = GBC.emit_jump(compiler, Opcodes.loop_start())
    compiler = GBC.compile_node(compiler, node)   # recurse into body
    body_start = start_idx + 1
    {_,  compiler} = GBC.emit(compiler, Opcodes.loop_end(), body_start)
    GBC.patch_jump(compiler, start_idx, GBC.current_offset(compiler))
  end)

{code_object, _compiler} = GBC.compile(compiler, ast, Opcodes.halt())
```

The six command handlers vanish entirely — replaced by the instruction map. Only the `loop` handler, which needs stateful two-pass logic, remains as an imperative function.

## Pluggable Instruction Sets

The instruction map also enables **instruction set composition**: one `GenericByteCodeCompiler` can be configured to target different underlying VMs by swapping the opcode values.

```elixir
# Target our custom stack VM
brainfuck_stack_compiler =
  GBC.new()
  |> GBC.set_instruction_map(%{
    "RIGHT" => StackVMOpcodes.right(),   # 0x01
    "INC"   => StackVMOpcodes.inc(),     # 0x03
    # ...
  })

# Target a hypothetical register VM variant
brainfuck_reg_compiler =
  GBC.new()
  |> GBC.set_instruction_map(%{
    "RIGHT" => RegVMOpcodes.right(),     # 0xA1
    "INC"   => RegVMOpcodes.inc(),       # 0xA3
    # ...
  })
```

Same AST in, different bytecode out. The language's grammar and parse rules never change — only the opcode values do. This is how the toolchain supports compiling the same source language to multiple target VMs.

## Instruction Map vs Handler Precedence — Worked Example

Consider a language where `+` means both "increment" (as a statement) and "add" (in an expression). The instruction map cannot handle context-sensitive mappings — it only knows token types, not grammatical context.

```elixir
# WRONG: maps "PLUS" globally — cannot distinguish statement vs expression
|> GBC.set_instruction_map(%{"PLUS" => 0x03})   # always INC?

# RIGHT: register handlers for the specific rule names where context matters
|> GBC.register_handler("increment_stmt", fn c, _, _ ->
  {_, c} = GBC.emit(c, 0x03)   # INC
  c
end)
|> GBC.register_handler("add_expr", fn c, node, _ ->
  c = GBC.compile_node(c, left_child(node))
  c = GBC.compile_node(c, right_child(node))
  {_, c} = GBC.emit(c, 0x10)   # ADD
  c
end)
```

**Rule of thumb:** Use `set_instruction_map` for tokens that always mean the same opcode regardless of where they appear. Use `register_handler` for rule names where the context changes the emitted code.

For Brainfuck, all six commands are context-free (a `+` always means INC, wherever it appears). The instruction map is the right tool. For a language with expressions, statements, and type coercions, you will mostly use handlers.

## Renaming from GenericCompiler

The existing `CodingAdventures.BytecodeCompiler.GenericCompiler` is renamed to `CodingAdventures.BytecodeCompiler.GenericByteCodeCompiler`. The old name is kept as a deprecated alias for one release cycle:

```elixir
# Deprecated alias — remove after all callers are migrated
defmodule CodingAdventures.BytecodeCompiler.GenericCompiler do
  @deprecated "Use GenericByteCodeCompiler instead"
  defdelegate new(),                              to: GenericByteCodeCompiler
  defdelegate emit(c, op, operand \\ nil),        to: GenericByteCodeCompiler
  defdelegate compile(c, ast, halt \\ nil),       to: GenericByteCodeCompiler
  # ... etc
end
```

The API surface is otherwise unchanged — existing callers compile with a simple module rename.
