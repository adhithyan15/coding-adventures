defmodule CodingAdventures.VirtualMachine.Types do
  @moduledoc """
  Core types shared by all virtual machine implementations.

  ## The Building Blocks of a VM

  Every virtual machine — from the simplest calculator to the Java Virtual
  Machine — operates on the same fundamental concepts:

  - **Instruction**: a single operation the VM can perform (e.g., "push 42",
    "add", "jump to address 7"). Each instruction has an opcode (a number
    identifying the operation) and an optional operand (extra data the
    operation needs).

  - **CodeObject**: a compiled program — a bundle of instructions plus the
    data they reference (constants like numbers and strings, variable names).
    Think of it as the "executable" that the VM loads and runs.

  - **VMTrace**: a snapshot of one step of execution. By recording a trace
    for every instruction, we can replay the entire computation and
    understand exactly what happened at each step. This is the VM's
    contribution to coding-adventures' "trace everything" philosophy.

  - **CallFrame**: when the VM calls a function, it saves its current
    position (like a bookmark) so it can return later. Each call pushes
    a frame onto the call stack; each return pops one off.

  - **BuiltinFunction**: a function implemented in Elixir (not in bytecode)
    that the VM can call. Think of these as "system calls" — things like
    print, input, or math functions that are easier to implement in the
    host language.

  ## Modules

  Each type is defined in its own module file for independent compilation:

  - `CodingAdventures.VirtualMachine.Types.Instruction` — single bytecode instruction
  - `CodingAdventures.VirtualMachine.Types.CodeObject` — compiled program bundle
  - `CodingAdventures.VirtualMachine.Types.VMTrace` — execution step snapshot
  - `CodingAdventures.VirtualMachine.Types.CallFrame` — function call context
  - `CodingAdventures.VirtualMachine.Types.BuiltinFunction` — host-language callable

  ## Why Structs?

  Each type is a simple Elixir struct with named fields. Structs give us:

  - **Pattern matching**: `%Instruction{opcode: 0x01}` matches only PUSH instructions
  - **Default values**: missing fields get sensible defaults
  - **Clarity**: field names document what each piece of data means
  - **Compile-time checks**: typos in field names are caught immediately
  """
end
