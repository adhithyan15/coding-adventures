"""Intel8080GateLevelSimulator — SIM00-conforming gate-level simulator.

This is the top-level simulator class that implements the
`Simulator[Intel8080State]` protocol from `simulator_protocol`.

It delegates all execution to `ControlUnit`, which in turn uses:
  - `Decoder8080` for combinational instruction decode
  - `ALU8080` for gate-level arithmetic/logic
  - `RegisterFile` (Register8 arrays) for register storage
  - `Register16` (16 D flip-flop arrays) for PC and SP

The output type is `Intel8080State` — the same frozen dataclass used by
the behavioral simulator. Both simulators are interchangeable as
`Simulator[Intel8080State]` and produce bit-for-bit identical output.

=== How to use ===

    from intel8080_gatelevel import Intel8080GateLevelSimulator

    sim = Intel8080GateLevelSimulator()
    result = sim.execute(bytes([
        0x3E, 0x0A,  # MVI A, 10
        0x06, 0x05,  # MVI B, 5
        0x80,        # ADD B   (routes through 8 full-adder stages)
        0x76,        # HLT
    ]))
    assert result.final_state.a == 15
    assert result.final_state.flag_cy is False
"""

from __future__ import annotations

from intel8080_simulator import Intel8080State
from simulator_protocol import ExecutionResult, StepTrace

from intel8080_gatelevel.control import ControlUnit
from intel8080_gatelevel.register_file import (
    REG_A,
    REG_B,
    REG_C,
    REG_D,
    REG_E,
    REG_H,
    REG_L,
)


class Intel8080GateLevelSimulator:
    """Gate-level simulator for the Intel 8080A microprocessor.

    Implements `Simulator[Intel8080State]`. Every arithmetic/logic operation
    routes through real gate functions from the `logic_gates` and `arithmetic`
    packages. No host arithmetic shortcuts.

    Usage:
        >>> sim = Intel8080GateLevelSimulator()
        >>> result = sim.execute(bytes([0x3E, 0x0A, 0x76]))  # MVI A,10; HLT
        >>> result.final_state.a
        10

    I/O ports:
        >>> sim.set_input_port(5, 0xAB)
        >>> result = sim.execute(bytes([0xDB, 0x05, 0x76]))  # IN 5; HLT
        >>> result.final_state.a
        0xAB
    """

    def __init__(self) -> None:
        """Create a gate-level simulator with a fresh ControlUnit."""
        self._cu = ControlUnit()

    # ─── SIM00 Protocol ───────────────────────────────────────────────────

    def reset(self) -> None:
        """Reset the simulator to power-on state.

        Clears all registers, flags, and halted status. Memory is preserved.
        """
        self._cu.reset()

    def load(self, program: bytes) -> None:
        """Load a program into memory starting at address 0x0000.

        Overwrites any previous program. Addresses beyond the program
        length retain their previous values (or zero after reset).

        Args:
            program: Byte sequence to load. Length must be ≤ 65536.
        """
        for i, byte in enumerate(program):
            self._cu._memory[i] = byte  # noqa: SLF001

    def step(self) -> StepTrace:
        """Execute one instruction, returning a trace.

        Returns:
            StepTrace with pc_before, pc_after, mnemonic, description.

        Raises:
            RuntimeError: If the simulator is halted and step() is called.
        """
        if self._cu.halted:
            msg = "CPU is halted — call reset() before stepping again"
            raise RuntimeError(msg)
        trace = self._cu.step()
        if trace is None:
            msg = "step() returned None (halted state)"
            raise RuntimeError(msg)
        return trace

    def execute(self, program: bytes, *, max_steps: int = 100_000) -> ExecutionResult:
        """Load and execute a program until HLT or max_steps.

        Pre-loaded input ports are preserved across the internal reset.

        Args:
            program:   Bytecode to execute.
            max_steps: Safety limit to prevent infinite loops (default 100,000).

        Returns:
            ExecutionResult with halted, steps, final_state, traces, error.
        """
        # Preserve input ports across reset
        saved_ports = list(self._cu._input_ports)  # noqa: SLF001
        saved_outputs = list(self._cu._output_ports)  # noqa: SLF001

        self.reset()
        self._cu._input_ports = saved_ports  # noqa: SLF001
        self._cu._output_ports = saved_outputs  # noqa: SLF001
        self.load(program)

        traces: list[StepTrace] = []
        error: str | None = None
        steps = 0

        try:
            while not self._cu.halted and steps < max_steps:
                trace = self._cu.step()
                if trace is not None:
                    traces.append(trace)
                steps += 1
        except Exception as exc:  # noqa: BLE001
            error = str(exc)

        return ExecutionResult(
            halted=self._cu.halted,
            steps=steps,
            final_state=self.get_state(),
            traces=traces,
            error=error,
        )

    def get_state(self) -> Intel8080State:
        """Return an immutable snapshot of the current CPU state.

        Reads all register values from the flip-flop arrays, captures
        flag states, and packages them into an Intel8080State.
        """
        cu = self._cu
        rf = cu._rf  # noqa: SLF001
        flags = cu._flags  # noqa: SLF001

        # Read 7 working registers from their flip-flop arrays
        a = rf.read(REG_A)
        b = rf.read(REG_B)
        c = rf.read(REG_C)
        d = rf.read(REG_D)
        e = rf.read(REG_E)
        h = rf.read(REG_H)
        lo = rf.read(REG_L)

        return Intel8080State(
            a=a, b=b, c=c, d=d, e=e, h=h, l=lo,
            sp=cu._sp.read(),  # noqa: SLF001
            pc=cu._pc.read(),  # noqa: SLF001
            flag_s=flags.s,
            flag_z=flags.z,
            flag_ac=flags.ac,
            flag_p=flags.p,
            flag_cy=flags.cy,
            interrupts_enabled=cu._inte,  # noqa: SLF001
            halted=cu.halted,
            memory=tuple(cu._memory),  # noqa: SLF001
            input_ports=tuple(cu._input_ports),  # noqa: SLF001
            output_ports=tuple(cu._output_ports),  # noqa: SLF001
        )

    # ─── I/O port configuration ──────────────────────────────────────────

    def set_input_port(self, port: int, value: int) -> None:
        """Pre-load an input port value.

        In the real 8080, input ports are driven by external hardware.
        This method simulates that external hardware setting the port value
        before an IN instruction reads it.

        Args:
            port:  Port number (0–255).
            value: Byte value (0–255).

        Raises:
            ValueError: If port or value is out of range.
        """
        if not 0 <= port <= 255:
            msg = f"port must be 0–255, got {port}"
            raise ValueError(msg)
        if not 0 <= value <= 255:
            msg = f"value must be 0–255, got {value}"
            raise ValueError(msg)
        self._cu._input_ports[port] = value  # noqa: SLF001

    def get_output_port(self, port: int) -> int:
        """Read the current value of an output port.

        Args:
            port: Port number (0–255).

        Returns:
            Last byte written to this port by an OUT instruction (0 if never written).

        Raises:
            ValueError: If port is out of range.
        """
        if not 0 <= port <= 255:
            msg = f"port must be 0–255, got {port}"
            raise ValueError(msg)
        return self._cu._output_ports[port]  # noqa: SLF001
