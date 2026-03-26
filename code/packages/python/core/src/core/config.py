"""CoreConfig -- complete configuration for a processor core.

# The Core: a Motherboard for Micro-Architecture

A processor core is not a single piece of hardware. It is a composition of
many sub-components, each independently designed and tested:

  - Pipeline (D04): moves instructions through stages (IF, ID, EX, MEM, WB)
  - Branch Predictor (D02): guesses which way branches will go
  - Hazard Detection (D03): detects data, control, and structural hazards
  - Cache Hierarchy (D01): L1I, L1D, optional L2 for fast memory access
  - Register File: fast storage for operands and results
  - Clock: drives everything in lockstep

The Core itself defines no new micro-architectural behavior. It wires the
parts together, like a motherboard connects CPU, RAM, and peripherals.
The same Core can run ARM, RISC-V, or any custom ISA -- the ISA decoder
is injected from outside.

# Configuration

Every parameter that a real CPU architect would tune is exposed in
CoreConfig. Change the branch predictor and you get different accuracy.
Double the L1 cache and you get fewer misses. Deepen the pipeline and
you get higher clock speeds but worse misprediction penalties.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from branch_predictor import (
    AlwaysNotTakenPredictor,
    AlwaysTakenPredictor,
    BackwardTakenForwardNotTaken,
    BranchPredictor,
    OneBitPredictor,
    TwoBitPredictor,
    TwoBitState,
)
from cache import CacheConfig
from cpu_pipeline import PipelineConfig, classic_5_stage, deep_13_stage

# =========================================================================
# RegisterFileConfig -- configuration for the register file
# =========================================================================


@dataclass(frozen=True)
class RegisterFileConfig:
    """Configuration for the general-purpose register file.

    Real-world register file sizes:

        MIPS:     32 registers, 32-bit  (R0 hardwired to zero)
        ARMv8:    31 registers, 64-bit  (X0-X30, no zero register)
        RISC-V:   32 registers, 32/64-bit (x0 hardwired to zero)
        x86-64:   16 registers, 64-bit  (RAX, RBX, ..., R15)

    The zero_register convention (RISC-V, MIPS) simplifies instruction
    encoding: any instruction can discard its result by writing to R0,
    and any instruction can use "zero" as an operand without a special
    immediate encoding.

    Attributes:
        count: Number of general-purpose registers. Typical: 16 or 32.
        width: Bit width of each register: 32 or 64.
        zero_register: Whether register 0 is hardwired to zero.
    """

    count: int = 16
    width: int = 32
    zero_register: bool = True


def default_register_file_config() -> RegisterFileConfig:
    """Return sensible defaults: 16 registers, 32-bit, R0 hardwired to zero."""
    return RegisterFileConfig(count=16, width=32, zero_register=True)


# =========================================================================
# FPUnitConfig -- configuration for the floating-point unit
# =========================================================================


@dataclass(frozen=True)
class FPUnitConfig:
    """Configuration for the optional floating-point unit.

    Not all cores have an FP unit. Microcontrollers (ARM Cortex-M0) and
    efficiency cores often omit it to save area and power. When fp_unit
    is None in CoreConfig, the core has no floating-point support.

    Attributes:
        formats: Supported FP formats, e.g. ["fp16", "fp32", "fp64"].
        pipeline_depth: How many cycles an FP operation takes.
            Typical: 3-5 for add/multiply, 10-20 for divide.
    """

    formats: tuple[str, ...] = ()
    pipeline_depth: int = 4


# =========================================================================
# CoreConfig -- complete configuration for a processor core
# =========================================================================


@dataclass
class CoreConfig:
    """Every tunable parameter for a processor core.

    This is the "spec sheet" for the core. A CPU architect decides these
    values based on the target workload, power budget, and die area.

    Changing any parameter affects measurable performance:

        Deeper pipeline         -> higher clock speed, worse misprediction penalty
        Better branch predictor -> fewer pipeline flushes
        Larger L1 cache         -> fewer cache misses
        More registers          -> fewer spills to memory
        Forwarding enabled      -> fewer stall cycles

    Attributes:
        name: Human-readable identifier (e.g. "Simple", "CortexA78Like").
        pipeline: Pipeline stage configuration.
        branch_predictor_type: Algorithm name for the predictor.
        branch_predictor_size: Number of entries in the prediction table.
        btb_size: Number of entries in the Branch Target Buffer.
        hazard_detection: Whether hazard detection is enabled.
        forwarding: Whether data forwarding paths are enabled.
        register_file: Register file configuration, or None for defaults.
        fp_unit: Floating-point unit config, or None for no FP support.
        l1i_cache: L1 instruction cache config, or None for defaults.
        l1d_cache: L1 data cache config, or None for defaults.
        l2_cache: Unified L2 cache config, or None for no L2.
        memory_size: Main memory size in bytes. Default: 65536 (64KB).
        memory_latency: DRAM access latency in cycles. Default: 100.
    """

    name: str = "Default"

    # --- Pipeline ---
    pipeline: PipelineConfig = field(default_factory=classic_5_stage)

    # --- Branch Prediction ---
    branch_predictor_type: str = "static_always_not_taken"
    branch_predictor_size: int = 256
    btb_size: int = 64

    # --- Hazard Handling ---
    hazard_detection: bool = True
    forwarding: bool = True

    # --- Register File ---
    register_file: RegisterFileConfig | None = None

    # --- Floating Point ---
    fp_unit: FPUnitConfig | None = None

    # --- Cache Hierarchy ---
    l1i_cache: CacheConfig | None = None
    l1d_cache: CacheConfig | None = None
    l2_cache: CacheConfig | None = None

    # --- Memory ---
    memory_size: int = 65536
    memory_latency: int = 100


def default_core_config() -> CoreConfig:
    """Return a minimal, sensible configuration for testing.

    This is the "teaching core" -- a 5-stage pipeline with static prediction,
    small caches, and 16 registers. Equivalent to a 1980s RISC microprocessor.
    """
    return CoreConfig(
        name="Default",
        pipeline=classic_5_stage(),
        branch_predictor_type="static_always_not_taken",
        branch_predictor_size=256,
        btb_size=64,
        hazard_detection=True,
        forwarding=True,
        register_file=None,
        fp_unit=None,
        l1i_cache=None,
        l1d_cache=None,
        l2_cache=None,
        memory_size=65536,
        memory_latency=100,
    )


# =========================================================================
# Preset Configurations -- famous real-world cores approximated
# =========================================================================


def simple_config() -> CoreConfig:
    """Return a minimal teaching core.

    Inspired by the MIPS R2000 (1985):
      - 5-stage pipeline (IF, ID, EX, MEM, WB)
      - Static predictor (always not taken)
      - 4KB direct-mapped L1I and L1D caches
      - No L2 cache
      - 16 registers, 32-bit
      - No floating point

    Expected IPC: ~0.7-0.9 on simple programs.
    """
    l1i = CacheConfig(
        name="L1I",
        total_size=4096,
        line_size=64,
        associativity=1,
        access_latency=1,
        write_policy="write-back",
    )
    l1d = CacheConfig(
        name="L1D",
        total_size=4096,
        line_size=64,
        associativity=1,
        access_latency=1,
        write_policy="write-back",
    )
    reg_cfg = RegisterFileConfig(count=16, width=32, zero_register=True)

    return CoreConfig(
        name="Simple",
        pipeline=classic_5_stage(),
        branch_predictor_type="static_always_not_taken",
        branch_predictor_size=256,
        btb_size=64,
        hazard_detection=True,
        forwarding=True,
        register_file=reg_cfg,
        fp_unit=None,
        l1i_cache=l1i,
        l1d_cache=l1d,
        l2_cache=None,
        memory_size=65536,
        memory_latency=100,
    )


def cortex_a78_like_config() -> CoreConfig:
    """Approximate the ARM Cortex-A78 performance core.

    The Cortex-A78 (2020) is used in Snapdragon 888 and Dimensity 9000:
      - 13-stage pipeline (deep for high frequency)
      - 2-bit predictor with 4096 entries (simplified vs real TAGE)
      - 64KB 4-way L1I and L1D
      - 256KB 8-way L2
      - 31 registers, 64-bit (ARMv8)
      - FP32 and FP64 support

    Expected IPC: ~0.85-0.95 (our model is in-order; real A78 is out-of-order).
    """
    l1i = CacheConfig(
        name="L1I",
        total_size=65536,
        line_size=64,
        associativity=4,
        access_latency=1,
        write_policy="write-back",
    )
    l1d = CacheConfig(
        name="L1D",
        total_size=65536,
        line_size=64,
        associativity=4,
        access_latency=1,
        write_policy="write-back",
    )
    l2 = CacheConfig(
        name="L2",
        total_size=262144,
        line_size=64,
        associativity=8,
        access_latency=12,
        write_policy="write-back",
    )
    reg_cfg = RegisterFileConfig(count=31, width=64, zero_register=False)
    fp_cfg = FPUnitConfig(formats=("fp32", "fp64"), pipeline_depth=4)

    return CoreConfig(
        name="CortexA78Like",
        pipeline=deep_13_stage(),
        branch_predictor_type="two_bit",
        branch_predictor_size=4096,
        btb_size=1024,
        hazard_detection=True,
        forwarding=True,
        register_file=reg_cfg,
        fp_unit=fp_cfg,
        l1i_cache=l1i,
        l1d_cache=l1d,
        l2_cache=l2,
        memory_size=1048576,
        memory_latency=100,
    )


# =========================================================================
# MultiCoreConfig -- configuration for a multi-core processor
# =========================================================================


@dataclass
class MultiCoreConfig:
    """Configuration for a multi-core CPU.

    In a multi-core system, each core has its own L1 and L2 caches but
    shares an L3 cache and main memory. The memory controller serializes
    requests from multiple cores.

    Real-world multi-core counts:

        Raspberry Pi 4:     4 cores (Cortex-A72)
        Apple M4:           4P + 6E = 10 cores
        AMD Ryzen 9 7950X:  16 cores
        Server chips:       64-128 cores

    Attributes:
        num_cores: Number of processor cores.
        core_config: Configuration shared by all cores.
        l3_cache: Shared L3 cache config, or None for no L3.
        memory_size: Total shared memory in bytes.
        memory_latency: DRAM access latency in cycles.
    """

    num_cores: int = 2
    core_config: CoreConfig = field(default_factory=simple_config)
    l3_cache: CacheConfig | None = None
    memory_size: int = 1048576
    memory_latency: int = 100


def default_multi_core_config() -> MultiCoreConfig:
    """Return a 2-core configuration for testing."""
    return MultiCoreConfig(
        num_cores=2,
        core_config=simple_config(),
        l3_cache=None,
        memory_size=1048576,
        memory_latency=100,
    )


# =========================================================================
# Helper: create branch predictor from config
# =========================================================================


def create_branch_predictor(
    typ: str,
    size: int,
) -> BranchPredictor:
    """Build a BranchPredictor from config strings.

    This factory function decouples the config (which uses strings) from the
    concrete predictor types. The Core calls this once during construction.

    Args:
        typ: Predictor algorithm name.
        size: Number of entries in the prediction table.

    Returns:
        A BranchPredictor instance matching the requested type.
    """
    if typ == "static_always_taken":
        return AlwaysTakenPredictor()
    elif typ == "static_always_not_taken":
        return AlwaysNotTakenPredictor()
    elif typ == "static_btfnt":
        return BackwardTakenForwardNotTaken()
    elif typ == "one_bit":
        return OneBitPredictor(table_size=size)
    elif typ == "two_bit":
        return TwoBitPredictor(
            table_size=size, initial_state=TwoBitState.WEAKLY_NOT_TAKEN
        )
    else:
        # Fall back to always-not-taken for unknown types.
        return AlwaysNotTakenPredictor()
