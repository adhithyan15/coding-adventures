defmodule CodingAdventures.StarlarkVm do
  @moduledoc """
  Starlark VM — A complete Starlark bytecode interpreter built on GenericVM.

  ## Overview

  This package provides a Starlark runtime that executes Starlark bytecode
  produced by the `starlark_ast_to_bytecode_compiler`. It registers all 46+
  opcode handlers and 23 built-in functions with the `GenericVM` framework.

  ## Architecture

  The Starlark VM sits at the top of the execution stack:

      Source Code -> Lexer -> Parser -> Compiler -> **VM**

  It plugs into the GenericVM by:
  1. Registering a handler function for each Starlark opcode
  2. Registering built-in functions (len, range, sorted, etc.)
  3. Providing factory functions to create configured VMs

  ## Key Types

  - `StarlarkFunction` — a compiled function object with code, params, defaults
  - `StarlarkIterator` — an iterator wrapper for for-loop support
  - `StarlarkResult` — the result of executing a Starlark program

  ## Quick Example

      alias CodingAdventures.StarlarkVm

      result = StarlarkVm.execute_starlark("x = 1 + 2\\nprint(x)\\n")
      result.variables["x"]  #=> 3
      result.output           #=> ["3"]
  """

  alias CodingAdventures.StarlarkVm.Vm

  defdelegate create_starlark_vm(), to: Vm
  defdelegate create_starlark_vm(opts), to: Vm
  defdelegate execute_starlark(source), to: Vm
end
