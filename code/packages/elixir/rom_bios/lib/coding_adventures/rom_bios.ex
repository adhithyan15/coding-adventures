defmodule CodingAdventures.RomBios do
  @moduledoc "ROM and BIOS firmware for the simulated computer."

  @hardware_info_address 0x00001000
  @hardware_info_size 28

  def hardware_info_address, do: @hardware_info_address
  def hardware_info_size, do: @hardware_info_size

  defmodule ROM do
    defstruct [:base_address, :size, :data]

    def new(config, firmware) when is_list(firmware), do: new(config, :erlang.list_to_binary(firmware))
    def new(%{base_address: base, size: size}, firmware) when byte_size(firmware) <= size do
      padding = :binary.copy(<<0>>, size - byte_size(firmware))
      %__MODULE__{base_address: base, size: size, data: <<firmware::binary, padding::binary>>}
    end

    def read(%__MODULE__{base_address: base, size: size, data: data}, address) do
      offset = address - base
      if offset >= 0 and offset < size, do: :binary.at(data, offset), else: 0
    end

    def read_word(%__MODULE__{base_address: base, size: size, data: data}, address) do
      import Bitwise
      offset = address - base
      if offset >= 0 and offset + 3 < size do
        <<_::binary-size(offset), b0, b1, b2, b3, _::binary>> = data
        (b0 ||| (b1 <<< 8) ||| (b2 <<< 16) ||| (b3 <<< 24)) &&& 0xFFFFFFFF
      else
        0
      end
    end

    def write(rom, _address, _value), do: rom
    def contains?(%__MODULE__{base_address: base, size: size}, address), do: address >= base and address < base + size
  end

  defmodule HardwareInfo do
    defstruct memory_size: 0, display_columns: 80, display_rows: 25,
              framebuffer_base: 0xFFFB0000, idt_base: 0, idt_entries: 256,
              bootloader_entry: 0x00010000

    def to_bytes(%__MODULE__{} = h) do
      import Bitwise
      for field <- [h.memory_size, h.display_columns, h.display_rows, h.framebuffer_base,
                    h.idt_base, h.idt_entries, h.bootloader_entry], into: <<>> do
        v = field &&& 0xFFFFFFFF
        <<v &&& 0xFF, (v >>> 8) &&& 0xFF, (v >>> 16) &&& 0xFF, (v >>> 24) &&& 0xFF>>
      end
    end

    def from_bytes(<<a::little-32, b::little-32, c::little-32, d::little-32,
                     e::little-32, f::little-32, g::little-32, _::binary>>) do
      %__MODULE__{memory_size: a, display_columns: b, display_rows: c,
                  framebuffer_base: d, idt_base: e, idt_entries: f, bootloader_entry: g}
    end
  end
end
