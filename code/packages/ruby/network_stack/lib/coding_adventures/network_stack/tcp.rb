# frozen_string_literal: true

# ============================================================================
# Layer 4: TCP — Transmission Control Protocol
# ============================================================================
#
# TCP provides a **reliable, ordered, byte-stream** service on top of IP's
# unreliable, unordered packet delivery. It is the workhorse of the Internet:
# web browsing (HTTP), email (SMTP), file transfer (FTP), and secure shells
# (SSH) all run on TCP.
#
# TCP achieves reliability through:
#   1. **Sequence numbers** — every byte is numbered, so the receiver can
#      detect missing or reordered data.
#   2. **Acknowledgments** — the receiver tells the sender which bytes it
#      has received ("I got everything up to byte 1000").
#   3. **Retransmission** — if an acknowledgment doesn't arrive within a
#      timeout, the sender resends the data.
#   4. **Flow control** — the receiver advertises a "window size" telling
#      the sender how much buffer space it has. The sender won't overwhelm
#      a slow receiver.
#
# TCP connections go through a well-defined state machine with 11 states,
# from CLOSED through TIME_WAIT. The most important transitions are:
#
#   Three-way handshake (connection setup):
#     Client: SYN          ->  SYN_SENT
#     Server: SYN+ACK      ->  SYN_RECEIVED
#     Client: ACK           ->  ESTABLISHED (both sides)
#
#   Four-way teardown (connection close):
#     Initiator: FIN        ->  FIN_WAIT_1
#     Responder: ACK        ->  CLOSE_WAIT
#     Responder: FIN        ->  LAST_ACK
#     Initiator: ACK        ->  TIME_WAIT -> CLOSED
#
# ============================================================================

module CodingAdventures
  module NetworkStack
    # TCP flag constants — these are bit flags that can be combined.
    # For example, SYN+ACK = 0x02 | 0x10 = 0x12.
    TCP_FIN = 0x01  # Finish — sender is done sending
    TCP_SYN = 0x02  # Synchronize — initiate connection
    TCP_RST = 0x04  # Reset — abort the connection
    TCP_PSH = 0x08  # Push — deliver data to application immediately
    TCP_ACK = 0x10  # Acknowledge — ack_num field is valid

    # The 11 TCP states. Every TCP connection is always in exactly one of
    # these states.
    module TCPState
      CLOSED       = :closed
      LISTEN       = :listen
      SYN_SENT     = :syn_sent
      SYN_RECEIVED = :syn_received
      ESTABLISHED  = :established
      FIN_WAIT_1   = :fin_wait_1
      FIN_WAIT_2   = :fin_wait_2
      CLOSE_WAIT   = :close_wait
      LAST_ACK     = :last_ack
      TIME_WAIT    = :time_wait
      CLOSING      = :closing
    end

    # ========================================================================
    # TCPHeader
    # ========================================================================
    #
    # The TCP header is at least 20 bytes and contains:
    #   - Source and destination port numbers (2 bytes each)
    #   - Sequence number (4 bytes) — position of first data byte
    #   - Acknowledgment number (4 bytes) — next expected byte from other side
    #   - Data offset (4 bits) — header length in 32-bit words
    #   - Flags (6 bits) — SYN, ACK, FIN, RST, PSH, URG
    #   - Window size (2 bytes) — receiver's available buffer space
    #
    # Wire format:
    #
    #    0                   1                   2                   3
    #    0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    #   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    #   |          Source Port          |       Destination Port        |
    #   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    #   |                        Sequence Number                       |
    #   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    #   |                    Acknowledgment Number                     |
    #   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    #   |  Data |           |U|A|P|R|S|F|                              |
    #   | Offset| Reserved  |R|C|S|S|Y|I|          Window Size         |
    #   |       |           |G|K|H|T|N|N|                              |
    #   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    #
    # ========================================================================
    class TCPHeader
      attr_accessor :src_port, :dst_port, :seq_num, :ack_num,
        :data_offset, :flags, :window_size

      def initialize(
        src_port:,
        dst_port:,
        seq_num: 0,
        ack_num: 0,
        data_offset: 5,
        flags: 0,
        window_size: 65535
      )
        @src_port    = src_port
        @dst_port    = dst_port
        @seq_num     = seq_num
        @ack_num     = ack_num
        @data_offset = data_offset
        @flags       = flags
        @window_size = window_size
      end

      # Serialize the TCP header into a 20-byte array.
      def serialize
        bytes = Array.new(20, 0)

        # Bytes 0-1: Source Port
        bytes[0] = (@src_port >> 8) & 0xFF
        bytes[1] = @src_port & 0xFF

        # Bytes 2-3: Destination Port
        bytes[2] = (@dst_port >> 8) & 0xFF
        bytes[3] = @dst_port & 0xFF

        # Bytes 4-7: Sequence Number
        bytes[4] = (@seq_num >> 24) & 0xFF
        bytes[5] = (@seq_num >> 16) & 0xFF
        bytes[6] = (@seq_num >> 8) & 0xFF
        bytes[7] = @seq_num & 0xFF

        # Bytes 8-11: Acknowledgment Number
        bytes[8]  = (@ack_num >> 24) & 0xFF
        bytes[9]  = (@ack_num >> 16) & 0xFF
        bytes[10] = (@ack_num >> 8) & 0xFF
        bytes[11] = @ack_num & 0xFF

        # Byte 12: Data Offset (4 bits) + Reserved (4 bits)
        bytes[12] = (@data_offset & 0x0F) << 4

        # Byte 13: Flags
        bytes[13] = @flags & 0x3F

        # Bytes 14-15: Window Size
        bytes[14] = (@window_size >> 8) & 0xFF
        bytes[15] = @window_size & 0xFF

        # Bytes 16-19: Checksum + Urgent Pointer (both 0 for simplicity)
        bytes
      end

      # Deserialize a 20-byte array into a TCPHeader.
      def self.deserialize(bytes)
        return nil if bytes.length < 20

        src_port    = (bytes[0] << 8) | bytes[1]
        dst_port    = (bytes[2] << 8) | bytes[3]
        seq_num     = (bytes[4] << 24) | (bytes[5] << 16) | (bytes[6] << 8) | bytes[7]
        ack_num     = (bytes[8] << 24) | (bytes[9] << 16) | (bytes[10] << 8) | bytes[11]
        data_offset = (bytes[12] >> 4) & 0x0F
        flags       = bytes[13] & 0x3F
        window_size = (bytes[14] << 8) | bytes[15]

        new(
          src_port: src_port,
          dst_port: dst_port,
          seq_num: seq_num,
          ack_num: ack_num,
          data_offset: data_offset,
          flags: flags,
          window_size: window_size
        )
      end

      # Helper methods for checking flags
      def syn? = (@flags & TCP_SYN) != 0
      def ack? = (@flags & TCP_ACK) != 0
      def fin? = (@flags & TCP_FIN) != 0
      def rst? = (@flags & TCP_RST) != 0
      def psh? = (@flags & TCP_PSH) != 0
    end

    # ========================================================================
    # TCPConnection — The TCP State Machine
    # ========================================================================
    #
    # Each TCP connection is an instance of this class. It tracks the current
    # state, sequence numbers, and send/receive buffers. The state machine
    # transitions are driven by calling methods like initiate_connect(),
    # handle_segment(), and initiate_close().
    #
    # This is a simplified implementation — a real TCP would handle:
    #   - Congestion control (slow start, congestion avoidance)
    #   - Silly window syndrome avoidance
    #   - Nagle's algorithm
    #   - Out-of-order segment reassembly
    #   - Selective acknowledgments (SACK)
    #
    # But the core state machine and 3-way handshake are faithfully implemented.
    #
    # ========================================================================
    class TCPConnection
      attr_accessor :state, :local_port, :remote_port, :remote_ip,
        :seq_num, :ack_num, :send_buffer, :recv_buffer

      def initialize(local_port:, remote_ip: nil, remote_port: nil)
        @state       = TCPState::CLOSED
        @local_port  = local_port
        @remote_ip   = remote_ip
        @remote_port = remote_port
        @seq_num     = rand(0..65535)  # Initial Sequence Number (ISN)
        @ack_num     = 0
        @send_buffer = []
        @recv_buffer = []
      end

      # Initiate an active open (client side of the 3-way handshake).
      #
      # Transitions: CLOSED -> SYN_SENT
      # Returns: a TCPHeader representing the SYN segment to send.
      #
      def initiate_connect(remote_ip, remote_port)
        @remote_ip   = remote_ip
        @remote_port = remote_port
        @state       = TCPState::SYN_SENT

        TCPHeader.new(
          src_port: @local_port,
          dst_port: @remote_port,
          seq_num: @seq_num,
          ack_num: 0,
          flags: TCP_SYN
        )
      end

      # Start listening for incoming connections (server side).
      #
      # Transitions: CLOSED -> LISTEN
      #
      def initiate_listen
        @state = TCPState::LISTEN
      end

      # Handle an incoming TCP segment. This is the heart of the state machine.
      #
      # Returns: a TCPHeader to send as a response, or nil if no response needed.
      #
      # The state transitions follow RFC 793 (simplified):
      #
      #   State         Received    Action                  New State
      #   ─────         ────────    ──────                  ─────────
      #   LISTEN        SYN         send SYN+ACK            SYN_RECEIVED
      #   SYN_SENT      SYN+ACK    send ACK                ESTABLISHED
      #   SYN_RECEIVED  ACK         (none)                  ESTABLISHED
      #   ESTABLISHED   FIN         send ACK                CLOSE_WAIT
      #   ESTABLISHED   data+ACK    send ACK, buffer data   ESTABLISHED
      #   FIN_WAIT_1    ACK         (none)                  FIN_WAIT_2
      #   FIN_WAIT_1    FIN+ACK     send ACK                TIME_WAIT
      #   FIN_WAIT_2    FIN         send ACK                TIME_WAIT
      #   CLOSE_WAIT    (close)     send FIN                LAST_ACK
      #   LAST_ACK      ACK         (none)                  CLOSED
      #   TIME_WAIT     (timeout)   (none)                  CLOSED
      #
      def handle_segment(header, payload = [])
        case @state
        when TCPState::LISTEN
          handle_listen(header)
        when TCPState::SYN_SENT
          handle_syn_sent(header)
        when TCPState::SYN_RECEIVED
          handle_syn_received(header)
        when TCPState::ESTABLISHED
          handle_established(header, payload)
        when TCPState::FIN_WAIT_1
          handle_fin_wait_1(header)
        when TCPState::FIN_WAIT_2
          handle_fin_wait_2(header)
        when TCPState::LAST_ACK
          handle_last_ack(header)
        when TCPState::CLOSING
          handle_closing(header)
        end
      end

      # Queue data for sending. Returns a TCPHeader for the data segment.
      #
      # In a real TCP, data would be broken into MSS-sized segments and
      # queued for transmission with retransmission tracking. Here we
      # send it all in one segment for simplicity.
      #
      def send_data(data)
        return nil unless @state == TCPState::ESTABLISHED

        @send_buffer.concat(data)
        header = TCPHeader.new(
          src_port: @local_port,
          dst_port: @remote_port,
          seq_num: @seq_num,
          ack_num: @ack_num,
          flags: TCP_ACK | TCP_PSH
        )
        @seq_num += data.length
        header
      end

      # Read data from the receive buffer.
      #
      # Returns up to `count` bytes, removing them from the buffer.
      #
      def receive(count)
        result = @recv_buffer.shift(count)
        result
      end

      # Initiate connection close (active close).
      #
      # Transitions: ESTABLISHED -> FIN_WAIT_1
      # Returns: a TCPHeader for the FIN segment.
      #
      def initiate_close
        return nil unless @state == TCPState::ESTABLISHED || @state == TCPState::CLOSE_WAIT

        if @state == TCPState::ESTABLISHED
          @state = TCPState::FIN_WAIT_1
        elsif @state == TCPState::CLOSE_WAIT
          @state = TCPState::LAST_ACK
        end

        header = TCPHeader.new(
          src_port: @local_port,
          dst_port: @remote_port,
          seq_num: @seq_num,
          ack_num: @ack_num,
          flags: TCP_FIN | TCP_ACK
        )
        @seq_num += 1
        header
      end

      private

      def handle_listen(header)
        return nil unless header.syn?

        @remote_port = header.src_port
        @ack_num     = header.seq_num + 1
        @state       = TCPState::SYN_RECEIVED

        TCPHeader.new(
          src_port: @local_port,
          dst_port: @remote_port,
          seq_num: @seq_num,
          ack_num: @ack_num,
          flags: TCP_SYN | TCP_ACK
        )
      end

      def handle_syn_sent(header)
        return nil unless header.syn? && header.ack?

        @ack_num = header.seq_num + 1
        @seq_num += 1  # SYN consumes one sequence number
        @state   = TCPState::ESTABLISHED

        TCPHeader.new(
          src_port: @local_port,
          dst_port: @remote_port,
          seq_num: @seq_num,
          ack_num: @ack_num,
          flags: TCP_ACK
        )
      end

      def handle_syn_received(header)
        return nil unless header.ack?

        @seq_num += 1  # SYN consumes one sequence number
        @state = TCPState::ESTABLISHED
        nil
      end

      def handle_established(header, payload)
        if header.fin?
          # Passive close: the other side wants to close
          @ack_num = header.seq_num + 1
          @state   = TCPState::CLOSE_WAIT
          return TCPHeader.new(
            src_port: @local_port,
            dst_port: @remote_port,
            seq_num: @seq_num,
            ack_num: @ack_num,
            flags: TCP_ACK
          )
        end

        # Data segment — buffer the payload and acknowledge
        if payload && !payload.empty?
          @recv_buffer.concat(payload)
          @ack_num = header.seq_num + payload.length
          return TCPHeader.new(
            src_port: @local_port,
            dst_port: @remote_port,
            seq_num: @seq_num,
            ack_num: @ack_num,
            flags: TCP_ACK
          )
        end

        nil
      end

      def handle_fin_wait_1(header)
        if header.fin? && header.ack?
          # Simultaneous close or FIN+ACK response
          @ack_num = header.seq_num + 1
          @state   = TCPState::TIME_WAIT
          return TCPHeader.new(
            src_port: @local_port,
            dst_port: @remote_port,
            seq_num: @seq_num,
            ack_num: @ack_num,
            flags: TCP_ACK
          )
        elsif header.ack?
          @state = TCPState::FIN_WAIT_2
          nil
        elsif header.fin?
          @ack_num = header.seq_num + 1
          @state   = TCPState::CLOSING
          TCPHeader.new(
            src_port: @local_port,
            dst_port: @remote_port,
            seq_num: @seq_num,
            ack_num: @ack_num,
            flags: TCP_ACK
          )
        end
      end

      def handle_fin_wait_2(header)
        return nil unless header.fin?

        @ack_num = header.seq_num + 1
        @state   = TCPState::TIME_WAIT

        TCPHeader.new(
          src_port: @local_port,
          dst_port: @remote_port,
          seq_num: @seq_num,
          ack_num: @ack_num,
          flags: TCP_ACK
        )
      end

      def handle_last_ack(header)
        return nil unless header.ack?

        @state = TCPState::CLOSED
        nil
      end

      def handle_closing(header)
        return nil unless header.ack?

        @state = TCPState::TIME_WAIT
        nil
      end
    end
  end
end
