"""SharedWire -- a simulated network cable connecting multiple NICs.

==========================================================================
How Real Networks Work (Simplified)
==========================================================================

In early Ethernet (10BASE2, 10BASE5), all computers were literally connected
to the same coaxial cable. When one computer sent a packet, every other
computer on the cable received it. Each computer's NIC would check the
destination MAC address in the packet header and either accept or ignore it.

Modern Ethernet uses switches instead of shared cables, but the broadcast
model is still fundamental to how Ethernet works (ARP, DHCP, etc.).

Our SharedWire simulates the original shared-cable model: when one NIC
sends a packet, all other NICs on the same wire receive a copy. This is
the simplest network model and sufficient for learning about device drivers
and basic networking.

==========================================================================
Implementation
==========================================================================

The wire maintains a list of connected NICs. When broadcast() is called,
it iterates over all NICs and pushes the packet into each one's receive
queue -- except the sender (you don't hear your own voice echoing back
on a phone call, and a NIC doesn't receive its own packets).
"""

from __future__ import annotations

from collections import deque
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from device_driver_framework.simulated_nic import SimulatedNIC


class SharedWire:
    """A simulated network cable connecting multiple NICs.

    When one NIC sends a packet via this wire, all other connected NICs
    receive a copy. This models the shared-medium behavior of early
    Ethernet.

    Usage:
        wire = SharedWire()
        nic_a = SimulatedNIC("nic0", 0, mac_a, wire)
        nic_b = SimulatedNIC("nic1", 1, mac_b, wire)
        # nic_a and nic_b are now connected

        nic_a.send_packet(b"Hello!")
        # nic_b can now receive_packet() -> b"Hello!"
        # nic_a does NOT receive its own packet
    """

    def __init__(self) -> None:
        # List of all NICs connected to this wire.
        # In a real network, this would be the set of all devices
        # physically attached to the same cable or switch.
        self._nics: list[SimulatedNIC] = []

    def connect(self, nic: SimulatedNIC) -> None:
        """Connect a NIC to this wire.

        Called during NIC initialization. After connecting, the NIC will
        receive packets broadcast by other NICs on the same wire.
        """
        if nic not in self._nics:
            self._nics.append(nic)

    def disconnect(self, nic: SimulatedNIC) -> None:
        """Disconnect a NIC from this wire.

        After disconnecting, the NIC will no longer receive packets.
        """
        if nic in self._nics:
            self._nics.remove(nic)

    def broadcast(self, data: bytes, sender: SimulatedNIC) -> None:
        """Broadcast a packet to all connected NICs except the sender.

        This models how early Ethernet worked: a packet sent on the shared
        cable is visible to every device on the cable. Each NIC checks the
        destination address and decides whether to accept or ignore it.

        In our simplified model, we skip address filtering -- every NIC
        receives every packet. A more realistic model would check MAC
        addresses, but that's a concern for a higher-level network stack.

        Args:
            data: The raw packet bytes to broadcast.
            sender: The NIC that sent the packet (excluded from delivery).
        """
        for nic in self._nics:
            if nic is not sender:
                nic.rx_queue.append(data)

    @property
    def nic_count(self) -> int:
        """Return the number of NICs connected to this wire."""
        return len(self._nics)
