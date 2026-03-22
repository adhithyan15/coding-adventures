"""SimulatedNIC -- a network interface card backed by in-memory packet queues.

==========================================================================
How Network Cards Work
==========================================================================

A Network Interface Card (NIC) is the hardware that connects a computer
to a network. When you plug in an Ethernet cable or connect to WiFi,
you are using a NIC.

Every NIC has a MAC address -- a 6-byte unique identifier assigned at
the factory. Think of it as a mailing address: when another computer
sends you a packet, it addresses the packet to your MAC address.

  MAC address format: XX:XX:XX:XX:XX:XX
  Example: DE:AD:BE:EF:00:01

  The first 3 bytes identify the manufacturer (OUI).
  The last 3 bytes identify the specific device.

==========================================================================
Sending and Receiving Packets
==========================================================================

Sending a packet:
  1. The OS kernel places packet data in a transmit buffer.
  2. The NIC reads the buffer and converts the data into electrical
     signals on the wire (or radio signals for WiFi).
  3. The NIC raises an interrupt to tell the kernel "transmission done."

Receiving a packet:
  1. The NIC detects electrical signals on the wire.
  2. It decodes them into packet data and places it in a receive buffer.
  3. The NIC raises interrupt 35 (IRQ for network).
  4. The kernel's ISR reads the packet from the receive buffer.

Our simulation replaces the wire with a SharedWire object (an in-memory
queue). Sending pushes to the wire; receiving pops from a local queue
that the wire populates.

==========================================================================
SimulatedNIC Design
==========================================================================

Two SimulatedNICs connected to the same SharedWire can exchange packets.
When NIC A sends, the wire delivers the packet to NIC B's receive queue
(and vice versa). Packets are NOT echoed back to the sender.
"""

from collections import deque

from device_driver_framework.device import NetworkDevice
from device_driver_framework.shared_wire import SharedWire


class SimulatedNIC(NetworkDevice):
    """A simulated network interface card backed by in-memory queues.

    Packets sent via send_packet() are broadcast to all other NICs on
    the same SharedWire. Incoming packets are queued in rx_queue and
    retrieved via receive_packet().

    Args:
        name: Device name (default "nic0").
        minor: Minor number (default 0).
        mac_address: 6-byte MAC address for this NIC.
        wire: The SharedWire connecting this NIC to others.
        interrupt_number: IRQ for packet received (default 35).
    """

    def __init__(
        self,
        name: str = "nic0",
        minor: int = 0,
        mac_address: bytes = b"\xDE\xAD\xBE\xEF\x00\x01",
        wire: SharedWire | None = None,
        interrupt_number: int = 35,
    ) -> None:
        super().__init__(
            name=name,
            major=4,  # Major 4 = NIC driver (from the spec)
            minor=minor,
            mac_address=mac_address,
            interrupt_number=interrupt_number,
        )
        # The receive queue: packets waiting to be read by the kernel.
        # The SharedWire pushes packets here when another NIC sends.
        self.rx_queue: deque[bytes] = deque()

        # The wire this NIC is connected to. Can be None if the NIC
        # is not yet connected to a network.
        self._wire = wire

    def init(self) -> None:
        """Initialize the NIC.

        Clears the receive queue and connects to the shared wire.
        On a real NIC, initialization would also involve:
        - Programming the MAC address into the NIC's registers
        - Allocating DMA buffers for transmit and receive
        - Enabling the NIC's interrupt on the PIC/APIC
        - Negotiating link speed with the switch (auto-negotiation)
        """
        self.rx_queue.clear()
        if self._wire is not None:
            self._wire.connect(self)
        self.initialized = True

    def send_packet(self, data: bytes) -> int:
        """Send a packet over the network.

        The packet is broadcast to all other NICs on the same wire.
        Returns the number of bytes sent, or -1 if not connected.

        In a real NIC, sending involves:
          1. Copying data to the transmit ring buffer
          2. Telling the NIC "there's data to send" (doorbell register)
          3. The NIC converts bytes to electrical signals
          4. An interrupt fires when transmission is complete

        Our simulation skips the electrical signals and just pushes
        the data onto the shared wire.

        Args:
            data: The raw packet bytes to send.

        Returns:
            Number of bytes sent, or -1 on error.
        """
        if self._wire is None:
            return -1
        self._wire.broadcast(data, sender=self)
        return len(data)

    def receive_packet(self) -> bytes | None:
        """Receive the next packet from the network.

        Returns the oldest packet in the receive queue, or None if no
        packets are waiting. This is non-blocking: if nothing has arrived,
        it returns immediately with None.

        In a real NIC, the packet would already be in a DMA buffer.
        The kernel's ISR (triggered by interrupt 35) would call this
        method to retrieve the packet data.

        Returns:
            The packet data as bytes, or None if the queue is empty.
        """
        if not self.rx_queue:
            return None
        return self.rx_queue.popleft()

    def has_packet(self) -> bool:
        """Check whether there is a packet waiting to be received.

        Returns True if at least one packet is in the receive queue.
        This allows the kernel to check for data without actually
        consuming it (useful for poll/select-style I/O multiplexing).
        """
        return len(self.rx_queue) > 0

    @property
    def wire(self) -> SharedWire | None:
        """The SharedWire this NIC is connected to."""
        return self._wire
