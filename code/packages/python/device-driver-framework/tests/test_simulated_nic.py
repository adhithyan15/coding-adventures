"""Tests for SimulatedNIC and SharedWire."""

from device_driver_framework.device import DeviceType
from device_driver_framework.shared_wire import SharedWire
from device_driver_framework.simulated_nic import SimulatedNIC


class TestSharedWire:
    """Verify SharedWire connect, disconnect, and broadcast behavior."""

    def test_connect(self) -> None:
        """Connecting a NIC should increase nic_count."""
        wire = SharedWire()
        nic = SimulatedNIC(wire=wire)
        wire.connect(nic)
        assert wire.nic_count == 1

    def test_connect_idempotent(self) -> None:
        """Connecting the same NIC twice should not duplicate it."""
        wire = SharedWire()
        nic = SimulatedNIC(wire=wire)
        wire.connect(nic)
        wire.connect(nic)
        assert wire.nic_count == 1

    def test_disconnect(self) -> None:
        """Disconnecting should remove the NIC."""
        wire = SharedWire()
        nic = SimulatedNIC(wire=wire)
        wire.connect(nic)
        wire.disconnect(nic)
        assert wire.nic_count == 0

    def test_disconnect_nonexistent(self) -> None:
        """Disconnecting a NIC that isn't connected should be a no-op."""
        wire = SharedWire()
        nic = SimulatedNIC(wire=wire)
        wire.disconnect(nic)  # Should not raise
        assert wire.nic_count == 0

    def test_broadcast_delivers_to_others(self) -> None:
        """broadcast() should deliver to all connected NICs except sender."""
        wire = SharedWire()
        nic_a = SimulatedNIC(name="nic0", minor=0, wire=wire)
        nic_b = SimulatedNIC(name="nic1", minor=1, wire=wire)
        wire.connect(nic_a)
        wire.connect(nic_b)

        wire.broadcast(b"test packet", sender=nic_a)
        assert len(nic_b.rx_queue) == 1
        assert nic_b.rx_queue[0] == b"test packet"
        assert len(nic_a.rx_queue) == 0  # Sender doesn't get its own packet

    def test_broadcast_to_multiple(self) -> None:
        """broadcast() should deliver to ALL other NICs."""
        wire = SharedWire()
        nic_a = SimulatedNIC(name="nic0", minor=0, wire=wire)
        nic_b = SimulatedNIC(name="nic1", minor=1, wire=wire)
        nic_c = SimulatedNIC(name="nic2", minor=2, wire=wire)
        wire.connect(nic_a)
        wire.connect(nic_b)
        wire.connect(nic_c)

        wire.broadcast(b"hello", sender=nic_a)
        assert len(nic_b.rx_queue) == 1
        assert len(nic_c.rx_queue) == 1
        assert len(nic_a.rx_queue) == 0


class TestSimulatedNIC:
    """Verify SimulatedNIC send/receive and configuration."""

    def test_default_configuration(self) -> None:
        """Default NIC should have correct name, major, minor, irq."""
        nic = SimulatedNIC()
        assert nic.name == "nic0"
        assert nic.device_type == DeviceType.NETWORK
        assert nic.major == 4
        assert nic.minor == 0
        assert nic.interrupt_number == 35
        assert len(nic.mac_address) == 6

    def test_mac_address(self) -> None:
        """Custom MAC address should be stored correctly."""
        mac = b"\x01\x02\x03\x04\x05\x06"
        nic = SimulatedNIC(mac_address=mac)
        assert nic.mac_address == mac

    def test_receive_packet_empty(self) -> None:
        """receive_packet() with empty queue should return None."""
        nic = SimulatedNIC()
        assert nic.receive_packet() is None

    def test_has_packet_empty(self) -> None:
        """has_packet() with empty queue should return False."""
        nic = SimulatedNIC()
        assert nic.has_packet() is False

    def test_send_and_receive(self) -> None:
        """Sending from NIC A should be receivable by NIC B on same wire."""
        wire = SharedWire()
        nic_a = SimulatedNIC(
            name="nic0", minor=0,
            mac_address=b"\xDE\xAD\xBE\xEF\x00\x01",
            wire=wire,
        )
        nic_b = SimulatedNIC(
            name="nic1", minor=1,
            mac_address=b"\xDE\xAD\xBE\xEF\x00\x02",
            wire=wire,
        )
        nic_a.init()
        nic_b.init()

        sent = nic_a.send_packet(b"Hello from A!")
        assert sent == len(b"Hello from A!")
        assert nic_b.has_packet() is True
        packet = nic_b.receive_packet()
        assert packet == b"Hello from A!"

    def test_sender_does_not_receive_own_packet(self) -> None:
        """A NIC should NOT receive its own sent packet."""
        wire = SharedWire()
        nic_a = SimulatedNIC(name="nic0", minor=0, wire=wire)
        nic_b = SimulatedNIC(name="nic1", minor=1, wire=wire)
        nic_a.init()
        nic_b.init()

        nic_a.send_packet(b"echo?")
        assert nic_a.has_packet() is False
        assert nic_a.receive_packet() is None

    def test_send_without_wire(self) -> None:
        """Sending without a wire should return -1."""
        nic = SimulatedNIC(wire=None)
        assert nic.send_packet(b"data") == -1

    def test_init_clears_queue_and_connects(self) -> None:
        """init() should clear rx_queue and connect to the wire."""
        wire = SharedWire()
        nic = SimulatedNIC(wire=wire)
        nic.rx_queue.append(b"stale packet")
        nic.init()
        assert nic.initialized is True
        assert len(nic.rx_queue) == 0
        assert wire.nic_count == 1

    def test_multiple_packets_fifo(self) -> None:
        """Multiple received packets should be returned in FIFO order."""
        wire = SharedWire()
        nic_a = SimulatedNIC(name="nic0", minor=0, wire=wire)
        nic_b = SimulatedNIC(name="nic1", minor=1, wire=wire)
        nic_a.init()
        nic_b.init()

        nic_a.send_packet(b"first")
        nic_a.send_packet(b"second")
        nic_a.send_packet(b"third")

        assert nic_b.receive_packet() == b"first"
        assert nic_b.receive_packet() == b"second"
        assert nic_b.receive_packet() == b"third"
        assert nic_b.receive_packet() is None

    def test_bidirectional_communication(self) -> None:
        """Both NICs should be able to send and receive."""
        wire = SharedWire()
        nic_a = SimulatedNIC(name="nic0", minor=0, wire=wire)
        nic_b = SimulatedNIC(name="nic1", minor=1, wire=wire)
        nic_a.init()
        nic_b.init()

        nic_a.send_packet(b"ping")
        assert nic_b.receive_packet() == b"ping"

        nic_b.send_packet(b"pong")
        assert nic_a.receive_packet() == b"pong"

    def test_wire_property(self) -> None:
        """The wire property should return the SharedWire."""
        wire = SharedWire()
        nic = SimulatedNIC(wire=wire)
        assert nic.wire is wire

    def test_init_without_wire(self) -> None:
        """init() without a wire should still work (just sets initialized)."""
        nic = SimulatedNIC(wire=None)
        nic.init()
        assert nic.initialized is True
