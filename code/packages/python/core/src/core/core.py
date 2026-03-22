"""Core -- a configurable processor core.

A Core is a complete processor core that composes all D-series sub-components:

  - Pipeline (D04): manages instruction flow through stages
  - Branch Predictor (D02): speculative fetch direction
  - Hazard Unit (D03): data, control, and structural hazard detection
  - Cache Hierarchy (D01): L1I + L1D + optional L2
  - Register File: fast operand storage
  - Clock: cycle-accurate timing
  - Memory Controller: access to backing memory

The Core wires these together by providing callback functions to the
pipeline. When the pipeline needs to fetch an instruction, it calls the
Core's fetch callback, which reads from the L1I cache. When it needs to
decode, it calls the ISA decoder. And so on.

# Construction

The Core is constructed from a CoreConfig and an ISADecoder::

    config = simple_config()
    decoder = MockDecoder()
    c = Core(config, decoder)

# Execution

The Core runs one cycle at a time via step(), or until halt via run()::

    c.step()              # advance one clock cycle
    stats = c.run(1000)   # run up to 1000 cycles

# ISA Independence

The Core does not know what instructions mean. The ISADecoder provides
instruction semantics. The same Core can run ARM, RISC-V, or any custom
ISA by swapping the decoder.
"""

from __future__ import annotations

from branch_predictor import BranchTargetBuffer
from cache import Cache, CacheConfig, CacheHierarchy
from clock import Clock
from cpu_pipeline import (
    HazardAction,
    HazardResponse,
    Pipeline,
    PipelineSnapshot,
    PipelineToken,
    StageCategory,
    classic_5_stage,
)
from hazard_detection import HazardAction as HDAction
from hazard_detection import HazardUnit, PipelineSlot

from core.config import CoreConfig, create_branch_predictor
from core.decoder import ISADecoder
from core.memory_controller import MemoryController
from core.register_file import RegisterFile
from core.stats import CoreStats


class Core:
    """A complete processor core that composes all sub-components.

    The Core wires sub-components together using callback functions. The
    pipeline calls the Core's callbacks for fetch, decode, execute, memory,
    and writeback -- and the Core delegates to the appropriate sub-component.
    """

    def __init__(self, config: CoreConfig, decoder: ISADecoder) -> None:
        """Create a fully-wired processor core.

        # What Happens During Construction

        1. The register file is created from the config.
        2. Main memory is allocated and wrapped in a MemoryController.
        3. Caches are created (L1I, L1D, optional L2) and assembled.
        4. The branch predictor and BTB are created.
        5. The hazard unit is created.
        6. The pipeline is created with callbacks wired to the Core's methods.
        7. The clock is created.

        Args:
            config: Complete core configuration.
            decoder: ISA decoder for instruction semantics.
        """
        self._config = config
        self._decoder = decoder

        # --- 1. Register File ---
        self._reg_file = RegisterFile(config.register_file)

        # --- 2. Memory ---
        mem_size = config.memory_size if config.memory_size > 0 else 65536
        memory = bytearray(mem_size)
        mem_latency = config.memory_latency if config.memory_latency > 0 else 100
        self._mem_ctrl = MemoryController(memory, mem_latency)

        # --- 3. Cache Hierarchy ---
        self._cache_hierarchy = self._build_cache_hierarchy(config, mem_latency)

        # --- 4. Branch Predictor + BTB ---
        self._predictor = create_branch_predictor(
            config.branch_predictor_type, config.branch_predictor_size
        )
        btb_size = config.btb_size if config.btb_size > 0 else 64
        self._btb = BranchTargetBuffer(size=btb_size)

        # --- 5. Hazard Unit ---
        num_fp_units = 1 if config.fp_unit is not None else 0
        self._hazard_unit = HazardUnit(
            num_alus=1,
            num_fp_units=num_fp_units,
            split_caches=True,
        )

        # --- 6. Pipeline ---
        pipeline_config = config.pipeline
        if not pipeline_config.stages:
            pipeline_config = classic_5_stage()

        self._pipeline = Pipeline(
            config=pipeline_config,
            fetch_fn=self._fetch_callback,
            decode_fn=self._decode_callback,
            execute_fn=self._execute_callback,
            memory_fn=self._memory_callback,
            writeback_fn=self._writeback_callback,
        )

        # Wire optional callbacks.
        if config.hazard_detection:
            self._pipeline.set_hazard_func(self._hazard_callback)
        self._pipeline.set_predict_func(self._predict_callback)

        # --- 7. Clock ---
        self._clk = Clock(frequency_hz=1_000_000_000)  # 1 GHz nominal

        # --- Execution state ---
        self._halted = False
        self._cycle = 0
        self._instructions_completed = 0
        self._forward_count = 0
        self._stall_count = 0
        self._flush_count = 0

    # =========================================================================
    # Cache hierarchy construction
    # =========================================================================

    @staticmethod
    def _build_cache_hierarchy(
        config: CoreConfig,
        mem_latency: int,
    ) -> CacheHierarchy:
        """Create the L1I, L1D, and optional L2 caches.

        Args:
            config: Core configuration with cache settings.
            mem_latency: Main memory latency in cycles.

        Returns:
            A CacheHierarchy wiring all cache levels together.
        """
        # Default L1I: 4KB direct-mapped, 64B lines, 1-cycle latency.
        l1i_cfg = config.l1i_cache
        if l1i_cfg is None:
            l1i_cfg = CacheConfig(
                name="L1I",
                total_size=4096,
                line_size=64,
                associativity=1,
                access_latency=1,
                write_policy="write-back",
            )
        l1i = Cache(l1i_cfg)

        # Default L1D: 4KB direct-mapped, 64B lines, 1-cycle latency.
        l1d_cfg = config.l1d_cache
        if l1d_cfg is None:
            l1d_cfg = CacheConfig(
                name="L1D",
                total_size=4096,
                line_size=64,
                associativity=1,
                access_latency=1,
                write_policy="write-back",
            )
        l1d = Cache(l1d_cfg)

        # Optional L2.
        l2 = Cache(config.l2_cache) if config.l2_cache is not None else None

        return CacheHierarchy(
            l1i=l1i,
            l1d=l1d,
            l2=l2,
            l3=None,
            main_memory_latency=mem_latency,
        )

    # =========================================================================
    # Pipeline Callbacks -- the Core provides these to the pipeline
    # =========================================================================

    def _fetch_callback(self, pc: int) -> int:
        """Called by the pipeline's IF stage.

        Reads the raw instruction bits from memory at the given PC.
        Also reads from the instruction cache hierarchy for statistics.

        Args:
            pc: Program counter to fetch from.

        Returns:
            Raw instruction bits (32-bit integer).
        """
        # Read from instruction cache hierarchy for statistics.
        self._cache_hierarchy.read(pc, is_instruction=True, cycle=self._cycle)

        # Read the actual instruction bits from memory.
        return self._mem_ctrl.read_word(pc)

    def _decode_callback(
        self,
        raw: int,
        token: PipelineToken,
    ) -> PipelineToken:
        """Called by the pipeline's ID stage.

        Delegates to the injected ISA decoder.

        Args:
            raw: Raw instruction bits.
            token: Token to fill with decoded fields.

        Returns:
            The token with decoded fields.
        """
        return self._decoder.decode(raw, token)

    def _execute_callback(self, token: PipelineToken) -> PipelineToken:
        """Called by the pipeline's EX stage.

        Delegates to the ISA decoder's execute method, then updates the
        branch predictor and BTB with actual outcomes.

        Args:
            token: Decoded instruction token.

        Returns:
            The token with execution results.
        """
        result = self._decoder.execute(token, self._reg_file)

        # Update branch predictor with actual outcome.
        if result.is_branch:
            self._predictor.update(
                result.pc, result.branch_taken, result.branch_target
            )
            if result.branch_taken:
                self._btb.update(
                    result.pc, result.branch_target, branch_type="conditional"
                )

        return result

    def _memory_callback(self, token: PipelineToken) -> PipelineToken:
        """Called by the pipeline's MEM stage.

        For load instructions: reads data from cache/memory.
        For store instructions: writes data to cache/memory.

        Args:
            token: Instruction token with effective address in alu_result.

        Returns:
            The token, with mem_data filled for loads.
        """
        if token.mem_read:
            # Load: read from data cache hierarchy.
            self._cache_hierarchy.read(
                token.alu_result, is_instruction=False, cycle=self._cycle
            )

            # Read the actual word from memory.
            token.mem_data = self._mem_ctrl.read_word(token.alu_result)
            token.write_data = token.mem_data

        elif token.mem_write:
            # Store: write to data cache hierarchy.
            data = [token.write_data & 0xFF]
            self._cache_hierarchy.write(token.alu_result, data, cycle=self._cycle)

            # Write the actual word to memory.
            self._mem_ctrl.write_word(token.alu_result, token.write_data)

        return token

    def _writeback_callback(self, token: PipelineToken) -> None:
        """Called by the pipeline's WB stage.

        For register-writing instructions, writes write_data to register Rd.

        Args:
            token: Completed instruction token.
        """
        if token.reg_write and token.rd >= 0:
            self._reg_file.write(token.rd, token.write_data)

    def _hazard_callback(
        self,
        stages: list[PipelineToken | None],
    ) -> HazardResponse:
        """Check for hazards at the start of each cycle.

        Translates the pipeline's stage contents into PipelineSlots for the
        hazard unit, then converts the hazard result back into a HazardResponse.

        Args:
            stages: Current contents of each pipeline stage.

        Returns:
            HazardResponse telling the pipeline what to do.
        """
        num_stages = len(stages)
        pipeline_cfg = self._config.pipeline
        if not pipeline_cfg.stages:
            pipeline_cfg = classic_5_stage()

        # Find the IF, ID, EX, MEM stages by category.
        if_tok: PipelineToken | None = None
        id_tok: PipelineToken | None = None
        ex_tok: PipelineToken | None = None
        mem_tok: PipelineToken | None = None

        for i, stage in enumerate(pipeline_cfg.stages):
            if i >= num_stages:
                break
            tok = stages[i]
            if stage.category == StageCategory.FETCH:
                if if_tok is None:
                    if_tok = tok
            elif stage.category == StageCategory.DECODE:
                # Use the LAST decode stage (closest to EX).
                id_tok = tok
            elif stage.category == StageCategory.EXECUTE:
                if ex_tok is None:
                    ex_tok = tok
            elif stage.category == StageCategory.MEMORY and mem_tok is None:
                mem_tok = tok

        # Convert PipelineTokens to PipelineSlots.
        if_slot = _token_to_slot(if_tok)
        id_slot = _token_to_slot(id_tok)
        ex_slot = _token_to_slot(ex_tok)
        mem_slot = _token_to_slot(mem_tok)

        # Run hazard detection.
        result = self._hazard_unit.check(if_slot, id_slot, ex_slot, mem_slot)

        # Convert HazardResult to HazardResponse.
        response = HazardResponse(action=HazardAction.NONE)

        if result.action == HDAction.STALL:
            response.action = HazardAction.STALL
            response.stall_stages = result.stall_cycles
            self._stall_count += 1

        elif result.action == HDAction.FLUSH:
            response.action = HazardAction.FLUSH
            response.flush_count = result.flush_count
            # Redirect PC to the correct target.
            if ex_tok is not None and ex_tok.is_branch:
                if ex_tok.branch_taken:
                    response.redirect_pc = ex_tok.branch_target
                else:
                    response.redirect_pc = (
                        ex_tok.pc + self._decoder.instruction_size()
                    )
            self._flush_count += 1

        elif result.action == HDAction.FORWARD_FROM_EX:
            response.action = HazardAction.FORWARD_FROM_EX
            if result.forwarded_value is not None:
                response.forward_value = result.forwarded_value
            response.forward_source = result.forwarded_from
            self._forward_count += 1

        elif result.action == HDAction.FORWARD_FROM_MEM:
            response.action = HazardAction.FORWARD_FROM_MEM
            if result.forwarded_value is not None:
                response.forward_value = result.forwarded_value
            response.forward_source = result.forwarded_from
            self._forward_count += 1

        return response

    def _predict_callback(self, pc: int) -> int:
        """Predict the next PC given the current PC.

        Consults the branch predictor for direction and the BTB for target.

        Args:
            pc: Current program counter.

        Returns:
            Predicted next PC (either sequential or branch target).
        """
        prediction = self._predictor.predict(pc)
        instr_size = self._decoder.instruction_size()

        if prediction.taken:
            target = self._btb.lookup(pc)
            if target is not None:
                return target

        # Default: sequential fetch.
        return pc + instr_size

    # =========================================================================
    # Public API -- step, run, load_program, etc.
    # =========================================================================

    def load_program(self, program: bytes, start_address: int) -> None:
        """Load machine code into memory starting at the given address.

        The program bytes are written to main memory. The PC is set to
        start_address before calling run() or step().

        Args:
            program: Raw machine code bytes.
            start_address: Memory address to load the program at.
        """
        self._mem_ctrl.load_program(program, start_address)
        self._pipeline.set_pc(start_address)

    def step(self) -> PipelineSnapshot:
        """Execute one clock cycle.

        Advances the pipeline by one step, which:
          - Checks for hazards (stalls, flushes, forwards)
          - Moves tokens through pipeline stages
          - Executes stage callbacks
          - Updates statistics

        Returns:
            Pipeline snapshot for this cycle.
        """
        if self._halted:
            return self._pipeline.snapshot()

        self._cycle += 1
        snap = self._pipeline.step()

        # Check if the pipeline halted this cycle.
        if self._pipeline.halted:
            self._halted = True

        # Track completed instructions.
        self._instructions_completed = (
            self._pipeline.stats().instructions_completed
        )

        return snap

    def run(self, max_cycles: int) -> CoreStats:
        """Execute until halt or max_cycles is reached.

        Returns aggregate statistics for the entire run.

        Args:
            max_cycles: Maximum number of cycles to execute.

        Returns:
            CoreStats with performance data from all sub-components.
        """
        while self._cycle < max_cycles and not self._halted:
            self.step()
        return self.stats()

    def stats(self) -> CoreStats:
        """Return aggregate statistics from all sub-components."""
        p_stats = self._pipeline.stats()

        core_stats = CoreStats(
            instructions_completed=p_stats.instructions_completed,
            total_cycles=p_stats.total_cycles,
            pipeline_stats=p_stats,
            predictor_stats=self._predictor.stats,
            cache_stats={},
            forward_count=self._forward_count,
            stall_count=self._stall_count,
            flush_count=self._flush_count,
        )

        # Collect cache stats.
        if self._cache_hierarchy.l1i is not None:
            core_stats.cache_stats["L1I"] = self._cache_hierarchy.l1i.stats
        if self._cache_hierarchy.l1d is not None:
            core_stats.cache_stats["L1D"] = self._cache_hierarchy.l1d.stats
        if self._cache_hierarchy.l2 is not None:
            core_stats.cache_stats["L2"] = self._cache_hierarchy.l2.stats

        return core_stats

    @property
    def halted(self) -> bool:
        """Return True if a halt instruction has completed."""
        return self._halted

    def read_register(self, index: int) -> int:
        """Read a general-purpose register.

        Args:
            index: Register number.

        Returns:
            The register value.
        """
        return self._reg_file.read(index)

    def write_register(self, index: int, value: int) -> None:
        """Write a general-purpose register.

        Args:
            index: Register number.
            value: Value to write.
        """
        self._reg_file.write(index, value)

    @property
    def register_file(self) -> RegisterFile:
        """Return the core's register file."""
        return self._reg_file

    @property
    def memory_controller(self) -> MemoryController:
        """Return the core's memory controller."""
        return self._mem_ctrl

    @property
    def cycle(self) -> int:
        """Return the current cycle number."""
        return self._cycle

    @property
    def config(self) -> CoreConfig:
        """Return the core configuration."""
        return self._config

    @property
    def pipeline(self) -> Pipeline:
        """Return the underlying pipeline."""
        return self._pipeline

    @property
    def predictor(self) -> object:
        """Return the branch predictor."""
        return self._predictor

    @property
    def cache_hierarchy(self) -> CacheHierarchy:
        """Return the cache hierarchy."""
        return self._cache_hierarchy


# =========================================================================
# Helper: convert PipelineToken to hazard-detection PipelineSlot
# =========================================================================


def _token_to_slot(tok: PipelineToken | None) -> PipelineSlot:
    """Convert a PipelineToken to a hazard-detection PipelineSlot.

    This bridges the gap between the pipeline package (PipelineToken) and
    the hazard-detection package (PipelineSlot). The Core must translate
    between the two because the packages are deliberately decoupled.

    Args:
        tok: The pipeline token, or None for an empty stage.

    Returns:
        A PipelineSlot describing the instruction for hazard detection.
    """
    if tok is None or tok.is_bubble:
        return PipelineSlot(valid=False)

    # Source registers.
    source_regs: list[int] = []
    if tok.rs1 >= 0:
        source_regs.append(tok.rs1)
    if tok.rs2 >= 0:
        source_regs.append(tok.rs2)

    # Destination register and value.
    dest_reg: int | None = None
    dest_value: int | None = None
    if tok.rd >= 0 and tok.reg_write:
        dest_reg = tok.rd
        if tok.alu_result != 0 or tok.write_data != 0:
            val = tok.alu_result
            if tok.write_data != 0:
                val = tok.write_data
            dest_value = val

    return PipelineSlot(
        valid=True,
        pc=tok.pc,
        source_regs=tuple(source_regs),
        dest_reg=dest_reg,
        dest_value=dest_value,
        is_branch=tok.is_branch,
        branch_taken=tok.branch_taken,
        branch_predicted_taken=False,  # Default assumption
        mem_read=tok.mem_read,
        mem_write=tok.mem_write,
        uses_alu=True,  # Most instructions use the ALU
        uses_fp=False,
    )
