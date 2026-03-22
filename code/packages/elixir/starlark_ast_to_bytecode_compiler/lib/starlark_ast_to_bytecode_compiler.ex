defmodule CodingAdventures.StarlarkAstToBytecodeCompiler do
  @moduledoc """
  Starlark AST-to-Bytecode Compiler — Translates Starlark ASTs into bytecode.

  ## Overview

  This package is the bridge between the Starlark parser and the Starlark VM.
  It takes the Abstract Syntax Tree (AST) produced by `starlark_parser` and
  transforms it into bytecode instructions that the `starlark_vm` can execute.

  The compilation pipeline looks like this:

      Starlark source code
          | (starlark_lexer)
          v
      Token stream
          | (starlark_parser)
          v
      AST (maps with :rule_name and :children)
          | (THIS PACKAGE)
          v
      CodeObject (bytecode instructions + constant/name pools)
          | (starlark_vm)
          v
      Execution result

  ## Architecture

  The compiler is built on `GenericCompiler` from the `bytecode_compiler`
  package. GenericCompiler is a pluggable framework: you register handler
  functions for each AST node type, and the compiler dispatches to them
  during tree traversal.

  This package provides:

  - **Opcodes** — 46 Starlark-specific bytecode instruction codes
  - **Rule Handlers** — ~55 functions that handle each grammar rule
  - **Operator Maps** — mappings from operator symbols to opcodes
  - **compile_starlark/1** — convenience function for source-to-bytecode

  ## Quick Example

      alias CodingAdventures.StarlarkAstToBytecodeCompiler

      # Compile source code to bytecode
      code_object = StarlarkAstToBytecodeCompiler.compile_starlark("x = 1 + 2\\n")

      # The code_object contains instructions, constants, and names
      # ready for the StarlarkVM to execute.
  """

  alias CodingAdventures.StarlarkAstToBytecodeCompiler.Compiler

  defdelegate compile_starlark(source), to: Compiler
  defdelegate create_compiler(), to: Compiler
end
