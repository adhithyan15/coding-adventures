"""Signals — Software interrupts for inter-process communication.

Signals are the Unix mechanism for asynchronous process notification. They are
like text messages between processes: one process can "send" a signal to
another, and the recipient can choose how to respond.

Real-World Analogy
==================

Imagine you are working at your desk (a running process). Various things can
interrupt you:

    - Your phone rings (SIGINT) — you can answer it or ignore it.
    - Your boss walks in and says "you're fired" (SIGKILL) — you MUST leave.
      You cannot ignore this. You cannot negotiate. You are done.
    - Someone taps your shoulder and says "please finish up" (SIGTERM) — you
      can either stop immediately or finish what you're doing first.
    - Your child tugs your sleeve (SIGCHLD) — your child needs attention.
    - Someone says "freeze!" (SIGSTOP) — you must stop moving. You cannot
      choose to keep going.
    - Someone says "continue" (SIGCONT) — you can resume what you were doing.

Signal Numbers
==============

We implement the six most important POSIX signals. The numbers are standardized
across all Unix systems:

    Signal   Number  Default Action  Can Catch?  Purpose
    ------   ------  --------------  ----------  -------
    SIGINT      2    Terminate       Yes         Ctrl+C pressed
    SIGKILL     9    Terminate       NO          Force kill (uncatchable)
    SIGTERM    15    Terminate       Yes         Polite termination request
    SIGCHLD    17    Ignore          Yes         Child status changed
    SIGCONT    18    Continue        Yes*        Resume stopped process
    SIGSTOP    19    Stop            NO          Force stop (uncatchable)

    * SIGCONT always resumes a stopped process, but a handler can also run.

Signal Delivery Flow
====================

When process A sends a signal to process B:

    1. The signal is added to B's pending_signals list.
    2. When B is next scheduled (context switch to B), the kernel checks
       B's pending_signals.
    3. For each pending signal:
       a. If the signal is masked (blocked), skip it — leave it pending.
       b. If the signal is SIGKILL or SIGSTOP, apply the action immediately
          (these cannot be masked or caught).
       c. If B has a custom handler for this signal, redirect B's execution
          to the handler address.
       d. If no custom handler, apply the default action (usually terminate).
"""

from __future__ import annotations

from enum import IntEnum
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from process_manager.pcb import ProcessControlBlock

from process_manager.pcb import ProcessState

# =============================================================================
# Signal Enum — The Six Essential POSIX Signals
# =============================================================================
#
# Why IntEnum instead of Enum?
# Because signal numbers are meaningful integers in the POSIX standard. Using
# IntEnum lets us compare signals with integers (Signal.SIGINT == 2) and use
# them as dictionary keys interchangeably with ints.


class Signal(IntEnum):
    """POSIX signals for inter-process communication.

    Each signal has a specific purpose and default behavior. Two signals
    (SIGKILL and SIGSTOP) are special: they cannot be caught, blocked, or
    ignored. This is a safety mechanism — the kernel must always be able to
    kill or stop a runaway process.
    """

    SIGINT = 2  # Interrupt — sent by Ctrl+C.
    SIGKILL = 9  # Kill — unconditional termination.
    SIGTERM = 15  # Terminate — polite "please exit."
    SIGCHLD = 17  # Child status changed.
    SIGCONT = 18  # Continue — resume a stopped process.
    SIGSTOP = 19  # Stop — suspend the process.


# =============================================================================
# Default Signal Actions
# =============================================================================
#
# When a signal has no custom handler, the kernel applies a default action.
# Most signals default to "terminate the process." A few are special:
#
#   SIGCHLD:  Ignored by default. Parents only care about SIGCHLD if they
#             explicitly register a handler.
#   SIGCONT:  Always resumes a stopped process, regardless of handlers.
#   SIGSTOP:  Always stops the process (cannot be caught).
#   SIGKILL:  Always kills the process (cannot be caught).

# Signals that terminate the process by default.
_FATAL_BY_DEFAULT: frozenset[int] = frozenset(
    {Signal.SIGINT, Signal.SIGKILL, Signal.SIGTERM}
)

# Signals that can NEVER be caught, blocked, or ignored.
# These ensure the kernel can always control a process.
_UNCATCHABLE: frozenset[int] = frozenset({Signal.SIGKILL, Signal.SIGSTOP})


# =============================================================================
# SignalManager — Handles Signal Delivery and Processing
# =============================================================================
#
# The SignalManager is the kernel's signal subsystem. It is responsible for:
#   1. Accepting signals sent to a process (send_signal).
#   2. Delivering pending signals when a process is scheduled (deliver_pending).
#   3. Managing custom signal handlers (register_handler).
#   4. Managing the signal mask (mask_signal, unmask_signal).
#
# The SignalManager does NOT own the processes — it operates on PCBs passed
# to it by the ProcessManager.


class SignalManager:
    """Handles signal delivery, masking, and handler registration.

    The SignalManager processes signals according to POSIX semantics:

    1. SIGKILL and SIGSTOP are always delivered immediately and cannot be
       caught, masked, or ignored.
    2. Masked signals remain in the pending queue until unmasked.
    3. Signals with custom handlers redirect execution to the handler.
    4. Signals without handlers use the default action (terminate or ignore).

    Example:
        >>> from process_manager.pcb import ProcessControlBlock
        >>> from process_manager.signals import Signal, SignalManager
        >>> sm = SignalManager()
        >>> pcb = ProcessControlBlock(pid=1, name="myproc")
        >>> sm.send_signal(pcb, Signal.SIGTERM)
        True
        >>> pcb.pending_signals
        [15]
    """

    def send_signal(
        self, process: ProcessControlBlock, signal: Signal
    ) -> bool:
        """Send a signal to a process.

        The signal is added to the process's pending_signals list. It will be
        delivered the next time deliver_pending() is called (typically when the
        process is scheduled to run).

        Special cases:
            - SIGKILL: Immediately terminates the process (state -> ZOMBIE).
              Cannot be blocked or caught.
            - SIGSTOP: Immediately stops the process (state -> BLOCKED).
              Cannot be blocked or caught.
            - SIGCONT: If the process is BLOCKED (stopped), resumes it
              (state -> READY). Also added to pending if a handler exists.

        Args:
            process: The target process's PCB.
            signal: The signal to send.

        Returns:
            True if the signal was accepted (even if it will be processed
            later), False if the process is in a state that cannot receive
            signals (TERMINATED).
        """
        # A terminated process (fully cleaned up) cannot receive signals.
        if process.state == ProcessState.TERMINATED:
            return False

        # --- SIGKILL: Unconditional termination ---
        # This is the "nuclear option." The process is immediately terminated.
        # No handler runs. No cleanup. The process is just... done.
        if signal == Signal.SIGKILL:
            process.state = ProcessState.ZOMBIE
            return True

        # --- SIGSTOP: Unconditional stop ---
        # The process is suspended. It will not run until SIGCONT is sent.
        # Like SIGKILL, this cannot be caught or ignored.
        if signal == Signal.SIGSTOP:
            process.state = ProcessState.BLOCKED
            return True

        # --- SIGCONT: Resume a stopped process ---
        # If the process is currently stopped (BLOCKED), resume it.
        # SIGCONT is unique: it always resumes the process, AND it can
        # also trigger a handler if one is registered.
        if signal == Signal.SIGCONT:
            if process.state == ProcessState.BLOCKED:
                process.state = ProcessState.READY
            # Still add to pending so a handler can run if registered.
            process.pending_signals.append(int(signal))
            return True

        # --- All other signals: Add to pending list ---
        # The signal will be delivered by deliver_pending() when the process
        # is next scheduled.
        process.pending_signals.append(int(signal))
        return True

    def deliver_pending(
        self, process: ProcessControlBlock
    ) -> Signal | None:
        """Deliver the next pending signal to a process.

        This is called by the scheduler just before a process is about to run.
        It checks the process's pending_signals list and delivers the first
        signal that is not masked.

        Signal delivery means one of:
            1. If the process has a custom handler for this signal, return the
               signal so the caller can redirect execution to the handler.
            2. If no handler and the signal is fatal by default, set the
               process state to ZOMBIE.
            3. If no handler and the signal is not fatal (e.g., SIGCHLD),
               silently discard it.

        Args:
            process: The process whose pending signals to check.

        Returns:
            The delivered Signal if a handler exists (so the caller can
            redirect the PC), or None if no actionable pending signals.

        Truth Table:
            Signal    Masked?  Handler?  Action
            ------    -------  --------  ------
            SIGTERM   No       Yes       Return SIGTERM (caller sets PC)
            SIGTERM   No       No        Terminate (state -> ZOMBIE)
            SIGTERM   Yes      -         Skip (leave pending)
            SIGCHLD   No       Yes       Return SIGCHLD (caller sets PC)
            SIGCHLD   No       No        Discard (SIGCHLD ignored by default)
            SIGKILL   -        -         Already handled in send_signal
        """
        # Walk the pending list looking for a deliverable signal.
        for i, sig_num in enumerate(process.pending_signals):
            # Skip masked signals — they stay pending until unmasked.
            if sig_num in process.signal_mask:
                continue

            # Remove this signal from the pending list.
            process.pending_signals.pop(i)

            # Convert int back to Signal enum for type safety.
            signal = Signal(sig_num)

            # Does the process have a custom handler?
            if sig_num in process.signal_handlers:
                # Return the signal — the caller (ProcessManager) will
                # redirect the process's PC to the handler address.
                return signal

            # No custom handler. Apply the default action.
            if sig_num in _FATAL_BY_DEFAULT:
                # Default for SIGINT, SIGTERM: terminate.
                process.state = ProcessState.ZOMBIE
                return signal

            # Non-fatal signal with no handler (e.g., SIGCHLD): silently
            # discard. The process doesn't care about this signal.
            return None

        # No deliverable signals found.
        return None

    def register_handler(
        self,
        process: ProcessControlBlock,
        signal: Signal,
        handler_addr: int,
    ) -> None:
        """Register a custom signal handler for a process.

        When the given signal is delivered, the process's PC will be
        redirected to handler_addr instead of applying the default action.

        SIGKILL and SIGSTOP cannot have custom handlers — they are always
        handled by the kernel. Attempting to register a handler for them
        is silently ignored (following POSIX behavior).

        Args:
            process: The process to register the handler for.
            signal: The signal to handle.
            handler_addr: The virtual address of the handler function.

        Example:
            A web server registers a SIGTERM handler that gracefully closes
            all connections before exiting:

                signal_manager.register_handler(
                    server_pcb, Signal.SIGTERM, 0x00040000
                )

            Now when SIGTERM is sent, instead of being killed immediately,
            the server runs its cleanup code at address 0x00040000.
        """
        # SIGKILL and SIGSTOP are uncatchable — the kernel always handles them.
        if int(signal) in _UNCATCHABLE:
            return

        process.signal_handlers[int(signal)] = handler_addr

    def mask_signal(
        self, process: ProcessControlBlock, signal: Signal
    ) -> None:
        """Block a signal from being delivered to a process.

        A masked signal stays in the pending_signals list but is not
        delivered until unmask_signal() is called. This is useful when a
        process is in a critical section and cannot be interrupted.

        SIGKILL and SIGSTOP cannot be masked — they must always be
        deliverable. Attempting to mask them is silently ignored.

        Args:
            process: The process to mask the signal for.
            signal: The signal to mask.
        """
        if int(signal) in _UNCATCHABLE:
            return
        process.signal_mask.add(int(signal))

    def unmask_signal(
        self, process: ProcessControlBlock, signal: Signal
    ) -> None:
        """Unblock a previously masked signal.

        After unmasking, if the signal is in the pending list, it will be
        delivered on the next call to deliver_pending().

        Args:
            process: The process to unmask the signal for.
            signal: The signal to unmask.
        """
        process.signal_mask.discard(int(signal))

    def is_fatal(self, signal: Signal) -> bool:
        """Check if a signal terminates the process by default.

        Returns True for signals whose default action is to terminate the
        process: SIGINT, SIGKILL, SIGTERM. Returns False for signals that
        are ignored by default (SIGCHLD) or have special behavior (SIGCONT,
        SIGSTOP).

        Args:
            signal: The signal to check.

        Returns:
            True if the default action is termination.

        Example:
            >>> sm = SignalManager()
            >>> sm.is_fatal(Signal.SIGKILL)
            True
            >>> sm.is_fatal(Signal.SIGCHLD)
            False
        """
        return int(signal) in _FATAL_BY_DEFAULT
