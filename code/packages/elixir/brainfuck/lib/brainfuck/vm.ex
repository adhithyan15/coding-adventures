defmodule CodingAdventures.Brainfuck.VM do
  @moduledoc """
  Brainfuck VM Factory — Plugging Brainfuck Into the GenericVM.

  ## The Factory Pattern

  This module provides `create_brainfuck_vm/1` — a factory function that
  creates a GenericVM fully configured for Brainfuck. It:

  1. Creates a fresh GenericVM instance.
  2. Attaches Brainfuck-specific state (tape, data pointer, input buffer)
     via the GenericVM's `extra` map.
  3. Registers all 9 opcode handlers.

  The result is a GenericVM that speaks Brainfuck — same execution engine
  as any other language, different language semantics.

  ## Convenience Executor

  For simple use cases, `execute_brainfuck/2` wraps the full pipeline:

      result = execute_brainfuck("++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.")

  This translates the source, creates a VM, and executes in one call.

  ## BrainfuckResult

  The result of execution is a `%BrainfuckResult{}` struct containing:

  - `output` — the program's text output (all `.` commands concatenated)
  - `tape` — the final state of all 30,000 cells
  - `dp` — the final data pointer position
  - `traces` — step-by-step execution traces for debugging
  - `steps` — total number of instructions executed
  """

  alias CodingAdventures.VirtualMachine.GenericVM
  alias CodingAdventures.Brainfuck.{Handlers, Translator}

  # =========================================================================
  # Result struct
  # =========================================================================

  defmodule BrainfuckResult do
    @moduledoc """
    The result of executing a Brainfuck program.

    ## Fields

    - `output` — The program's output string (concatenation of all `.` commands).
    - `tape` — The final state of the tape (all 30,000 cells).
    - `dp` — The final data pointer position.
    - `traces` — Step-by-step execution traces (for debugging/visualization).
    - `steps` — Total number of instructions executed.

    ## Example

        result = CodingAdventures.Brainfuck.VM.execute_brainfuck("+++.")
        result.output   #=> <<3>>    (ASCII character 3, non-printable)
        result.tape     #=> [3, 0, 0, ...]
        result.dp       #=> 0
        result.steps    #=> 5        (3 INCs + 1 OUTPUT + 1 HALT)
    """
    defstruct [:output, :tape, :dp, :traces, :steps]
  end

  # =========================================================================
  # Factory function
  # =========================================================================

  @doc """
  Create a GenericVM configured for Brainfuck execution.

  This is the factory function that wires up Brainfuck's handlers and
  state. The returned VM is ready to execute any Brainfuck CodeObject.

  ## Parameters

  - `input_data` — Input to feed to `,` commands. Each byte is one cell
    value. Default is `""` (all `,` commands produce 0 / EOF).

  ## Returns

  A `%GenericVM{}` with Brainfuck handlers registered and tape initialized.

  ## Example

      vm = create_brainfuck_vm("AB")
      # vm now has a 30,000-cell tape, dp at 0, and "AB" as input
  """
  def create_brainfuck_vm(input_data \\ "") do
    vm = GenericVM.new()

    # -- Attach Brainfuck-specific state via the extra map -----------------
    # The tape is a list of 30,000 zeros. The data pointer starts at 0.
    # Input is stored as a binary string with a read position.
    vm =
      vm
      |> GenericVM.put_extra(:tape, List.duplicate(0, Handlers.tape_size()))
      |> GenericVM.put_extra(:dp, 0)
      |> GenericVM.put_extra(:input_buffer, input_data)
      |> GenericVM.put_extra(:input_pos, 0)

    # -- Register all 9 opcode handlers -----------------------------------
    Enum.reduce(Handlers.handlers(), vm, fn {opcode, handler}, vm ->
      GenericVM.register_opcode(vm, opcode, handler)
    end)
  end

  # =========================================================================
  # Convenience executor
  # =========================================================================

  @doc """
  Translate and execute a Brainfuck program in one call.

  This is the convenience function for quick execution. It handles
  the full pipeline: source -> translate -> create VM -> execute -> result.

  ## Parameters

  - `source` — The Brainfuck source code.
  - `input_data` — Input bytes for `,` commands (default `""`).

  ## Returns

  A `%BrainfuckResult{}` with the program's output, final tape state,
  and execution traces.

  ## Examples

  Simple addition (2 + 5 = 7):

      result = execute_brainfuck("++>+++++[<+>-]")
      result.tape |> Enum.at(0)
      #=> 7

  Hello character (ASCII 72 = 'H'):

      result = execute_brainfuck("+++++++++[>++++++++<-]>.")
      result.output
      #=> "H"
  """
  def execute_brainfuck(source, input_data \\ "") do
    code = Translator.translate(source)
    vm = create_brainfuck_vm(input_data)
    {traces, vm} = GenericVM.execute(vm, code)

    %BrainfuckResult{
      output: Enum.join(vm.output),
      tape: GenericVM.get_extra(vm, :tape),
      dp: GenericVM.get_extra(vm, :dp),
      traces: traces,
      steps: length(traces)
    }
  end
end
