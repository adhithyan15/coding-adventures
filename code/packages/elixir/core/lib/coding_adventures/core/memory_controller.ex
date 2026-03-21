defmodule CodingAdventures.Core.MemoryController do
  @moduledoc """
  Manages access to shared main memory from multiple cores.

  ## Why a Memory Controller?

  In a multi-core system, multiple cores may request memory access in the
  same clock cycle. Real memory (DRAM) can only handle a limited number of
  concurrent requests, so the memory controller queues and serializes them.

  ## Memory Model

  The underlying memory is a flat binary (byte array). Word reads/writes use
  little-endian byte ordering, matching modern ARM and x86 architectures.

  ## Latency Simulation

  Each async memory request takes `latency` cycles to complete. The controller
  counts down remaining cycles on each `tick/1`. When a request reaches zero
  remaining cycles, its data is delivered.
  """

  use Bitwise

  @type t :: %__MODULE__{
          memory: binary(),
          latency: pos_integer(),
          pending_reads: [pending_read()],
          pending_writes: [pending_write()],
          completed_reads: [read_result()]
        }

  @type pending_read :: %{
          address: non_neg_integer(),
          num_bytes: pos_integer(),
          requester_id: non_neg_integer(),
          cycles_left: pos_integer()
        }

  @type pending_write :: %{
          address: non_neg_integer(),
          data: binary(),
          requester_id: non_neg_integer(),
          cycles_left: pos_integer()
        }

  @type read_result :: %{
          requester_id: non_neg_integer(),
          address: non_neg_integer(),
          data: binary()
        }

  defstruct memory: <<>>,
            latency: 100,
            pending_reads: [],
            pending_writes: [],
            completed_reads: []

  @doc """
  Creates a memory controller with the given memory size and access latency.
  """
  @spec new(non_neg_integer(), pos_integer()) :: t()
  def new(memory_size, latency) do
    %__MODULE__{
      memory: :binary.copy(<<0>>, memory_size),
      latency: latency
    }
  end

  @doc "Returns the total size of memory in bytes."
  @spec memory_size(t()) :: non_neg_integer()
  def memory_size(%__MODULE__{memory: memory}), do: byte_size(memory)

  @doc """
  Reads a 32-bit word from memory at the given address.
  Little-endian byte order.
  """
  @spec read_word(t(), integer()) :: integer()
  def read_word(%__MODULE__{memory: memory}, address) do
    if address < 0 or address + 4 > byte_size(memory) do
      0
    else
      <<_::binary-size(address), b0, b1, b2, b3, _::binary>> = memory
      b0 ||| (b1 <<< 8) ||| (b2 <<< 16) ||| (b3 <<< 24)
    end
  end

  @doc """
  Writes a 32-bit word to memory at the given address.
  Little-endian byte order. Returns the updated controller.
  """
  @spec write_word(t(), integer(), integer()) :: t()
  def write_word(%__MODULE__{memory: memory} = mc, address, value) do
    if address < 0 or address + 4 > byte_size(memory) do
      mc
    else
      bytes = <<
        band(value, 0xFF),
        band(bsr(value, 8), 0xFF),
        band(bsr(value, 16), 0xFF),
        band(bsr(value, 24), 0xFF)
      >>

      <<before::binary-size(address), _::binary-size(4), rest::binary>> = memory
      %{mc | memory: before <> bytes <> rest}
    end
  end

  @doc """
  Loads program bytes into memory starting at the given address.
  Returns the updated controller.
  """
  @spec load_program(t(), [byte()], non_neg_integer()) :: t()
  def load_program(%__MODULE__{memory: memory} = mc, program_bytes, start_address) do
    prog_binary = :erlang.list_to_binary(program_bytes)
    prog_size = byte_size(prog_binary)

    if start_address < 0 or start_address + prog_size > byte_size(memory) do
      mc
    else
      <<before::binary-size(start_address), _::binary-size(prog_size), after::binary>> = memory
      %{mc | memory: before <> prog_binary <> after}
    end
  end

  @doc "Submits an async read request."
  @spec request_read(t(), non_neg_integer(), pos_integer(), non_neg_integer()) :: t()
  def request_read(%__MODULE__{} = mc, address, num_bytes, requester_id) do
    req = %{address: address, num_bytes: num_bytes, requester_id: requester_id, cycles_left: mc.latency}
    %{mc | pending_reads: mc.pending_reads ++ [req]}
  end

  @doc "Submits an async write request."
  @spec request_write(t(), non_neg_integer(), binary(), non_neg_integer()) :: t()
  def request_write(%__MODULE__{} = mc, address, data, requester_id) do
    req = %{address: address, data: data, requester_id: requester_id, cycles_left: mc.latency}
    %{mc | pending_writes: mc.pending_writes ++ [req]}
  end

  @doc """
  Advances the memory controller by one cycle.

  Returns `{updated_controller, completed_read_results}`.
  """
  @spec tick(t()) :: {t(), [read_result()]}
  def tick(%__MODULE__{} = mc) do
    # Process reads
    {completed, still_pending} =
      Enum.reduce(mc.pending_reads, {[], []}, fn req, {done, pending} ->
        req = %{req | cycles_left: req.cycles_left - 1}
        if req.cycles_left <= 0 do
          data = read_bytes(mc.memory, req.address, req.num_bytes)
          result = %{requester_id: req.requester_id, address: req.address, data: data}
          {done ++ [result], pending}
        else
          {done, pending ++ [req]}
        end
      end)

    # Process writes
    {mc_memory, still_pending_writes} =
      Enum.reduce(mc.pending_writes, {mc.memory, []}, fn req, {mem, pending} ->
        req = %{req | cycles_left: req.cycles_left - 1}
        if req.cycles_left <= 0 do
          mem = write_bytes(mem, req.address, req.data)
          {mem, pending}
        else
          {mem, pending ++ [req]}
        end
      end)

    mc = %{mc |
      memory: mc_memory,
      pending_reads: still_pending,
      pending_writes: still_pending_writes,
      completed_reads: completed
    }

    {mc, completed}
  end

  @doc "Returns the number of in-flight requests."
  @spec pending_count(t()) :: non_neg_integer()
  def pending_count(%__MODULE__{pending_reads: reads, pending_writes: writes}) do
    length(reads) + length(writes)
  end

  # Internal: read bytes from binary memory.
  defp read_bytes(memory, address, num_bytes) do
    if address < 0 or address + num_bytes > byte_size(memory) do
      :binary.copy(<<0>>, num_bytes)
    else
      binary_part(memory, address, num_bytes)
    end
  end

  # Internal: write bytes into binary memory.
  defp write_bytes(memory, address, data) when is_binary(data) do
    data_size = byte_size(data)
    if address < 0 or address + data_size > byte_size(memory) do
      memory
    else
      <<before::binary-size(address), _::binary-size(data_size), after::binary>> = memory
      before <> data <> after
    end
  end
end
