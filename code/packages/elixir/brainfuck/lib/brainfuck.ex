defmodule CodingAdventures.Brainfuck do
  @moduledoc """
  Brainfuck — The Simplest Possible Language on the GenericVM.

  ## What is Brainfuck?

  Brainfuck is an esoteric programming language created by Urban Mueller
  in 1993. It has exactly 8 commands and operates on a tape of 30,000
  byte cells with a movable data pointer. Despite its extreme minimalism,
  Brainfuck is Turing-complete — it can compute anything any other
  programming language can.

  ## Why Brainfuck?

  Brainfuck is the perfect proof that the GenericVM works for radically
  different languages. If the same execution engine can run both a
  high-level language like Starlark and a primitive tape machine like
  Brainfuck, it truly is generic.

  The implementation also demonstrates:
  - **Opcode registration**: 9 opcodes, each with a simple handler
  - **Extra state**: tape, data pointer, and input buffer stored in `vm.extra`
  - **Translation**: source -> bytecode with bracket matching
  - **Immutability**: every handler returns a new VM state

  ## Quick Start

      # Run a program
      result = CodingAdventures.Brainfuck.execute_brainfuck("+++.")
      result.output  #=> <<3>>

      # Hello World
      hello = "++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++."
      result = CodingAdventures.Brainfuck.execute_brainfuck(hello)
      result.output  #=> "Hello World!\\n"

  ## Modules

  - `CodingAdventures.Brainfuck.Opcodes` — opcode definitions
  - `CodingAdventures.Brainfuck.Translator` — source -> bytecode
  - `CodingAdventures.Brainfuck.Handlers` — opcode handler functions
  - `CodingAdventures.Brainfuck.VM` — factory and executor
  """

  @doc "Translate Brainfuck source to a CodeObject."
  defdelegate translate(source), to: CodingAdventures.Brainfuck.Translator

  @doc "Create a GenericVM configured for Brainfuck. Optional input_data for `,` commands."
  defdelegate create_brainfuck_vm(input_data \\ ""), to: CodingAdventures.Brainfuck.VM

  @doc "Translate and execute a Brainfuck program. Returns a `%BrainfuckResult{}`."
  defdelegate execute_brainfuck(source, input_data \\ ""), to: CodingAdventures.Brainfuck.VM
end
