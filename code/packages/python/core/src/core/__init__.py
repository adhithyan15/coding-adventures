"""Core -- configurable processor core integrating all D-series components.

This package integrates all D-series micro-architectural components into a
complete processor core:

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

Quick start::

    from core import (
        Core, simple_config, MockDecoder,
        encode_program, encode_addi, encode_halt,
    )

    config = simple_config()
    decoder = MockDecoder()
    c = Core(config, decoder)
    program = encode_program(encode_addi(1, 0, 42), encode_halt())
    c.load_program(program, 0)
    stats = c.run(100)
    print(f"R1 = {c.read_register(1)}")  # R1 = 42
    print(f"IPC: {stats.ipc():.3f}")
"""

from core.config import (
    CoreConfig,
    FPUnitConfig,
    MultiCoreConfig,
    RegisterFileConfig,
    cortex_a78_like_config,
    create_branch_predictor,
    default_core_config,
    default_multi_core_config,
    default_register_file_config,
    simple_config,
)
from core.core import Core
from core.decoder import (
    ISADecoder,
    MockDecoder,
    encode_add,
    encode_addi,
    encode_branch,
    encode_halt,
    encode_load,
    encode_nop,
    encode_program,
    encode_store,
    encode_sub,
)
from core.interrupt_controller import (
    AcknowledgedInterrupt,
    InterruptController,
    PendingInterrupt,
)
from core.memory_controller import (
    MemoryController,
    MemoryReadResult,
    MemoryRequest,
    MemoryWriteRequest,
)
from core.multi_core import MultiCoreCPU
from core.register_file import RegisterFile
from core.stats import CoreStats

__all__ = [
    # Config
    "CoreConfig",
    "FPUnitConfig",
    "MultiCoreConfig",
    "RegisterFileConfig",
    "cortex_a78_like_config",
    "create_branch_predictor",
    "default_core_config",
    "default_multi_core_config",
    "default_register_file_config",
    "simple_config",
    # Core
    "Core",
    # Decoder
    "ISADecoder",
    "MockDecoder",
    "encode_add",
    "encode_addi",
    "encode_branch",
    "encode_halt",
    "encode_load",
    "encode_nop",
    "encode_program",
    "encode_store",
    "encode_sub",
    # Interrupt Controller
    "AcknowledgedInterrupt",
    "InterruptController",
    "PendingInterrupt",
    # Memory Controller
    "MemoryController",
    "MemoryReadResult",
    "MemoryRequest",
    "MemoryWriteRequest",
    # Multi-Core
    "MultiCoreCPU",
    # Register File
    "RegisterFile",
    # Stats
    "CoreStats",
]
