defmodule CodingAdventures.WasmExecution.LinearMemory do
  @moduledoc """
  WASM linear memory implementation using Elixir binaries.

  ## What is Linear Memory?

  WebAssembly's memory model is a contiguous, byte-addressable array of
  bytes called "linear memory". It is measured in "pages" where each page
  is exactly 65,536 bytes (64 KiB).

  ## Immutable Design with Binaries

  In Elixir, all data is immutable. We represent the memory buffer as an
  Elixir binary (bitstring). Each write operation produces a new binary.
  For small WASM programs this is perfectly adequate. For production use,
  one would use `:atomics` or a NIF for mutable random-access memory.

  ## Little-Endian Byte Ordering

  WASM always uses little-endian byte order. The least-significant byte
  comes first in memory.
  """

  alias CodingAdventures.WasmExecution.TrapError

  @page_size 65_536

  defstruct [:buffer, :current_pages, :max_pages]

  @type t :: %__MODULE__{
          buffer: binary(),
          current_pages: non_neg_integer(),
          max_pages: non_neg_integer() | nil
        }

  @doc """
  Create a new LinearMemory with the given initial pages and optional max.
  """
  @spec new(non_neg_integer(), non_neg_integer() | nil) :: t()
  def new(initial_pages, max_pages \\ nil) do
    buffer = :binary.copy(<<0>>, initial_pages * @page_size)

    %__MODULE__{
      buffer: buffer,
      current_pages: initial_pages,
      max_pages: max_pages
    }
  end

  @doc "Return the page size constant (65536)."
  def page_size, do: @page_size

  @doc "Return the current memory size in pages."
  @spec size(t()) :: non_neg_integer()
  def size(%__MODULE__{current_pages: pages}), do: pages

  @doc "Return the current memory size in bytes."
  @spec byte_length(t()) :: non_neg_integer()
  def byte_length(%__MODULE__{} = mem), do: byte_size(mem.buffer)

  # ===========================================================================
  # Full-Width Loads
  # ===========================================================================

  @doc "Load 4 bytes as a signed 32-bit integer (little-endian)."
  @spec load_i32(t(), non_neg_integer()) :: integer()
  def load_i32(%__MODULE__{} = mem, offset) do
    bounds_check!(mem, offset, 4)
    <<_::binary-size(offset), value::little-signed-integer-size(32), _::binary>> = mem.buffer
    value
  end

  @doc "Load 8 bytes as a signed 64-bit integer (little-endian)."
  @spec load_i64(t(), non_neg_integer()) :: integer()
  def load_i64(%__MODULE__{} = mem, offset) do
    bounds_check!(mem, offset, 8)
    <<_::binary-size(offset), value::little-signed-integer-size(64), _::binary>> = mem.buffer
    value
  end

  @doc "Load 4 bytes as a 32-bit float (little-endian)."
  @spec load_f32(t(), non_neg_integer()) :: float()
  def load_f32(%__MODULE__{} = mem, offset) do
    bounds_check!(mem, offset, 4)
    <<_::binary-size(offset), value::little-float-size(32), _::binary>> = mem.buffer
    value
  end

  @doc "Load 8 bytes as a 64-bit float (little-endian)."
  @spec load_f64(t(), non_neg_integer()) :: float()
  def load_f64(%__MODULE__{} = mem, offset) do
    bounds_check!(mem, offset, 8)
    <<_::binary-size(offset), value::little-float-size(64), _::binary>> = mem.buffer
    value
  end

  # ===========================================================================
  # Narrow Loads for i32
  # ===========================================================================

  @doc "Load 1 byte, sign-extend to i32."
  def load_i32_8s(%__MODULE__{} = mem, offset) do
    bounds_check!(mem, offset, 1)
    <<_::binary-size(offset), value::signed-integer-size(8), _::binary>> = mem.buffer
    value
  end

  @doc "Load 1 byte, zero-extend to i32."
  def load_i32_8u(%__MODULE__{} = mem, offset) do
    bounds_check!(mem, offset, 1)
    <<_::binary-size(offset), value::unsigned-integer-size(8), _::binary>> = mem.buffer
    value
  end

  @doc "Load 2 bytes (little-endian), sign-extend to i32."
  def load_i32_16s(%__MODULE__{} = mem, offset) do
    bounds_check!(mem, offset, 2)
    <<_::binary-size(offset), value::little-signed-integer-size(16), _::binary>> = mem.buffer
    value
  end

  @doc "Load 2 bytes (little-endian), zero-extend to i32."
  def load_i32_16u(%__MODULE__{} = mem, offset) do
    bounds_check!(mem, offset, 2)
    <<_::binary-size(offset), value::little-unsigned-integer-size(16), _::binary>> = mem.buffer
    value
  end

  # ===========================================================================
  # Narrow Loads for i64
  # ===========================================================================

  def load_i64_8s(%__MODULE__{} = mem, offset) do
    bounds_check!(mem, offset, 1)
    <<_::binary-size(offset), value::signed-integer-size(8), _::binary>> = mem.buffer
    value
  end

  def load_i64_8u(%__MODULE__{} = mem, offset) do
    bounds_check!(mem, offset, 1)
    <<_::binary-size(offset), value::unsigned-integer-size(8), _::binary>> = mem.buffer
    value
  end

  def load_i64_16s(%__MODULE__{} = mem, offset) do
    bounds_check!(mem, offset, 2)
    <<_::binary-size(offset), value::little-signed-integer-size(16), _::binary>> = mem.buffer
    value
  end

  def load_i64_16u(%__MODULE__{} = mem, offset) do
    bounds_check!(mem, offset, 2)
    <<_::binary-size(offset), value::little-unsigned-integer-size(16), _::binary>> = mem.buffer
    value
  end

  def load_i64_32s(%__MODULE__{} = mem, offset) do
    bounds_check!(mem, offset, 4)
    <<_::binary-size(offset), value::little-signed-integer-size(32), _::binary>> = mem.buffer
    value
  end

  def load_i64_32u(%__MODULE__{} = mem, offset) do
    bounds_check!(mem, offset, 4)
    <<_::binary-size(offset), value::little-unsigned-integer-size(32), _::binary>> = mem.buffer
    value
  end

  # ===========================================================================
  # Full-Width Stores
  # ===========================================================================

  @doc "Store a 32-bit integer (little-endian). Returns updated memory."
  @spec store_i32(t(), non_neg_integer(), integer()) :: t()
  def store_i32(%__MODULE__{} = mem, offset, value) do
    bounds_check!(mem, offset, 4)
    write_bytes(mem, offset, <<value::little-signed-integer-size(32)>>)
  end

  @doc "Store a 64-bit integer (little-endian)."
  @spec store_i64(t(), non_neg_integer(), integer()) :: t()
  def store_i64(%__MODULE__{} = mem, offset, value) do
    bounds_check!(mem, offset, 8)
    write_bytes(mem, offset, <<value::little-signed-integer-size(64)>>)
  end

  @doc "Store a 32-bit float (little-endian)."
  @spec store_f32(t(), non_neg_integer(), float()) :: t()
  def store_f32(%__MODULE__{} = mem, offset, value) do
    bounds_check!(mem, offset, 4)
    write_bytes(mem, offset, <<value::little-float-size(32)>>)
  end

  @doc "Store a 64-bit float (little-endian)."
  @spec store_f64(t(), non_neg_integer(), float()) :: t()
  def store_f64(%__MODULE__{} = mem, offset, value) do
    bounds_check!(mem, offset, 8)
    write_bytes(mem, offset, <<value::little-float-size(64)>>)
  end

  # ===========================================================================
  # Narrow Stores
  # ===========================================================================

  def store_i32_8(%__MODULE__{} = mem, offset, value) do
    bounds_check!(mem, offset, 1)
    write_bytes(mem, offset, <<value::signed-integer-size(8)>>)
  end

  def store_i32_16(%__MODULE__{} = mem, offset, value) do
    bounds_check!(mem, offset, 2)
    write_bytes(mem, offset, <<value::little-signed-integer-size(16)>>)
  end

  def store_i64_8(%__MODULE__{} = mem, offset, value) do
    bounds_check!(mem, offset, 1)
    write_bytes(mem, offset, <<Bitwise.band(value, 0xFF)::integer-size(8)>>)
  end

  def store_i64_16(%__MODULE__{} = mem, offset, value) do
    bounds_check!(mem, offset, 2)
    write_bytes(mem, offset, <<Bitwise.band(value, 0xFFFF)::little-integer-size(16)>>)
  end

  def store_i64_32(%__MODULE__{} = mem, offset, value) do
    bounds_check!(mem, offset, 4)
    write_bytes(mem, offset, <<Bitwise.band(value, 0xFFFFFFFF)::little-integer-size(32)>>)
  end

  # ===========================================================================
  # Memory Growth
  # ===========================================================================

  @doc """
  Grow memory by `delta_pages` pages. Returns `{old_page_count, updated_memory}`
  on success, or `{-1, unchanged_memory}` if growth would exceed the maximum.
  """
  @spec grow(t(), non_neg_integer()) :: {integer(), t()}
  def grow(%__MODULE__{} = mem, delta_pages) do
    old_pages = mem.current_pages
    new_pages = old_pages + delta_pages

    cond do
      mem.max_pages != nil and new_pages > mem.max_pages ->
        {-1, mem}

      new_pages > 65_536 ->
        {-1, mem}

      true ->
        new_bytes = :binary.copy(<<0>>, delta_pages * @page_size)
        new_buffer = mem.buffer <> new_bytes
        {old_pages, %{mem | buffer: new_buffer, current_pages: new_pages}}
    end
  end

  # ===========================================================================
  # Raw Byte Access
  # ===========================================================================

  @doc "Write raw bytes into memory at the given offset. Returns updated memory."
  @spec write_raw_bytes(t(), non_neg_integer(), binary()) :: t()
  def write_raw_bytes(%__MODULE__{} = mem, offset, data) when is_binary(data) do
    bounds_check!(mem, offset, byte_size(data))
    write_bytes(mem, offset, data)
  end

  # ===========================================================================
  # Internal Helpers
  # ===========================================================================

  defp bounds_check!(%__MODULE__{} = mem, offset, width) do
    if offset < 0 or offset + width > byte_size(mem.buffer) do
      raise TrapError,
            "Out of bounds memory access: offset=#{offset}, size=#{width}, " <>
              "memory size=#{byte_size(mem.buffer)}"
    end
  end

  defp write_bytes(%__MODULE__{} = mem, offset, data) do
    data_size = byte_size(data)
    buf = mem.buffer
    prefix_size = offset
    suffix_start = offset + data_size
    suffix_size = byte_size(buf) - suffix_start

    <<prefix::binary-size(prefix_size), _::binary-size(data_size),
      suffix::binary-size(suffix_size)>> = buf

    %{mem | buffer: <<prefix::binary, data::binary, suffix::binary>>}
  end
end
