# frozen_string_literal: true

module CodingAdventures
  module DeviceDriverFramework
    # SharedWire — a simulated network cable connecting multiple NICs.
    #
    # In a real network, devices are connected by physical cables (Ethernet)
    # or radio waves (WiFi). When one device sends a packet, it travels along
    # the medium and is received by other devices on the same network segment.
    #
    # Our SharedWire simulates this. It maintains a list of connected NICs.
    # When one NIC calls broadcast(), the packet is delivered to every other
    # NIC on the wire (but not back to the sender — that would be an echo).
    #
    # This is a simplified model of a shared medium (like early Ethernet hubs
    # or a WiFi channel). In a real network, you would also need to handle
    # collisions, addressing, and routing. We skip those for simplicity.
    #
    # Example:
    #   wire = SharedWire.new
    #   nic_a = SimulatedNIC.new(name: "nic0", wire: wire, mac_address: [0xAA] * 6)
    #   nic_b = SimulatedNIC.new(name: "nic1", wire: wire, mac_address: [0xBB] * 6)
    #   nic_a.init
    #   nic_b.init
    #   nic_a.send_packet([1, 2, 3])
    #   nic_b.receive_packet  # => [1, 2, 3]
    class SharedWire
      attr_reader :connected_nics

      def initialize
        @connected_nics = []
      end

      # Connect a NIC to this wire.
      #
      # @param nic [SimulatedNIC] The NIC to connect
      def connect(nic)
        @connected_nics << nic unless @connected_nics.include?(nic)
      end

      # Disconnect a NIC from this wire.
      #
      # @param nic [SimulatedNIC] The NIC to disconnect
      def disconnect(nic)
        @connected_nics.delete(nic)
      end

      # Broadcast a packet to all NICs on the wire except the sender.
      #
      # This simulates how a packet travels along a physical cable: every
      # device on the wire sees it, but the sender does not receive its own
      # transmission (no echo).
      #
      # @param data [Array<Integer>] The packet data
      # @param sender [SimulatedNIC] The NIC that sent the packet
      def broadcast(data, sender:)
        @connected_nics.each do |nic|
          nic.enqueue_packet(data.dup) unless nic.equal?(sender)
        end
      end
    end
  end
end
