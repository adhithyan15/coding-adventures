defmodule CodingAdventures.CompilerIr do
  @moduledoc """
  Elixir port of the general-purpose intermediate representation (IR)
  for the AOT native compiler pipeline.

  This is the foundation layer of the compiler. It defines the data
  types and operations that every pipeline stage uses.

  ## Architecture

  The IR is:
  - **Linear**: no basic blocks, no SSA, no phi nodes — just a flat
    list of instructions with explicit labels and jumps.
  - **Register-based**: infinite virtual registers (v0, v1, ...) that
    the backend's register allocator maps to physical registers.
  - **Target-independent**: backends map IR to any physical ISA without
    changing the frontend.
  - **Versioned**: the `.version` directive in the text format enables
    forward compatibility as new opcodes are added.

  ## Public API

  Most callers will use these five functions:

  - `IrProgram.new/1` — create a program
  - `IrProgram.add_instruction/2` — append an instruction
  - `IrProgram.add_data/2` — append a data declaration
  - `Printer.print/1` — render a program as IR text
  - `Parser.parse/1` — parse IR text back into a program (roundtrip)

  ## Modules

  - `CodingAdventures.CompilerIr.IrOp` — opcode atom constants
  - `CodingAdventures.CompilerIr.IrRegister` — virtual register operand
  - `CodingAdventures.CompilerIr.IrImmediate` — integer immediate operand
  - `CodingAdventures.CompilerIr.IrLabel` — label operand
  - `CodingAdventures.CompilerIr.IrInstruction` — a single instruction
  - `CodingAdventures.CompilerIr.IrDataDecl` — data segment declaration
  - `CodingAdventures.CompilerIr.IrProgram` — a complete compiled program
  - `CodingAdventures.CompilerIr.IDGenerator` — unique monotonic IDs
  - `CodingAdventures.CompilerIr.Printer` — program → text
  - `CodingAdventures.CompilerIr.Parser` — text → program

  ## Example

      alias CodingAdventures.CompilerIr.{IrProgram, IrInstruction, IrDataDecl,
                                          IrRegister, IrImmediate, IrLabel,
                                          IDGenerator, Printer, Parser}

      # Create a program
      program = IrProgram.new("_start")

      # Set up an ID generator
      {id0, gen} = IDGenerator.next(IDGenerator.new())

      # Build a minimal "load and halt" program
      program =
        program
        |> IrProgram.add_data(%IrDataDecl{label: "tape", size: 30000, init: 0})
        |> IrProgram.add_instruction(%IrInstruction{opcode: :label,
             operands: [%IrLabel{name: "_start"}], id: -1})
        |> IrProgram.add_instruction(%IrInstruction{opcode: :load_addr,
             operands: [%IrRegister{index: 0}, %IrLabel{name: "tape"}], id: id0})
        |> IrProgram.add_instruction(%IrInstruction{opcode: :halt,
             operands: [], id: 1})

      # Print and roundtrip
      text = Printer.print(program)
      {:ok, reparsed} = Parser.parse(text)
      length(reparsed.instructions) == length(program.instructions)  # true
  """
end
