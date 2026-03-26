"""Tests for the UDP layer — header and socket."""

from network_stack.udp import UDPHeader, UDPSocket


class TestUDPHeader:
    """Tests for UDP header serialization and deserialization."""

    def test_serialize_and_deserialize_roundtrip(self) -> None:
        """A UDP header should survive a serialize -> deserialize roundtrip."""
        header = UDPHeader(
            src_port=12345,
            dst_port=53,
            length=42,
            checksum=0,
        )
        raw = header.serialize()
        recovered = UDPHeader.deserialize(raw)

        assert recovered.src_port == 12345
        assert recovered.dst_port == 53
        assert recovered.length == 42
        assert recovered.checksum == 0

    def test_serialize_length(self) -> None:
        """A UDP header should be exactly 8 bytes."""
        header = UDPHeader()
        assert len(header.serialize()) == 8

    def test_deserialize_too_short_raises(self) -> None:
        """Deserializing less than 8 bytes should raise ValueError."""
        try:
            UDPHeader.deserialize(b"\x00" * 7)
            assert False, "Expected ValueError"  # noqa: B011
        except ValueError as e:
            assert "too short" in str(e)

    def test_default_values(self) -> None:
        """Default header should have ports=0, length=8, checksum=0."""
        header = UDPHeader()
        assert header.src_port == 0
        assert header.dst_port == 0
        assert header.length == 8
        assert header.checksum == 0


class TestUDPSocket:
    """Tests for UDP socket send/receive operations."""

    def test_send_to(self) -> None:
        """send_to should create a header with correct ports and length."""
        sock = UDPSocket(local_port=12345)
        header, payload = sock.send_to(b"hello", dest_ip=0x08080808,
                                       dest_port=53)

        assert header.src_port == 12345
        assert header.dst_port == 53
        assert header.length == 8 + 5  # header + "hello"
        assert payload == b"hello"

    def test_deliver_and_receive(self) -> None:
        """deliver() should make data available via receive_from()."""
        sock = UDPSocket(local_port=53)
        sock.deliver(b"response", src_ip=0x08080808, src_port=12345)

        result = sock.receive_from()
        assert result is not None
        data, src_ip, src_port = result
        assert data == b"response"
        assert src_ip == 0x08080808
        assert src_port == 12345

    def test_receive_from_empty(self) -> None:
        """receive_from() on empty queue should return None."""
        sock = UDPSocket(local_port=53)
        assert sock.receive_from() is None

    def test_has_data(self) -> None:
        """has_data should reflect whether the receive queue has entries."""
        sock = UDPSocket(local_port=53)
        assert not sock.has_data

        sock.deliver(b"data", src_ip=0, src_port=0)
        assert sock.has_data

        sock.receive_from()
        assert not sock.has_data

    def test_multiple_datagrams(self) -> None:
        """Multiple datagrams should be received in FIFO order."""
        sock = UDPSocket(local_port=53)
        sock.deliver(b"first", src_ip=1, src_port=100)
        sock.deliver(b"second", src_ip=2, src_port=200)

        result1 = sock.receive_from()
        assert result1 is not None
        assert result1[0] == b"first"

        result2 = sock.receive_from()
        assert result2 is not None
        assert result2[0] == b"second"

        assert sock.receive_from() is None

    def test_send_to_empty_data(self) -> None:
        """send_to with empty data should still produce a valid header."""
        sock = UDPSocket(local_port=5000)
        header, payload = sock.send_to(b"", dest_ip=0, dest_port=80)
        assert header.length == 8  # header only, no payload
        assert payload == b""
