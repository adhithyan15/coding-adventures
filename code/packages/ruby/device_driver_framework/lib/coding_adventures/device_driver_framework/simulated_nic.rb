# frozen_string_literal: true

module CodingAdventures
  module DeviceDriverFramework
    # SimulatedNIC — a network interface card backed by in-memory packet queues.
    #
    # A NIC (Network Interface Card) is the hardware that connects a computer
    # to a network. It sends and receives packets — discrete chunks of data
    # with headers (who sent it, who should receive it) and payloads (the
    # actual data).
    #
    # Our SimulatedNIC uses a SharedWire to exchange packets with other NICs.
    # When you call send_packet(), the data is broadcast to all other NICs on
    # the same wire. When another NIC sends a packet, it appears in this NIC's
    # receive queue (rx_queue).
    #
    # The rx_queue is a FIFO (first-in, first-out) queue. Packets are received
    # in the order they were sent. In a real system, interrupt 35 would fire
    # when a packet arrives, prompting the kernel to call receive_packet().
    #
    # Example:
    #   wire = SharedWire.new
    #   nic = SimulatedNIC.new(name: "nic0", wire: wire,
    #                          mac_address: [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01])
    #   nic.init
    #   nic.send_packet([0x48, 0x65, 0x6C, 0x6C, 0x6F])
    class SimulatedNIC < NetworkDevice
      # @param name [String] Device name (default "nic0")
      # @param minor [Integer] Minor number (default 0)
      # @param wire [SharedWire] The shared network medium
      # @param mac_address [Array<Integer>] 6-byte MAC address
      def initialize(name: "nic0", minor: 0, wire:, mac_address: [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01])
        super(
          name: name,
          major: 4,
          minor: minor,
          interrupt_number: 35,
          mac_address: mac_address
        )
        @wire = wire
        @rx_queue = []
      end

      # Initialize the NIC: clear the receive queue and connect to the wire.
      #
      # In real hardware, init() would reset the NIC, configure DMA buffers,
      # and enable the receive interrupt. For simulation, we just clear state
      # and register with the wire.
      def init
        super
        @rx_queue.clear
        @wire.connect(self)
      end

      # Send a packet over the network.
      #
      # The packet is broadcast to all other NICs on the same wire. In a real
      # network, the NIC would add framing (preamble, CRC) and transmit the
      # packet as electrical signals. We skip that and just copy the data.
      #
      # @param data [Array<Integer>] The packet data to send
      # @return [Integer] Number of bytes sent
      def send_packet(data)
        @wire.broadcast(data, sender: self)
        data.length
      end

      # Receive the next packet from the receive queue.
      #
      # This is non-blocking: if no packet has arrived, it returns nil
      # immediately. In a real system, the kernel would typically wait for
      # interrupt 35 before calling this method.
      #
      # @return [Array<Integer>, nil] Packet data, or nil if queue is empty
      def receive_packet
        @rx_queue.shift
      end

      # Check whether a packet is waiting in the receive queue.
      #
      # @return [Boolean] true if at least one packet is available
      def has_packet?
        !@rx_queue.empty?
      end

      # Enqueue a received packet. Called by SharedWire.broadcast() when
      # another NIC sends data.
      #
      # This is the simulation equivalent of the NIC hardware depositing a
      # received frame into a DMA buffer and raising interrupt 35.
      #
      # @param data [Array<Integer>] The received packet data
      def enqueue_packet(data)
        @rx_queue << data
      end
    end
  end
end
