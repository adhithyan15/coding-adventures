defmodule CodingAdventures.BytecodeCompiler do
  @moduledoc """
  Bytecode Compiler — Layer 4 of the Computing Stack
  ===================================================

  This package provides `GenericCompiler`, a pluggable AST-to-bytecode
  compiler framework. Rather than hardcoding how to compile any particular
  language, GenericCompiler lets you *register* rule handlers that teach it
  how to compile each kind of AST node.

  ## How It Fits in the Stack

  The computing stack flows like a pipeline:

      Source Code -> Lexer -> Parser -> **Compiler** -> Virtual Machine

  The lexer produces tokens. The parser arranges them into an AST (Abstract
  Syntax Tree). This compiler walks the AST and emits bytecode instructions
  that the virtual machine can execute.

  ## Why "Generic"?

  Different languages have different AST shapes, but the *mechanics* of
  compilation are the same: walk nodes, emit instructions, manage constant
  pools and name tables. GenericCompiler handles all that plumbing. You
  just tell it what to do for each AST node type.

  ## Quick Example

      alias CodingAdventures.BytecodeCompiler.GenericCompiler

      compiler = GenericCompiler.new()

      compiler = GenericCompiler.register_rule(compiler, "number", fn compiler, node ->
        token = hd(node.children)
        value = String.to_integer(token.value)
        {index, compiler} = GenericCompiler.add_constant(compiler, value)
        {_idx, compiler} = GenericCompiler.emit(compiler, 0x01, index)
        compiler
      end)

      ast = %{rule_name: "number", children: [%{type: "NUMBER", value: "42"}]}
      {code_object, _compiler} = GenericCompiler.compile(compiler, ast)
  """

  defdelegate new(), to: CodingAdventures.BytecodeCompiler.GenericCompiler
  defdelegate register_rule(compiler, rule_name, handler), to: CodingAdventures.BytecodeCompiler.GenericCompiler
  defdelegate emit(compiler, opcode), to: CodingAdventures.BytecodeCompiler.GenericCompiler
  defdelegate emit(compiler, opcode, operand), to: CodingAdventures.BytecodeCompiler.GenericCompiler
  defdelegate compile(compiler, ast), to: CodingAdventures.BytecodeCompiler.GenericCompiler
  defdelegate compile(compiler, ast, halt_opcode), to: CodingAdventures.BytecodeCompiler.GenericCompiler
end
