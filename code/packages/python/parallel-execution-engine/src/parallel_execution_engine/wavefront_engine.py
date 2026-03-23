"""WavefrontEngine — SIMD parallel execution (AMD GCN/RDNA style).

=== What is a Wavefront? ===

AMD calls their parallel execution unit a "wavefront." It's 64 lanes on GCN
(Graphics Core Next) or 32 lanes on RDNA (Radeon DNA). A wavefront is
fundamentally different from an NVIDIA warp:

    NVIDIA Warp (SIMT):                AMD Wavefront (SIMD):
    ┌──────────────────────────┐       ┌──────────────────────────┐
    │ 32 threads               │       │ 32 lanes                 │
    │ Each has its own regs    │       │ ONE vector register file  │
    │ Logically own PC         │       │ ONE program counter       │
    │ HW manages divergence    │       │ Explicit EXEC mask        │
    └──────────────────────────┘       └──────────────────────────┘

The critical architectural difference:

    SIMT (NVIDIA): "32 independent threads that HAPPEN to run together"
    SIMD (AMD):    "1 instruction that operates on a 32-wide vector"

In SIMT, thread 7 has its own R0 register. In SIMD, there IS no "thread 7"
— there's lane 7 of vector register v0, which is v0[7].

=== AMD's Two Register Files ===

AMD wavefronts have TWO types of registers, which is architecturally unique:

    Vector GPRs (VGPRs):              Scalar GPRs (SGPRs):
    ┌────────────────────────┐        ┌────────────────────────┐
    │ v0: [l0][l1]...[l31]  │        │ s0:  42.0              │
    │ v1: [l0][l1]...[l31]  │        │ s1:  3.14              │
    │ ...                    │        │ ...                    │
    │ v255:[l0][l1]...[l31]  │        │ s103: 0.0              │
    └────────────────────────┘        └────────────────────────┘
    One value PER LANE                One value for ALL LANES

SGPRs are used for values that are the SAME across all lanes: constants,
loop counters, memory base addresses. This is efficient — compute the
address ONCE in scalar, then use it in every lane. NVIDIA doesn't have
this distinction; every thread computes everything independently.

=== The EXEC Mask ===

AMD uses a register called EXEC to control which lanes execute each
instruction. Unlike NVIDIA's hardware-managed divergence, the EXEC mask
is explicitly set by instructions:

    v_cmp_lt_f32 vcc, v0, s0     // Compare: which lanes have v0 < s0?
    s_and_saveexec_b32 s[2:3], vcc  // EXEC = EXEC & vcc, save old EXEC
    // ... only lanes where v0 < s0 execute here ...
    s_or_b32 exec, exec, s[2:3]  // Restore full EXEC mask

This explicit mask management is the programmer's (or compiler's)
responsibility, unlike NVIDIA where the hardware manages it automatically.

=== Simplification for Our Simulator ===

For educational clarity, we use GPUCore instances per lane internally
(just like WarpEngine), but expose the AMD-style interface externally:
vector registers, scalar registers, and explicit EXEC mask. This lets
students see the SIMD-vs-SIMT distinction in the API without needing
a completely different execution backend.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING

from fp_arithmetic import FP32, FloatFormat, bits_to_float, float_to_bits
from gpu_core import GenericISA, GPUCore

from parallel_execution_engine.protocols import (
    DivergenceInfo,
    EngineTrace,
    ExecutionModel,
)

if TYPE_CHECKING:
    from clock import Clock, ClockEdge
    from gpu_core import Instruction, InstructionSet


# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------


@dataclass
class WavefrontConfig:
    """Configuration for an AMD-style SIMD wavefront engine.

    Real-world reference values:

        Architecture │ Wave Width │ VGPRs │ SGPRs │ LDS
        ─────────────┼────────────┼───────┼───────┼─────────
        AMD GCN      │ 64         │ 256   │ 104   │ 64 KB
        AMD RDNA     │ 32         │ 256   │ 104   │ 64 KB
        Our default  │ 32         │ 256   │ 104   │ 64 KB

    Fields:
        wave_width:    Number of SIMD lanes (64 for GCN, 32 for RDNA).
        num_vgprs:     Vector general-purpose registers per lane.
        num_sgprs:     Scalar general-purpose registers (shared by all lanes).
        lds_size:      Local Data Store size in bytes (shared memory).
        float_format:  FP format for register values.
        isa:           Instruction set to use.
    """

    wave_width: int = 32
    num_vgprs: int = 256
    num_sgprs: int = 104
    lds_size: int = 65536
    float_format: FloatFormat = FP32
    isa: InstructionSet = field(default_factory=GenericISA)


# ---------------------------------------------------------------------------
# Vector Register File — one value per lane per register
# ---------------------------------------------------------------------------


class VectorRegisterFile:
    """AMD-style vector register file: num_vgprs registers x wave_width lanes.

    Each "register" is actually a vector of wave_width values. When you
    write to v3[lane 5], you're writing to one slot in a 2D array:

        ┌────────────────────────────────────────────┐
        │         Lane 0   Lane 1   Lane 2  ...      │
        │ v0:    [ 1.0  ] [ 2.0  ] [ 3.0  ]  ...    │
        │ v1:    [ 0.5  ] [ 0.5  ] [ 0.5  ]  ...    │
        │ v2:    [ 0.0  ] [ 0.0  ] [ 0.0  ]  ...    │
        │ ...                                        │
        └────────────────────────────────────────────┘

    This is fundamentally different from NVIDIA where each thread has
    its own separate register file. Here, ALL lanes share ONE register
    file, but each lane gets its own "column" within each register.
    """

    def __init__(
        self,
        num_vgprs: int,
        wave_width: int,
        fmt: FloatFormat = FP32,
    ) -> None:
        self.num_vgprs = num_vgprs
        self.wave_width = wave_width
        self.fmt = fmt
        self._zero = float_to_bits(0.0, fmt)
        # 2D storage: _data[reg_index][lane_index] = FloatBits
        self._data = [
            [float_to_bits(0.0, fmt) for _ in range(wave_width)]
            for _ in range(num_vgprs)
        ]

    def read(self, vreg: int, lane: int) -> float:
        """Read one lane of a vector register as a Python float."""
        return bits_to_float(self._data[vreg][lane])

    def write(self, vreg: int, lane: int, value: float) -> None:
        """Write a Python float to one lane of a vector register."""
        self._data[vreg][lane] = float_to_bits(value, self.fmt)

    def read_all_lanes(self, vreg: int) -> list[float]:
        """Read all lanes of a vector register."""
        return [
            bits_to_float(self._data[vreg][lane])
            for lane in range(self.wave_width)
        ]


# ---------------------------------------------------------------------------
# Scalar Register File — one value shared across all lanes
# ---------------------------------------------------------------------------


class ScalarRegisterFile:
    """AMD-style scalar register file: num_sgprs single-value registers.

    Scalar registers hold values that are the SAME for all lanes:
    constants, loop counters, memory base addresses. Computing these
    once in scalar instead of per-lane saves power and register space.

        ┌─────────────────────────┐
        │ s0:   42.0              │  ← same for all lanes
        │ s1:   3.14159           │
        │ s2:   0.0               │
        │ ...                     │
        │ s103: 0.0               │
        └─────────────────────────┘
    """

    def __init__(self, num_sgprs: int, fmt: FloatFormat = FP32) -> None:
        self.num_sgprs = num_sgprs
        self.fmt = fmt
        self._data = [float_to_bits(0.0, fmt) for _ in range(num_sgprs)]

    def read(self, sreg: int) -> float:
        """Read a scalar register as a Python float."""
        return bits_to_float(self._data[sreg])

    def write(self, sreg: int, value: float) -> None:
        """Write a Python float to a scalar register."""
        self._data[sreg] = float_to_bits(value, self.fmt)


# ---------------------------------------------------------------------------
# WavefrontEngine — the SIMD parallel execution engine
# ---------------------------------------------------------------------------


class WavefrontEngine:
    """SIMD wavefront execution engine (AMD GCN/RDNA style).

    One instruction stream, one wide vector ALU, explicit EXEC mask.
    Internally uses GPUCore per lane for instruction execution, but
    exposes the AMD-style vector/scalar register interface.

    === Key Differences from WarpEngine ===

    1. ONE program counter (not per-thread PCs).
    2. Vector registers are a 2D array (vreg x lane), not per-thread.
    3. Scalar registers are shared across all lanes.
    4. EXEC mask is explicitly controlled, not hardware-managed.
    5. No divergence stack — mask management is programmer/compiler's job.

    Example:
        >>> from clock import Clock
        >>> from gpu_core import limm, fmul, halt
        >>> clock = Clock()
        >>> engine = WavefrontEngine(WavefrontConfig(wave_width=4), clock)
        >>> engine.load_program([limm(0, 2.0), fmul(2, 0, 1), halt()])
        >>> for lane in range(4):
        ...     engine.set_lane_register(lane, 1, float(lane + 1))
        >>> traces = engine.run()
    """

    def __init__(self, config: WavefrontConfig, clock: Clock) -> None:
        self._config = config
        self._clock = clock
        self._cycle = 0
        self._program: list[Instruction] = []

        # The EXEC mask: True = lane is active, False = lane is masked off.
        # In AMD hardware, EXEC is a 32-bit or 64-bit register. Here we
        # use a list of bools for clarity.
        self._exec_mask: list[bool] = [True] * config.wave_width

        # Vector and scalar register files (AMD-style)
        self._vrf = VectorRegisterFile(
            num_vgprs=config.num_vgprs,
            wave_width=config.wave_width,
            fmt=config.float_format,
        )
        self._srf = ScalarRegisterFile(
            num_sgprs=config.num_sgprs, fmt=config.float_format
        )

        # Internal: one GPUCore per lane for instruction execution.
        # This is a simulation convenience — real AMD hardware has a
        # single wide SIMD ALU, not separate cores per lane.
        self._lanes: list[GPUCore] = [
            GPUCore(
                isa=config.isa,
                fmt=config.float_format,
                num_registers=config.num_vgprs,
                memory_size=config.lds_size // max(config.wave_width, 1),
            )
            for _ in range(config.wave_width)
        ]

        self._all_halted = False

    # --- Properties ---

    @property
    def name(self) -> str:
        """Engine name for traces."""
        return "WavefrontEngine"

    @property
    def width(self) -> int:
        """Number of SIMD lanes."""
        return self._config.wave_width

    @property
    def execution_model(self) -> ExecutionModel:
        """This is a SIMD engine."""
        return ExecutionModel.SIMD

    @property
    def exec_mask(self) -> list[bool]:
        """The current EXEC mask (which lanes are active)."""
        return list(self._exec_mask)

    @property
    def halted(self) -> bool:
        """True if the wavefront has halted."""
        return self._all_halted

    @property
    def config(self) -> WavefrontConfig:
        """The configuration this engine was created with."""
        return self._config

    @property
    def vrf(self) -> VectorRegisterFile:
        """Access to the vector register file."""
        return self._vrf

    @property
    def srf(self) -> ScalarRegisterFile:
        """Access to the scalar register file."""
        return self._srf

    # --- Program loading ---

    def load_program(self, program: list[Instruction]) -> None:
        """Load a program into the wavefront.

        The same program is loaded into all lane cores. Unlike SIMT where
        each thread can (logically) have a different PC, the wavefront has
        ONE shared PC for all lanes.

        Args:
            program: A list of Instructions.
        """
        self._program = list(program)
        for lane in self._lanes:
            lane.load_program(self._program)
        self._exec_mask = [True] * self._config.wave_width
        self._all_halted = False
        self._cycle = 0

    # --- Register setup ---

    def set_lane_register(self, lane: int, vreg: int, value: float) -> None:
        """Set a per-lane vector register value.

        This writes to both the VRF (our AMD-style register file) and
        the internal GPUCore for that lane (for execution).

        Args:
            lane: Which lane (0 to wave_width - 1).
            vreg: Which vector register.
            value: The float value.
        """
        if lane < 0 or lane >= self._config.wave_width:
            msg = f"Lane {lane} out of range [0, {self._config.wave_width})"
            raise IndexError(msg)
        self._vrf.write(vreg, lane, value)
        self._lanes[lane].registers.write_float(vreg, value)

    def set_scalar_register(self, sreg: int, value: float) -> None:
        """Set a scalar register value (shared across all lanes).

        In AMD hardware, scalar values are broadcast to all lanes when
        used in vector instructions. We simulate this by writing the
        value to the SRF.

        Args:
            sreg: Which scalar register.
            value: The float value.
        """
        if sreg < 0 or sreg >= self._config.num_sgprs:
            msg = f"Scalar register {sreg} out of range [0, {self._config.num_sgprs})"
            raise IndexError(msg)
        self._srf.write(sreg, value)

    def set_exec_mask(self, mask: list[bool]) -> None:
        """Explicitly set the EXEC mask.

        In AMD hardware, the EXEC mask is set by comparison instructions:
            v_cmp_lt_f32 vcc, v0, s0    // vcc = which lanes have v0 < s0?
            s_and_saveexec s[2:3], vcc  // EXEC = EXEC & vcc

        In our simulator, you can set it directly for testing.

        Args:
            mask: List of bools, one per lane. True = active.
        """
        if len(mask) != self._config.wave_width:
            msg = (
                f"Mask length {len(mask)} != wave_width "
                f"{self._config.wave_width}"
            )
            raise ValueError(msg)
        self._exec_mask = list(mask)

    # --- Execution ---

    def step(self, clock_edge: ClockEdge) -> EngineTrace:
        """Execute one cycle: issue one instruction to all active lanes.

        Unlike SIMT, ALL lanes share the same PC. The EXEC mask determines
        which lanes actually execute. Masked-off lanes don't update their
        registers, but the PC still advances for the whole wavefront.

        Args:
            clock_edge: The clock edge that triggered this step.

        Returns:
            An EngineTrace describing what happened.
        """
        self._cycle += 1

        if self._all_halted:
            return self._make_halted_trace()

        mask_before = list(self._exec_mask)

        # Execute on active lanes only
        unit_traces: dict[int, str] = {}

        for lane_id in range(self._config.wave_width):
            lane_core = self._lanes[lane_id]
            if self._exec_mask[lane_id] and not lane_core.halted:
                try:
                    trace = lane_core.step()
                    unit_traces[lane_id] = trace.description
                    if trace.halted:
                        unit_traces[lane_id] = "HALTED"
                except RuntimeError:
                    unit_traces[lane_id] = "(error)"
            elif lane_core.halted:
                unit_traces[lane_id] = "(halted)"
            else:
                # Lane is masked off — still advance its PC to stay in sync
                # with the rest of the wavefront. But don't execute.
                # In real AMD HW, masked lanes simply skip the write-back.
                if not lane_core.halted:
                    try:
                        lane_core.step()
                        unit_traces[lane_id] = "(masked — result discarded)"
                    except RuntimeError:
                        unit_traces[lane_id] = "(masked — error)"
                else:
                    unit_traces[lane_id] = "(halted)"

        # Sync VRF with internal core registers for active lanes
        for lane_id in range(self._config.wave_width):
            if self._exec_mask[lane_id]:
                for vreg in range(min(self._config.num_vgprs, 32)):
                    val = self._lanes[lane_id].registers.read_float(vreg)
                    self._vrf.write(vreg, lane_id, val)

        # Check if all lanes halted
        if all(lane.halted for lane in self._lanes):
            self._all_halted = True

        active_count = sum(
            1
            for i in range(self._config.wave_width)
            if self._exec_mask[i] and not self._lanes[i].halted
        )
        total = self._config.wave_width

        # Build description
        first_desc = next(
            (
                unit_traces[i]
                for i in range(self._config.wave_width)
                if i in unit_traces
                and unit_traces[i]
                not in (
                    "(masked — result discarded)",
                    "(halted)",
                    "(error)",
                    "(masked — error)",
                    "HALTED",
                )
            ),
            "no active lanes",
        )

        current_mask = [
            self._exec_mask[i] and not self._lanes[i].halted
            for i in range(self._config.wave_width)
        ]

        return EngineTrace(
            cycle=self._cycle,
            engine_name=self.name,
            execution_model=self.execution_model,
            description=f"{first_desc} — {active_count}/{total} lanes active",
            unit_traces=unit_traces,
            active_mask=current_mask,
            active_count=active_count,
            total_count=total,
            utilization=active_count / total if total > 0 else 0.0,
            divergence_info=DivergenceInfo(
                active_mask_before=mask_before,
                active_mask_after=list(self._exec_mask),
                reconvergence_pc=-1,
                divergence_depth=0,
            ),
        )

    def run(self, max_cycles: int = 10000) -> list[EngineTrace]:
        """Run until all lanes halt or max_cycles reached.

        Args:
            max_cycles: Safety limit.

        Returns:
            List of EngineTrace records.
        """
        from clock import ClockEdge

        traces: list[EngineTrace] = []
        for cycle_num in range(1, max_cycles + 1):
            edge = ClockEdge(
                cycle=cycle_num, value=1, is_rising=True, is_falling=False
            )
            trace = self.step(edge)
            traces.append(trace)
            if self._all_halted:
                break
        else:
            if not self._all_halted:
                msg = f"WavefrontEngine: max_cycles ({max_cycles}) reached"
                raise RuntimeError(msg)
        return traces

    def _make_halted_trace(self) -> EngineTrace:
        """Produce a trace for when all lanes are halted."""
        return EngineTrace(
            cycle=self._cycle,
            engine_name=self.name,
            execution_model=self.execution_model,
            description="All lanes halted",
            unit_traces={
                i: "(halted)" for i in range(self._config.wave_width)
            },
            active_mask=[False] * self._config.wave_width,
            active_count=0,
            total_count=self._config.wave_width,
            utilization=0.0,
        )

    def reset(self) -> None:
        """Reset to initial state."""
        for lane in self._lanes:
            lane.reset()
            if self._program:
                lane.load_program(self._program)
        self._exec_mask = [True] * self._config.wave_width
        self._all_halted = False
        self._cycle = 0
        self._vrf = VectorRegisterFile(
            num_vgprs=self._config.num_vgprs,
            wave_width=self._config.wave_width,
            fmt=self._config.float_format,
        )
        self._srf = ScalarRegisterFile(
            num_sgprs=self._config.num_sgprs, fmt=self._config.float_format
        )

    def __repr__(self) -> str:
        active = sum(self._exec_mask)
        return (
            f"WavefrontEngine(width={self._config.wave_width}, "
            f"active_lanes={active}, halted={self._all_halted})"
        )
