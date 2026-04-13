"""Generic simulator protocol — the shared contract all architecture simulators
must implement.

──────────────────────────────────────────────────────────────────────────────
WHAT IS A SIMULATOR?
──────────────────────────────────────────────────────────────────────────────

A *simulator* is a program that faithfully mimics another program or piece of
hardware.  In this repo, every hardware simulator does one thing: accepts a
block of machine-code bytes and executes them as if it were the real chip.

Think of it like a film projection booth.  The projector (simulator) doesn't
know or care which film reel (program bytes) you hand it — it faithfully plays
back whatever you load.  The Intel 4004 simulator plays 4004 machine code; an
ARM simulator would play ARM machine code; a RISC-V simulator would play RISC-V
machine code.

After execution finishes, you can inspect the film's "state" — in our case the
CPU registers, flags, program counter, and RAM contents — to see exactly what
the hardware would have done.

──────────────────────────────────────────────────────────────────────────────
WHY A GENERIC PROTOCOL?
──────────────────────────────────────────────────────────────────────────────

This repo has (or will have) many simulators:

    Intel4004Simulator   — 4-bit accumulator, 1971
    Intel8008Simulator   — 8-bit, 1972
    ARM1Simulator        — 32-bit RISC, 1985
    RiscVSimulator       — open-standard RISC, 2015
    …

All of them need to do the same things:

  1. Load binary machine code.
  2. Execute it step by step (for debugging) or all at once (for testing).
  3. Return a snapshot of final state so tests can assert correctness.
  4. Support reset so the same simulator object can run multiple programs.

If every simulator has a *different* API, the compiler pipeline code becomes
a jungle of ``if isinstance(sim, Intel4004Simulator): ...`` checks.

Instead, we define a single ``Simulator[StateT]`` Protocol.  Any class that
has the right methods *automatically* satisfies the protocol — no inheritance
needed.  This is Python's structural subtyping (also called "duck typing with
types").  The compiler pipeline only ever sees ``Simulator[StateT]`` and never
needs to know which chip is underneath.

──────────────────────────────────────────────────────────────────────────────
GENERICS: WHY Simulator[StateT]?
──────────────────────────────────────────────────────────────────────────────

Different architectures have different internal state.  The Intel 4004 has a
4-bit accumulator, 16 × 4-bit registers, and a 3-level hardware stack.  An
ARM chip has 16 × 32-bit registers and a different flag set entirely.  We want
mypy to catch mismatches at type-check time:

    # This should be a type error — wrong state type:
    sim: Simulator[Intel4004State] = ARMSimulator()

The ``StateT`` type variable is *covariant*: a simulator that returns a subtype
of the expected state is also valid.  In practice, you will usually write:

    sim: Simulator[Intel4004State] = Intel4004Simulator()

and never think about variance.  The generics are there to help mypy, not to
complicate your code.

──────────────────────────────────────────────────────────────────────────────
THE STEP TRACE: DEBUGGING AND VISUALIZATION
──────────────────────────────────────────────────────────────────────────────

Every time ``step()`` executes one instruction, it returns a ``StepTrace``
capturing:

  - Where the PC was before and after
  - The human-readable mnemonic (e.g., ``"ADD R2"``, ``"JUN 0x100"``)
  - A plain-English description of what happened

This is invaluable for two reasons:

  1. *Debugging*: when a test fails, you can print the trace to see exactly
     which instruction went wrong.
  2. *Visualization*: the trace list from ``ExecutionResult.traces`` is perfect
     input for step-through debugger UIs, timing diagrams, and interactive
     drill-down visualizations.

──────────────────────────────────────────────────────────────────────────────
THE END-TO-END TESTING LOOP
──────────────────────────────────────────────────────────────────────────────

The whole point of having a generic simulator protocol is to make end-to-end
testing dead simple:

    1. Write Nib source code:        ``let x = 1 + 2``
    2. Compile to IR:                ``nib_compiler.compile(source)``
    3. Assemble to machine bytes:    ``assembler.assemble(ir)``
    4. Execute:                      ``result = sim.execute(binary)``
    5. Assert state:                 ``assert result.final_state.accumulator == 3``

Step 4 is the same line of code regardless of whether the simulator is an
Intel 4004, ARM, or RISC-V simulator.  That uniformity is what this protocol
buys you.

``ExecutionResult.ok`` is a one-liner shorthand for the common case:

    if result.ok:
        print("Program ran to completion cleanly!")
    else:
        print(f"Error: {result.error}")

──────────────────────────────────────────────────────────────────────────────
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Generic, Protocol, TypeVar, runtime_checkable

# ---------------------------------------------------------------------------
# Type variables
# ---------------------------------------------------------------------------

StateT = TypeVar("StateT", covariant=True)  # type: ignore[misc]


# ---------------------------------------------------------------------------
# StepTrace
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class StepTrace:
    """Record of a single instruction execution.

    Frozen (immutable) so that callers cannot accidentally mutate a trace
    after it has been appended to the trace list.

    Attributes
    ----------
    pc_before:
        The program counter *before* the instruction was executed.  This is
        the ROM/memory address from which the instruction was fetched.  Always
        expressed in terms of the architecture's native address space (e.g.,
        0–4095 for the Intel 4004's 12-bit ROM).
    pc_after:
        The program counter *after* the instruction has executed.  For most
        instructions this is ``pc_before + instruction_size``.  For jumps,
        calls, and returns it will be the jump target.
    mnemonic:
        The disassembled instruction name as a short human-readable string.
        Examples: ``"NOP"``, ``"ADD R2"``, ``"JUN 0x100"``, ``"LDM 7"``.
        This is what a human would see in an assembly listing.
    description:
        A longer plain-English description of what this instruction did.
        Examples: ``"LDM 7 @ 0x003"``, ``"ADD R2 @ 0x010"``.

    Examples
    --------
    >>> trace = StepTrace(pc_before=0, pc_after=1, mnemonic="NOP", description="NOP @ 0x000")
    >>> trace.pc_before
    0
    >>> trace.mnemonic
    'NOP'
    >>> # Frozen — mutations raise FrozenInstanceError:
    >>> trace.pc_before = 99  # doctest: +ELLIPSIS
    Traceback (most recent call last):
        ...
    dataclasses.FrozenInstanceError: ...
    """

    pc_before: int
    pc_after: int
    mnemonic: str
    description: str


# ---------------------------------------------------------------------------
# ExecutionResult
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class ExecutionResult(Generic[StateT]):
    """Result of running a program to completion (or hitting max_steps).

    Frozen (immutable) so the result is a true snapshot — you cannot
    accidentally mutate it after the fact.

    Attributes
    ----------
    halted:
        ``True`` if execution stopped because the program executed a HALT
        instruction.  ``False`` if execution stopped because ``max_steps``
        was exceeded — i.e., the program may still be running.
    steps:
        Total number of instructions executed.
    final_state:
        Full snapshot of the CPU/VM state at the moment execution stopped.
        The type of this snapshot is architecture-specific: for the Intel 4004
        it is ``Intel4004State``; for ARM it would be ``ARMState``, etc.
    error:
        ``None`` if execution ended cleanly (program ran to HALT with no
        errors).  A non-None string describes what went wrong — e.g.,
        ``"max_steps (100000) exceeded"`` or ``"illegal opcode 0xFF"``.
    traces:
        Full execution trace, one ``StepTrace`` per instruction executed.
        The list is in order from first instruction to last.

    Properties
    ----------
    ok:
        ``True`` when the program halted cleanly with no error.  This is
        the primary way to check success:

            result = sim.execute(binary)
            if result.ok:
                state = result.final_state
            else:
                print(result.error)

    Examples
    --------
    >>> from dataclasses import dataclass
    >>> @dataclass(frozen=True)
    ... class FakeState:
    ...     value: int
    ...
    >>> # Success path
    >>> trace = StepTrace(pc_before=0, pc_after=1, mnemonic="HLT", description="HLT @ 0x000")
    >>> result = ExecutionResult(halted=True, steps=1, final_state=FakeState(42), error=None, traces=[trace])
    >>> result.ok
    True
    >>> result.final_state.value
    42
    >>>
    >>> # Max-steps exceeded path
    >>> result2 = ExecutionResult(halted=False, steps=100000, final_state=FakeState(0), error="max_steps (100000) exceeded", traces=[])
    >>> result2.ok
    False
    >>> result2.error
    'max_steps (100000) exceeded'
    """

    halted: bool
    steps: int
    final_state: StateT
    error: str | None
    traces: list[StepTrace] = field(default_factory=list)

    @property
    def ok(self) -> bool:
        """Return True if the program halted cleanly with no error.

        A program is considered "ok" only when *both* conditions hold:
          1. It reached a HALT instruction (``halted == True``).
          2. No error was recorded (``error is None``).

        Hitting ``max_steps`` without halting is NOT ok — the program may be
        in an infinite loop or may need more time to finish.

        This is the primary success check in end-to-end tests:

            result = sim.execute(binary)
            assert result.ok, f"Program failed: {result.error}"
        """
        return self.halted and self.error is None


# ---------------------------------------------------------------------------
# Simulator Protocol
# ---------------------------------------------------------------------------
@runtime_checkable
class Simulator(Protocol[StateT]):
    """Generic interface all architecture simulators implement.

    StateT is the architecture-specific state snapshot type (e.g.
    ``Intel4004State``, ``ARMState``, ``RiscVState``).  The protocol is
    *structural* — no explicit inheritance needed.  Any class that has
    these five methods with the right signatures automatically satisfies it.

    Think of this like an electrical socket standard.  The socket (protocol)
    defines the interface; any appliance (simulator) that fits the socket shape
    works, regardless of who manufactured it or what it does internally.

    Type Parameter
    --------------
    StateT:
        The type of the state snapshot returned by ``get_state()`` and stored
        in ``ExecutionResult.final_state``.  It should be an immutable
        (frozen) dataclass capturing the complete CPU state at a point in time.

    Methods
    -------
    load(program):
        Load binary machine code into the simulator's program memory.
        The program is a ``bytes`` object — the raw bytes you would burn into
        a ROM chip or flash into a microcontroller.

    step():
        Execute exactly one instruction and return a ``StepTrace`` describing
        what happened.  Useful for step-through debugging and for building the
        trace list manually.  Raises ``RuntimeError`` if the CPU is halted.

    execute(program, max_steps):
        The main entry point for end-to-end testing.  Loads the program,
        resets state, runs until HALT or ``max_steps``, and returns a
        full ``ExecutionResult``.

    get_state():
        Return a *frozen snapshot* of the current CPU state.  This should be
        a new immutable object every time — changes to the simulator's internal
        state after calling ``get_state()`` must not affect the snapshot.

    reset():
        Reset all CPU state back to power-on defaults.  Registers cleared,
        PC at 0, carry cleared, RAM zeroed, stack empty.

    Usage
    -----
    Implementing a concrete simulator::

        from simulator_protocol import Simulator, ExecutionResult, StepTrace
        from dataclasses import dataclass

        @dataclass(frozen=True)
        class ToyState:
            accumulator: int
            pc: int

        class ToySimulator:
            def __init__(self) -> None:
                self._acc = 0
                self._pc = 0
                self._halted = False

            def load(self, program: bytes) -> None:
                self._program = program

            def step(self) -> StepTrace:
                ...  # decode and execute one instruction

            def execute(self, program: bytes, max_steps: int = 100_000) -> ExecutionResult[ToyState]:
                ...  # load + run loop

            def get_state(self) -> ToyState:
                return ToyState(accumulator=self._acc, pc=self._pc)

            def reset(self) -> None:
                self._acc = 0
                self._pc = 0
                self._halted = False

    Using the protocol as a type annotation::

        def run_test(sim: Simulator[ToyState], binary: bytes) -> ToyState:
            result = sim.execute(binary)
            if not result.ok:
                raise RuntimeError(f"Simulation failed: {result.error}")
            return result.final_state
    """

    def load(self, program: bytes) -> None:
        """Load a binary program into the simulator's program memory.

        Parameters
        ----------
        program:
            Raw machine-code bytes.  For ROM-based architectures (Intel 4004,
            ARM Thumb), this is written to instruction memory starting at
            address 0.  For RAM-based architectures, it is loaded into the
            appropriate memory region.
        """
        ...

    def step(self) -> StepTrace:
        """Execute a single instruction and return a trace of what happened.

        Returns
        -------
        StepTrace:
            The trace of the instruction that was just executed, including
            the PC before and after, the mnemonic, and a description.

        Raises
        ------
        RuntimeError:
            If the CPU is halted (cannot execute further instructions) or
            if no program has been loaded.
        """
        ...

    def execute(self, program: bytes, max_steps: int = 100_000) -> ExecutionResult[StateT]:
        """Load program, run to HALT or max_steps, return full result.

        This is the main entry point for end-to-end testing.  It:
          1. Resets the simulator to power-on state.
          2. Loads the program bytes into memory.
          3. Executes instructions one by one.
          4. Stops at HALT or when ``max_steps`` is reached.
          5. Returns an ``ExecutionResult`` with the final state and trace.

        Parameters
        ----------
        program:
            The binary machine-code bytes to execute.
        max_steps:
            Maximum number of instructions to execute before giving up.
            Default is 100,000.  Programs that exceed this are likely in
            an infinite loop or need a higher limit.

        Returns
        -------
        ExecutionResult[StateT]:
            Full result including:
            - ``halted``: whether HALT was reached
            - ``steps``: how many instructions executed
            - ``final_state``: snapshot of CPU state at termination
            - ``error``: None on clean halt, error string otherwise
            - ``traces``: full per-instruction trace list
        """
        ...

    def get_state(self) -> StateT:
        """Return a frozen snapshot of the current internal state.

        The returned object must be immutable — it must not change even if
        the simulator continues executing.  In practice, this means returning
        a new frozen dataclass instance with copies of all mutable collections
        (lists converted to tuples, etc.).

        Returns
        -------
        StateT:
            An immutable snapshot of the CPU's state at this moment.
        """
        ...

    def reset(self) -> None:
        """Reset the simulator to its initial power-on state.

        After ``reset()``:
        - All registers are cleared to 0.
        - The program counter is at 0.
        - Carry/overflow flags are cleared.
        - RAM/memory is zeroed.
        - The hardware stack is empty.
        - ``halted`` is False.

        Any previously loaded program remains in ROM/flash memory if the
        architecture separates ROM from RAM, but all *execution state* is
        reset.
        """
        ...
