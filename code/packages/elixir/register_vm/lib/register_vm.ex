defmodule CodingAdventures.RegisterVM do
  @moduledoc """
  Register-based Virtual Machine — public API.

  This module is the entry point for the `register_vm` package. It provides
  two execution functions that delegate to the interpreter engine.

  ## What is this?

  A register-based VM is an alternative execution model to the more common
  stack-based VM. Instead of pushing and popping values on an operand stack,
  this VM uses:

  1. An **accumulator register** — a single "working" register that most
     instructions read from and write to implicitly.

  2. A **register file** — a fixed array of numbered registers per call frame,
     used to hold intermediate values and function arguments.

  3. A **feedback vector** — a per-function array of observation slots that
     record what types appear at dynamic dispatch sites at runtime.

  ## Architecture Inspiration

  This design is directly inspired by V8's Ignition bytecode interpreter,
  which was introduced in 2016 to replace V8's older full-codegen compiler.
  Ignition uses an accumulator-centric register machine for the same reasons:

  - Fewer instructions than a stack VM (no redundant push/pop)
  - Simpler to analyse statically (register sources are explicit)
  - Feedback vectors enable TurboFan (V8's JIT) to specialize hot code paths

  ## Quick Start

      alias CodingAdventures.RegisterVM
      alias CodingAdventures.RegisterVM.Types.{CodeObject, RegisterInstruction}
      alias CodingAdventures.RegisterVM.Opcodes

      code = %CodeObject{
        instructions: [
          %RegisterInstruction{opcode: Opcodes.lda_smi(), operands: [7]},
          %RegisterInstruction{opcode: Opcodes.add_smi(), operands: [3, 0]},
          %RegisterInstruction{opcode: Opcodes.halt(), operands: []}
        ],
        constants: [],
        names: [],
        register_count: 0,
        feedback_slot_count: 1,
        name: "add_example"
      }

      {:ok, result} = RegisterVM.execute(code)
      result.return_value  # => 10
  """

  defdelegate execute(code), to: CodingAdventures.RegisterVM.Interpreter
  defdelegate execute_with_trace(code), to: CodingAdventures.RegisterVM.Interpreter
end
