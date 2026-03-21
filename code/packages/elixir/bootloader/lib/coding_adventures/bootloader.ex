defmodule CodingAdventures.Bootloader do
  @moduledoc "Bootloader code generator and disk image."

  @default_entry 0x00010000
  @default_kernel_load 0x00020000
  @default_stack 0x0006FFF0
  @disk_kernel_offset 0x00080000
  @boot_protocol_magic 0xB007CAFE

  def default_entry, do: @default_entry
  def default_kernel_load, do: @default_kernel_load
  def default_stack, do: @default_stack
  def disk_kernel_offset, do: @disk_kernel_offset
  def boot_protocol_magic, do: @boot_protocol_magic

  defmodule Config do
    defstruct entry_address: 0x00010000, kernel_disk_offset: 0x00080000,
              kernel_load_address: 0x00020000, kernel_size: 0, stack_base: 0x0006FFF0
  end

  alias CodingAdventures.RiscvSimulator.Encoding

  def generate(%Config{} = config) do
    instructions = generate_instructions(config)
    Encoding.assemble(instructions)
  end

  def generate_instructions(%Config{} = _config) do
    import Bitwise
    # Simplified bootloader: load kernel entry address into t0, set sp, jump
    [
      Encoding.encode_lui(5, 1),            # t0 = 0x1000 (boot protocol addr)
      Encoding.encode_lw(6, 5, 0),          # t1 = mem[0x1000] (magic)
      Encoding.encode_lui(5, 0x00020),       # t0 = kernel load address (0x20000)
      Encoding.encode_lui(2, 0x00070),       # sp = 0x70000
      Encoding.encode_addi(2, 2, -16),       # sp = 0x6FFF0
      Encoding.encode_jalr(0, 5, 0),         # jump to kernel
    ]
  end

  def instruction_count(config), do: length(generate_instructions(config))

  defmodule DiskImage do
    defstruct [:data]

    @default_size 2 * 1024 * 1024

    def new(size \\ @default_size), do: %__MODULE__{data: :binary.copy(<<0>>, size)}

    def load_kernel(%__MODULE__{} = disk, kernel_binary) when is_list(kernel_binary) do
      load_kernel(disk, :erlang.list_to_binary(kernel_binary))
    end
    def load_kernel(%__MODULE__{} = disk, kernel_binary) when is_binary(kernel_binary) do
      load_at(disk, 0x00080000, kernel_binary)
    end

    def load_at(%__MODULE__{data: data} = disk, offset, bytes) when is_list(bytes) do
      load_at(disk, offset, :erlang.list_to_binary(bytes))
    end
    def load_at(%__MODULE__{data: data} = disk, offset, bytes) when is_binary(bytes) do
      len = byte_size(bytes)
      <<before::binary-size(offset), _old::binary-size(len), rest::binary>> = data
      %{disk | data: <<before::binary, bytes::binary, rest::binary>>}
    end

    def size(%__MODULE__{data: data}), do: byte_size(data)

    def read_word(%__MODULE__{data: data}, offset) do
      import Bitwise
      <<_::binary-size(offset), b0, b1, b2, b3, _::binary>> = data
      (b0 ||| (b1 <<< 8) ||| (b2 <<< 16) ||| (b3 <<< 24)) &&& 0xFFFFFFFF
    end
  end
end
