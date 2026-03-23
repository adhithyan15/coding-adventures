# Tests for the Device Driver Framework
#
# These tests verify all three device families (Character, Block, Network),
# the DeviceRegistry, and the concrete simulated implementations. They follow
# the testing strategy outlined in the D12 specification.

defmodule CodingAdventures.DeviceDriverFrameworkTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.DeviceDriverFramework
  alias CodingAdventures.DeviceDriverFramework.{
    SimulatedDisk,
    SimulatedKeyboard,
    SimulatedDisplay,
    SimulatedNIC,
    SharedWire,
    DeviceRegistry
  }

  # ==========================================================================
  # DeviceType Tests
  # ==========================================================================

  describe "device_type" do
    test "has three distinct integer values" do
      assert DeviceDriverFramework.device_type_value(:character) == 0
      assert DeviceDriverFramework.device_type_value(:block) == 1
      assert DeviceDriverFramework.device_type_value(:network) == 2
    end

    test "round-trips from integer to atom" do
      assert DeviceDriverFramework.device_type_from_value(0) == :character
      assert DeviceDriverFramework.device_type_from_value(1) == :block
      assert DeviceDriverFramework.device_type_from_value(2) == :network
    end
  end

  # ==========================================================================
  # SimulatedDisk Tests
  # ==========================================================================

  describe "SimulatedDisk" do
    setup do
      # Create a small disk for testing: 8 blocks of 512 bytes = 4 KB
      disk = SimulatedDisk.new(total_blocks: 8) |> SimulatedDisk.init()
      %{disk: disk}
    end

    test "has correct default properties", %{disk: disk} do
      assert disk.name == "disk0"
      assert disk.device_type == :block
      assert disk.major == 3
      assert disk.minor == 0
      assert disk.interrupt_number == 34
      assert disk.block_size == 512
      assert disk.total_blocks == 8
      assert disk.initialized == true
    end

    test "allows custom properties" do
      custom = SimulatedDisk.new(
        name: "disk1", major: 3, minor: 1,
        block_size: 1024, total_blocks: 16
      ) |> SimulatedDisk.init()

      assert custom.name == "disk1"
      assert custom.minor == 1
      assert custom.block_size == 1024
      assert custom.total_blocks == 16
    end

    test "reads all zeros from a fresh disk", %{disk: disk} do
      {:ok, data} = SimulatedDisk.read_block(disk, 0)
      assert byte_size(data) == 512
      assert data == :binary.copy(<<0>>, 512)
    end

    test "round-trips data through write and read", %{disk: disk} do
      # Build a repeating pattern
      pattern = :binary.copy(<<0xAB>>, 512)
      {:ok, disk} = SimulatedDisk.write_block(disk, 5, pattern)
      {:ok, read_back} = SimulatedDisk.read_block(disk, 5)
      assert read_back == pattern
    end

    test "does not affect adjacent blocks when writing", %{disk: disk} do
      data = :binary.copy(<<0xAA>>, 512)
      {:ok, disk} = SimulatedDisk.write_block(disk, 3, data)

      {:ok, block2} = SimulatedDisk.read_block(disk, 2)
      {:ok, block4} = SimulatedDisk.read_block(disk, 4)
      assert block2 == :binary.copy(<<0>>, 512)
      assert block4 == :binary.copy(<<0>>, 512)
    end

    test "returns error on read of out-of-bounds block", %{disk: disk} do
      assert {:error, _} = SimulatedDisk.read_block(disk, 8)
      assert {:error, _} = SimulatedDisk.read_block(disk, -1)
      assert {:error, _} = SimulatedDisk.read_block(disk, 100)
    end

    test "returns error on write of out-of-bounds block", %{disk: disk} do
      data = :binary.copy(<<0>>, 512)
      assert {:error, _} = SimulatedDisk.write_block(disk, 8, data)
      assert {:error, _} = SimulatedDisk.write_block(disk, -1, data)
    end

    test "returns error when write data does not match block size", %{disk: disk} do
      short_data = :binary.copy(<<0>>, 256)
      assert {:error, msg} = SimulatedDisk.write_block(disk, 0, short_data)
      assert msg =~ "does not match block size"

      long_data = :binary.copy(<<0>>, 1024)
      assert {:error, _} = SimulatedDisk.write_block(disk, 0, long_data)
    end

    test "can read and write the last block", %{disk: disk} do
      data = :binary.copy(<<0xBB>>, 512)
      {:ok, disk} = SimulatedDisk.write_block(disk, 7, data)
      {:ok, read_back} = SimulatedDisk.read_block(disk, 7)
      assert read_back == data
    end
  end

  # ==========================================================================
  # SimulatedKeyboard Tests
  # ==========================================================================

  describe "SimulatedKeyboard" do
    setup do
      kb = SimulatedKeyboard.new() |> SimulatedKeyboard.init()
      %{kb: kb}
    end

    test "has correct default properties", %{kb: kb} do
      assert kb.name == "keyboard0"
      assert kb.device_type == :character
      assert kb.major == 2
      assert kb.minor == 0
      assert kb.interrupt_number == 33
      assert kb.initialized == true
    end

    test "returns empty binary when buffer is empty", %{kb: kb} do
      {data, _kb} = SimulatedKeyboard.read(kb, 10)
      assert data == <<>>
    end

    test "returns enqueued bytes in FIFO order", %{kb: kb} do
      kb = SimulatedKeyboard.enqueue_keys(kb, [72, 105, 33])
      {data, _kb} = SimulatedKeyboard.read(kb, 3)
      assert data == <<72, 105, 33>>
    end

    test "returns only available bytes when count exceeds buffer", %{kb: kb} do
      kb = SimulatedKeyboard.enqueue_keys(kb, [65, 66])
      {data, _kb} = SimulatedKeyboard.read(kb, 10)
      assert data == <<65, 66>>
    end

    test "drains the buffer progressively", %{kb: kb} do
      kb = SimulatedKeyboard.enqueue_keys(kb, [1, 2, 3, 4, 5])

      {first, kb} = SimulatedKeyboard.read(kb, 2)
      assert first == <<1, 2>>

      {second, kb} = SimulatedKeyboard.read(kb, 2)
      assert second == <<3, 4>>

      {third, kb} = SimulatedKeyboard.read(kb, 10)
      assert third == <<5>>

      {empty_result, _kb} = SimulatedKeyboard.read(kb, 1)
      assert empty_result == <<>>
    end

    test "write returns -1 (keyboards are read-only)", %{kb: kb} do
      {result, _kb} = SimulatedKeyboard.write(kb, <<72, 105>>)
      assert result == -1
    end

    test "init clears the buffer", %{kb: kb} do
      kb = SimulatedKeyboard.enqueue_keys(kb, [1, 2, 3])
      kb = SimulatedKeyboard.init(kb)
      {data, _kb} = SimulatedKeyboard.read(kb, 10)
      assert data == <<>>
    end
  end

  # ==========================================================================
  # SimulatedDisplay Tests
  # ==========================================================================

  describe "SimulatedDisplay" do
    setup do
      display = SimulatedDisplay.new(columns: 80, rows: 25) |> SimulatedDisplay.init()
      %{display: display}
    end

    test "has correct default properties", %{display: display} do
      assert display.name == "display0"
      assert display.device_type == :character
      assert display.major == 1
      assert display.minor == 0
      assert display.interrupt_number == -1
      assert display.initialized == true
      assert display.columns == 80
      assert display.rows == 25
    end

    test "framebuffer size is columns * rows * 2", %{display: display} do
      assert byte_size(display.framebuffer) == 80 * 25 * 2
    end

    test "init clears screen to spaces with default attribute", %{display: display} do
      # Check first cell
      assert :binary.at(display.framebuffer, 0) == 0x20  # space
      assert :binary.at(display.framebuffer, 1) == 0x07  # default attribute

      # Check last cell
      last_offset = (80 * 25 - 1) * 2
      assert :binary.at(display.framebuffer, last_offset) == 0x20
      assert :binary.at(display.framebuffer, last_offset + 1) == 0x07
    end

    test "init resets cursor to (0, 0)", %{display: display} do
      display = %{display | cursor_row: 5, cursor_col: 10}
      display = SimulatedDisplay.init(display)
      assert display.cursor_row == 0
      assert display.cursor_col == 0
    end

    test "writes characters to framebuffer", %{display: display} do
      {written, display} = SimulatedDisplay.write(display, <<0x48, 0x69>>)
      assert written == 2
      assert SimulatedDisplay.get_char_at(display, 0, 0) == 0x48
      assert SimulatedDisplay.get_char_at(display, 0, 1) == 0x69
    end

    test "advances cursor after writing", %{display: display} do
      {_written, display} = SimulatedDisplay.write(display, <<0x48, 0x69>>)
      assert display.cursor_row == 0
      assert display.cursor_col == 2
    end

    test "wraps to next line when reaching end of column", %{display: display} do
      line = :binary.copy(<<0x41>>, 80)
      {_written, display} = SimulatedDisplay.write(display, line)
      assert display.cursor_row == 1
      assert display.cursor_col == 0
    end

    test "read returns empty binary (displays are write-only)", %{display: display} do
      {data, _display} = SimulatedDisplay.read(display, 10)
      assert data == <<>>
    end

    test "clear_screen resets all cells and cursor", %{display: display} do
      {_written, display} = SimulatedDisplay.write(display, <<0x48, 0x65, 0x6C, 0x6C, 0x6F>>)
      display = SimulatedDisplay.clear_screen(display)
      assert display.cursor_row == 0
      assert display.cursor_col == 0
      assert SimulatedDisplay.get_char_at(display, 0, 0) == 0x20
    end

    test "writes with correct attribute byte", %{display: display} do
      {_written, display} = SimulatedDisplay.write(display, <<0x41>>)
      assert :binary.at(display.framebuffer, 0) == 0x41
      assert :binary.at(display.framebuffer, 1) == 0x07
    end

    test "handles custom display configuration" do
      small = SimulatedDisplay.new(columns: 40, rows: 12) |> SimulatedDisplay.init()
      assert small.columns == 40
      assert small.rows == 12
      assert byte_size(small.framebuffer) == 40 * 12 * 2
    end

    test "stops writing when screen is full" do
      small = SimulatedDisplay.new(columns: 4, rows: 2) |> SimulatedDisplay.init()
      data = :binary.copy(<<0x41>>, 10)
      {written, small} = SimulatedDisplay.write(small, data)
      assert written == 10
      # Cursor should be past the last row
      assert small.cursor_row == 2
    end
  end

  # ==========================================================================
  # SharedWire & SimulatedNIC Tests
  # ==========================================================================

  describe "SharedWire and SimulatedNIC" do
    setup do
      nic_a = SimulatedNIC.new(
        name: "nic0",
        mac_address: <<0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01>>
      ) |> SimulatedNIC.init()

      nic_b = SimulatedNIC.new(
        name: "nic1", minor: 1,
        mac_address: <<0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x02>>
      ) |> SimulatedNIC.init()

      %{nic_a: nic_a, nic_b: nic_b}
    end

    test "has correct default properties", %{nic_a: nic_a} do
      assert nic_a.name == "nic0"
      assert nic_a.device_type == :network
      assert nic_a.major == 4
      assert nic_a.minor == 0
      assert nic_a.interrupt_number == 35
      assert nic_a.initialized == true
    end

    test "mac address is exactly 6 bytes", %{nic_a: nic_a, nic_b: nic_b} do
      assert byte_size(nic_a.mac_address) == 6
      assert byte_size(nic_b.mac_address) == 6
    end

    test "receive_packet returns nil when queue is empty", %{nic_a: nic_a} do
      {packet, _nic} = SimulatedNIC.receive_packet(nic_a)
      assert packet == nil
    end

    test "has_packet? returns false when queue is empty", %{nic_a: nic_a} do
      assert SimulatedNIC.has_packet?(nic_a) == false
    end

    test "sends packet from NIC A that appears in NIC B", %{nic_a: nic_a, nic_b: nic_b} do
      packet = <<0x01, 0x02, 0x03, 0x04>>
      {sent, _nic_a, [updated_b]} = SimulatedNIC.send_packet(nic_a, packet, [nic_b])

      assert sent == 4
      assert SimulatedNIC.has_packet?(updated_b) == true

      {received, _nic_b} = SimulatedNIC.receive_packet(updated_b)
      assert received == packet
    end

    test "sender does NOT receive its own packet", %{nic_a: nic_a, nic_b: nic_b} do
      packet = <<0x01, 0x02, 0x03>>
      {_sent, nic_a, _others} = SimulatedNIC.send_packet(nic_a, packet, [nic_a, nic_b])

      # nic_a should not have its own packet (the returned nic_a is unchanged)
      assert SimulatedNIC.has_packet?(nic_a) == false
    end

    test "broadcasts to all NICs except sender", %{nic_a: nic_a, nic_b: nic_b} do
      nic_c = SimulatedNIC.new(
        name: "nic2", minor: 2,
        mac_address: <<0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x03>>
      ) |> SimulatedNIC.init()

      packet = <<0xAA, 0xBB>>
      {_sent, _nic_a, updated} = SimulatedNIC.send_packet(nic_a, packet, [nic_b, nic_c])

      [updated_b, updated_c] = updated
      {recv_b, _} = SimulatedNIC.receive_packet(updated_b)
      {recv_c, _} = SimulatedNIC.receive_packet(updated_c)
      assert recv_b == packet
      assert recv_c == packet
    end

    test "preserves packet ordering (FIFO)", %{nic_a: nic_a, nic_b: nic_b} do
      {_s1, nic_a, [nic_b]} = SimulatedNIC.send_packet(nic_a, <<0x01>>, [nic_b])
      {_s2, nic_a, [nic_b]} = SimulatedNIC.send_packet(nic_a, <<0x02>>, [nic_b])
      {_s3, _nic_a, [nic_b]} = SimulatedNIC.send_packet(nic_a, <<0x03>>, [nic_b])

      {p1, nic_b} = SimulatedNIC.receive_packet(nic_b)
      {p2, nic_b} = SimulatedNIC.receive_packet(nic_b)
      {p3, nic_b} = SimulatedNIC.receive_packet(nic_b)
      {p4, _nic_b} = SimulatedNIC.receive_packet(nic_b)

      assert p1 == <<0x01>>
      assert p2 == <<0x02>>
      assert p3 == <<0x03>>
      assert p4 == nil
    end

    test "send_packet returns error for empty packet", %{nic_a: nic_a} do
      assert {:error, -1} = SimulatedNIC.send_packet(nic_a, <<>>)
    end

    test "bidirectional communication works", %{nic_a: nic_a, nic_b: nic_b} do
      {_sent, _nic_a, [updated_b]} = SimulatedNIC.send_packet(nic_a, <<0x01>>, [nic_b])
      {_sent, _nic_b, [updated_a]} = SimulatedNIC.send_packet(updated_b, <<0x02>>, [nic_a])

      {recv_b, _} = SimulatedNIC.receive_packet(updated_b)
      {recv_a, _} = SimulatedNIC.receive_packet(updated_a)
      assert recv_b == <<0x01>>
      assert recv_a == <<0x02>>
    end

    test "init clears the receive queue", %{nic_a: nic_a, nic_b: nic_b} do
      {_sent, _nic_a, [nic_b]} = SimulatedNIC.send_packet(nic_a, <<0x01>>, [nic_b])
      assert SimulatedNIC.has_packet?(nic_b) == true

      nic_b = SimulatedNIC.init(nic_b)
      assert SimulatedNIC.has_packet?(nic_b) == false
    end

    test "uses default mac address when none provided" do
      nic = SimulatedNIC.new() |> SimulatedNIC.init()
      assert nic.mac_address == <<0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01>>
    end
  end

  # ==========================================================================
  # DeviceRegistry Tests
  # ==========================================================================

  describe "DeviceRegistry" do
    setup do
      disk = SimulatedDisk.new(total_blocks: 8) |> SimulatedDisk.init()
      kb = SimulatedKeyboard.new() |> SimulatedKeyboard.init()
      display = SimulatedDisplay.new() |> SimulatedDisplay.init()
      reg = DeviceRegistry.new()
      %{disk: disk, kb: kb, display: display, reg: reg}
    end

    test "registers and looks up by name", %{disk: disk, reg: reg} do
      {:ok, reg} = DeviceRegistry.register(reg, disk)
      found = DeviceRegistry.lookup_by_name(reg, "disk0")
      assert found.name == "disk0"
    end

    test "registers and looks up by major/minor", %{disk: disk, reg: reg} do
      {:ok, reg} = DeviceRegistry.register(reg, disk)
      found = DeviceRegistry.lookup_by_major_minor(reg, 3, 0)
      assert found.name == "disk0"
    end

    test "returns nil for unknown name", %{reg: reg} do
      assert DeviceRegistry.lookup_by_name(reg, "nonexistent") == nil
    end

    test "returns nil for unknown major/minor", %{reg: reg} do
      assert DeviceRegistry.lookup_by_major_minor(reg, 99, 99) == nil
    end

    test "returns error on duplicate name", %{disk: disk, reg: reg} do
      {:ok, reg} = DeviceRegistry.register(reg, disk)
      disk2 = SimulatedDisk.new(name: "disk0", minor: 1) |> SimulatedDisk.init()
      assert {:error, msg} = DeviceRegistry.register(reg, disk2)
      assert msg =~ "already registered"
    end

    test "returns error on duplicate major/minor", %{disk: disk, reg: reg} do
      {:ok, reg} = DeviceRegistry.register(reg, disk)
      disk2 = SimulatedDisk.new(name: "disk1", major: 3, minor: 0) |> SimulatedDisk.init()
      assert {:error, msg} = DeviceRegistry.register(reg, disk2)
      assert msg =~ "already registered"
    end

    test "returns error when registering uninitialized device", %{reg: reg} do
      uninit = SimulatedDisk.new()
      assert {:error, msg} = DeviceRegistry.register(reg, uninit)
      assert msg =~ "must be initialized"
    end

    test "lists all registered devices", %{disk: disk, kb: kb, display: display, reg: reg} do
      {:ok, reg} = DeviceRegistry.register(reg, disk)
      {:ok, reg} = DeviceRegistry.register(reg, kb)
      {:ok, reg} = DeviceRegistry.register(reg, display)

      all = DeviceRegistry.list_all(reg)
      assert length(all) == 3
      names = Enum.map(all, & &1.name)
      assert "disk0" in names
      assert "keyboard0" in names
      assert "display0" in names
    end

    test "lists devices by type", %{disk: disk, kb: kb, display: display, reg: reg} do
      {:ok, reg} = DeviceRegistry.register(reg, disk)
      {:ok, reg} = DeviceRegistry.register(reg, kb)
      {:ok, reg} = DeviceRegistry.register(reg, display)

      blocks = DeviceRegistry.list_by_type(reg, :block)
      assert length(blocks) == 1
      assert hd(blocks).name == "disk0"

      chars = DeviceRegistry.list_by_type(reg, :character)
      assert length(chars) == 2

      nets = DeviceRegistry.list_by_type(reg, :network)
      assert length(nets) == 0
    end

    test "unregisters a device by name", %{disk: disk, reg: reg} do
      {:ok, reg} = DeviceRegistry.register(reg, disk)
      {:ok, reg} = DeviceRegistry.unregister(reg, "disk0")
      assert DeviceRegistry.lookup_by_name(reg, "disk0") == nil
      assert DeviceRegistry.lookup_by_major_minor(reg, 3, 0) == nil
      assert DeviceRegistry.list_all(reg) == []
    end

    test "unregister returns error for unknown name", %{reg: reg} do
      assert {:error, :not_found} = DeviceRegistry.unregister(reg, "nonexistent")
    end

    test "handles multiple devices of different types", %{disk: disk, kb: kb, display: display, reg: reg} do
      nic = SimulatedNIC.new() |> SimulatedNIC.init()
      {:ok, reg} = DeviceRegistry.register(reg, disk)
      {:ok, reg} = DeviceRegistry.register(reg, kb)
      {:ok, reg} = DeviceRegistry.register(reg, display)
      {:ok, reg} = DeviceRegistry.register(reg, nic)

      assert length(DeviceRegistry.list_all(reg)) == 4
      assert length(DeviceRegistry.list_by_type(reg, :character)) == 2
      assert length(DeviceRegistry.list_by_type(reg, :block)) == 1
      assert length(DeviceRegistry.list_by_type(reg, :network)) == 1
    end
  end

  # ==========================================================================
  # Integration Tests
  # ==========================================================================

  describe "integration" do
    test "writes through registry to display and verifies framebuffer" do
      reg = DeviceRegistry.new()
      display = SimulatedDisplay.new() |> SimulatedDisplay.init()
      {:ok, reg} = DeviceRegistry.register(reg, display)

      device = DeviceRegistry.lookup_by_name(reg, "display0")
      assert device.device_type == :character

      {_written, updated_display} = SimulatedDisplay.write(device, <<0x48, 0x69>>)
      assert SimulatedDisplay.get_char_at(updated_display, 0, 0) == 0x48
      assert SimulatedDisplay.get_char_at(updated_display, 0, 1) == 0x69
    end

    test "keyboard ISR -> buffer -> read through registry" do
      reg = DeviceRegistry.new()
      kb = SimulatedKeyboard.new() |> SimulatedKeyboard.init()
      {:ok, _reg} = DeviceRegistry.register(reg, kb)

      # ISR deposits a keystroke
      kb = SimulatedKeyboard.enqueue_keys(kb, [0x41])

      # Read through the keyboard interface
      {data, _kb} = SimulatedKeyboard.read(kb, 1)
      assert data == <<0x41>>
    end

    test "network roundtrip: NIC A sends, NIC B receives" do
      reg = DeviceRegistry.new()
      nic_a = SimulatedNIC.new(name: "nic0") |> SimulatedNIC.init()
      nic_b = SimulatedNIC.new(
        name: "nic1", minor: 1,
        mac_address: <<0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x02>>
      ) |> SimulatedNIC.init()

      {:ok, reg} = DeviceRegistry.register(reg, nic_a)
      {:ok, _reg} = DeviceRegistry.register(reg, nic_b)

      # Send from A, receive on B
      {sent, _nic_a, [updated_b]} = SimulatedNIC.send_packet(nic_a, <<0xCA, 0xFE>>, [nic_b])
      assert sent == 2

      {packet, _nic_b} = SimulatedNIC.receive_packet(updated_b)
      assert packet == <<0xCA, 0xFE>>
    end

    test "disk block I/O through registry" do
      reg = DeviceRegistry.new()
      disk = SimulatedDisk.new(total_blocks: 8) |> SimulatedDisk.init()
      {:ok, reg} = DeviceRegistry.register(reg, disk)

      device = DeviceRegistry.lookup_by_major_minor(reg, 3, 0)
      assert device.device_type == :block

      # Write and read a block
      data = <<0xFE>> <> :binary.copy(<<0>>, 510) <> <<0xED>>
      {:ok, device} = SimulatedDisk.write_block(device, 0, data)
      {:ok, read_back} = SimulatedDisk.read_block(device, 0)
      assert :binary.at(read_back, 0) == 0xFE
      assert :binary.at(read_back, 511) == 0xED
    end
  end

  # ==========================================================================
  # SharedWire Tests
  # ==========================================================================

  describe "SharedWire" do
    test "creates an empty wire" do
      wire = SharedWire.new()
      assert wire.connected_nics == []
    end

    test "connects NICs to the wire" do
      wire = SharedWire.new()
      wire = SharedWire.connect(wire, "nic0")
      wire = SharedWire.connect(wire, "nic1")
      assert wire.connected_nics == ["nic0", "nic1"]
    end
  end
end
