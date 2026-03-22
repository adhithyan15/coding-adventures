"""Tests for the Ethernet layer — frame serialization and ARP table."""

from network_stack.ethernet import ARPTable, EthernetFrame, ETHERTYPE_ARP, ETHERTYPE_IPV4


class TestEthernetFrame:
    """Tests for EthernetFrame serialization and deserialization."""

    def test_serialize_and_deserialize_roundtrip(self) -> None:
        """A frame should survive a serialize -> deserialize roundtrip."""
        frame = EthernetFrame(
            dest_mac=b"\xaa\xbb\xcc\xdd\xee\xff",
            src_mac=b"\x11\x22\x33\x44\x55\x66",
            ether_type=ETHERTYPE_IPV4,
            payload=b"Hello, Ethernet!",
        )
        raw = frame.serialize()
        recovered = EthernetFrame.deserialize(raw)

        assert recovered.dest_mac == frame.dest_mac
        assert recovered.src_mac == frame.src_mac
        assert recovered.ether_type == frame.ether_type
        assert recovered.payload == frame.payload

    def test_serialize_format(self) -> None:
        """Verify the exact byte layout of a serialized frame."""
        frame = EthernetFrame(
            dest_mac=b"\x01\x02\x03\x04\x05\x06",
            src_mac=b"\x0a\x0b\x0c\x0d\x0e\x0f",
            ether_type=ETHERTYPE_ARP,
            payload=b"\xde\xad",
        )
        raw = frame.serialize()

        # First 6 bytes: dest MAC
        assert raw[0:6] == b"\x01\x02\x03\x04\x05\x06"
        # Next 6 bytes: src MAC
        assert raw[6:12] == b"\x0a\x0b\x0c\x0d\x0e\x0f"
        # Next 2 bytes: EtherType (0x0806 big-endian)
        assert raw[12:14] == b"\x08\x06"
        # Remaining: payload
        assert raw[14:] == b"\xde\xad"

    def test_serialize_length(self) -> None:
        """Frame length should be 14 (header) + len(payload)."""
        payload = b"test data"
        frame = EthernetFrame(
            dest_mac=b"\x00" * 6,
            src_mac=b"\x00" * 6,
            ether_type=ETHERTYPE_IPV4,
            payload=payload,
        )
        assert len(frame.serialize()) == 14 + len(payload)

    def test_deserialize_empty_payload(self) -> None:
        """A frame with no payload should deserialize correctly."""
        frame = EthernetFrame(
            dest_mac=b"\xff" * 6,
            src_mac=b"\x00" * 6,
            ether_type=ETHERTYPE_ARP,
            payload=b"",
        )
        raw = frame.serialize()
        recovered = EthernetFrame.deserialize(raw)
        assert recovered.payload == b""

    def test_deserialize_too_short_raises(self) -> None:
        """Deserializing less than 14 bytes should raise ValueError."""
        try:
            EthernetFrame.deserialize(b"\x00" * 13)
            assert False, "Expected ValueError"  # noqa: B011
        except ValueError as e:
            assert "too short" in str(e)

    def test_broadcast_mac(self) -> None:
        """Broadcast frames use FF:FF:FF:FF:FF:FF as destination."""
        broadcast = b"\xff\xff\xff\xff\xff\xff"
        frame = EthernetFrame(
            dest_mac=broadcast,
            src_mac=b"\x11" * 6,
            ether_type=ETHERTYPE_ARP,
            payload=b"ARP request",
        )
        raw = frame.serialize()
        recovered = EthernetFrame.deserialize(raw)
        assert recovered.dest_mac == broadcast

    def test_different_ether_types(self) -> None:
        """EtherType field should be preserved for both IPv4 and ARP."""
        for etype in [ETHERTYPE_IPV4, ETHERTYPE_ARP]:
            frame = EthernetFrame(
                dest_mac=b"\x00" * 6,
                src_mac=b"\x00" * 6,
                ether_type=etype,
                payload=b"data",
            )
            recovered = EthernetFrame.deserialize(frame.serialize())
            assert recovered.ether_type == etype


class TestARPTable:
    """Tests for the ARP table (IP -> MAC mapping cache)."""

    def test_lookup_miss(self) -> None:
        """Looking up an unknown IP should return None."""
        table = ARPTable()
        assert table.lookup(0x0A000001) is None

    def test_update_and_lookup(self) -> None:
        """After updating, the MAC should be retrievable."""
        table = ARPTable()
        mac = b"\xaa\xbb\xcc\xdd\xee\xff"
        table.update(0x0A000001, mac)
        assert table.lookup(0x0A000001) == mac

    def test_update_overwrites(self) -> None:
        """Updating the same IP should overwrite the old MAC."""
        table = ARPTable()
        mac1 = b"\x11\x22\x33\x44\x55\x66"
        mac2 = b"\xaa\xbb\xcc\xdd\xee\xff"
        table.update(0x0A000001, mac1)
        table.update(0x0A000001, mac2)
        assert table.lookup(0x0A000001) == mac2

    def test_multiple_entries(self) -> None:
        """Multiple IPs should be tracked independently."""
        table = ARPTable()
        mac_a = b"\x11" * 6
        mac_b = b"\x22" * 6
        table.update(0x0A000001, mac_a)
        table.update(0x0A000002, mac_b)
        assert table.lookup(0x0A000001) == mac_a
        assert table.lookup(0x0A000002) == mac_b

    def test_entries_returns_copy(self) -> None:
        """entries() should return a copy, not the internal dict."""
        table = ARPTable()
        table.update(0x0A000001, b"\x11" * 6)
        entries = table.entries()
        assert len(entries) == 1
        # Modifying the returned dict should not affect the table
        entries[0x0A000099] = b"\xff" * 6
        assert table.lookup(0x0A000099) is None
