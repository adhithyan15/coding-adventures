"""MultiCoreCPU -- multiple cores sharing L3 cache and memory.

MultiCoreCPU connects multiple processor cores to shared resources:

  - Each core has private L1I, L1D, and optional L2 caches
  - All cores share an optional L3 cache
  - All cores share main memory via a MemoryController
  - An InterruptController routes interrupts to specific cores

# Architecture Diagram

    Core 0: L1I + L1D + L2 (private)
    Core 1: L1I + L1D + L2 (private)
    Core 2: L1I + L1D + L2 (private)
    Core 3: L1I + L1D + L2 (private)
            |    |    |    |
       ==============================
       Shared L3 Cache (optional)
       ==============================
                  |
       Memory Controller (serializes requests)
                  |
       Shared Main Memory (DRAM)

# Execution Model

All cores run on the same clock. Each call to step() advances every core
by one cycle. Cores are independent -- they do not share register files
or pipeline state. They only interact through shared memory.

# Cache Coherence

This implementation does NOT model cache coherence (MESI protocol, etc.).
Writes by one core become visible to other cores only when they reach
main memory. Cache coherence is a future extension.
"""

from __future__ import annotations

from cache import Cache
from cpu_pipeline import PipelineSnapshot

from core.config import MultiCoreConfig
from core.core import Core
from core.decoder import ISADecoder
from core.interrupt_controller import InterruptController
from core.memory_controller import MemoryController
from core.stats import CoreStats


class MultiCoreCPU:
    """Connects multiple processor cores to shared resources.

    All cores share the same main memory. Each core gets its own ISA
    decoder from the decoders list. If len(decoders) < num_cores, the
    first decoder is reused for remaining cores.
    """

    def __init__(
        self,
        config: MultiCoreConfig,
        decoders: list[ISADecoder],
    ) -> None:
        """Create a multi-core processor.

        Args:
            config: Multi-core configuration.
            decoders: ISA decoder for each core. If shorter than num_cores,
                the first decoder is reused.

        Raises:
            ValueError: If any core fails to initialize (propagated from Pipeline).
        """
        # Allocate shared memory.
        mem_size = config.memory_size if config.memory_size > 0 else 1048576
        shared_memory = bytearray(mem_size)

        mem_latency = config.memory_latency if config.memory_latency > 0 else 100
        self._mem_ctrl = MemoryController(shared_memory, mem_latency)

        # Optional shared L3 cache.
        self._l3_cache: Cache | None = None
        if config.l3_cache is not None:
            self._l3_cache = Cache(config.l3_cache)

        # Create cores.
        num_cores = max(config.num_cores, 1)
        self._cores: list[Core] = []

        for i in range(num_cores):
            # Select decoder for this core.
            decoder = decoders[i] if i < len(decoders) else decoders[0]

            # Override core config to use shared memory size.
            core_cfg = config.core_config
            # We need to copy the config to avoid mutation -- use a new instance.
            from dataclasses import replace

            core_cfg = replace(
                core_cfg,
                memory_size=mem_size,
                memory_latency=mem_latency,
            )

            c = Core(core_cfg, decoder)

            # Replace the core's memory controller with the shared one.
            c._mem_ctrl = self._mem_ctrl

            self._cores.append(c)

        self._config = config
        self._shared_memory = shared_memory
        self._interrupt_ctrl = InterruptController(num_cores)
        self._cycle = 0

    def load_program(
        self, core_id: int, program: bytes, start_address: int,
    ) -> None:
        """Load a program into memory for a specific core.

        Since all cores share memory, the program is written to the shared
        memory at the given address. The specified core's PC is set to
        start_address.

        Args:
            core_id: Which core to set the PC for.
            program: Raw machine code bytes.
            start_address: Memory address to load the program at.
        """
        if core_id < 0 or core_id >= len(self._cores):
            return

        # Write program to shared memory.
        self._mem_ctrl.load_program(program, start_address)

        # Set the core's PC.
        self._cores[core_id].pipeline.set_pc(start_address)

    def step(self) -> list[PipelineSnapshot]:
        """Advance all cores by one clock cycle.

        Each core's step() is called in order. The memory controller is
        also ticked to process pending requests.

        Returns:
            Pipeline snapshot from each core.
        """
        self._cycle += 1

        snapshots: list[PipelineSnapshot] = []
        for c in self._cores:
            snapshots.append(c.step())

        # Tick the shared memory controller.
        self._mem_ctrl.tick()

        return snapshots

    def run(self, max_cycles: int) -> list[CoreStats]:
        """Execute all cores until all have halted or max_cycles is reached.

        Args:
            max_cycles: Maximum number of cycles to execute.

        Returns:
            Per-core statistics.
        """
        while self._cycle < max_cycles:
            if self.all_halted:
                break
            self.step()
        return self.stats()

    @property
    def cores(self) -> list[Core]:
        """Return the array of cores."""
        return self._cores

    def stats(self) -> list[CoreStats]:
        """Return per-core statistics."""
        return [c.stats() for c in self._cores]

    @property
    def interrupt_controller(self) -> InterruptController:
        """Return the interrupt controller."""
        return self._interrupt_ctrl

    @property
    def shared_memory_controller(self) -> MemoryController:
        """Return the shared memory controller."""
        return self._mem_ctrl

    @property
    def cycle(self) -> int:
        """Return the global cycle count."""
        return self._cycle

    @property
    def all_halted(self) -> bool:
        """Return True if every core has halted."""
        return all(c.halted for c in self._cores)
