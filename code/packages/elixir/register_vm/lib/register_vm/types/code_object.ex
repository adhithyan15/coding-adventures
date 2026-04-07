defmodule CodingAdventures.RegisterVM.Types.CodeObject do
  @moduledoc """
  A CodeObject represents a compiled unit of executable code — roughly
  equivalent to a JavaScript function or a Python code object.

  ## Analogy

  Think of a CodeObject as the "recipe card" for a function. It describes:
  - What ingredients (constants, names) are needed
  - What steps (instructions) to perform
  - How much workspace (registers) is required
  - How many slots to allocate for learning (feedback_vector)

  In V8's Ignition interpreter, each JavaScript function is compiled into a
  BytecodeArray (our `instructions` list) alongside a ConstantPool (our
  `constants` list). This Elixir implementation mirrors that design.

  ## Fields

  - `instructions` — ordered list of `%RegisterInstruction{}` structs. The
    interpreter walks this list sequentially, using `ip` (instruction pointer)
    as the index. Think of it as an array of assembly-level ops.

  - `constants` — pool of literal values referenced by index. When bytecode
    says `LdaConstant 3`, it means "load constants[3] into the accumulator."
    This avoids embedding large values directly in the instruction stream.

  - `names` — pool of string identifiers for variable and property lookups.
    `LdaGlobal 0` means "load the global variable whose name is names[0]."
    Separating names from constants allows the optimizer to deduplicate them.

  - `register_count` — how many register slots to allocate when creating a
    call frame for this function. Registers are like local scratch variables
    in a CPU: fast, indexed, and fixed in number per function. The compiler
    decides this number statically.

  - `feedback_slot_count` — number of "observation slots" to allocate in the
    feedback vector for this function. Each dynamic-dispatch instruction
    (arithmetic, property load, call site) owns one slot where the interpreter
    records what types it saw at runtime. A JIT compiler can later read these
    slots to specialize the hot code path.

  - `parameter_count` — how many arguments this function expects. Used when
    setting up the call frame: args are copied into registers[0..parameter_count-1].

  - `name` — human-readable debug name shown in stack traces and trace output.
  """

  defstruct [
    :instructions,
    :constants,
    :names,
    :register_count,
    :feedback_slot_count,
    parameter_count: 0,
    name: "anonymous"
  ]
end
