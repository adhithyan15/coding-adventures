defmodule CodingAdventures.CpuSimulator.Memory do
  @moduledoc "Byte-addressable RAM simulation using a binary."

  defstruct [:data, :size]

  def new(size) when size >= 1 do
    %__MODULE__{data: :binary.copy(<<0>>, size), size: size}
  end

  def read_byte(%__MODULE__{data: data, size: size}, address)
      when address >= 0 and address < size do
    :binary.at(data, address)
  end

  def write_byte(%__MODULE__{data: data, size: size} = mem, address, value)
      when address >= 0 and address < size do
    <<before::binary-size(address), _old, rest::binary>> = data
    %{mem | data: <<before::binary, Bitwise.band(value, 0xFF), rest::binary>>}
  end

  def read_word(%__MODULE__{} = mem, address) do
    import Bitwise
    b0 = read_byte(mem, address)
    b1 = read_byte(mem, address + 1)
    b2 = read_byte(mem, address + 2)
    b3 = read_byte(mem, address + 3)
    (b0 ||| (b1 <<< 8) ||| (b2 <<< 16) ||| (b3 <<< 24)) &&& 0xFFFFFFFF
  end

  def write_word(%__MODULE__{} = mem, address, value) do
    import Bitwise
    v = value &&& 0xFFFFFFFF
    mem
    |> write_byte(address, v &&& 0xFF)
    |> write_byte(address + 1, (v >>> 8) &&& 0xFF)
    |> write_byte(address + 2, (v >>> 16) &&& 0xFF)
    |> write_byte(address + 3, (v >>> 24) &&& 0xFF)
  end

  def load_bytes(%__MODULE__{} = mem, address, bytes) when is_list(bytes) do
    load_bytes(mem, address, :erlang.list_to_binary(bytes))
  end

  def load_bytes(%__MODULE__{data: data} = mem, address, bytes) when is_binary(bytes) do
    len = byte_size(bytes)
    <<before::binary-size(address), _old::binary-size(len), rest::binary>> = data
    %{mem | data: <<before::binary, bytes::binary, rest::binary>>}
  end

  def dump(%__MODULE__{data: data}, start, length) do
    <<_::binary-size(start), slice::binary-size(length), _::binary>> = data
    :binary.bin_to_list(slice)
  end
end
