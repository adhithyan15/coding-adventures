"""Tests for SwitchMatrix — programmable routing crossbar.

Coverage targets:
- Connection creation and routing
- Fan-out (one source, multiple destinations)
- Disconnect and clear
- Validation (unknown ports, self-connection, duplicate destination)
"""

from __future__ import annotations

import pytest

from fpga.switch_matrix import SwitchMatrix

PORTS = {"north", "south", "east", "west", "clb_out"}


# ─── Creation ─────────────────────────────────────────────────────────

class TestSwitchMatrixCreation:
    def test_create_with_ports(self) -> None:
        sm = SwitchMatrix(PORTS)
        assert sm.ports == frozenset(PORTS)

    def test_no_initial_connections(self) -> None:
        sm = SwitchMatrix(PORTS)
        assert sm.connection_count == 0
        assert sm.connections == {}

    def test_rejects_empty_ports(self) -> None:
        with pytest.raises(ValueError, match="non-empty"):
            SwitchMatrix(set())

    def test_rejects_empty_string_port(self) -> None:
        with pytest.raises(ValueError, match="non-empty strings"):
            SwitchMatrix({"", "a"})


# ─── Connect ──────────────────────────────────────────────────────────

class TestConnect:
    def test_simple_connection(self) -> None:
        sm = SwitchMatrix(PORTS)
        sm.connect("clb_out", "east")
        assert sm.connections == {"east": "clb_out"}
        assert sm.connection_count == 1

    def test_multiple_connections(self) -> None:
        sm = SwitchMatrix(PORTS)
        sm.connect("clb_out", "east")
        sm.connect("north", "south")
        assert sm.connection_count == 2

    def test_fan_out(self) -> None:
        """One source can drive multiple destinations."""
        sm = SwitchMatrix(PORTS)
        sm.connect("clb_out", "east")
        sm.connect("clb_out", "west")
        sm.connect("clb_out", "north")
        assert sm.connection_count == 3

    def test_rejects_unknown_source(self) -> None:
        sm = SwitchMatrix(PORTS)
        with pytest.raises(ValueError, match="unknown source"):
            sm.connect("invalid", "east")

    def test_rejects_unknown_destination(self) -> None:
        sm = SwitchMatrix(PORTS)
        with pytest.raises(ValueError, match="unknown destination"):
            sm.connect("east", "invalid")

    def test_rejects_self_connection(self) -> None:
        sm = SwitchMatrix(PORTS)
        with pytest.raises(ValueError, match="to itself"):
            sm.connect("east", "east")

    def test_rejects_duplicate_destination(self) -> None:
        sm = SwitchMatrix(PORTS)
        sm.connect("north", "south")
        with pytest.raises(ValueError, match="already connected"):
            sm.connect("east", "south")


# ─── Disconnect ───────────────────────────────────────────────────────

class TestDisconnect:
    def test_disconnect(self) -> None:
        sm = SwitchMatrix(PORTS)
        sm.connect("north", "south")
        sm.disconnect("south")
        assert sm.connection_count == 0

    def test_disconnect_allows_reconnection(self) -> None:
        sm = SwitchMatrix(PORTS)
        sm.connect("north", "south")
        sm.disconnect("south")
        sm.connect("east", "south")
        assert sm.connections == {"south": "east"}

    def test_rejects_unknown_port(self) -> None:
        sm = SwitchMatrix(PORTS)
        with pytest.raises(ValueError, match="unknown port"):
            sm.disconnect("invalid")

    def test_rejects_not_connected(self) -> None:
        sm = SwitchMatrix(PORTS)
        with pytest.raises(ValueError, match="not connected"):
            sm.disconnect("south")


# ─── Clear ────────────────────────────────────────────────────────────

class TestClear:
    def test_clear_removes_all(self) -> None:
        sm = SwitchMatrix(PORTS)
        sm.connect("north", "south")
        sm.connect("east", "west")
        sm.clear()
        assert sm.connection_count == 0
        assert sm.connections == {}


# ─── Route ────────────────────────────────────────────────────────────

class TestRoute:
    def test_basic_routing(self) -> None:
        sm = SwitchMatrix(PORTS)
        sm.connect("clb_out", "east")
        result = sm.route({"clb_out": 1})
        assert result == {"east": 1}

    def test_fan_out_routing(self) -> None:
        sm = SwitchMatrix(PORTS)
        sm.connect("clb_out", "east")
        sm.connect("clb_out", "west")
        result = sm.route({"clb_out": 1})
        assert result == {"east": 1, "west": 1}

    def test_multiple_sources(self) -> None:
        sm = SwitchMatrix(PORTS)
        sm.connect("north", "south")
        sm.connect("east", "west")
        result = sm.route({"north": 1, "east": 0})
        assert result == {"south": 1, "west": 0}

    def test_missing_source_in_inputs(self) -> None:
        """If the source isn't in the inputs dict, destination is not routed."""
        sm = SwitchMatrix(PORTS)
        sm.connect("north", "south")
        result = sm.route({"east": 1})
        assert result == {}

    def test_empty_inputs(self) -> None:
        sm = SwitchMatrix(PORTS)
        sm.connect("north", "south")
        result = sm.route({})
        assert result == {}

    def test_no_connections(self) -> None:
        sm = SwitchMatrix(PORTS)
        result = sm.route({"north": 1, "south": 0})
        assert result == {}


# ─── Properties ───────────────────────────────────────────────────────

class TestProperties:
    def test_connections_returns_copy(self) -> None:
        sm = SwitchMatrix(PORTS)
        sm.connect("north", "south")
        conns = sm.connections
        conns["east"] = "west"  # Mutate the copy
        assert sm.connection_count == 1  # Original unchanged
