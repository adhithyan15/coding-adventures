# Device Driver Framework
# =======================
#
# A device driver is a piece of software that knows how to talk to a specific
# piece of hardware. Without drivers, every program that wanted to read from a
# disk would need to know the exact protocol for that specific disk model —
# the register addresses, the timing requirements, the error codes.
#
# Device drivers provide a **uniform interface** over diverse hardware. A program
# says "read 512 bytes from block 7" and the driver translates that into whatever
# specific commands the hardware needs. The program never knows (or cares) whether
# the disk is a spinning platter, a solid-state drive, or an in-memory simulation.
#
# **Analogy:** Think of a universal remote control. You press "Volume Up" and it
# works on your Samsung TV, your Sony soundbar, and your LG projector. Device
# drivers are the universal remote for your operating system.
#
# This module implements three device families:
#   1. Character devices — byte streams (keyboard, serial, display)
#   2. Block devices — fixed-size blocks (disk)
#   3. Network devices — packets (NIC)
#
# Plus a DeviceRegistry for registering and looking up devices.
#
# Because Elixir is a functional language, devices are represented as structs
# with functions that return new structs (immutable data). This is different
# from the TypeScript/Python implementations where devices are mutable objects.

defmodule CodingAdventures.DeviceDriverFramework do
  @moduledoc """
  Device driver framework: character, block, and network device abstractions.

  Provides a unified interface for three device families (character, block,
  network), a device registry, and concrete simulated implementations.
  """

  # ==========================================================================
  # Device Types
  # ==========================================================================
  #
  # Not all hardware behaves the same way:
  #   - A keyboard produces one byte at a time (character device)
  #   - A disk reads/writes fixed-size chunks (block device)
  #   - A network card sends/receives packets (network device)
  #
  # We use atoms instead of integer enums because Elixir idiom prefers atoms
  # for categorical values. The integer values are available via device_type_value/1
  # for interoperability with other languages.

  @type device_type :: :character | :block | :network

  @doc "Convert a device type atom to its integer value (for cross-language compatibility)."
  def device_type_value(:character), do: 0
  def device_type_value(:block), do: 1
  def device_type_value(:network), do: 2

  @doc "Convert an integer device type to its atom representation."
  def device_type_from_value(0), do: :character
  def device_type_from_value(1), do: :block
  def device_type_from_value(2), do: :network

  # ==========================================================================
  # SimulatedDisk (Block Device)
  # ==========================================================================
  #
  # Wraps an in-memory binary to simulate a block storage device. This is the
  # "hard drive" for our simulated computer.
  #
  # How it works:
  #   - The disk is a flat binary, divided into fixed-size blocks.
  #   - read_block(disk, n) extracts bytes from offset n*block_size.
  #   - write_block(disk, n, data) replaces bytes at that offset.
  #
  # Default configuration:
  #   - block_size = 512 bytes (standard sector size since IBM PC/AT, 1984)
  #   - total_blocks = 2048 (giving a 1 MB disk)
  #   - interrupt_number = 34 (disk I/O complete)
  #
  # In a real system, disk reads take milliseconds. In simulation, it's just
  # binary slicing — instantaneous.

  defmodule SimulatedDisk do
    @moduledoc "In-memory block device simulating a hard disk."

    defstruct [
      :name, :device_type, :major, :minor, :interrupt_number,
      :initialized, :block_size, :total_blocks, :storage
    ]

    @doc """
    Create a new simulated disk.

    Options:
      - name: device name (default "disk0")
      - major: major device number (default 3)
      - minor: minor device number (default 0)
      - interrupt_number: interrupt for I/O complete (default 34)
      - block_size: bytes per block (default 512)
      - total_blocks: number of blocks (default 2048, giving 1 MB)
    """
    def new(opts \\ []) do
      block_size = Keyword.get(opts, :block_size, 512)
      total_blocks = Keyword.get(opts, :total_blocks, 2048)

      %__MODULE__{
        name: Keyword.get(opts, :name, "disk0"),
        device_type: :block,
        major: Keyword.get(opts, :major, 3),
        minor: Keyword.get(opts, :minor, 0),
        interrupt_number: Keyword.get(opts, :interrupt_number, 34),
        initialized: false,
        block_size: block_size,
        total_blocks: total_blocks,
        # A fresh disk is all zeros — like a formatted drive.
        storage: :binary.copy(<<0>>, block_size * total_blocks)
      }
    end

    @doc "Initialize the disk. Marks it as ready for use."
    def init(%__MODULE__{} = disk) do
      %{disk | initialized: true}
    end

    @doc """
    Read exactly `block_size` bytes from the given block number.

    Returns `{:ok, binary}` on success or `{:error, reason}` on failure.

    The offset calculation:
      offset = block_number * block_size
      data   = storage[offset .. offset + block_size - 1]
    """
    def read_block(%__MODULE__{} = disk, block_number) do
      cond do
        block_number < 0 or block_number >= disk.total_blocks ->
          {:error, "Block number #{block_number} out of range [0, #{disk.total_blocks})"}
        true ->
          offset = block_number * disk.block_size
          data = binary_part(disk.storage, offset, disk.block_size)
          {:ok, data}
      end
    end

    @doc """
    Write exactly `block_size` bytes to the given block number.

    Returns `{:ok, updated_disk}` on success or `{:error, reason}` on failure.

    The data must be exactly `block_size` bytes — you cannot write a partial
    block. The filesystem layer handles partial-block read-modify-write cycles.
    """
    def write_block(%__MODULE__{} = disk, block_number, data) when is_binary(data) do
      cond do
        block_number < 0 or block_number >= disk.total_blocks ->
          {:error, "Block number #{block_number} out of range [0, #{disk.total_blocks})"}
        byte_size(data) != disk.block_size ->
          {:error, "Data length #{byte_size(data)} does not match block size #{disk.block_size}"}
        true ->
          offset = block_number * disk.block_size
          # Build the new storage: prefix + new data + suffix
          prefix = binary_part(disk.storage, 0, offset)
          suffix_offset = offset + disk.block_size
          suffix_length = byte_size(disk.storage) - suffix_offset
          suffix = binary_part(disk.storage, suffix_offset, suffix_length)
          {:ok, %{disk | storage: prefix <> data <> suffix}}
      end
    end
  end

  # ==========================================================================
  # SimulatedKeyboard (Character Device)
  # ==========================================================================
  #
  # Wraps a FIFO buffer to simulate a keyboard. In a real system, each keypress
  # triggers interrupt 33, and the keyboard ISR reads the scancode and places
  # it in a buffer. User programs then read from this buffer.
  #
  # Key properties:
  #   - Read-only: write() returns {:error, -1}
  #   - Non-blocking: read() returns empty binary if buffer is empty
  #   - FIFO ordering: keys come out in the order they were pressed

  defmodule SimulatedKeyboard do
    @moduledoc "FIFO keyboard buffer as a character device."

    defstruct [
      :name, :device_type, :major, :minor, :interrupt_number,
      :initialized, buffer: []
    ]

    @doc """
    Create a new simulated keyboard.

    Options:
      - name: device name (default "keyboard0")
      - major: major number (default 2)
      - minor: minor number (default 0)
      - interrupt_number: keyboard interrupt (default 33)
    """
    def new(opts \\ []) do
      %__MODULE__{
        name: Keyword.get(opts, :name, "keyboard0"),
        device_type: :character,
        major: Keyword.get(opts, :major, 2),
        minor: Keyword.get(opts, :minor, 0),
        interrupt_number: Keyword.get(opts, :interrupt_number, 33),
        initialized: false,
        buffer: []
      }
    end

    @doc "Initialize the keyboard. Clears any buffered keystrokes."
    def init(%__MODULE__{} = kb) do
      %{kb | initialized: true, buffer: []}
    end

    @doc """
    Read up to `count` bytes from the keyboard buffer.

    Returns `{bytes_read_binary, updated_keyboard}`. If the buffer is empty,
    returns an empty binary. If fewer bytes are available than requested,
    returns only what's available — non-blocking behavior.
    """
    def read(%__MODULE__{buffer: buf} = kb, count) do
      {taken, remaining} = Enum.split(buf, count)
      data = :erlang.list_to_binary(taken)
      {data, %{kb | buffer: remaining}}
    end

    @doc """
    Write to the keyboard — always fails with -1.

    You cannot "write" to a keyboard. It's an input-only device.
    """
    def write(%__MODULE__{} = kb, _data) do
      {-1, kb}
    end

    @doc """
    Enqueue keystrokes into the buffer.

    In a real system, the keyboard ISR (interrupt 33 handler) would call
    this after reading the scancode from the keyboard controller.
    """
    def enqueue_keys(%__MODULE__{buffer: buf} = kb, bytes) when is_list(bytes) do
      %{kb | buffer: buf ++ bytes}
    end
  end

  # ==========================================================================
  # SimulatedDisplay (Character Device)
  # ==========================================================================
  #
  # Simulates a text-mode display as a character device. The display uses a
  # framebuffer of columns * rows * 2 bytes (character + attribute per cell).
  #
  # VGA text mode uses 2 bytes per cell:
  #   byte 0: ASCII character code
  #   byte 1: attribute (foreground color in low 4 bits, background in high 4 bits)
  #
  # Standard dimensions: 80 columns x 25 rows = 4000 bytes total.
  #
  # Key properties:
  #   - Write-only as character device: read() returns empty binary
  #   - No interrupts: displays don't generate interrupts (interrupt = -1)
  #   - Cursor tracking: knows where the next character goes

  defmodule SimulatedDisplay do
    @moduledoc "Text-mode framebuffer display as a character device."

    defstruct [
      :name, :device_type, :major, :minor, :interrupt_number,
      :initialized, :columns, :rows, :framebuffer,
      :default_attribute, cursor_row: 0, cursor_col: 0
    ]

    @doc """
    Create a new simulated display.

    Options:
      - name: device name (default "display0")
      - major: major number (default 1)
      - minor: minor number (default 0)
      - columns: display width (default 80)
      - rows: display height (default 25)
      - default_attribute: color attribute byte (default 0x07 = light gray on black)
    """
    def new(opts \\ []) do
      columns = Keyword.get(opts, :columns, 80)
      rows = Keyword.get(opts, :rows, 25)
      default_attr = Keyword.get(opts, :default_attribute, 0x07)

      %__MODULE__{
        name: Keyword.get(opts, :name, "display0"),
        device_type: :character,
        major: Keyword.get(opts, :major, 1),
        minor: Keyword.get(opts, :minor, 0),
        interrupt_number: -1,
        initialized: false,
        columns: columns,
        rows: rows,
        default_attribute: default_attr,
        # Each cell is 2 bytes: [char, attr]. Initialize with spaces.
        framebuffer: build_clear_framebuffer(columns, rows, default_attr),
        cursor_row: 0,
        cursor_col: 0
      }
    end

    @doc "Initialize the display by clearing the screen and resetting the cursor."
    def init(%__MODULE__{} = display) do
      display
      |> clear_screen()
      |> Map.put(:initialized, true)
    end

    @doc """
    Read from the display — returns empty binary.

    Displays are write-only as character devices. You cannot "read" the screen
    through this interface.
    """
    def read(%__MODULE__{} = display, _count) do
      {<<>>, display}
    end

    @doc """
    Write characters to the display at the current cursor position.

    Each byte is an ASCII character code. Returns `{bytes_written, updated_display}`.
    """
    def write(%__MODULE__{} = display, data) when is_binary(data) do
      bytes = :binary.bin_to_list(data)
      updated = Enum.reduce(bytes, display, &put_char(&2, &1))
      {byte_size(data), updated}
    end

    @doc """
    Write a single character at the current cursor position.

    Framebuffer layout:
      offset = (row * columns + col) * 2
      framebuffer[offset]     = character code
      framebuffer[offset + 1] = attribute byte
    """
    def put_char(%__MODULE__{cursor_row: row, rows: rows} = display, _char_code)
        when row >= rows do
      # Screen is full — no scrolling in this simple simulation
      display
    end

    def put_char(%__MODULE__{} = display, char_code) do
      offset = (display.cursor_row * display.columns + display.cursor_col) * 2

      # Replace the 2 bytes at the cursor position
      prefix = binary_part(display.framebuffer, 0, offset)
      suffix_start = offset + 2
      suffix_len = byte_size(display.framebuffer) - suffix_start
      suffix = binary_part(display.framebuffer, suffix_start, suffix_len)

      new_fb = prefix <> <<char_code, display.default_attribute>> <> suffix

      # Advance the cursor
      new_col = display.cursor_col + 1
      {final_row, final_col} =
        if new_col >= display.columns do
          {display.cursor_row + 1, 0}
        else
          {display.cursor_row, new_col}
        end

      %{display | framebuffer: new_fb, cursor_row: final_row, cursor_col: final_col}
    end

    @doc "Clear the screen: fill with spaces + default attribute, reset cursor to (0,0)."
    def clear_screen(%__MODULE__{} = display) do
      %{display |
        framebuffer: build_clear_framebuffer(display.columns, display.rows, display.default_attribute),
        cursor_row: 0,
        cursor_col: 0
      }
    end

    @doc "Read the character byte at a given (row, col) position."
    def get_char_at(%__MODULE__{} = display, row, col) do
      offset = (row * display.columns + col) * 2
      :binary.at(display.framebuffer, offset)
    end

    # Build a framebuffer filled with spaces (0x20) and the given attribute.
    defp build_clear_framebuffer(columns, rows, attribute) do
      cell = <<0x20, attribute>>
      :binary.copy(cell, columns * rows)
    end
  end

  # ==========================================================================
  # SharedWire
  # ==========================================================================
  #
  # A simulated network cable connecting multiple NICs. When one NIC sends a
  # packet, every other NIC on the wire receives a copy. The sender does NOT
  # receive its own packet (like real Ethernet).
  #
  # In Elixir's functional style, the wire doesn't hold mutable state.
  # Instead, the broadcast function takes the list of connected NICs and
  # returns updated NICs with packets enqueued.
  #
  # We use an Agent to hold the wire state (list of NIC names) so that
  # NICs can discover each other. But for simplicity in our educational
  # implementation, we pass the wire as a struct containing NIC references.

  defmodule SharedWire do
    @moduledoc """
    A simulated network cable connecting multiple NICs.

    When one NIC sends, all other connected NICs receive the packet.
    """

    defstruct connected_nics: []

    @doc "Create a new empty wire."
    def new, do: %__MODULE__{}

    @doc "Connect a NIC (by name) to this wire."
    def connect(%__MODULE__{connected_nics: nics} = wire, nic_name) do
      %{wire | connected_nics: nics ++ [nic_name]}
    end
  end

  # ==========================================================================
  # SimulatedNIC (Network Device)
  # ==========================================================================
  #
  # A network interface card backed by in-memory packet queues. Two NICs
  # connected to the same SharedWire can exchange packets.
  #
  # MAC addresses:
  #   Every NIC has a 6-byte MAC (Media Access Control) address, like a
  #   mailing address burned into the card at the factory. Example:
  #   <<0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01>>
  #
  # In Elixir's functional style, sending a packet returns updated NICs
  # (the receivers get packets added to their rx_queues). This differs from
  # the OOP implementations where the wire mutates the NIC objects directly.

  defmodule SimulatedNIC do
    @moduledoc "Network interface card with packet queues and MAC address."

    defstruct [
      :name, :device_type, :major, :minor, :interrupt_number,
      :initialized, :mac_address, rx_queue: []
    ]

    @doc """
    Create a new simulated NIC.

    Options:
      - name: device name (default "nic0")
      - major: major number (default 4)
      - minor: minor number (default 0)
      - interrupt_number: packet received interrupt (default 35)
      - mac_address: 6-byte binary (default <<0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01>>)
    """
    def new(opts \\ []) do
      %__MODULE__{
        name: Keyword.get(opts, :name, "nic0"),
        device_type: :network,
        major: Keyword.get(opts, :major, 4),
        minor: Keyword.get(opts, :minor, 0),
        interrupt_number: Keyword.get(opts, :interrupt_number, 35),
        initialized: false,
        mac_address: Keyword.get(opts, :mac_address, <<0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01>>),
        rx_queue: []
      }
    end

    @doc "Initialize the NIC. Clears the receive queue."
    def init(%__MODULE__{} = nic) do
      %{nic | initialized: true, rx_queue: []}
    end

    @doc """
    Send a packet from this NIC to all other NICs on the wire.

    Returns `{bytes_sent, updated_sender, updated_other_nics}`.
    The other_nics map is keyed by NIC name with packets enqueued.

    In functional style, we return the updated receiver NICs rather than
    mutating them in place.
    """
    def send_packet(%__MODULE__{} = _sender, <<>>) do
      {:error, -1}
    end

    def send_packet(%__MODULE__{} = sender, data, other_nics) when is_binary(data) do
      # Enqueue the packet in every other NIC's rx_queue
      updated_others =
        Enum.map(other_nics, fn nic ->
          if nic.name != sender.name do
            enqueue_packet(nic, data)
          else
            nic
          end
        end)

      {byte_size(data), sender, updated_others}
    end

    @doc """
    Receive the next packet from the receive queue.

    Returns `{packet_or_nil, updated_nic}`. Non-blocking: returns nil
    if no packets are waiting.
    """
    def receive_packet(%__MODULE__{rx_queue: []} = nic) do
      {nil, nic}
    end

    def receive_packet(%__MODULE__{rx_queue: [packet | remaining]} = nic) do
      {packet, %{nic | rx_queue: remaining}}
    end

    @doc "Check whether there are packets waiting in the receive queue."
    def has_packet?(%__MODULE__{rx_queue: q}), do: length(q) > 0

    @doc "Add a packet to the receive queue (called during broadcast)."
    def enqueue_packet(%__MODULE__{rx_queue: q} = nic, data) when is_binary(data) do
      %{nic | rx_queue: q ++ [data]}
    end
  end

  # ==========================================================================
  # DeviceRegistry
  # ==========================================================================
  #
  # The registry is the kernel's phonebook for devices. When a driver
  # initializes a device, it registers it here. When the kernel needs to
  # perform I/O, it looks up the device here.
  #
  # Two lookup strategies:
  #   1. By name: "disk0" → SimulatedDisk struct
  #   2. By major/minor: {3, 0} → SimulatedDisk struct
  #
  # In Elixir, the registry is an immutable struct. Each operation returns
  # a new registry. This is different from OOP languages where the registry
  # is mutable.

  defmodule DeviceRegistry do
    @moduledoc """
    Registry for looking up devices by name or major/minor number.

    An immutable struct — each operation returns a new registry.
    """

    defstruct devices_by_name: %{}, devices_by_major_minor: %{}, all_devices: []

    @doc "Create a new empty registry."
    def new, do: %__MODULE__{}

    @doc """
    Register a device in the registry.

    Returns `{:ok, updated_registry}` on success, or `{:error, reason}` if:
      - The device is not initialized
      - A device with the same name already exists
      - A device with the same (major, minor) pair already exists
    """
    def register(%__MODULE__{} = reg, device) do
      key = {device.major, device.minor}

      cond do
        !device.initialized ->
          {:error, "Device \"#{device.name}\" must be initialized before registration"}
        Map.has_key?(reg.devices_by_name, device.name) ->
          {:error, "Device with name \"#{device.name}\" is already registered"}
        Map.has_key?(reg.devices_by_major_minor, key) ->
          {:error, "Device with major=#{device.major}, minor=#{device.minor} is already registered"}
        true ->
          {:ok, %{reg |
            devices_by_name: Map.put(reg.devices_by_name, device.name, device),
            devices_by_major_minor: Map.put(reg.devices_by_major_minor, key, device),
            all_devices: reg.all_devices ++ [device]
          }}
      end
    end

    @doc """
    Remove a device from the registry by name.

    Returns `{:ok, updated_registry}` if found, or `{:error, :not_found}`.
    """
    def unregister(%__MODULE__{} = reg, device_name) do
      case Map.get(reg.devices_by_name, device_name) do
        nil ->
          {:error, :not_found}
        device ->
          key = {device.major, device.minor}
          {:ok, %{reg |
            devices_by_name: Map.delete(reg.devices_by_name, device_name),
            devices_by_major_minor: Map.delete(reg.devices_by_major_minor, key),
            all_devices: Enum.reject(reg.all_devices, &(&1.name == device_name))
          }}
      end
    end

    @doc "Look up a device by name. Returns the device or nil."
    def lookup_by_name(%__MODULE__{} = reg, device_name) do
      Map.get(reg.devices_by_name, device_name)
    end

    @doc "Look up a device by major/minor pair. Returns the device or nil."
    def lookup_by_major_minor(%__MODULE__{} = reg, major, minor) do
      Map.get(reg.devices_by_major_minor, {major, minor})
    end

    @doc "Return all registered devices."
    def list_all(%__MODULE__{} = reg), do: reg.all_devices

    @doc "Return all devices of a specific type (:character, :block, :network)."
    def list_by_type(%__MODULE__{} = reg, device_type) do
      Enum.filter(reg.all_devices, &(&1.device_type == device_type))
    end
  end
end
