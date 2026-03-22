"""
UDP — Layer 4 (Transport), Unreliable Datagrams
================================================

UDP (User Datagram Protocol) is the simpler sibling of TCP. Where TCP provides
reliable, ordered, connection-oriented byte streams, UDP provides unreliable,
unordered, connectionless **datagrams**.

Why Use UDP?
------------

If TCP is registered mail with tracking, UDP is a postcard. Postcards are:

- **Faster**: No three-way handshake, no acknowledgments, no retransmissions.
  You write the message and drop it in the mailbox.
- **Simpler**: No state machine, no sequence numbers, no flow control.
- **Unreliable**: The postcard might get lost, arrive out of order, or arrive
  twice. The sender never knows.

UDP is used when speed matters more than reliability:

- **DNS**: A single question-and-answer exchange. If the reply is lost,
  just ask again.
- **Video streaming**: A lost frame is better than a delayed frame.
- **Online gaming**: The newest position update matters; old ones are
  irrelevant.

UDP Header Format
-----------------

The UDP header is beautifully simple — just 8 bytes::

    0                   1                   2                   3
    0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
   |          Source Port          |       Destination Port        |
   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
   |            Length             |           Checksum            |
   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

Compare this with TCP's 20 bytes of header overhead. UDP trades features
for efficiency.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass


@dataclass
class UDPHeader:
    """
    A UDP datagram header — exactly 8 bytes.

    Attributes
    ----------
    src_port : int
        Source port (0-65535). Can be 0 if the sender doesn't expect a reply.
    dst_port : int
        Destination port (0-65535).
    length : int
        Total datagram size (header + payload) in bytes. Minimum 8 (header
        only, no payload).
    checksum : int
        Error detection. Can be 0 (meaning "no checksum computed") in IPv4.
        We always set it to 0 in this teaching implementation.
    """

    src_port: int = 0
    dst_port: int = 0
    length: int = 8
    checksum: int = 0

    def serialize(self) -> bytes:
        """
        Convert the header to 8 raw bytes in network byte order.

        Layout::

            Bytes 0-1: src_port
            Bytes 2-3: dst_port
            Bytes 4-5: length
            Bytes 6-7: checksum
        """
        return struct.pack(
            "!HHHH",
            self.src_port,
            self.dst_port,
            self.length,
            self.checksum,
        )

    @classmethod
    def deserialize(cls, data: bytes) -> UDPHeader:
        """
        Parse 8 bytes into a UDPHeader.

        Raises ValueError if data is too short.
        """
        if len(data) < 8:
            msg = f"UDP header too short: {len(data)} bytes (minimum 8)"
            raise ValueError(msg)

        src_port, dst_port, length, checksum = struct.unpack("!HHHH", data[:8])

        return cls(
            src_port=src_port,
            dst_port=dst_port,
            length=length,
            checksum=checksum,
        )


class UDPSocket:
    """
    A UDP socket — sends and receives individual datagrams.

    Unlike TCP, there is no connection setup. Each datagram is independent:
    you specify the destination address with every send, and each received
    datagram tells you where it came from.

    Think of this like a mailbox: you can send a letter to anyone without
    establishing a relationship first, and you can receive letters from
    anyone who knows your address.

    Parameters
    ----------
    local_port : int
        The port this socket is bound to. Other hosts send to this port.

    Example
    -------
    >>> sock = UDPSocket(local_port=53)
    >>> header, payload = sock.send_to(b"query", dest_ip=0x08080808, dest_port=53)
    >>> sock.deliver(b"response", src_ip=0x08080808, src_port=53)
    >>> data, ip, port = sock.receive_from()
    """

    def __init__(self, local_port: int = 0) -> None:
        self.local_port = local_port
        # Queue of received datagrams: (data, src_ip, src_port)
        self._recv_queue: list[tuple[bytes, int, int]] = []

    def send_to(
        self, data: bytes, dest_ip: int, dest_port: int
    ) -> tuple[UDPHeader, bytes]:
        """
        Create a UDP datagram for the given destination.

        Returns the header and payload bytes. The caller (IP layer) is
        responsible for actually transmitting the datagram.

        Parameters
        ----------
        data : bytes
            The payload to send.
        dest_ip : int
            Destination IP (not used in the header, but returned for
            the caller's convenience).
        dest_port : int
            Destination port number.

        Returns
        -------
        tuple[UDPHeader, bytes]
            The UDP header and the payload.
        """
        header = UDPHeader(
            src_port=self.local_port,
            dst_port=dest_port,
            length=8 + len(data),
        )
        return header, data

    def receive_from(self) -> tuple[bytes, int, int] | None:
        """
        Receive the next datagram from the queue.

        Returns (data, src_ip, src_port) or None if no datagrams are
        available.

        Unlike TCP's ``recv()``, this returns the sender's address because
        UDP is connectionless — each datagram can come from a different host.
        """
        if not self._recv_queue:
            return None
        return self._recv_queue.pop(0)

    def deliver(self, data: bytes, src_ip: int, src_port: int) -> None:
        """
        Deliver a received datagram to this socket.

        This simulates the kernel's job of demultiplexing incoming datagrams
        and routing them to the correct socket based on the destination port.

        Parameters
        ----------
        data : bytes
            The datagram payload.
        src_ip : int
            Source IP address (who sent this).
        src_port : int
            Source port number.
        """
        self._recv_queue.append((data, src_ip, src_port))

    @property
    def has_data(self) -> bool:
        """Return True if there are datagrams waiting to be read."""
        return len(self._recv_queue) > 0
