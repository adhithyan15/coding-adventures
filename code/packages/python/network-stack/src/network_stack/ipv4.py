"""
IPv4 — Layer 3 (Network)
========================

IP (Internet Protocol) is the routing layer. While Ethernet delivers frames
between devices on the same local network, IP delivers packets across networks
— from your laptop to a server on the other side of the world.

How IP Routing Works
--------------------

Imagine you're sending a letter from New York to Tokyo. The letter doesn't
fly directly — it goes through a chain of post offices:

    Your mailbox -> Local post office -> Regional hub -> Airport ->
    Tokyo airport -> Tokyo regional hub -> Tokyo local office -> Recipient

At each hop, the postal worker looks at the destination address and decides
"which direction should I send this?" That's routing.

IP works the same way. Each router has a **routing table** — a list of rules
like "if the destination is in the 10.0.0.0/24 network, send it out
interface eth0 to gateway 10.0.0.1."

IPv4 Header Format
------------------

The IPv4 header is 20 bytes (minimum, with no options)::

    0                   1                   2                   3
    0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
   |Version|  IHL  |    (unused)   |         Total Length          |
   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
   |         Identification        |   (flags/frag, unused here)  |
   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
   |  TTL  |    Protocol   |       Header Checksum                |
   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
   |                    Source IP Address                          |
   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
   |                 Destination IP Address                        |
   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

The Checksum Algorithm
----------------------

The IP checksum is a simple error-detection mechanism:

1. Treat the header as a sequence of 16-bit words
2. Sum them all up (with the checksum field set to 0)
3. Add any carry bits back into the lower 16 bits
4. Take the one's complement (flip all bits)

This catches most single-bit errors and many multi-bit errors, though it's
not as robust as CRC. TCP and UDP have their own checksums for the payload.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass

# Protocol numbers — these go in the IPv4 header's "protocol" field to tell
# the receiver which Layer 4 protocol the payload contains.
PROTOCOL_TCP = 6
PROTOCOL_UDP = 17


@dataclass
class IPv4Header:
    """
    An IPv4 packet header — 20 bytes of routing and control information.

    Attributes
    ----------
    version : int
        IP version, always 4 for IPv4.
    ihl : int
        Internet Header Length in 32-bit words. Minimum 5 (= 20 bytes).
    total_length : int
        Total packet size (header + payload) in bytes.
    ttl : int
        Time To Live. Decremented at each router; packet is discarded when
        TTL reaches 0. Prevents infinite routing loops.
    protocol : int
        Layer 4 protocol: 6 = TCP, 17 = UDP.
    src_ip : int
        Source IP address as a 32-bit integer.
        Example: 10.0.0.1 = 0x0A000001
    dst_ip : int
        Destination IP address as a 32-bit integer.
    checksum : int
        Header checksum for error detection.
    identification : int
        Used for fragment reassembly (not implemented here).
    """

    version: int = 4
    ihl: int = 5
    total_length: int = 20
    ttl: int = 64
    protocol: int = PROTOCOL_TCP
    src_ip: int = 0
    dst_ip: int = 0
    checksum: int = 0
    identification: int = 0

    def serialize(self) -> bytes:
        """
        Convert the header to 20 raw bytes in network byte order.

        The first byte packs version (4 bits) and IHL (4 bits) together.
        We set the checksum to 0, serialize, compute the checksum over
        the result, then patch it in.

        Layout::

            Byte 0:    version(4) | ihl(4)
            Byte 1:    0 (type of service, unused)
            Bytes 2-3: total_length
            Bytes 4-5: identification
            Bytes 6-7: 0 (flags/fragment offset, unused)
            Byte 8:    ttl
            Byte 9:    protocol
            Bytes 10-11: checksum (computed after packing)
            Bytes 12-15: src_ip
            Bytes 16-19: dst_ip
        """
        # Pack version and IHL into a single byte:
        #   version occupies the upper 4 bits, IHL the lower 4.
        version_ihl = (self.version << 4) | self.ihl

        # First pass: pack with checksum = 0
        header = struct.pack(
            "!BBHHHBBHII",
            version_ihl,        # B: version + IHL
            0,                  # B: type of service (unused)
            self.total_length,  # H: total length
            self.identification,  # H: identification
            0,                  # H: flags + fragment offset (unused)
            self.ttl,           # B: time to live
            self.protocol,      # B: protocol
            0,                  # H: checksum (placeholder)
            self.src_ip,        # I: source IP
            self.dst_ip,        # I: destination IP
        )

        # Compute checksum over the header with checksum field = 0
        computed = self._compute_checksum_over(header)

        # Patch the checksum into bytes 10-11
        header = header[:10] + struct.pack("!H", computed) + header[12:]
        self.checksum = computed

        return header

    @classmethod
    def deserialize(cls, data: bytes) -> IPv4Header:
        """
        Parse 20 bytes of raw data into an IPv4Header.

        Raises ValueError if the data is too short.
        """
        if len(data) < 20:
            msg = f"IPv4 header too short: {len(data)} bytes (minimum 20)"
            raise ValueError(msg)

        (
            version_ihl,
            _tos,
            total_length,
            identification,
            _flags_frag,
            ttl,
            protocol,
            checksum,
            src_ip,
            dst_ip,
        ) = struct.unpack("!BBHHHBBHII", data[:20])

        version = (version_ihl >> 4) & 0x0F
        ihl = version_ihl & 0x0F

        return cls(
            version=version,
            ihl=ihl,
            total_length=total_length,
            ttl=ttl,
            protocol=protocol,
            src_ip=src_ip,
            dst_ip=dst_ip,
            checksum=checksum,
            identification=identification,
        )

    def compute_checksum(self) -> int:
        """
        Compute the header checksum.

        The algorithm (RFC 1071):
        1. Set the checksum field to 0
        2. Treat the header as a series of 16-bit integers
        3. Sum them all (using 32-bit arithmetic to capture carries)
        4. Fold carries: add the high 16 bits to the low 16 bits, repeat
        5. Take the one's complement (bitwise NOT, masked to 16 bits)

        This is the same algorithm used by TCP and UDP, though they
        also include a pseudo-header with IP addresses.
        """
        # Serialize with checksum = 0
        saved = self.checksum
        self.checksum = 0
        header = struct.pack(
            "!BBHHHBBHII",
            (self.version << 4) | self.ihl,
            0,
            self.total_length,
            self.identification,
            0,
            self.ttl,
            self.protocol,
            0,
            self.src_ip,
            self.dst_ip,
        )
        self.checksum = saved
        return self._compute_checksum_over(header)

    @staticmethod
    def _compute_checksum_over(data: bytes) -> int:
        """
        Compute the Internet checksum over arbitrary data.

        This is a building block used by IP, TCP, and UDP checksums.

        The algorithm treats the data as a sequence of 16-bit big-endian
        integers, sums them with carry folding, and returns the one's
        complement.
        """
        # Step 1: Sum all 16-bit words
        total = 0
        for i in range(0, len(data), 2):
            if i + 1 < len(data):
                word = (data[i] << 8) | data[i + 1]
            else:
                # If odd number of bytes, pad with zero
                word = data[i] << 8
            total += word

        # Step 2: Fold carries — add high 16 bits to low 16 bits
        while total > 0xFFFF:
            total = (total >> 16) + (total & 0xFFFF)

        # Step 3: One's complement
        return (~total) & 0xFFFF


# ============================================================================
# RoutingTable
# ============================================================================

class RoutingTable:
    """
    A simple IP routing table using longest-prefix matching.

    How Routing Tables Work
    -----------------------

    A routing table is a list of rules, each saying:

        "If the destination IP matches network/mask, send it to gateway
         via interface."

    When multiple rules match, the one with the longest prefix (most specific
    network mask) wins. This is called **longest-prefix matching**.

    Example::

        Network        Mask             Gateway      Interface
        10.0.0.0       255.255.255.0    0.0.0.0      eth0    (direct)
        10.0.1.0       255.255.255.0    10.0.0.1     eth0    (via router)
        0.0.0.0        0.0.0.0          10.0.0.1     eth0    (default route)

    A packet to 10.0.0.5 matches the first rule (10.0.0.0/24) and is
    delivered directly on eth0. A packet to 8.8.8.8 only matches the
    default route (0.0.0.0/0) and is forwarded to the gateway 10.0.0.1.
    """

    def __init__(self) -> None:
        # Each route: (network, mask, gateway, interface)
        # network and mask are 32-bit ints, gateway is 32-bit int,
        # interface is a string name.
        self._routes: list[tuple[int, int, int, str]] = []

    def add_route(
        self, network: int, mask: int, gateway: int, interface: str
    ) -> None:
        """
        Add a route to the table.

        Parameters
        ----------
        network : int
            Network address (e.g., 0x0A000000 for 10.0.0.0).
        mask : int
            Subnet mask (e.g., 0xFFFFFF00 for /24).
        gateway : int
            Next-hop router IP. Use 0 for directly connected networks.
        interface : str
            Name of the outgoing interface (e.g., "eth0").
        """
        self._routes.append((network, mask, gateway, interface))

    def lookup(self, dest_ip: int) -> tuple[int, str] | None:
        """
        Find the best route for a destination IP using longest-prefix match.

        Returns (next_hop_ip, interface) or None if no route matches.
        The next_hop is the gateway IP if nonzero, otherwise the dest_ip
        itself (meaning the destination is directly connected).

        Longest-prefix matching: we pick the route with the most 1-bits
        in the mask. If the mask is 255.255.255.0 (24 one-bits), it's more
        specific than 255.255.0.0 (16 one-bits), so it wins.
        """
        best_match: tuple[int, str] | None = None
        best_mask = -1  # Track the longest prefix seen

        for network, mask, gateway, interface in self._routes:
            # Does the destination match this route?
            # Apply the mask to the destination and compare with the network.
            if (dest_ip & mask) == (network & mask):
                # Count the mask bits to determine specificity
                mask_bits = bin(mask).count("1")
                if mask_bits > best_mask:
                    best_mask = mask_bits
                    # If gateway is 0, the host is directly connected
                    next_hop = gateway if gateway != 0 else dest_ip
                    best_match = (next_hop, interface)

        return best_match


# ============================================================================
# IPLayer
# ============================================================================

class IPLayer:
    """
    The IP layer — creates outgoing packets and parses incoming ones.

    This is the glue between the transport layer (TCP/UDP) and the data link
    layer (Ethernet). When TCP wants to send a segment, it hands the data to
    IPLayer, which wraps it in an IPv4 header with source and destination
    addresses and passes it down to Ethernet.

    Parameters
    ----------
    local_ip : int
        This host's IP address (32-bit int).
    routing_table : RoutingTable
        The routing table used to determine next-hop for outgoing packets.
    """

    def __init__(self, local_ip: int, routing_table: RoutingTable) -> None:
        self.local_ip = local_ip
        self.routing_table = routing_table

    def create_packet(self, dest_ip: int, protocol: int, payload: bytes) -> bytes:
        """
        Create an IP packet ready for transmission.

        Builds an IPv4 header, sets the total_length to header + payload,
        computes the checksum, and returns the complete packet bytes.
        """
        header = IPv4Header(
            src_ip=self.local_ip,
            dst_ip=dest_ip,
            protocol=protocol,
            total_length=20 + len(payload),
            ttl=64,
        )
        return header.serialize() + payload

    def parse_packet(self, data: bytes) -> tuple[IPv4Header, bytes] | None:
        """
        Parse a received IP packet into header and payload.

        Returns None if the packet is too short to be valid.
        The payload starts at byte offset (IHL * 4) — usually byte 20.
        """
        if len(data) < 20:
            return None

        header = IPv4Header.deserialize(data)
        payload_offset = header.ihl * 4
        payload = data[payload_offset:]

        return (header, payload)
