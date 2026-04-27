defmodule CodingAdventures.BrainfuckIrCompiler do
  @moduledoc """
  Elixir port of the Brainfuck AOT compiler frontend.

  This package translates a Brainfuck AST (from `CodingAdventures.Brainfuck.Parser`)
  into target-independent IR (from `CodingAdventures.CompilerIr`), plus a
  source map chain (from `CodingAdventures.CompilerSourceMap`).

  ## Usage

      alias CodingAdventures.BrainfuckIrCompiler
      alias CodingAdventures.BrainfuckIrCompiler.BuildConfig

      # Parse Brainfuck source into an AST
      {:ok, ast} = CodingAdventures.Brainfuck.parse("+[>+<-].")

      # Compile to IR
      config = BuildConfig.release_config()
      {:ok, result} = BrainfuckIrCompiler.compile(ast, "hello.bf", config)

      # Access the IR program
      result.program.entry_label   # => "_start"
      result.program.version       # => 1

      # Print the IR
      ir_text = CodingAdventures.CompilerIr.Printer.print(result.program)

  ## Modules

  - `CodingAdventures.BrainfuckIrCompiler.BuildConfig` — compilation flags
  - `CodingAdventures.BrainfuckIrCompiler.CompileResult` — compilation output
  - `CodingAdventures.BrainfuckIrCompiler.Compiler` — the compiler itself
  """

  alias CodingAdventures.BrainfuckIrCompiler.{Compiler, BuildConfig, CompileResult}
  alias CodingAdventures.Parser.ASTNode

  @doc """
  Compile a Brainfuck AST into an IR program and source map chain.

  See `CodingAdventures.BrainfuckIrCompiler.Compiler.compile/3` for full docs.
  """
  @spec compile(ASTNode.t(), String.t(), BuildConfig.t()) ::
          {:ok, CompileResult.t()} | {:error, String.t()}
  defdelegate compile(ast, filename, config), to: Compiler
end
