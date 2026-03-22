# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module DeviceDriverFramework
    class TestSimulatedNIC < Minitest::Test
      def setup
        @wire = SharedWire.new
        @nic_a = SimulatedNIC.new(
          name: "nic0", minor: 0, wire: @wire,
          mac_address: [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x01]
        )
        @nic_b = SimulatedNIC.new(
          name: "nic1", minor: 1, wire: @wire,
          mac_address: [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x02]
        )
        @nic_a.init
        @nic_b.init
      end

      # --- Basic properties ---

      def test_default_name
        wire = SharedWire.new
        nic = SimulatedNIC.new(wire: wire)
        assert_equal "nic0", nic.name
      end

      def test_device_type_is_network
        assert_equal DeviceType::NETWORK, @nic_a.device_type
      end

      def test_major_number_is_4
        assert_equal 4, @nic_a.major
      end

      def test_interrupt_number_is_35
        assert_equal 35, @nic_a.interrupt_number
      end

      def test_mac_address_is_6_bytes
        assert_equal 6, @nic_a.mac_address.length
      end

      def test_mac_address_stored_correctly
        assert_equal [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x01], @nic_a.mac_address
      end

      # --- Receiving from empty queue ---

      def test_receive_empty_returns_nil
        assert_nil @nic_a.receive_packet
      end

      def test_has_packet_false_when_empty
        refute @nic_a.has_packet?
      end

      # --- Sending and receiving ---

      def test_send_packet_delivers_to_other_nic
        @nic_a.send_packet([1, 2, 3, 4, 5])
        assert @nic_b.has_packet?
        result = @nic_b.receive_packet
        assert_equal [1, 2, 3, 4, 5], result
      end

      def test_send_packet_returns_byte_count
        result = @nic_a.send_packet([1, 2, 3])
        assert_equal 3, result
      end

      def test_packet_not_echoed_to_sender
        # NIC A sends — it should NOT receive its own packet
        @nic_a.send_packet([1, 2, 3])
        refute @nic_a.has_packet?
        assert_nil @nic_a.receive_packet
      end

      def test_fifo_ordering
        @nic_a.send_packet([1])
        @nic_a.send_packet([2])
        @nic_a.send_packet([3])

        assert_equal [1], @nic_b.receive_packet
        assert_equal [2], @nic_b.receive_packet
        assert_equal [3], @nic_b.receive_packet
      end

      def test_bidirectional_communication
        @nic_a.send_packet([0xAA])
        @nic_b.send_packet([0xBB])

        # A should have received B's packet
        assert_equal [0xBB], @nic_a.receive_packet
        # B should have received A's packet
        assert_equal [0xAA], @nic_b.receive_packet
      end

      # --- Multiple NICs on same wire ---

      def test_broadcast_to_multiple_nics
        nic_c = SimulatedNIC.new(
          name: "nic2", minor: 2, wire: @wire,
          mac_address: [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x03]
        )
        nic_c.init

        @nic_a.send_packet([42])

        assert_equal [42], @nic_b.receive_packet
        assert_equal [42], nic_c.receive_packet
        assert_nil @nic_a.receive_packet  # Sender does not receive
      end

      # --- Init clears queue ---

      def test_init_clears_receive_queue
        @nic_a.send_packet([1, 2, 3])
        assert @nic_b.has_packet?
        @nic_b.init
        refute @nic_b.has_packet?
      end

      # --- Packet data is independent copy ---

      def test_received_packet_is_independent_copy
        original = [1, 2, 3]
        @nic_a.send_packet(original)
        received = @nic_b.receive_packet
        # Modify original — should not affect received
        original[0] = 99
        assert_equal 1, received[0]
      end

      # --- has_packet? after receive ---

      def test_has_packet_after_receive_drains
        @nic_a.send_packet([1])
        assert @nic_b.has_packet?
        @nic_b.receive_packet
        refute @nic_b.has_packet?
      end
    end
  end
end
