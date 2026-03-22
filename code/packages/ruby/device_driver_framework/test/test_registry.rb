# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module DeviceDriverFramework
    class TestDeviceRegistry < Minitest::Test
      def setup
        @registry = DeviceRegistry.new
      end

      # --- Registration ---

      def test_register_and_lookup_by_name
        disk = SimulatedDisk.new
        disk.init
        @registry.register(disk)

        found = @registry.lookup_by_name("disk0")
        assert_equal disk, found
      end

      def test_register_and_lookup_by_major_minor
        disk = SimulatedDisk.new
        disk.init
        @registry.register(disk)

        found = @registry.lookup_by_major_minor(3, 0)
        assert_equal disk, found
      end

      def test_register_requires_initialized_device
        disk = SimulatedDisk.new
        # Not initialized!
        assert_raises(ArgumentError) { @registry.register(disk) }
      end

      def test_duplicate_name_raises_error
        disk1 = SimulatedDisk.new(name: "disk0")
        disk1.init
        @registry.register(disk1)

        disk2 = SimulatedDisk.new(name: "disk0", minor: 1)
        disk2.init
        assert_raises(ArgumentError) { @registry.register(disk2) }
      end

      def test_duplicate_major_minor_raises_error
        disk1 = SimulatedDisk.new(name: "disk0", minor: 0)
        disk1.init
        @registry.register(disk1)

        disk2 = SimulatedDisk.new(name: "disk1", minor: 0)
        disk2.init
        assert_raises(ArgumentError) { @registry.register(disk2) }
      end

      # --- Lookup ---

      def test_lookup_by_name_returns_nil_for_missing
        assert_nil @registry.lookup_by_name("nonexistent")
      end

      def test_lookup_by_major_minor_returns_nil_for_missing
        assert_nil @registry.lookup_by_major_minor(99, 99)
      end

      # --- Listing ---

      def test_list_all_returns_all_devices
        disk = SimulatedDisk.new
        disk.init
        kb = SimulatedKeyboard.new
        kb.init

        @registry.register(disk)
        @registry.register(kb)

        all = @registry.list_all
        assert_equal 2, all.length
        assert_includes all, disk
        assert_includes all, kb
      end

      def test_list_all_returns_copy
        disk = SimulatedDisk.new
        disk.init
        @registry.register(disk)

        list1 = @registry.list_all
        list1.clear
        assert_equal 1, @registry.list_all.length
      end

      def test_list_by_type_filters_correctly
        disk = SimulatedDisk.new
        disk.init
        kb = SimulatedKeyboard.new
        kb.init
        display = SimulatedDisplay.new
        display.init

        @registry.register(disk)
        @registry.register(kb)
        @registry.register(display)

        block_devices = @registry.list_by_type(DeviceType::BLOCK)
        assert_equal 1, block_devices.length
        assert_equal disk, block_devices.first

        char_devices = @registry.list_by_type(DeviceType::CHARACTER)
        assert_equal 2, char_devices.length
      end

      def test_list_by_type_returns_empty_for_no_matches
        disk = SimulatedDisk.new
        disk.init
        @registry.register(disk)

        network_devices = @registry.list_by_type(DeviceType::NETWORK)
        assert_empty network_devices
      end

      # --- Unregister ---

      def test_unregister_removes_device
        disk = SimulatedDisk.new
        disk.init
        @registry.register(disk)

        removed = @registry.unregister("disk0")
        assert_equal disk, removed
        assert_nil @registry.lookup_by_name("disk0")
        assert_nil @registry.lookup_by_major_minor(3, 0)
        assert_empty @registry.list_all
      end

      def test_unregister_returns_nil_for_missing
        assert_nil @registry.unregister("nonexistent")
      end

      # --- Size ---

      def test_size_tracks_registrations
        assert_equal 0, @registry.size

        disk = SimulatedDisk.new
        disk.init
        @registry.register(disk)
        assert_equal 1, @registry.size

        kb = SimulatedKeyboard.new
        kb.init
        @registry.register(kb)
        assert_equal 2, @registry.size
      end

      # --- Integration: full boot sequence ---

      def test_full_boot_sequence
        # Simulate the boot sequence from the spec:
        # register display, keyboard, disk, and NIC
        display = SimulatedDisplay.new
        display.init
        @registry.register(display)

        kb = SimulatedKeyboard.new
        kb.init
        @registry.register(kb)

        disk = SimulatedDisk.new
        disk.init
        @registry.register(disk)

        wire = SharedWire.new
        nic = SimulatedNIC.new(wire: wire)
        nic.init
        @registry.register(nic)

        assert_equal 4, @registry.size
        assert_equal display, @registry.lookup_by_name("display0")
        assert_equal kb, @registry.lookup_by_major_minor(2, 0)
        assert_equal disk, @registry.lookup_by_name("disk0")
        assert_equal nic, @registry.lookup_by_name("nic0")

        # Verify type filtering
        assert_equal 2, @registry.list_by_type(DeviceType::CHARACTER).length
        assert_equal 1, @registry.list_by_type(DeviceType::BLOCK).length
        assert_equal 1, @registry.list_by_type(DeviceType::NETWORK).length
      end
    end
  end
end
