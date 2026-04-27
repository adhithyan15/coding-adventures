defmodule CodingAdventures.CompilerIr.IrRegister do
  @moduledoc """
  A virtual register operand.

  Virtual registers are named `v0`, `v1`, `v2`, ... (the `index` field).
  There are infinitely many — the backend's register allocator maps them
  to physical registers.

  ## Brainfuck register allocation

  Brainfuck compiles to a fixed set of virtual registers:

  | Register | Role                      |
  |----------|---------------------------|
  | v0       | tape base address         |
  | v1       | tape pointer offset       |
  | v2       | temporary (cell value)    |
  | v3       | temporary (bounds checks) |
  | v4       | syscall argument register |
  | v5       | max pointer (tape_size-1) |
  | v6       | zero constant             |

  Future languages (BASIC, Lua) with more complexity will use a register
  allocator in the backend to assign virtual registers to physical ones.

  ## Examples

      iex> IrRegister.to_string(%IrRegister{index: 0})
      "v0"

      iex> IrRegister.to_string(%IrRegister{index: 5})
      "v5"
  """

  defstruct [:index]

  @type t :: %__MODULE__{
          index: non_neg_integer()
        }

  @doc "Return the text representation of this register (e.g. \"v2\")."
  @spec to_string(t()) :: String.t()
  def to_string(%__MODULE__{index: i}), do: "v#{i}"
end

defmodule CodingAdventures.CompilerIr.IrImmediate do
  @moduledoc """
  A literal integer operand.

  Immediates are signed integers that appear directly in instructions.
  In the IR text format they are printed without a prefix:

      ADD_IMM v1, v1, 1    ← 1 is an IrImmediate
      AND_IMM v2, v2, 255  ← 255 is an IrImmediate
      ADD_IMM v1, v1, -1   ← -1 is an IrImmediate

  The value is always an integer (never a float). The backend is
  responsible for clamping or sign-extending as required by the target ISA.
  """

  defstruct [:value]

  @type t :: %__MODULE__{
          value: integer()
        }

  @doc "Return the text representation of this immediate (e.g. \"42\")."
  @spec to_string(t()) :: String.t()
  def to_string(%__MODULE__{value: v}), do: Integer.to_string(v)
end

defmodule CodingAdventures.CompilerIr.IrLabel do
  @moduledoc """
  A named label operand — a jump target or data reference.

  Labels are strings like `"loop_0_start"`, `"_start"`, `"tape"`,
  `"__trap_oob"`. They resolve to addresses during code generation.

  A label appears as an operand in three contexts:

  1. **Definition** — `LABEL _start` defines where execution begins.
  2. **Jump target** — `JUMP loop_0_start` refers to a label by name.
  3. **Data reference** — `LOAD_ADDR v0, tape` loads the address of a
     data segment into a register.

  The IR text format prints labels without quotes:

      LABEL    _start         ; #0
      LOAD_ADDR v0, tape      ; #2
      JUMP     loop_0_start   ; #10
  """

  defstruct [:name]

  @type t :: %__MODULE__{
          name: String.t()
        }

  @doc "Return the text representation of this label (the bare name)."
  @spec to_string(t()) :: String.t()
  def to_string(%__MODULE__{name: n}), do: n
end

defmodule CodingAdventures.CompilerIr.IrInstruction do
  @moduledoc """
  A single IR instruction.

  Every instruction has:

  - `opcode`   — the operation to perform (an `IrOp` atom)
  - `operands` — the arguments: registers, immediates, and/or labels
  - `id`       — a unique monotonic integer used for source mapping

  The `id` field is the key connecting an instruction to the source map
  chain. Each instruction gets a unique ID from `IDGenerator.next/1`, and
  that ID flows through all pipeline stages so debuggers can trace machine
  code back to source.

  Labels and comments use `id: -1` because they produce no machine code
  and therefore cannot be mapped to a machine-code offset.

  ## Text format

      ADD_IMM    v1, v1, 1   ; #3
      BRANCH_Z   v2, loop_0_end  ; #7
      _start:                    ← label (no ID)
      ; this is a comment        ← comment (no ID)
  """

  alias CodingAdventures.CompilerIr.IrOp

  # An operand is one of the three operand structs.
  @type operand ::
          CodingAdventures.CompilerIr.IrRegister.t()
          | CodingAdventures.CompilerIr.IrImmediate.t()
          | CodingAdventures.CompilerIr.IrLabel.t()

  defstruct opcode: :nop, operands: [], id: -1

  @type t :: %__MODULE__{
          opcode: IrOp.t(),
          operands: [operand()],
          id: integer()
        }
end

defmodule CodingAdventures.CompilerIr.IrDataDecl do
  @moduledoc """
  A data segment declaration.

  Declares a named region of memory with a given size and initial byte
  value. For Brainfuck, this is the tape:

      .data tape 30000 0

  Meaning: a 30,000-byte region labelled "tape", initialised to all zeros.

  The `init` value is repeated for every byte in the region. `init: 0`
  is equivalent to `.bss` in most assembly formats (zero-initialised data),
  which is typically placed in a separate section by the linker and backed
  by the OS's demand-zero mechanism.

  ## Fields

  - `label` — the name used to reference this region (e.g. `"tape"`)
  - `size`  — the number of bytes to allocate (positive integer)
  - `init`  — initial byte value 0–255 (usually 0)
  """

  defstruct [:label, :size, init: 0]

  @type t :: %__MODULE__{
          label: String.t(),
          size: pos_integer(),
          init: 0..255
        }
end

defmodule CodingAdventures.CompilerIr.IrProgram do
  @moduledoc """
  A complete IR program — the output of compilation.

  An `IrProgram` contains:

  - `instructions` — the linear sequence of IR instructions (ordered)
  - `data`         — data segment declarations (`.data` / `.bss` equivalents)
  - `entry_label`  — the label where execution begins (typically `"_start"`)
  - `version`      — IR version (1 = Brainfuck subset, grows over time)

  ## Execution model

  The instruction list is flat and ordered. Execution flows from index 0
  to the end, with labels, jumps, and branches altering the path. There
  are no basic blocks or SSA form — the IR is deliberately simple to make
  backends easy to write.

  ## Creating a program

      program = IrProgram.new("_start")
      program = IrProgram.add_instruction(program, %IrInstruction{...})
      program = IrProgram.add_data(program, %IrDataDecl{...})

  Note: in Elixir, structs are immutable. `add_instruction/2` returns a
  new `IrProgram` with the instruction appended — it does not modify the
  original.
  """

  alias CodingAdventures.CompilerIr.{IrInstruction, IrDataDecl}

  defstruct instructions: [], data: [], entry_label: "_start", version: 1

  @type t :: %__MODULE__{
          instructions: [IrInstruction.t()],
          data: [IrDataDecl.t()],
          entry_label: String.t(),
          version: pos_integer()
        }

  @doc """
  Create a new IR program with the given entry label and version 1.

  ## Examples

      iex> p = IrProgram.new("_start")
      iex> p.entry_label
      "_start"
      iex> p.version
      1
  """
  @spec new(String.t()) :: t()
  def new(entry_label) do
    %__MODULE__{entry_label: entry_label, version: 1}
  end

  @doc """
  Append an instruction to the program.

  Returns a new `IrProgram` with the instruction added at the end.
  Instructions are stored in emission order.

  ## Examples

      iex> p = IrProgram.new("_start")
      iex> instr = %IrInstruction{opcode: :halt, operands: [], id: 0}
      iex> p2 = IrProgram.add_instruction(p, instr)
      iex> length(p2.instructions)
      1
  """
  @spec add_instruction(t(), IrInstruction.t()) :: t()
  def add_instruction(%__MODULE__{} = program, %IrInstruction{} = instr) do
    %{program | instructions: program.instructions ++ [instr]}
  end

  @doc """
  Append a data declaration to the program.

  Returns a new `IrProgram` with the data declaration added.

  ## Examples

      iex> p = IrProgram.new("_start")
      iex> d = %IrDataDecl{label: "tape", size: 30000, init: 0}
      iex> p2 = IrProgram.add_data(p, d)
      iex> length(p2.data)
      1
  """
  @spec add_data(t(), IrDataDecl.t()) :: t()
  def add_data(%__MODULE__{} = program, %IrDataDecl{} = decl) do
    %{program | data: program.data ++ [decl]}
  end
end

defmodule CodingAdventures.CompilerIr.IDGenerator do
  @moduledoc """
  Monotonically increasing unique instruction ID generator.

  Every IR instruction in the pipeline needs a unique ID for source
  mapping. The `IDGenerator` ensures no two instructions share an ID,
  even across multiple compilation passes.

  Since Elixir data is immutable, the generator is a plain struct that
  threads through the compiler state. Each call to `next/1` returns
  both the new ID and the updated generator.

  ## Usage

      gen = IDGenerator.new()
      {id1, gen} = IDGenerator.next(gen)   # id1 = 0
      {id2, gen} = IDGenerator.next(gen)   # id2 = 1
      {id3, gen} = IDGenerator.next(gen)   # id3 = 2

  ## Thread safety

  Because the generator is a value (not a process), it is inherently
  thread-safe — each caller has their own copy.
  """

  defstruct next: 0

  @type t :: %__MODULE__{
          next: non_neg_integer()
        }

  @doc "Create a new ID generator starting at 0."
  @spec new() :: t()
  def new, do: %__MODULE__{next: 0}

  @doc """
  Create a new ID generator starting at the given value.

  Useful when multiple compilers contribute instructions to the same
  program and IDs must not collide.
  """
  @spec new_from(non_neg_integer()) :: t()
  def new_from(start) when is_integer(start) and start >= 0 do
    %__MODULE__{next: start}
  end

  @doc """
  Return the next unique ID and the updated generator.

  The ID is the current counter value; the returned generator has the
  counter incremented by 1.

  ## Examples

      iex> gen = IDGenerator.new()
      iex> {0, gen2} = IDGenerator.next(gen)
      iex> {1, _gen3} = IDGenerator.next(gen2)
  """
  @spec next(t()) :: {non_neg_integer(), t()}
  def next(%__MODULE__{next: n} = gen) do
    {n, %{gen | next: n + 1}}
  end

  @doc """
  Return the current counter value without incrementing.

  This is the ID that will be returned by the next call to `next/1`.
  """
  @spec current(t()) :: non_neg_integer()
  def current(%__MODULE__{next: n}), do: n
end
