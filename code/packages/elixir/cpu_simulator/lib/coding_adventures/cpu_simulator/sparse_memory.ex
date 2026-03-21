defmodule CodingAdventures.CpuSimulator.SparseMemory do
  @moduledoc "Sparse memory -- maps non-contiguous address ranges to backing storage."

  defstruct [:regions]

  defmodule Region do
    @moduledoc false
    defstruct [:base, :size, :name, :read_only, :data]
  end

  def new(region_configs) do
    regions =
      Enum.map(region_configs, fn r ->
        data = Map.get(r, :data, :binary.copy(<<0>>, r.size))

        %Region{
          base: r.base,
          size: r.size,
          name: r.name,
          read_only: Map.get(r, :read_only, false),
          data: data
        }
      end)

    %__MODULE__{regions: regions}
  end

  defp find_region(%__MODULE__{regions: regions}, address, num_bytes) do
    import Bitwise
    addr = address &&& 0xFFFFFFFF

    result =
      Enum.find_value(regions, fn r ->
        if addr >= r.base and addr + num_bytes <= r.base + r.size do
          {r, addr - r.base}
        end
      end)

    case result do
      nil ->
        hex = Integer.to_string(addr, 16) |> String.pad_leading(8, "0")
        raise "SparseMemory: unmapped address 0x#{hex}"

      found ->
        found
    end
  end

  def read_byte(mem, address) do
    {region, offset} = find_region(mem, address, 1)
    :binary.at(region.data, offset)
  end

  def write_byte(mem, address, value) do
    import Bitwise
    {region, offset} = find_region(mem, address, 1)

    if region.read_only do
      mem
    else
      update_region(mem, region, offset, <<value &&& 0xFF>>)
    end
  end

  def read_word(mem, address) do
    import Bitwise
    {region, offset} = find_region(mem, address, 4)
    <<_::binary-size(offset), b0, b1, b2, b3, _::binary>> = region.data
    (b0 ||| (b1 <<< 8) ||| (b2 <<< 16) ||| (b3 <<< 24)) &&& 0xFFFFFFFF
  end

  def write_word(mem, address, value) do
    import Bitwise
    {region, offset} = find_region(mem, address, 4)

    if region.read_only do
      mem
    else
      v = value &&& 0xFFFFFFFF

      bytes =
        <<v &&& 0xFF, (v >>> 8) &&& 0xFF, (v >>> 16) &&& 0xFF, (v >>> 24) &&& 0xFF>>

      update_region(mem, region, offset, bytes)
    end
  end

  def load_bytes(mem, address, data) when is_list(data) do
    load_bytes(mem, address, :erlang.list_to_binary(data))
  end

  def load_bytes(mem, address, data) when is_binary(data) do
    {region, offset} = find_region(mem, address, byte_size(data))
    update_region(mem, region, offset, data)
  end

  def dump(mem, start, length) do
    {region, offset} = find_region(mem, start, length)
    <<_::binary-size(offset), slice::binary-size(length), _::binary>> = region.data
    :binary.bin_to_list(slice)
  end

  def region_count(%__MODULE__{regions: regions}), do: length(regions)

  defp update_region(mem, region, offset, new_bytes) do
    len = byte_size(new_bytes)
    <<before::binary-size(offset), _old::binary-size(len), rest::binary>> = region.data
    new_data = <<before::binary, new_bytes::binary, rest::binary>>
    new_region = %{region | data: new_data}

    new_regions =
      Enum.map(mem.regions, fn r ->
        if r.base == region.base, do: new_region, else: r
      end)

    %{mem | regions: new_regions}
  end
end
