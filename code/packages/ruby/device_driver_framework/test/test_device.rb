# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module DeviceDriverFramework
    class TestDeviceType < Minitest::Test
      # DeviceType assigns distinct integer values to each device family.
      # This ensures that CHARACTER, BLOCK, and NETWORK are distinguishable
      # and follow the spec (0, 1, 2).

      def test_character_type_is_zero
        assert_equal 0, DeviceType::CHARACTER
      end

      def test_block_type_is_one
        assert_equal 1, DeviceType::BLOCK
      end

      def test_network_type_is_two
        assert_equal 2, DeviceType::NETWORK
      end

      def test_all_types_are_distinct
        types = [DeviceType::CHARACTER, DeviceType::BLOCK, DeviceType::NETWORK]
        assert_equal 3, types.uniq.length
      end

      def test_name_for_character
        assert_equal "Character", DeviceType.name_for(DeviceType::CHARACTER)
      end

      def test_name_for_block
        assert_equal "Block", DeviceType.name_for(DeviceType::BLOCK)
      end

      def test_name_for_network
        assert_equal "Network", DeviceType.name_for(DeviceType::NETWORK)
      end

      def test_name_for_unknown
        assert_equal "Unknown", DeviceType.name_for(99)
      end
    end

    class TestDevice < Minitest::Test
      # Device is the base class with common fields. Every device has a name,
      # type, major/minor numbers, and interrupt number.

      def test_device_stores_all_fields
        dev = Device.new(
          name: "test0",
          device_type: DeviceType::CHARACTER,
          major: 1,
          minor: 0,
          interrupt_number: 33
        )
        assert_equal "test0", dev.name
        assert_equal DeviceType::CHARACTER, dev.device_type
        assert_equal 1, dev.major
        assert_equal 0, dev.minor
        assert_equal 33, dev.interrupt_number
      end

      def test_device_not_initialized_by_default
        dev = Device.new(name: "test0", device_type: DeviceType::BLOCK, major: 3, minor: 0)
        refute dev.initialized
      end

      def test_init_sets_initialized_flag
        dev = Device.new(name: "test0", device_type: DeviceType::BLOCK, major: 3, minor: 0)
        dev.init
        assert dev.initialized
      end

      def test_default_interrupt_number_is_negative_one
        dev = Device.new(name: "test0", device_type: DeviceType::BLOCK, major: 3, minor: 0)
        assert_equal(-1, dev.interrupt_number)
      end

      def test_to_s_includes_name_and_type
        dev = Device.new(name: "disk0", device_type: DeviceType::BLOCK, major: 3, minor: 0)
        str = dev.to_s
        assert_includes str, "disk0"
        assert_includes str, "Block"
        assert_includes str, "major=3"
        assert_includes str, "minor=0"
      end
    end

    class TestCharacterDevice < Minitest::Test
      def test_character_device_has_character_type
        dev = CharacterDevice.new(name: "char0", major: 1, minor: 0)
        assert_equal DeviceType::CHARACTER, dev.device_type
      end

      def test_read_raises_not_implemented
        dev = CharacterDevice.new(name: "char0", major: 1, minor: 0)
        assert_raises(NotImplementedError) { dev.read(10) }
      end

      def test_write_raises_not_implemented
        dev = CharacterDevice.new(name: "char0", major: 1, minor: 0)
        assert_raises(NotImplementedError) { dev.write([1, 2, 3]) }
      end
    end

    class TestBlockDevice < Minitest::Test
      def test_block_device_has_block_type
        dev = BlockDevice.new(name: "blk0", major: 3, minor: 0, total_blocks: 100)
        assert_equal DeviceType::BLOCK, dev.device_type
      end

      def test_block_device_stores_block_size_and_total
        dev = BlockDevice.new(name: "blk0", major: 3, minor: 0, block_size: 1024, total_blocks: 50)
        assert_equal 1024, dev.block_size
        assert_equal 50, dev.total_blocks
      end

      def test_default_block_size_is_512
        dev = BlockDevice.new(name: "blk0", major: 3, minor: 0, total_blocks: 100)
        assert_equal 512, dev.block_size
      end

      def test_read_block_raises_not_implemented
        dev = BlockDevice.new(name: "blk0", major: 3, minor: 0, total_blocks: 100)
        assert_raises(NotImplementedError) { dev.read_block(0) }
      end

      def test_write_block_raises_not_implemented
        dev = BlockDevice.new(name: "blk0", major: 3, minor: 0, total_blocks: 100)
        assert_raises(NotImplementedError) { dev.write_block(0, [0] * 512) }
      end
    end

    class TestNetworkDevice < Minitest::Test
      def test_network_device_has_network_type
        dev = NetworkDevice.new(name: "net0", major: 4, minor: 0, mac_address: [0xAA] * 6)
        assert_equal DeviceType::NETWORK, dev.device_type
      end

      def test_mac_address_is_stored
        mac = [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01]
        dev = NetworkDevice.new(name: "net0", major: 4, minor: 0, mac_address: mac)
        assert_equal mac, dev.mac_address
      end

      def test_mac_address_must_be_6_bytes
        assert_raises(ArgumentError) do
          NetworkDevice.new(name: "net0", major: 4, minor: 0, mac_address: [0xAA] * 5)
        end
      end

      def test_mac_address_is_frozen
        mac = [0xAA] * 6
        dev = NetworkDevice.new(name: "net0", major: 4, minor: 0, mac_address: mac)
        assert dev.mac_address.frozen?
      end

      def test_send_packet_raises_not_implemented
        dev = NetworkDevice.new(name: "net0", major: 4, minor: 0, mac_address: [0xAA] * 6)
        assert_raises(NotImplementedError) { dev.send_packet([1, 2, 3]) }
      end

      def test_receive_packet_raises_not_implemented
        dev = NetworkDevice.new(name: "net0", major: 4, minor: 0, mac_address: [0xAA] * 6)
        assert_raises(NotImplementedError) { dev.receive_packet }
      end

      def test_has_packet_raises_not_implemented
        dev = NetworkDevice.new(name: "net0", major: 4, minor: 0, mac_address: [0xAA] * 6)
        assert_raises(NotImplementedError) { dev.has_packet? }
      end
    end
  end
end
