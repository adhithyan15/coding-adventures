"""
TCP — Layer 4 (Transport), Reliable Stream
===========================================

TCP (Transmission Control Protocol) provides a **reliable, ordered, byte-stream**
connection between two endpoints. It is the workhorse of the Internet — HTTP,
SSH, email, and most application protocols run over TCP.

Why TCP Exists
--------------

IP packets can arrive out of order, get duplicated, or be lost entirely. TCP
solves all three problems:

1. **Reliability**: Every byte is acknowledged. If a segment is lost, it is
   retransmitted.
2. **Ordering**: Each byte has a sequence number. The receiver reorders
   segments that arrive out of order.
3. **Flow control**: The receiver advertises a window size telling the sender
   how much data it can accept.

The TCP State Machine
---------------------

A TCP connection goes through a series of states. This is the most important
diagram in all of networking::

    CLOSED
      |
      | (client calls connect)
      | send SYN
      v
    SYN_SENT ----recv SYN+ACK----> send ACK ----> ESTABLISHED
      |                                               |
      |                                    (either side calls close)
      |                                        send FIN
      |                                               |
      |                                               v
      |                                          FIN_WAIT_1
      |                                               |
      |                                       recv ACK of FIN
      |                                               |
      |                                               v
      |                                          FIN_WAIT_2
      |                                               |
      |                                          recv FIN
      |                                        send ACK
      |                                               |
      |                                               v
      |                                          TIME_WAIT
      |                                               |
      |                                          (timeout)
      |                                               v
      +-------------------------------------------> CLOSED

    Server side:

    CLOSED
      |
      | (server calls listen)
      v
    LISTEN
      |
      | recv SYN, send SYN+ACK
      v
    SYN_RECEIVED
      |
      | recv ACK
      v
    ESTABLISHED
      |
      | recv FIN, send ACK
      v
    CLOSE_WAIT
      |
      | (application calls close)
      | send FIN
      v
    LAST_ACK
      |
      | recv ACK
      v
    CLOSED

The Three-Way Handshake
-----------------------

TCP connections begin with a three-step ritual::

    Client                          Server
    ------                          ------
    SYN (seq=100)          ---->
                           <----    SYN+ACK (seq=300, ack=101)
    ACK (ack=301)          ---->
                                    ESTABLISHED!

Why three steps? Because both sides need to establish their initial sequence
numbers and confirm the other side received them. Two steps wouldn't be
enough — the server wouldn't know if the client received the SYN+ACK.

TCP Header Format
-----------------

::

    0                   1                   2                   3
    0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
   |          Source Port          |       Destination Port        |
   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
   |                        Sequence Number                       |
   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
   |                    Acknowledgment Number                     |
   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
   |  Data |           |U|A|P|R|S|F|                               |
   | Offset| Reserved  |R|C|S|S|Y|I|            Window             |
   |       |           |G|K|H|T|N|N|                               |
   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
   |           Checksum            |         Urgent Pointer        |
   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
"""

from __future__ import annotations

import struct
from dataclasses import dataclass
from enum import IntEnum

# ============================================================================
# TCP Flags
# ============================================================================
# These are single-bit flags in the TCP header that control the connection
# state machine. Multiple flags can be set simultaneously (e.g., SYN+ACK).

TCP_FIN = 0x01  # Finish: sender has no more data to send
TCP_SYN = 0x02  # Synchronize: initiates a connection (sets initial seq num)
TCP_RST = 0x04  # Reset: abruptly terminates the connection
TCP_PSH = 0x08  # Push: receiver should deliver data to application immediately
TCP_ACK = 0x10  # Acknowledge: the ack_num field is valid


# ============================================================================
# TCPState
# ============================================================================

class TCPState(IntEnum):
    """
    The states of a TCP connection.

    A connection transitions through these states during its lifecycle.
    The normal path for a client is::

        CLOSED -> SYN_SENT -> ESTABLISHED -> FIN_WAIT_1 -> FIN_WAIT_2 ->
        TIME_WAIT -> CLOSED

    The normal path for a server is::

        CLOSED -> LISTEN -> SYN_RECEIVED -> ESTABLISHED -> CLOSE_WAIT ->
        LAST_ACK -> CLOSED
    """

    CLOSED = 0
    LISTEN = 1
    SYN_SENT = 2
    SYN_RECEIVED = 3
    ESTABLISHED = 4
    FIN_WAIT_1 = 5
    FIN_WAIT_2 = 6
    CLOSE_WAIT = 7
    CLOSING = 8
    LAST_ACK = 9
    TIME_WAIT = 10


# ============================================================================
# TCPHeader
# ============================================================================

@dataclass
class TCPHeader:
    """
    A TCP segment header — 20 bytes minimum.

    Attributes
    ----------
    src_port : int
        Source port number (0-65535).
    dst_port : int
        Destination port number (0-65535).
    seq_num : int
        Sequence number of the first byte in this segment's payload.
        For SYN segments, this is the initial sequence number (ISN).
    ack_num : int
        If the ACK flag is set, this is the next byte the sender expects
        to receive from the remote side.
    data_offset : int
        Header length in 32-bit words (minimum 5 = 20 bytes).
    flags : int
        Combination of TCP flag constants (TCP_SYN | TCP_ACK, etc.).
    window_size : int
        How many bytes the sender is willing to accept (flow control).
    checksum : int
        Error detection (simplified in our implementation).
    """

    src_port: int = 0
    dst_port: int = 0
    seq_num: int = 0
    ack_num: int = 0
    data_offset: int = 5
    flags: int = 0
    window_size: int = 65535
    checksum: int = 0

    def serialize(self) -> bytes:
        """
        Convert the TCP header to 20 raw bytes.

        Layout::

            Bytes 0-1:   src_port
            Bytes 2-3:   dst_port
            Bytes 4-7:   seq_num
            Bytes 8-11:  ack_num
            Byte 12:     data_offset (upper 4 bits) | reserved (lower 4)
            Byte 13:     flags
            Bytes 14-15: window_size
            Bytes 16-17: checksum
            Bytes 18-19: urgent_pointer (always 0)
        """
        # The data_offset field is packed into the upper 4 bits of byte 12.
        offset_byte = (self.data_offset << 4) & 0xF0

        return struct.pack(
            "!HHIIBBHHH",
            self.src_port,
            self.dst_port,
            self.seq_num & 0xFFFFFFFF,
            self.ack_num & 0xFFFFFFFF,
            offset_byte,
            self.flags,
            self.window_size,
            self.checksum,
            0,  # urgent pointer
        )

    @classmethod
    def deserialize(cls, data: bytes) -> TCPHeader:
        """
        Parse 20 bytes into a TCPHeader.

        Raises ValueError if data is too short.
        """
        if len(data) < 20:
            msg = f"TCP header too short: {len(data)} bytes (minimum 20)"
            raise ValueError(msg)

        (
            src_port,
            dst_port,
            seq_num,
            ack_num,
            offset_byte,
            flags,
            window_size,
            checksum,
            _urgent,
        ) = struct.unpack("!HHIIBBHHH", data[:20])

        data_offset = (offset_byte >> 4) & 0x0F

        return cls(
            src_port=src_port,
            dst_port=dst_port,
            seq_num=seq_num,
            ack_num=ack_num,
            data_offset=data_offset,
            flags=flags,
            window_size=window_size,
            checksum=checksum,
        )


# ============================================================================
# TCPConnection
# ============================================================================

class TCPConnection:
    """
    A full TCP state machine for one connection.

    This class manages the lifecycle of a single TCP connection — from the
    three-way handshake through data transfer to the four-way close. It tracks
    sequence numbers, acknowledgment numbers, and the current state.

    Usage (Client Side)
    -------------------

    ::

        conn = TCPConnection(local_port=49152, remote_ip=0x0A000002,
                             remote_port=80)
        syn = conn.initiate_connect()           # SYN
        synack = ...                            # receive SYN+ACK from network
        ack = conn.handle_segment(synack)       # ACK -> ESTABLISHED
        data_seg = conn.send(b"GET / HTTP/1.1") # data segment
        ...

    Usage (Server Side)
    -------------------

    ::

        conn = TCPConnection(local_port=80)
        conn.initiate_listen()                  # -> LISTEN
        syn = ...                               # receive SYN from network
        synack = conn.handle_segment(syn)       # SYN+ACK -> SYN_RECEIVED
        ack = ...                               # receive ACK from network
        conn.handle_segment(ack)                # -> ESTABLISHED

    Parameters
    ----------
    local_port : int
        The port number on this side of the connection.
    remote_ip : int
        The IP address of the remote host (0 until connect is called).
    remote_port : int
        The port number on the remote side (0 until connect is called).
    """

    def __init__(
        self,
        local_port: int,
        remote_ip: int = 0,
        remote_port: int = 0,
    ) -> None:
        self.state = TCPState.CLOSED
        self.local_port = local_port
        self.remote_ip = remote_ip
        self.remote_port = remote_port

        # Sequence numbers:
        # - seq_num: the next byte WE will send
        # - ack_num: the next byte WE expect to receive from the remote
        self.seq_num = 0
        self.ack_num = 0

        # Buffers for data that hasn't been sent/consumed yet
        self.send_buffer = bytearray()
        self.recv_buffer = bytearray()

    def initiate_connect(self) -> TCPHeader:
        """
        Client-side: begin the three-way handshake by sending a SYN.

        Transitions: CLOSED -> SYN_SENT

        The SYN segment carries our initial sequence number (ISN). In a real
        implementation, the ISN would be randomized for security. We use 1000
        for clarity in testing.
        """
        self.seq_num = 1000  # Initial Sequence Number
        self.state = TCPState.SYN_SENT

        syn = TCPHeader(
            src_port=self.local_port,
            dst_port=self.remote_port,
            seq_num=self.seq_num,
            flags=TCP_SYN,
        )
        # SYN consumes one sequence number
        self.seq_num += 1
        return syn

    def initiate_listen(self) -> None:
        """
        Server-side: transition to LISTEN state, ready to accept connections.

        Transitions: CLOSED -> LISTEN
        """
        self.state = TCPState.LISTEN

    def handle_segment(
        self, header: TCPHeader, payload: bytes = b""
    ) -> TCPHeader | None:
        """
        Process an incoming TCP segment and produce a response.

        This is the heart of the TCP state machine. Depending on the current
        state and the flags in the incoming segment, the connection transitions
        to a new state and may produce a response segment.

        State Transitions Handled
        -------------------------

        - LISTEN + SYN -> SYN_RECEIVED (send SYN+ACK)
        - SYN_SENT + SYN+ACK -> ESTABLISHED (send ACK)
        - SYN_RECEIVED + ACK -> ESTABLISHED
        - ESTABLISHED + data -> ESTABLISHED (send ACK)
        - ESTABLISHED + FIN -> CLOSE_WAIT (send ACK)
        - FIN_WAIT_1 + ACK -> FIN_WAIT_2
        - FIN_WAIT_1 + FIN+ACK -> TIME_WAIT (send ACK)
        - FIN_WAIT_2 + FIN -> TIME_WAIT (send ACK)
        - LAST_ACK + ACK -> CLOSED
        - CLOSING + ACK -> TIME_WAIT

        Parameters
        ----------
        header : TCPHeader
            The incoming segment's header.
        payload : bytes
            Any data carried in the segment.

        Returns
        -------
        TCPHeader | None
            A response header to send back, or None if no response is needed.
        """
        flags = header.flags

        # -------------------------------------------------------------------
        # LISTEN state: waiting for incoming connections
        # -------------------------------------------------------------------
        if self.state == TCPState.LISTEN:
            if flags & TCP_SYN:
                # Received a SYN — begin server-side handshake
                self.remote_port = header.src_port
                self.ack_num = header.seq_num + 1
                self.seq_num = 3000  # Server ISN
                self.state = TCPState.SYN_RECEIVED

                synack = TCPHeader(
                    src_port=self.local_port,
                    dst_port=self.remote_port,
                    seq_num=self.seq_num,
                    ack_num=self.ack_num,
                    flags=TCP_SYN | TCP_ACK,
                )
                self.seq_num += 1  # SYN consumes one seq num
                return synack

        # -------------------------------------------------------------------
        # SYN_SENT state: client waiting for SYN+ACK
        # -------------------------------------------------------------------
        elif self.state == TCPState.SYN_SENT:
            if (flags & TCP_SYN) and (flags & TCP_ACK):
                # Received SYN+ACK — complete the handshake
                self.ack_num = header.seq_num + 1
                self.state = TCPState.ESTABLISHED

                ack = TCPHeader(
                    src_port=self.local_port,
                    dst_port=self.remote_port,
                    seq_num=self.seq_num,
                    ack_num=self.ack_num,
                    flags=TCP_ACK,
                )
                return ack

        # -------------------------------------------------------------------
        # SYN_RECEIVED state: server waiting for final ACK of handshake
        # -------------------------------------------------------------------
        elif self.state == TCPState.SYN_RECEIVED:
            if flags & TCP_ACK:
                self.state = TCPState.ESTABLISHED
                return None

        # -------------------------------------------------------------------
        # ESTABLISHED state: connection is open, data can flow
        # -------------------------------------------------------------------
        elif self.state == TCPState.ESTABLISHED:
            if flags & TCP_FIN:
                # Remote side wants to close
                self.ack_num = header.seq_num + 1
                self.state = TCPState.CLOSE_WAIT

                ack = TCPHeader(
                    src_port=self.local_port,
                    dst_port=self.remote_port,
                    seq_num=self.seq_num,
                    ack_num=self.ack_num,
                    flags=TCP_ACK,
                )
                return ack

            if payload:
                # Data segment — add to receive buffer, acknowledge
                self.recv_buffer.extend(payload)
                self.ack_num = header.seq_num + len(payload)

                ack = TCPHeader(
                    src_port=self.local_port,
                    dst_port=self.remote_port,
                    seq_num=self.seq_num,
                    ack_num=self.ack_num,
                    flags=TCP_ACK,
                )
                return ack

            # Pure ACK with no data — no response needed
            return None

        # -------------------------------------------------------------------
        # FIN_WAIT_1: we sent FIN, waiting for ACK
        # -------------------------------------------------------------------
        elif self.state == TCPState.FIN_WAIT_1:
            if (flags & TCP_FIN) and (flags & TCP_ACK):
                # Simultaneous close: remote sends FIN+ACK
                self.ack_num = header.seq_num + 1
                self.state = TCPState.TIME_WAIT

                ack = TCPHeader(
                    src_port=self.local_port,
                    dst_port=self.remote_port,
                    seq_num=self.seq_num,
                    ack_num=self.ack_num,
                    flags=TCP_ACK,
                )
                return ack

            if flags & TCP_ACK:
                # ACK of our FIN
                self.state = TCPState.FIN_WAIT_2
                return None

            if flags & TCP_FIN:
                # Simultaneous close: both sides sent FIN
                self.ack_num = header.seq_num + 1
                self.state = TCPState.CLOSING

                ack = TCPHeader(
                    src_port=self.local_port,
                    dst_port=self.remote_port,
                    seq_num=self.seq_num,
                    ack_num=self.ack_num,
                    flags=TCP_ACK,
                )
                return ack

        # -------------------------------------------------------------------
        # FIN_WAIT_2: our FIN was ACK'd, waiting for remote FIN
        # -------------------------------------------------------------------
        elif self.state == TCPState.FIN_WAIT_2:
            if flags & TCP_FIN:
                self.ack_num = header.seq_num + 1
                self.state = TCPState.TIME_WAIT

                ack = TCPHeader(
                    src_port=self.local_port,
                    dst_port=self.remote_port,
                    seq_num=self.seq_num,
                    ack_num=self.ack_num,
                    flags=TCP_ACK,
                )
                return ack

        # -------------------------------------------------------------------
        # CLOSING: both sides sent FIN, waiting for ACK of our FIN
        # -------------------------------------------------------------------
        elif self.state == TCPState.CLOSING:
            if flags & TCP_ACK:
                self.state = TCPState.TIME_WAIT
                return None

        # -------------------------------------------------------------------
        # CLOSE_WAIT: remote closed, we haven't closed yet
        # -------------------------------------------------------------------
        elif self.state == TCPState.CLOSE_WAIT:
            # Application needs to call initiate_close()
            return None

        # -------------------------------------------------------------------
        # LAST_ACK: we sent FIN (from CLOSE_WAIT), waiting for ACK
        # -------------------------------------------------------------------
        elif self.state == TCPState.LAST_ACK:
            if flags & TCP_ACK:
                self.state = TCPState.CLOSED
                return None

        # -------------------------------------------------------------------
        # TIME_WAIT: waiting for stale segments to expire (no-op here)
        # -------------------------------------------------------------------
        elif self.state == TCPState.TIME_WAIT:
            return None

        return None

    def send(self, data: bytes) -> TCPHeader | None:
        """
        Queue data for sending and return a segment if the connection is open.

        The data is added to the send buffer. If the connection is ESTABLISHED,
        we create a segment with the PSH+ACK flags (push = deliver immediately
        to application) and advance our sequence number.

        Returns None if the connection is not in ESTABLISHED state.
        """
        self.send_buffer.extend(data)

        if self.state != TCPState.ESTABLISHED:
            return None

        segment = TCPHeader(
            src_port=self.local_port,
            dst_port=self.remote_port,
            seq_num=self.seq_num,
            ack_num=self.ack_num,
            flags=TCP_PSH | TCP_ACK,
        )
        self.seq_num += len(data)
        return segment

    def receive(self, count: int) -> bytes:
        """
        Read up to ``count`` bytes from the receive buffer.

        This removes the bytes from the buffer (they can only be read once).
        Returns an empty bytes object if the buffer is empty.
        """
        result = bytes(self.recv_buffer[:count])
        del self.recv_buffer[:count]
        return result

    def initiate_close(self) -> TCPHeader | None:
        """
        Begin connection teardown by sending a FIN segment.

        Transitions:
        - ESTABLISHED -> FIN_WAIT_1 (active close)
        - CLOSE_WAIT -> LAST_ACK (passive close, responding to remote FIN)

        Returns the FIN segment to send, or None if close is not valid
        from the current state.
        """
        if self.state == TCPState.ESTABLISHED:
            self.state = TCPState.FIN_WAIT_1
        elif self.state == TCPState.CLOSE_WAIT:
            self.state = TCPState.LAST_ACK
        else:
            return None

        fin = TCPHeader(
            src_port=self.local_port,
            dst_port=self.remote_port,
            seq_num=self.seq_num,
            ack_num=self.ack_num,
            flags=TCP_FIN | TCP_ACK,
        )
        self.seq_num += 1  # FIN consumes one sequence number
        return fin
