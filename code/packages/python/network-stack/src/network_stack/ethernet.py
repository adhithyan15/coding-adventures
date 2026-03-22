"""
Ethernet — Layer 2 (Data Link)
==============================

Ethernet is the protocol that delivers frames between devices on the *same*
local network. Think of it as the local mail carrier: it only delivers between
neighboring houses on the same street. If a letter needs to go to another city,
the carrier hands it to the post office (the IP layer), which handles routing.

Frame Format
------------

Every Ethernet frame has this structure on the wire::

    +-----------+-----------+------------+---------+
    | Dest MAC  | Src MAC   | EtherType  | Payload |
    | (6 bytes) | (6 bytes) | (2 bytes)  | (var)   |
    +-----------+-----------+------------+---------+

- **Dest MAC**: The 6-byte hardware address of the intended recipient.
  The special address FF:FF:FF:FF:FF:FF is a *broadcast* — every device
  on the network reads it.
- **Src MAC**: The sender's hardware address.
- **EtherType**: A 2-byte field that tells the receiver what protocol the
  payload contains. 0x0800 = IPv4, 0x0806 = ARP.
- **Payload**: The actual data (an IP packet, an ARP message, etc.).

MAC Addresses
-------------

Every network interface card (NIC) has a unique 48-bit (6-byte) address burned
in at the factory. MAC addresses are Layer 2 identifiers — they only matter on
the local network segment. When a packet crosses a router, the MAC addresses
change at each hop, but the IP addresses stay the same.

ARP (Address Resolution Protocol)
----------------------------------

ARP bridges the gap between IP addresses (Layer 3) and MAC addresses (Layer 2).
When a host wants to send to IP 10.0.0.5 but doesn't know the MAC address, it
broadcasts an ARP request: "Who has 10.0.0.5?" The owner of that IP replies
with its MAC address.

The ARP table caches these mappings so we don't broadcast for every packet.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass, field

# ============================================================================
# EtherType Constants
# ============================================================================
# These 2-byte values in the Ethernet header tell the receiver how to interpret
# the payload. They are assigned by the IEEE.

ETHERTYPE_IPV4 = 0x0800  # Payload is an IPv4 packet
ETHERTYPE_ARP = 0x0806   # Payload is an ARP message


# ============================================================================
# EthernetFrame
# ============================================================================

@dataclass
class EthernetFrame:
    """
    An Ethernet frame — the fundamental unit of data on a local network.

    Attributes
    ----------
    dest_mac : bytes
        6-byte destination MAC address. Use b'\\xff\\xff\\xff\\xff\\xff\\xff'
        for broadcast.
    src_mac : bytes
        6-byte source MAC address.
    ether_type : int
        Protocol identifier (0x0800 for IPv4, 0x0806 for ARP).
    payload : bytes
        The data carried by this frame.

    Example
    -------
    >>> frame = EthernetFrame(
    ...     dest_mac=b'\\xaa\\xbb\\xcc\\xdd\\xee\\xff',
    ...     src_mac=b'\\x11\\x22\\x33\\x44\\x55\\x66',
    ...     ether_type=ETHERTYPE_IPV4,
    ...     payload=b'Hello, network!'
    ... )
    >>> raw = frame.serialize()
    >>> recovered = EthernetFrame.deserialize(raw)
    >>> recovered.payload
    b'Hello, network!'
    """

    dest_mac: bytes = field(default_factory=lambda: b"\x00" * 6)
    src_mac: bytes = field(default_factory=lambda: b"\x00" * 6)
    ether_type: int = ETHERTYPE_IPV4
    payload: bytes = b""

    def serialize(self) -> bytes:
        """
        Convert this frame to raw bytes for transmission.

        The wire format is simply the fields concatenated in order::

            [dest_mac: 6][src_mac: 6][ether_type: 2][payload: N]

        We use struct.pack for the 2-byte ether_type (big-endian, as network
        protocols require — this is called "network byte order").
        """
        return (
            self.dest_mac
            + self.src_mac
            + struct.pack("!H", self.ether_type)
            + self.payload
        )

    @classmethod
    def deserialize(cls, data: bytes) -> EthernetFrame:
        """
        Parse raw bytes from the wire into an EthernetFrame.

        The minimum frame size is 14 bytes (6 + 6 + 2, with empty payload).
        Everything after byte 14 is the payload.

        Parameters
        ----------
        data : bytes
            Raw bytes received from the wire.

        Returns
        -------
        EthernetFrame
            The parsed frame.

        Raises
        ------
        ValueError
            If the data is too short to be a valid Ethernet frame.
        """
        if len(data) < 14:
            msg = f"Ethernet frame too short: {len(data)} bytes (minimum 14)"
            raise ValueError(msg)

        dest_mac = data[0:6]
        src_mac = data[6:12]
        (ether_type,) = struct.unpack("!H", data[12:14])
        payload = data[14:]

        return cls(
            dest_mac=dest_mac,
            src_mac=src_mac,
            ether_type=ether_type,
            payload=payload,
        )


# ============================================================================
# ARPTable
# ============================================================================

class ARPTable:
    """
    Maps IP addresses to MAC addresses.

    This is the ARP cache — a simple dictionary that remembers which MAC
    address corresponds to which IP address. In a real OS, entries expire
    after a timeout (typically 20 minutes). Our simulation keeps entries
    forever.

    IP addresses are stored as 32-bit integers for efficient comparison.
    For example, 10.0.0.1 is stored as 0x0A000001 (167772161 in decimal).

    Example
    -------
    >>> table = ARPTable()
    >>> table.update(0x0A000001, b'\\xaa\\xbb\\xcc\\xdd\\xee\\xff')
    >>> table.lookup(0x0A000001)
    b'\\xaa\\xbb\\xcc\\xdd\\xee\\xff'
    >>> table.lookup(0x0A000002) is None
    True
    """

    def __init__(self) -> None:
        # Maps IP (as 32-bit int) to MAC (as 6-byte bytes)
        self._entries: dict[int, bytes] = {}

    def lookup(self, ip: int) -> bytes | None:
        """
        Look up the MAC address for the given IP.

        Returns None if the IP is not in the table — this means we would
        need to send an ARP request to discover the MAC address.
        """
        return self._entries.get(ip)

    def update(self, ip: int, mac: bytes) -> None:
        """
        Add or update a mapping from IP to MAC.

        This is called when we receive an ARP reply, or when we see any
        packet that reveals a (source IP, source MAC) pair.
        """
        self._entries[ip] = mac

    def entries(self) -> dict[int, bytes]:
        """Return a copy of all entries in the ARP table."""
        return dict(self._entries)
