"""
NetworkWire — Simulated Physical Medium
========================================

In the real world, Ethernet frames travel as electrical signals over copper
cables, as light pulses over fiber optics, or as radio waves over WiFi. Our
simulation needs a way to connect two network endpoints without real hardware.

The NetworkWire simulates an Ethernet cable connecting two endpoints (A and B).
When endpoint A sends a frame, it appears in B's receive queue, and vice versa.
This is a **full-duplex** connection — both sides can send simultaneously,
just like a real Ethernet cable.

::

    ┌──────────────┐                              ┌──────────────┐
    │  Endpoint A  │ ──── send_a() ────────────>  │  Endpoint B  │
    │  (Client)    │ <──── receive_a() ─────────  │  (Server)    │
    │              │                              │              │
    │              │ <──── send_b() ─────────────  │              │
    │              │ ──── receive_b() ──────────>  │              │
    └──────────────┘                              └──────────────┘

    Internally:
        send_a()    -> pushes to queue_a_to_b
        receive_b() -> pops from queue_a_to_b
        send_b()    -> pushes to queue_b_to_a
        receive_a() -> pops from queue_b_to_a

Why Two Queues?
---------------

A real Ethernet cable has separate transmit and receive wires (in full-duplex
mode). We model this with two independent queues:

- ``_queue_a_to_b``: frames sent by A, waiting to be received by B
- ``_queue_b_to_a``: frames sent by B, waiting to be received by A

This makes the simulation correct even if both sides send simultaneously.
"""

from __future__ import annotations


class NetworkWire:
    """
    A simulated Ethernet cable connecting two endpoints.

    Endpoint A is typically the client, and endpoint B is typically the
    server, but the wire is symmetric — either side can initiate.

    Example
    -------
    >>> wire = NetworkWire()
    >>> wire.send_a(b"Hello from A")
    >>> wire.has_data_for_b()
    True
    >>> wire.receive_b()
    b'Hello from A'
    >>> wire.send_b(b"Hello from B")
    >>> wire.receive_a()
    b'Hello from B'
    """

    def __init__(self) -> None:
        # Two independent queues for full-duplex communication
        self._queue_a_to_b: list[bytes] = []
        self._queue_b_to_a: list[bytes] = []

    def send_a(self, frame: bytes) -> None:
        """
        Endpoint A sends a frame.

        The frame is placed in the A-to-B queue and will be received
        by endpoint B on the next call to ``receive_b()``.
        """
        self._queue_a_to_b.append(frame)

    def send_b(self, frame: bytes) -> None:
        """
        Endpoint B sends a frame.

        The frame is placed in the B-to-A queue and will be received
        by endpoint A on the next call to ``receive_a()``.
        """
        self._queue_b_to_a.append(frame)

    def receive_a(self) -> bytes | None:
        """
        Endpoint A receives a frame sent by B.

        Returns the next frame from the B-to-A queue, or None if the
        queue is empty.
        """
        if not self._queue_b_to_a:
            return None
        return self._queue_b_to_a.pop(0)

    def receive_b(self) -> bytes | None:
        """
        Endpoint B receives a frame sent by A.

        Returns the next frame from the A-to-B queue, or None if the
        queue is empty.
        """
        if not self._queue_a_to_b:
            return None
        return self._queue_a_to_b.pop(0)

    def has_data_for_a(self) -> bool:
        """Return True if endpoint A has frames waiting."""
        return len(self._queue_b_to_a) > 0

    def has_data_for_b(self) -> bool:
        """Return True if endpoint B has frames waiting."""
        return len(self._queue_a_to_b) > 0
