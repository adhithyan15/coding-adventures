defmodule CodingAdventures.BlockRam.SinglePortRAM do
  @moduledoc """
  Single-Port RAM — memory with one read/write port.

  ## What is Single-Port RAM?

  Single-port RAM has one address bus and one data bus. On each clock cycle,
  you can either read OR write — but not both simultaneously. This is the
  simplest RAM configuration and is used when only one device needs to
  access the memory at a time.

  In an FPGA, single-port BRAM is commonly used for:
    - Lookup tables (sine/cosine tables, color palettes)
    - Instruction memory (when the CPU only fetches one instruction per cycle)
    - FIFO buffers (when read and write never happen in the same cycle)

  ## Interface

  The RAM has these signals:
    - `address` — which word to access (integer)
    - `data_in` — the data to write (list of bits, length = word width)
    - `write_enable` — 1 to write, 0 to read
    - `chip_enable` — 1 to activate the RAM, 0 to deactivate (high-Z)

  ## Functional Model

  The state is an `SRAMArray` struct. Operations return
  `{data_out, new_state}` tuples.
  """

  alias CodingAdventures.BlockRam.SRAMArray

  defstruct [:memory]

  @type t :: %__MODULE__{memory: SRAMArray.t()}

  @doc """
  Creates a new single-port RAM with the given depth and width.

  ## Examples

      iex> ram = CodingAdventures.BlockRam.SinglePortRAM.new(16, 8)
      iex> ram.memory.depth
      16
  """
  @spec new(pos_integer(), pos_integer()) :: t()
  def new(depth, width) do
    %__MODULE__{memory: SRAMArray.new(depth, width)}
  end

  @doc """
  Performs a read or write operation on the RAM.

  When chip_enable=0, the RAM is inactive and returns nil for each bit
  (high impedance), and the state is unchanged.

  When chip_enable=1 and write_enable=0, reads the word at the given
  address and returns it as a list of bits.

  When chip_enable=1 and write_enable=1, writes data_in to the given
  address. The data_out is the data that was just written (write-through).

  Returns `{data_out, new_ram}`.

  ## Examples

      iex> ram = CodingAdventures.BlockRam.SinglePortRAM.new(4, 4)
      iex> {_out, ram} = CodingAdventures.BlockRam.SinglePortRAM.access(ram, 0, [1, 0, 1, 0], 1, 1)
      iex> {data, _ram} = CodingAdventures.BlockRam.SinglePortRAM.access(ram, 0, [0, 0, 0, 0], 0, 1)
      iex> data
      [1, 0, 1, 0]
  """
  @spec access(t(), non_neg_integer(), [0 | 1], 0 | 1, 0 | 1) :: {[0 | 1 | nil], t()}
  def access(%__MODULE__{} = ram, _address, _data_in, _write_enable, 0) do
    # Chip not enabled — high impedance output, no state change
    data_out = List.duplicate(nil, ram.memory.width)
    {data_out, ram}
  end

  def access(%__MODULE__{} = ram, address, data_in, 1, 1) do
    # Write operation: store data_in at address
    new_memory = SRAMArray.write(ram.memory, address, data_in, 1)
    new_ram = %{ram | memory: new_memory}
    # Write-through: return the data that was written
    {data_in, new_ram}
  end

  def access(%__MODULE__{} = ram, address, _data_in, 0, 1) do
    # Read operation: fetch the word at address
    data_out = SRAMArray.read(ram.memory, address, 1)
    {data_out, ram}
  end
end

defmodule CodingAdventures.BlockRam.DualPortRAM do
  @moduledoc """
  Dual-Port RAM — memory with two independent read/write ports.

  ## What is Dual-Port RAM?

  Dual-port RAM has two completely independent sets of address, data, and
  control signals. Both ports can access the memory simultaneously, even
  at different addresses. This is a critical resource in FPGA designs:

    - Video frame buffers: one port writes pixels, the other reads for display
    - CPU caches: one port handles instruction fetch, the other handles data
    - Networking: one port receives packets, the other processes them

  ## Port Conflict Resolution

  When both ports write to the SAME address in the same cycle, we need a
  conflict resolution policy. Common policies include:
    - Port A wins (our default)
    - Port B wins
    - Undefined behavior (what real hardware often does)

  We implement "Port A wins" for determinism and testability.

  ## Functional Model

  The state is an `SRAMArray` struct shared between both ports.
  The `access/2` function takes two port specifications and returns
  results for both ports.
  """

  alias CodingAdventures.BlockRam.SRAMArray

  defstruct [:memory]

  @type t :: %__MODULE__{memory: SRAMArray.t()}

  @type port_spec :: %{
          address: non_neg_integer(),
          data_in: [0 | 1],
          write_enable: 0 | 1,
          chip_enable: 0 | 1
        }

  @doc """
  Creates a new dual-port RAM with the given depth and width.

  ## Examples

      iex> ram = CodingAdventures.BlockRam.DualPortRAM.new(16, 8)
      iex> ram.memory.depth
      16
  """
  @spec new(pos_integer(), pos_integer()) :: t()
  def new(depth, width) do
    %__MODULE__{memory: SRAMArray.new(depth, width)}
  end

  @doc """
  Performs simultaneous access on both ports.

  Each port is specified as a map with keys:
    - `:address` — the word address
    - `:data_in` — list of bits to write
    - `:write_enable` — 1 to write, 0 to read
    - `:chip_enable` — 1 to activate, 0 for high-Z

  If both ports write to the same address, Port A's data wins.

  Returns `{data_out_a, data_out_b, new_ram}`.

  ## Examples

      iex> ram = CodingAdventures.BlockRam.DualPortRAM.new(4, 4)
      iex> port_a = %{address: 0, data_in: [1, 0, 1, 0], write_enable: 1, chip_enable: 1}
      iex> port_b = %{address: 1, data_in: [0, 1, 0, 1], write_enable: 1, chip_enable: 1}
      iex> {_a_out, _b_out, ram} = CodingAdventures.BlockRam.DualPortRAM.access(ram, port_a, port_b)
      iex> port_a_read = %{address: 0, data_in: [0,0,0,0], write_enable: 0, chip_enable: 1}
      iex> port_b_read = %{address: 1, data_in: [0,0,0,0], write_enable: 0, chip_enable: 1}
      iex> {a_data, b_data, _ram} = CodingAdventures.BlockRam.DualPortRAM.access(ram, port_a_read, port_b_read)
      iex> a_data
      [1, 0, 1, 0]
      iex> b_data
      [0, 1, 0, 1]
  """
  @spec access(t(), port_spec(), port_spec()) :: {[0 | 1 | nil], [0 | 1 | nil], t()}
  def access(%__MODULE__{} = ram, port_a, port_b) do
    width = ram.memory.width

    # Process writes first, then reads.
    # Port A writes take priority over Port B writes at the same address.

    # Step 1: Apply Port B write (if any)
    memory_after_b =
      if port_b.chip_enable == 1 and port_b.write_enable == 1 do
        SRAMArray.write(ram.memory, port_b.address, port_b.data_in, 1)
      else
        ram.memory
      end

    # Step 2: Apply Port A write (if any) — overwrites Port B at same address
    memory_after_both =
      if port_a.chip_enable == 1 and port_a.write_enable == 1 do
        SRAMArray.write(memory_after_b, port_a.address, port_a.data_in, 1)
      else
        memory_after_b
      end

    # Step 3: Read from the final memory state
    data_out_a =
      if port_a.chip_enable == 1 do
        if port_a.write_enable == 1 do
          # Write-through: return the written data
          port_a.data_in
        else
          SRAMArray.read(memory_after_both, port_a.address, 1)
        end
      else
        List.duplicate(nil, width)
      end

    data_out_b =
      if port_b.chip_enable == 1 do
        if port_b.write_enable == 1 do
          # If Port A also wrote to the same address, Port B sees Port A's data
          if port_a.chip_enable == 1 and port_a.write_enable == 1 and
               port_a.address == port_b.address do
            port_a.data_in
          else
            port_b.data_in
          end
        else
          SRAMArray.read(memory_after_both, port_b.address, 1)
        end
      else
        List.duplicate(nil, width)
      end

    {data_out_a, data_out_b, %{ram | memory: memory_after_both}}
  end
end
