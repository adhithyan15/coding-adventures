# frozen_string_literal: true

# ============================================================================
# Layer 4: UDP — User Datagram Protocol
# ============================================================================
#
# UDP is the "anti-TCP" — it provides no reliability, no ordering, no flow
# control, and no connection state. You send a datagram (a self-contained
# packet of data); it either arrives or it doesn't. There are no
# acknowledgments, no retransmissions, and no sequence numbers.
#
# Why would anyone use such a "unreliable" protocol? Because simplicity
# and speed matter for certain applications:
#
#   - DNS lookups: one question, one answer — if it's lost, just ask again.
#   - Video streaming: a dropped frame is better than pausing to retransmit.
#   - Online games: the latest position update matters; old ones are useless.
#   - VoIP: stuttering is better than silence followed by a burst of audio.
#
# The entire UDP header is only 8 bytes (vs. TCP's minimum 20). This means
# less overhead per packet, which adds up when you're sending thousands of
# small messages per second.
#
# UDP Header format:
#
#    0                   1                   2                   3
#    0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
#   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#   |          Source Port          |       Destination Port        |
#   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#   |            Length             |           Checksum            |
#   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#
# ============================================================================

module CodingAdventures
  module NetworkStack
    # ========================================================================
    # UDPHeader
    # ========================================================================
    #
    # The 8-byte UDP header. Compare its simplicity to TCPHeader above —
    # no sequence numbers, no acknowledgments, no flags, no window size.
    # Just source port, destination port, length, and an optional checksum.
    #
    # ========================================================================
    class UDPHeader
      attr_accessor :src_port, :dst_port, :length, :checksum

      def initialize(src_port:, dst_port:, length: 8, checksum: 0)
        @src_port = src_port
        @dst_port = dst_port
        @length   = length
        @checksum = checksum
      end

      # Serialize the UDP header into an 8-byte array.
      def serialize
        bytes = Array.new(8, 0)
        bytes[0] = (@src_port >> 8) & 0xFF
        bytes[1] = @src_port & 0xFF
        bytes[2] = (@dst_port >> 8) & 0xFF
        bytes[3] = @dst_port & 0xFF
        bytes[4] = (@length >> 8) & 0xFF
        bytes[5] = @length & 0xFF
        bytes[6] = (@checksum >> 8) & 0xFF
        bytes[7] = @checksum & 0xFF
        bytes
      end

      # Deserialize an 8-byte array into a UDPHeader.
      def self.deserialize(bytes)
        return nil if bytes.length < 8

        src_port = (bytes[0] << 8) | bytes[1]
        dst_port = (bytes[2] << 8) | bytes[3]
        length   = (bytes[4] << 8) | bytes[5]
        checksum = (bytes[6] << 8) | bytes[7]

        new(src_port: src_port, dst_port: dst_port, length: length, checksum: checksum)
      end
    end

    # ========================================================================
    # UDPSocket — Connectionless Datagram Socket
    # ========================================================================
    #
    # Unlike TCP, a UDP socket has no connection state. You can send a
    # datagram to any address at any time (send_to), and datagrams from
    # any address arrive in a queue (receive_from).
    #
    # The recv_queue stores tuples of (data, src_ip, src_port) so the
    # application knows where each datagram came from — important because
    # there is no "connection" to track this automatically.
    #
    # ========================================================================
    class UDPSocket
      attr_reader :local_port, :recv_queue

      def initialize(local_port:)
        @local_port = local_port
        @recv_queue = []
      end

      # Prepare a datagram for sending. Returns [UDPHeader, data].
      #
      # Note: UDP doesn't actually "connect" to anything — each send_to
      # specifies the destination independently. The header just says
      # "from port X, to port Y, this many bytes."
      #
      def send_to(data, dst_port)
        header = UDPHeader.new(
          src_port: @local_port,
          dst_port: dst_port,
          length: 8 + data.length
        )
        [header, data]
      end

      # Deliver an incoming datagram to this socket.
      #
      # Called by the network stack when a UDP packet arrives addressed to
      # our port. The application will later call receive_from to retrieve it.
      #
      def deliver(data, src_ip, src_port)
        @recv_queue.push({data: data, src_ip: src_ip, src_port: src_port})
      end

      # Retrieve the next datagram from the receive queue.
      #
      # Returns {data:, src_ip:, src_port:} or nil if the queue is empty.
      #
      def receive_from
        @recv_queue.shift
      end
    end
  end
end
