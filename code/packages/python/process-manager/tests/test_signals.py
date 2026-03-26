"""Tests for Signal enum and SignalManager.

These tests verify signal delivery, masking, handler registration, and the
special semantics of SIGKILL, SIGSTOP, and SIGCONT.
"""

from process_manager.pcb import ProcessControlBlock, ProcessState
from process_manager.signals import Signal, SignalManager

# =============================================================================
# Signal Enum Tests
# =============================================================================


class TestSignal:
    """Verify POSIX signal numbers are correct."""

    def test_sigint(self) -> None:
        """SIGINT is signal 2 (Ctrl+C)."""
        assert Signal.SIGINT == 2

    def test_sigkill(self) -> None:
        """SIGKILL is signal 9 (unconditional kill)."""
        assert Signal.SIGKILL == 9

    def test_sigterm(self) -> None:
        """SIGTERM is signal 15 (polite termination)."""
        assert Signal.SIGTERM == 15

    def test_sigchld(self) -> None:
        """SIGCHLD is signal 17 (child status changed)."""
        assert Signal.SIGCHLD == 17

    def test_sigcont(self) -> None:
        """SIGCONT is signal 18 (resume stopped process)."""
        assert Signal.SIGCONT == 18

    def test_sigstop(self) -> None:
        """SIGSTOP is signal 19 (suspend process)."""
        assert Signal.SIGSTOP == 19

    def test_signal_count(self) -> None:
        """We implement exactly 6 signals."""
        assert len(Signal) == 6


# =============================================================================
# SignalManager Tests
# =============================================================================


class TestSignalManager:
    """Test signal delivery, masking, and handling."""

    def setup_method(self) -> None:
        """Create a fresh SignalManager and test process for each test."""
        self.sm = SignalManager()
        self.pcb = ProcessControlBlock(pid=1, name="test_proc")

    # --- Basic signal sending ---

    def test_send_signal_adds_to_pending(self) -> None:
        """Sending SIGTERM adds it to the pending list."""
        self.sm.send_signal(self.pcb, Signal.SIGTERM)
        assert 15 in self.pcb.pending_signals

    def test_send_multiple_signals(self) -> None:
        """Multiple signals can be pending simultaneously."""
        self.sm.send_signal(self.pcb, Signal.SIGTERM)
        self.sm.send_signal(self.pcb, Signal.SIGINT)
        assert len(self.pcb.pending_signals) == 2

    def test_send_to_terminated_fails(self) -> None:
        """Cannot send signals to a fully terminated process."""
        self.pcb.state = ProcessState.TERMINATED
        result = self.sm.send_signal(self.pcb, Signal.SIGTERM)
        assert result is False

    def test_send_returns_true_on_success(self) -> None:
        """send_signal returns True when the signal is accepted."""
        assert self.sm.send_signal(self.pcb, Signal.SIGTERM) is True

    # --- SIGKILL: uncatchable termination ---

    def test_sigkill_immediately_terminates(self) -> None:
        """SIGKILL sets state to ZOMBIE immediately — no handler can prevent it."""
        self.pcb.state = ProcessState.RUNNING
        self.sm.send_signal(self.pcb, Signal.SIGKILL)
        assert self.pcb.state == ProcessState.ZOMBIE

    def test_sigkill_not_added_to_pending(self) -> None:
        """SIGKILL acts immediately — it is not queued."""
        self.sm.send_signal(self.pcb, Signal.SIGKILL)
        assert len(self.pcb.pending_signals) == 0

    # --- SIGSTOP: uncatchable stop ---

    def test_sigstop_immediately_blocks(self) -> None:
        """SIGSTOP sets state to BLOCKED immediately."""
        self.pcb.state = ProcessState.RUNNING
        self.sm.send_signal(self.pcb, Signal.SIGSTOP)
        assert self.pcb.state == ProcessState.BLOCKED

    def test_sigstop_not_added_to_pending(self) -> None:
        """SIGSTOP acts immediately — it is not queued."""
        self.sm.send_signal(self.pcb, Signal.SIGSTOP)
        assert len(self.pcb.pending_signals) == 0

    # --- SIGCONT: resume ---

    def test_sigcont_resumes_blocked_process(self) -> None:
        """SIGCONT changes BLOCKED state to READY."""
        self.pcb.state = ProcessState.BLOCKED
        self.sm.send_signal(self.pcb, Signal.SIGCONT)
        assert self.pcb.state == ProcessState.READY

    def test_sigcont_added_to_pending(self) -> None:
        """SIGCONT is also added to pending for handler delivery."""
        self.sm.send_signal(self.pcb, Signal.SIGCONT)
        assert int(Signal.SIGCONT) in self.pcb.pending_signals

    def test_sigcont_on_ready_stays_ready(self) -> None:
        """SIGCONT on an already-ready process does not change state."""
        self.pcb.state = ProcessState.READY
        self.sm.send_signal(self.pcb, Signal.SIGCONT)
        assert self.pcb.state == ProcessState.READY

    # --- Signal delivery ---

    def test_deliver_pending_with_handler(self) -> None:
        """A signal with a custom handler returns the signal for PC redirect."""
        self.pcb.signal_handlers[int(Signal.SIGTERM)] = 0x40000
        self.pcb.pending_signals.append(int(Signal.SIGTERM))

        result = self.sm.deliver_pending(self.pcb)
        assert result == Signal.SIGTERM

    def test_deliver_removes_from_pending(self) -> None:
        """After delivery, the signal is removed from pending."""
        self.pcb.pending_signals.append(int(Signal.SIGTERM))
        self.sm.deliver_pending(self.pcb)
        assert int(Signal.SIGTERM) not in self.pcb.pending_signals

    def test_deliver_fatal_without_handler(self) -> None:
        """A fatal signal without a handler terminates the process."""
        self.pcb.pending_signals.append(int(Signal.SIGTERM))
        self.sm.deliver_pending(self.pcb)
        assert self.pcb.state == ProcessState.ZOMBIE

    def test_deliver_nonfatal_without_handler(self) -> None:
        """A non-fatal signal without a handler is discarded silently.

        SIGCHLD is ignored by default — if the process hasn't registered
        a handler for it, nothing happens.
        """
        self.pcb.pending_signals.append(int(Signal.SIGCHLD))
        result = self.sm.deliver_pending(self.pcb)
        assert result is None
        assert self.pcb.state == ProcessState.READY  # unchanged

    def test_deliver_no_pending(self) -> None:
        """deliver_pending returns None when no signals are pending."""
        result = self.sm.deliver_pending(self.pcb)
        assert result is None

    # --- Signal masking ---

    def test_masked_signal_not_delivered(self) -> None:
        """A masked signal stays in the pending list — not delivered."""
        self.sm.mask_signal(self.pcb, Signal.SIGTERM)
        self.pcb.pending_signals.append(int(Signal.SIGTERM))

        result = self.sm.deliver_pending(self.pcb)
        assert result is None
        # Still in pending:
        assert int(Signal.SIGTERM) in self.pcb.pending_signals

    def test_unmask_allows_delivery(self) -> None:
        """After unmasking, a pending signal can be delivered."""
        self.sm.mask_signal(self.pcb, Signal.SIGTERM)
        self.pcb.pending_signals.append(int(Signal.SIGTERM))

        # While masked, not delivered.
        assert self.sm.deliver_pending(self.pcb) is None

        # Unmask and deliver.
        self.sm.unmask_signal(self.pcb, Signal.SIGTERM)
        self.sm.deliver_pending(self.pcb)
        assert int(Signal.SIGTERM) not in self.pcb.pending_signals

    def test_sigkill_cannot_be_masked(self) -> None:
        """SIGKILL cannot be masked — it is always deliverable.

        Attempting to mask SIGKILL is silently ignored. This is a safety
        mechanism: the kernel must always be able to kill a process.
        """
        self.sm.mask_signal(self.pcb, Signal.SIGKILL)
        assert int(Signal.SIGKILL) not in self.pcb.signal_mask

    def test_sigstop_cannot_be_masked(self) -> None:
        """SIGSTOP cannot be masked — it is always deliverable."""
        self.sm.mask_signal(self.pcb, Signal.SIGSTOP)
        assert int(Signal.SIGSTOP) not in self.pcb.signal_mask

    # --- Handler registration ---

    def test_register_handler(self) -> None:
        """Registering a handler maps signal -> handler address."""
        self.sm.register_handler(self.pcb, Signal.SIGTERM, 0x40000)
        assert self.pcb.signal_handlers[int(Signal.SIGTERM)] == 0x40000

    def test_register_handler_sigkill_ignored(self) -> None:
        """Cannot register a handler for SIGKILL — it is always fatal."""
        self.sm.register_handler(self.pcb, Signal.SIGKILL, 0x40000)
        assert int(Signal.SIGKILL) not in self.pcb.signal_handlers

    def test_register_handler_sigstop_ignored(self) -> None:
        """Cannot register a handler for SIGSTOP — it is always stopping."""
        self.sm.register_handler(self.pcb, Signal.SIGSTOP, 0x40000)
        assert int(Signal.SIGSTOP) not in self.pcb.signal_handlers

    def test_register_handler_overwrite(self) -> None:
        """Registering a new handler overwrites the old one."""
        self.sm.register_handler(self.pcb, Signal.SIGTERM, 0x40000)
        self.sm.register_handler(self.pcb, Signal.SIGTERM, 0x50000)
        assert self.pcb.signal_handlers[int(Signal.SIGTERM)] == 0x50000

    # --- is_fatal ---

    def test_is_fatal_sigkill(self) -> None:
        """SIGKILL is always fatal."""
        assert self.sm.is_fatal(Signal.SIGKILL) is True

    def test_is_fatal_sigterm(self) -> None:
        """SIGTERM is fatal by default."""
        assert self.sm.is_fatal(Signal.SIGTERM) is True

    def test_is_fatal_sigint(self) -> None:
        """SIGINT is fatal by default."""
        assert self.sm.is_fatal(Signal.SIGINT) is True

    def test_is_fatal_sigchld(self) -> None:
        """SIGCHLD is not fatal — it is ignored by default."""
        assert self.sm.is_fatal(Signal.SIGCHLD) is False

    def test_is_fatal_sigcont(self) -> None:
        """SIGCONT is not fatal — it resumes processes."""
        assert self.sm.is_fatal(Signal.SIGCONT) is False

    def test_is_fatal_sigstop(self) -> None:
        """SIGSTOP is not fatal — it stops processes."""
        assert self.sm.is_fatal(Signal.SIGSTOP) is False

    # --- Deliver SIGINT with handler ---

    def test_deliver_sigint_with_handler(self) -> None:
        """SIGINT with a custom handler returns the signal for PC redirect."""
        self.pcb.signal_handlers[int(Signal.SIGINT)] = 0x30000
        self.pcb.pending_signals.append(int(Signal.SIGINT))
        result = self.sm.deliver_pending(self.pcb)
        assert result == Signal.SIGINT

    def test_deliver_sigint_without_handler(self) -> None:
        """SIGINT without a handler terminates (fatal by default)."""
        self.pcb.pending_signals.append(int(Signal.SIGINT))
        self.sm.deliver_pending(self.pcb)
        assert self.pcb.state == ProcessState.ZOMBIE
