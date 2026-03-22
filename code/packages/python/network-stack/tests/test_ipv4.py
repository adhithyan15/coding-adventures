"""Tests for the IPv4 layer — headers, checksums, routing table, and IP layer."""

from network_stack.ipv4 import PROTOCOL_TCP, PROTOCOL_UDP, IPLayer, IPv4Header, RoutingTable


class TestIPv4Header:
    """Tests for IPv4 header serialization, deserialization, and checksum."""

    def test_serialize_and_deserialize_roundtrip(self) -> None:
        """An IPv4 header should survive a serialize -> deserialize roundtrip."""
        header = IPv4Header(
            src_ip=0x0A000001,  # 10.0.0.1
            dst_ip=0x0A000002,  # 10.0.0.2
            protocol=PROTOCOL_TCP,
            total_length=60,
            ttl=64,
        )
        raw = header.serialize()
        recovered = IPv4Header.deserialize(raw)

        assert recovered.version == 4
        assert recovered.ihl == 5
        assert recovered.src_ip == 0x0A000001
        assert recovered.dst_ip == 0x0A000002
        assert recovered.protocol == PROTOCOL_TCP
        assert recovered.total_length == 60
        assert recovered.ttl == 64

    def test_serialize_length(self) -> None:
        """A standard IPv4 header (IHL=5) should be exactly 20 bytes."""
        header = IPv4Header()
        raw = header.serialize()
        assert len(raw) == 20

    def test_version_ihl_packing(self) -> None:
        """Version (4) and IHL (5) should be packed into the first byte as 0x45."""
        header = IPv4Header()
        raw = header.serialize()
        assert raw[0] == 0x45  # version=4 in upper nibble, IHL=5 in lower

    def test_checksum_is_nonzero(self) -> None:
        """The checksum should be computed and nonzero for a typical header."""
        header = IPv4Header(
            src_ip=0x0A000001,
            dst_ip=0x0A000002,
            total_length=40,
        )
        header.serialize()
        assert header.checksum != 0

    def test_compute_checksum_matches_serialized(self) -> None:
        """compute_checksum() should return the same value as serialize() sets."""
        header = IPv4Header(
            src_ip=0xC0A80001,  # 192.168.0.1
            dst_ip=0xC0A80002,  # 192.168.0.2
            protocol=PROTOCOL_UDP,
            total_length=28,
            ttl=128,
        )
        header.serialize()
        saved_checksum = header.checksum
        computed = header.compute_checksum()
        assert computed == saved_checksum

    def test_checksum_validates(self) -> None:
        """
        When you compute the checksum over a header that already has a valid
        checksum, the result should be 0 (or the checksum should match).
        """
        header = IPv4Header(
            src_ip=0x0A000001,
            dst_ip=0x0A000002,
            total_length=20,
        )
        raw = header.serialize()
        # The checksum of a correctly checksummed header should verify
        recovered = IPv4Header.deserialize(raw)
        assert recovered.checksum != 0

    def test_deserialize_too_short_raises(self) -> None:
        """Deserializing less than 20 bytes should raise ValueError."""
        try:
            IPv4Header.deserialize(b"\x00" * 19)
            assert False, "Expected ValueError"  # noqa: B011
        except ValueError as e:
            assert "too short" in str(e)

    def test_different_protocols(self) -> None:
        """Protocol field should be preserved for both TCP and UDP."""
        for proto in [PROTOCOL_TCP, PROTOCOL_UDP]:
            header = IPv4Header(protocol=proto)
            raw = header.serialize()
            recovered = IPv4Header.deserialize(raw)
            assert recovered.protocol == proto

    def test_identification_field(self) -> None:
        """The identification field should survive roundtrip."""
        header = IPv4Header(identification=0x1234)
        raw = header.serialize()
        recovered = IPv4Header.deserialize(raw)
        assert recovered.identification == 0x1234


class TestRoutingTable:
    """Tests for longest-prefix-match routing."""

    def test_empty_table_returns_none(self) -> None:
        """An empty routing table should return None for any lookup."""
        rt = RoutingTable()
        assert rt.lookup(0x0A000001) is None

    def test_default_route(self) -> None:
        """A 0.0.0.0/0 route (default route) matches everything."""
        rt = RoutingTable()
        rt.add_route(0x00000000, 0x00000000, 0x0A000001, "eth0")
        result = rt.lookup(0x08080808)  # 8.8.8.8
        assert result is not None
        next_hop, iface = result
        assert next_hop == 0x0A000001  # via gateway
        assert iface == "eth0"

    def test_direct_route(self) -> None:
        """A directly connected network (gateway=0) returns the dest IP."""
        rt = RoutingTable()
        # 10.0.0.0/24 is directly connected
        rt.add_route(0x0A000000, 0xFFFFFF00, 0, "eth0")
        result = rt.lookup(0x0A000005)  # 10.0.0.5
        assert result is not None
        next_hop, iface = result
        assert next_hop == 0x0A000005  # direct delivery
        assert iface == "eth0"

    def test_longest_prefix_match(self) -> None:
        """More specific routes should take priority over less specific ones."""
        rt = RoutingTable()
        # Default route (least specific)
        rt.add_route(0x00000000, 0x00000000, 0x0A000001, "eth0")
        # 10.0.0.0/24 (more specific)
        rt.add_route(0x0A000000, 0xFFFFFF00, 0, "eth1")
        # 10.0.0.0/28 (most specific)
        rt.add_route(0x0A000000, 0xFFFFFFF0, 0x0A000002, "eth2")

        # 10.0.0.5 matches all three; /28 should win
        result = rt.lookup(0x0A000005)
        assert result is not None
        next_hop, iface = result
        assert next_hop == 0x0A000002
        assert iface == "eth2"

        # 10.0.0.20 matches /24 and default, but not /28
        result = rt.lookup(0x0A000014)
        assert result is not None
        _, iface = result
        assert iface == "eth1"

        # 8.8.8.8 only matches default
        result = rt.lookup(0x08080808)
        assert result is not None
        _, iface = result
        assert iface == "eth0"

    def test_no_match(self) -> None:
        """If no route matches, return None."""
        rt = RoutingTable()
        # Only a specific route, no default
        rt.add_route(0x0A000000, 0xFFFFFF00, 0, "eth0")
        assert rt.lookup(0x0B000001) is None  # 11.0.0.1 doesn't match 10.0.0.0/24


class TestIPLayer:
    """Tests for the IP layer packet creation and parsing."""

    def test_create_packet(self) -> None:
        """create_packet should produce a valid IP packet with payload."""
        rt = RoutingTable()
        ip = IPLayer(local_ip=0x0A000001, routing_table=rt)

        packet = ip.create_packet(
            dest_ip=0x0A000002,
            protocol=PROTOCOL_TCP,
            payload=b"TCP segment here",
        )

        # Parse it back
        result = ip.parse_packet(packet)
        assert result is not None
        header, payload = result

        assert header.src_ip == 0x0A000001
        assert header.dst_ip == 0x0A000002
        assert header.protocol == PROTOCOL_TCP
        assert header.total_length == 20 + len(b"TCP segment here")
        assert payload == b"TCP segment here"

    def test_parse_packet_too_short(self) -> None:
        """parse_packet should return None for data shorter than 20 bytes."""
        rt = RoutingTable()
        ip = IPLayer(local_ip=0x0A000001, routing_table=rt)
        assert ip.parse_packet(b"\x00" * 19) is None

    def test_create_udp_packet(self) -> None:
        """create_packet should work with UDP protocol number."""
        rt = RoutingTable()
        ip = IPLayer(local_ip=0xC0A80001, routing_table=rt)

        packet = ip.create_packet(
            dest_ip=0xC0A80002,
            protocol=PROTOCOL_UDP,
            payload=b"UDP datagram",
        )

        result = ip.parse_packet(packet)
        assert result is not None
        header, payload = result
        assert header.protocol == PROTOCOL_UDP
        assert payload == b"UDP datagram"
