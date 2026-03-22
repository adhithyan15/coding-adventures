defmodule CodingAdventures.StarlarkInterpreter do
  @moduledoc """
  Starlark Interpreter — The complete execution pipeline.

  ## Chapter 1: What Is an Interpreter?

  An interpreter takes source code and executes it. Unlike a compiler that
  produces an executable file, an interpreter runs the program directly. Our
  Starlark interpreter uses a **multi-stage pipeline** internally:

      source code -> tokens -> AST -> bytecode -> execution

  Each stage is handled by a separate package:

  1. **Lexer** (built into the compiler): Breaks source text into tokens.
     `"x = 1 + 2"` -> `[NAME("x"), EQUALS, INT("1"), PLUS, INT("2")]`

  2. **Parser** (built into the compiler): Groups tokens into an AST.
     `[NAME, EQUALS, INT, PLUS, INT]` -> `AssignStmt(x, Add(1, 2))`

  3. **Compiler** (starlark_ast_to_bytecode_compiler): Translates AST into
     bytecode. `AssignStmt(x, Add(1, 2))` ->
     `[LOAD_CONST 1, LOAD_CONST 2, ADD, STORE_NAME x]`

  4. **VM** (starlark_vm): Executes bytecode on a virtual stack machine.

  This package chains them together and adds the critical `load()` function.

  ## Chapter 2: The load() Function

  `load()` is what makes BUILD files work. It's how a BUILD file imports
  rule definitions from a shared library:

      load("//rules/python.star", "py_library")

      py_library(
          name = "mylib",
          deps = ["//other:lib"],
      )

  When the VM encounters a `load()` call:

  1. **Resolve** the path — find the file contents
  2. **Execute** the file through the same interpreter pipeline
  3. **Extract** the requested symbols from the result
  4. **Inject** them into the current scope

  ## Chapter 3: Usage Examples

  **Simple execution:**

      result = StarlarkInterpreter.interpret("x = 1 + 2\\nprint(x)\\n")
      result.variables["x"]  #=> 3
      result.output           #=> ["3"]

  **With load():**

      files = %{
        "//rules/math.star" => "def double(n):\\n    return n * 2\\n"
      }
      result = StarlarkInterpreter.interpret(
        "load(\\"//rules/math.star\\", \\"double\\")\\nresult = double(21)\\n",
        file_resolver: files
      )
      result.variables["result"]  #=> 42

  **From a file:**

      result = StarlarkInterpreter.interpret_file("path/to/program.star")
  """

  alias CodingAdventures.StarlarkInterpreter.Interpreter

  defdelegate interpret(source, opts \\ []), to: Interpreter
  defdelegate interpret_file(path, opts \\ []), to: Interpreter
end
