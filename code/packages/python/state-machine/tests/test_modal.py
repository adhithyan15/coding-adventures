"""Tests for the Modal State Machine implementation."""

import pytest

from state_machine.dfa import DFA
from state_machine.modal import ModalStateMachine

# ============================================================
# Fixtures
# ============================================================


def _make_data_mode() -> DFA:
    """DFA for the DATA mode: reads chars, detects '<' for tag open."""
    return DFA(
        states={"text", "tag_detected"},
        alphabet={"char", "open_angle"},
        transitions={
            ("text", "char"): "text",
            ("text", "open_angle"): "tag_detected",
            ("tag_detected", "char"): "text",
            ("tag_detected", "open_angle"): "tag_detected",
        },
        initial="text",
        accepting={"text"},
    )


def _make_tag_mode() -> DFA:
    """DFA for the TAG mode: reads tag name chars, detects '>' for close."""
    return DFA(
        states={"reading_name", "tag_done"},
        alphabet={"char", "close_angle"},
        transitions={
            ("reading_name", "char"): "reading_name",
            ("reading_name", "close_angle"): "tag_done",
            ("tag_done", "char"): "reading_name",
            ("tag_done", "close_angle"): "tag_done",
        },
        initial="reading_name",
        accepting={"tag_done"},
    )


def _make_script_mode() -> DFA:
    """DFA for SCRIPT mode: reads raw chars until end-script detected."""
    return DFA(
        states={"raw"},
        alphabet={"char", "end_marker"},
        transitions={
            ("raw", "char"): "raw",
            ("raw", "end_marker"): "raw",
        },
        initial="raw",
        accepting={"raw"},
    )


@pytest.fixture()
def html_tokenizer() -> ModalStateMachine:
    """Simplified HTML tokenizer with 3 modes."""
    return ModalStateMachine(
        modes={
            "data": _make_data_mode(),
            "tag": _make_tag_mode(),
            "script": _make_script_mode(),
        },
        mode_transitions={
            ("data", "enter_tag"): "tag",
            ("tag", "exit_tag"): "data",
            ("tag", "enter_script"): "script",
            ("script", "exit_script"): "data",
        },
        initial_mode="data",
    )


# ============================================================
# Construction Tests
# ============================================================


class TestModalConstruction:
    """Tests for modal state machine construction."""

    def test_valid_construction(
        self, html_tokenizer: ModalStateMachine
    ) -> None:
        """Valid modal machine should construct without errors."""
        assert html_tokenizer.current_mode == "data"
        assert len(html_tokenizer.modes) == 3

    def test_no_modes_rejected(self) -> None:
        """Must have at least one mode."""
        with pytest.raises(ValueError, match="one mode"):
            ModalStateMachine(
                modes={}, mode_transitions={}, initial_mode="data"
            )

    def test_invalid_initial_mode(self) -> None:
        """Initial mode must exist."""
        with pytest.raises(ValueError, match="Initial mode"):
            ModalStateMachine(
                modes={"data": _make_data_mode()},
                mode_transitions={},
                initial_mode="missing",
            )

    def test_invalid_transition_source(self) -> None:
        """Mode transition source must be a valid mode."""
        with pytest.raises(ValueError, match="source"):
            ModalStateMachine(
                modes={"data": _make_data_mode()},
                mode_transitions={("missing", "trigger"): "data"},
                initial_mode="data",
            )

    def test_invalid_transition_target(self) -> None:
        """Mode transition target must be a valid mode."""
        with pytest.raises(ValueError, match="target"):
            ModalStateMachine(
                modes={"data": _make_data_mode()},
                mode_transitions={("data", "trigger"): "missing"},
                initial_mode="data",
            )


# ============================================================
# Mode Switching Tests
# ============================================================


class TestModeSwitching:
    """Tests for switching between modes."""

    def test_switch_mode(self, html_tokenizer: ModalStateMachine) -> None:
        """switch_mode should change the active mode."""
        assert html_tokenizer.current_mode == "data"
        html_tokenizer.switch_mode("enter_tag")
        assert html_tokenizer.current_mode == "tag"

    def test_switch_mode_returns_new_mode(
        self, html_tokenizer: ModalStateMachine
    ) -> None:
        """switch_mode should return the new mode name."""
        result = html_tokenizer.switch_mode("enter_tag")
        assert result == "tag"

    def test_switch_resets_target_dfa(
        self, html_tokenizer: ModalStateMachine
    ) -> None:
        """Switching modes should reset the target mode's DFA."""
        # Process some events in tag mode
        html_tokenizer.switch_mode("enter_tag")
        html_tokenizer.process("char")
        html_tokenizer.process("close_angle")
        assert html_tokenizer.active_machine.current_state == "tag_done"

        # Switch away and back — should reset
        html_tokenizer.switch_mode("exit_tag")
        html_tokenizer.switch_mode("enter_tag")
        assert (
            html_tokenizer.active_machine.current_state == "reading_name"
        )

    def test_switch_data_to_tag_to_data(
        self, html_tokenizer: ModalStateMachine
    ) -> None:
        """Full cycle: data → tag → data."""
        html_tokenizer.switch_mode("enter_tag")
        assert html_tokenizer.current_mode == "tag"
        html_tokenizer.switch_mode("exit_tag")
        assert html_tokenizer.current_mode == "data"

    def test_switch_to_script_mode(
        self, html_tokenizer: ModalStateMachine
    ) -> None:
        """data → tag → script → data cycle."""
        html_tokenizer.switch_mode("enter_tag")
        html_tokenizer.switch_mode("enter_script")
        assert html_tokenizer.current_mode == "script"
        html_tokenizer.switch_mode("exit_script")
        assert html_tokenizer.current_mode == "data"

    def test_invalid_trigger(
        self, html_tokenizer: ModalStateMachine
    ) -> None:
        """Invalid trigger should raise ValueError."""
        with pytest.raises(ValueError, match="No mode transition"):
            html_tokenizer.switch_mode("nonexistent_trigger")

    def test_mode_trace(self, html_tokenizer: ModalStateMachine) -> None:
        """Mode switches should be recorded in the trace."""
        html_tokenizer.switch_mode("enter_tag")
        html_tokenizer.switch_mode("exit_tag")

        trace = html_tokenizer.mode_trace
        assert len(trace) == 2
        assert trace[0].from_mode == "data"
        assert trace[0].trigger == "enter_tag"
        assert trace[0].to_mode == "tag"
        assert trace[1].from_mode == "tag"
        assert trace[1].trigger == "exit_tag"
        assert trace[1].to_mode == "data"


# ============================================================
# Processing Within Modes Tests
# ============================================================


class TestProcessingInModes:
    """Tests for processing events within a mode."""

    def test_process_in_data_mode(
        self, html_tokenizer: ModalStateMachine
    ) -> None:
        """Events should be processed by the active mode's DFA."""
        result = html_tokenizer.process("char")
        assert result == "text"

    def test_process_in_tag_mode(
        self, html_tokenizer: ModalStateMachine
    ) -> None:
        """After switching, events go to the new mode's DFA."""
        html_tokenizer.switch_mode("enter_tag")
        result = html_tokenizer.process("char")
        assert result == "reading_name"
        result = html_tokenizer.process("close_angle")
        assert result == "tag_done"

    def test_process_in_script_mode(
        self, html_tokenizer: ModalStateMachine
    ) -> None:
        """Script mode processes char events."""
        html_tokenizer.switch_mode("enter_tag")
        html_tokenizer.switch_mode("enter_script")
        result = html_tokenizer.process("char")
        assert result == "raw"

    def test_process_invalid_event_for_mode(
        self, html_tokenizer: ModalStateMachine
    ) -> None:
        """Events invalid for the current mode's DFA should raise."""
        # "close_angle" is not in data mode's alphabet
        with pytest.raises(ValueError):
            html_tokenizer.process("close_angle")

    def test_active_machine_property(
        self, html_tokenizer: ModalStateMachine
    ) -> None:
        """active_machine should return the current mode's DFA."""
        data_dfa = html_tokenizer.active_machine
        assert data_dfa.current_state == "text"

        html_tokenizer.switch_mode("enter_tag")
        tag_dfa = html_tokenizer.active_machine
        assert tag_dfa.current_state == "reading_name"


# ============================================================
# Reset Tests
# ============================================================


class TestModalReset:
    """Tests for reset()."""

    def test_reset_mode(self, html_tokenizer: ModalStateMachine) -> None:
        """Reset should return to initial mode."""
        html_tokenizer.switch_mode("enter_tag")
        html_tokenizer.reset()
        assert html_tokenizer.current_mode == "data"

    def test_reset_clears_trace(
        self, html_tokenizer: ModalStateMachine
    ) -> None:
        """Reset should clear the mode trace."""
        html_tokenizer.switch_mode("enter_tag")
        html_tokenizer.switch_mode("exit_tag")
        assert len(html_tokenizer.mode_trace) == 2

        html_tokenizer.reset()
        assert html_tokenizer.mode_trace == []

    def test_reset_resets_all_dfas(
        self, html_tokenizer: ModalStateMachine
    ) -> None:
        """Reset should reset all mode DFAs."""
        html_tokenizer.switch_mode("enter_tag")
        html_tokenizer.process("char")
        html_tokenizer.process("close_angle")

        html_tokenizer.reset()
        # Tag mode's DFA should be back at initial state
        html_tokenizer.switch_mode("enter_tag")
        assert html_tokenizer.active_machine.current_state == "reading_name"


# ============================================================
# Repr Tests
# ============================================================


class TestModalRepr:
    """Tests for __repr__."""

    def test_repr(self, html_tokenizer: ModalStateMachine) -> None:
        """repr should contain mode info."""
        r = repr(html_tokenizer)
        assert "ModalStateMachine" in r
        assert "data" in r
