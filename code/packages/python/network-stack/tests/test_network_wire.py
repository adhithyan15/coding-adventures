"""Tests for the NetworkWire — simulated physical medium."""

from network_stack.network_wire import NetworkWire


class TestNetworkWire:
    """Tests for bidirectional packet delivery."""

    def test_send_a_receive_b(self) -> None:
        """Frames sent by A should be received by B."""
        wire = NetworkWire()
        wire.send_a(b"Hello from A")
        assert wire.has_data_for_b()
        assert wire.receive_b() == b"Hello from A"

    def test_send_b_receive_a(self) -> None:
        """Frames sent by B should be received by A."""
        wire = NetworkWire()
        wire.send_b(b"Hello from B")
        assert wire.has_data_for_a()
        assert wire.receive_a() == b"Hello from B"

    def test_empty_receive_returns_none(self) -> None:
        """Receiving from an empty queue should return None."""
        wire = NetworkWire()
        assert wire.receive_a() is None
        assert wire.receive_b() is None

    def test_has_data_initially_false(self) -> None:
        """A new wire should have no data waiting."""
        wire = NetworkWire()
        assert not wire.has_data_for_a()
        assert not wire.has_data_for_b()

    def test_fifo_order(self) -> None:
        """Multiple frames should be delivered in FIFO order."""
        wire = NetworkWire()
        wire.send_a(b"first")
        wire.send_a(b"second")
        wire.send_a(b"third")

        assert wire.receive_b() == b"first"
        assert wire.receive_b() == b"second"
        assert wire.receive_b() == b"third"
        assert wire.receive_b() is None

    def test_full_duplex(self) -> None:
        """Both sides should be able to send simultaneously."""
        wire = NetworkWire()
        wire.send_a(b"from A")
        wire.send_b(b"from B")

        assert wire.receive_b() == b"from A"
        assert wire.receive_a() == b"from B"

    def test_independent_queues(self) -> None:
        """A-to-B and B-to-A queues should be independent."""
        wire = NetworkWire()
        wire.send_a(b"data")

        # B has data, A does not
        assert wire.has_data_for_b()
        assert not wire.has_data_for_a()

        # Receiving from A's queue should not affect B's
        assert wire.receive_a() is None
        assert wire.has_data_for_b()

    def test_receive_clears_queue(self) -> None:
        """After receiving, the queue should be empty."""
        wire = NetworkWire()
        wire.send_a(b"data")
        wire.receive_b()
        assert not wire.has_data_for_b()

    def test_large_frame(self) -> None:
        """Large frames should be transmitted correctly."""
        wire = NetworkWire()
        large_data = b"\xaa" * 10000
        wire.send_a(large_data)
        assert wire.receive_b() == large_data
