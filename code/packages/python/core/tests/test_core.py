"""Comprehensive tests for the Core package.

Tests are organized into sections matching the Go test file:
  - Core Assembly Tests
  - Single-Instruction Tests
  - Program Execution Tests
  - Statistics Tests
  - ISA Decoder Tests
  - Register File Tests
  - Memory Controller Tests
  - Interrupt Controller Tests
  - Configuration Tests
  - Branch Predictor Factory Tests
  - Multi-Core Tests
  - Performance Comparison Tests
"""

from __future__ import annotations

from core import (
    Core,
    CoreStats,
    FPUnitConfig,
    InterruptController,
    MemoryController,
    MockDecoder,
    MultiCoreCPU,
    RegisterFile,
    RegisterFileConfig,
    cortex_a78_like_config,
    create_branch_predictor,
    default_core_config,
    default_multi_core_config,
    default_register_file_config,
    encode_add,
    encode_addi,
    encode_branch,
    encode_halt,
    encode_load,
    encode_nop,
    encode_program,
    encode_store,
    encode_sub,
    simple_config,
)

# =========================================================================
# Test Helpers
# =========================================================================


def make_simple_core() -> Core:
    """Create a Core with simple_config() and MockDecoder."""
    return Core(simple_config(), MockDecoder())


def make_default_core() -> Core:
    """Create a Core with default_core_config() and MockDecoder."""
    return Core(default_core_config(), MockDecoder())


# =========================================================================
# Core Assembly Tests
# =========================================================================


class TestCoreConstruction:
    """Verify that a Core initializes all sub-components."""

    def test_simple_core_components(self) -> None:
        c = make_simple_core()
        assert c.pipeline is not None
        assert c.predictor is not None
        assert c.register_file is not None
        assert c.memory_controller is not None
        assert c.cache_hierarchy is not None

    def test_simple_config_runs(self) -> None:
        c = make_simple_core()
        program = encode_program(encode_halt())
        c.load_program(program, 0)
        stats = c.run(100)
        assert stats.total_cycles > 0

    def test_complex_config_runs(self) -> None:
        config = cortex_a78_like_config()
        c = Core(config, MockDecoder())
        program = encode_program(encode_halt())
        c.load_program(program, 0)
        stats = c.run(200)
        assert stats.total_cycles > 0

    def test_missing_optional(self) -> None:
        config = simple_config()
        config.l2_cache = None
        config.fp_unit = None
        c = Core(config, MockDecoder())
        program = encode_program(encode_addi(1, 0, 10), encode_halt())
        c.load_program(program, 0)
        c.run(100)
        assert c.halted

    def test_default_core_config(self) -> None:
        c = make_default_core()
        program = encode_program(encode_nop(), encode_halt())
        c.load_program(program, 0)
        c.run(100)
        assert c.halted


# =========================================================================
# Single-Instruction Tests
# =========================================================================


class TestSingleInstructions:
    """Verify individual instruction execution."""

    def test_nop(self) -> None:
        c = make_simple_core()
        program = encode_program(encode_nop(), encode_halt())
        c.load_program(program, 0)
        c.run(100)
        assert c.halted
        # NOP should not modify any register.
        for i in range(c.register_file.count):
            assert c.register_file.read(i) == 0

    def test_add(self) -> None:
        c = make_simple_core()
        program = encode_program(
            encode_addi(1, 0, 10),  # R1 = 10
            encode_addi(2, 0, 20),  # R2 = 20
            encode_nop(),
            encode_nop(),
            encode_add(3, 1, 2),    # R3 = R1 + R2 = 30
            encode_nop(),
            encode_nop(),
            encode_nop(),
            encode_nop(),
            encode_halt(),
        )
        c.load_program(program, 0)
        c.run(200)
        assert c.halted
        assert c.read_register(1) == 10
        assert c.read_register(2) == 20
        assert c.read_register(3) == 30

    def test_addi(self) -> None:
        c = make_simple_core()
        program = encode_program(
            encode_addi(1, 0, 42),
            encode_nop(),
            encode_nop(),
            encode_nop(),
            encode_nop(),
            encode_halt(),
        )
        c.load_program(program, 0)
        c.run(100)
        assert c.halted
        assert c.read_register(1) == 42

    def test_sub(self) -> None:
        c = make_simple_core()
        program = encode_program(
            encode_addi(1, 0, 50),
            encode_addi(2, 0, 20),
            encode_nop(),
            encode_nop(),
            encode_sub(3, 1, 2),   # R3 = 50 - 20 = 30
            encode_nop(),
            encode_nop(),
            encode_nop(),
            encode_nop(),
            encode_halt(),
        )
        c.load_program(program, 0)
        c.run(200)
        assert c.halted
        assert c.read_register(3) == 30

    def test_load(self) -> None:
        c = make_simple_core()
        # Store 0xDEAD at address 512.
        c.memory_controller.write_word(512, 0xDEAD)
        program = encode_program(
            encode_addi(1, 0, 0),    # R1 = 0
            encode_nop(),
            encode_nop(),
            encode_load(2, 1, 512),  # R2 = Memory[512]
            encode_nop(),
            encode_nop(),
            encode_nop(),
            encode_nop(),
            encode_halt(),
        )
        c.load_program(program, 0)
        c.run(200)
        assert c.halted
        assert c.read_register(2) == 0xDEAD

    def test_store(self) -> None:
        c = make_simple_core()
        program = encode_program(
            encode_addi(1, 0, 0),     # R1 = 0
            encode_addi(2, 0, 0x42),  # R2 = 0x42
            encode_nop(),
            encode_nop(),
            encode_store(1, 2, 512),  # Memory[512] = R2
            encode_nop(),
            encode_nop(),
            encode_nop(),
            encode_nop(),
            encode_halt(),
        )
        c.load_program(program, 0)
        c.run(200)
        assert c.halted
        assert c.memory_controller.read_word(512) == 0x42

    def test_halt(self) -> None:
        c = make_simple_core()
        program = encode_program(encode_halt())
        c.load_program(program, 0)
        c.run(100)
        assert c.halted


# =========================================================================
# Program Execution Tests
# =========================================================================


class TestProgramExecution:
    """Verify multi-instruction programs."""

    def test_simple_sequence(self) -> None:
        """LOAD, ADD, STORE sequence."""
        c = make_simple_core()
        c.memory_controller.write_word(512, 100)
        program = encode_program(
            encode_addi(1, 0, 0),     # R1 = 0
            encode_nop(),
            encode_nop(),
            encode_load(2, 1, 512),   # R2 = 100
            encode_nop(),
            encode_nop(),
            encode_nop(),
            encode_addi(3, 2, 50),    # R3 = 150
            encode_nop(),
            encode_nop(),
            encode_store(1, 3, 516),  # Memory[516] = 150
            encode_nop(),
            encode_nop(),
            encode_nop(),
            encode_nop(),
            encode_halt(),
        )
        c.load_program(program, 0)
        c.run(500)
        assert c.halted
        assert c.read_register(2) == 100
        assert c.read_register(3) == 150
        assert c.memory_controller.read_word(516) == 150

    def test_counting_program(self) -> None:
        """Unrolled counting loop: R1 should reach 5."""
        c = make_simple_core()
        program = encode_program(
            encode_addi(1, 0, 0),  # R1 = 0
            encode_addi(2, 0, 1),  # R2 = 1
            encode_nop(),
            encode_nop(),
            encode_add(1, 1, 2),   # R1 = 1
            encode_nop(),
            encode_nop(),
            encode_nop(),
            encode_add(1, 1, 2),   # R1 = 2
            encode_nop(),
            encode_nop(),
            encode_nop(),
            encode_add(1, 1, 2),   # R1 = 3
            encode_nop(),
            encode_nop(),
            encode_nop(),
            encode_add(1, 1, 2),   # R1 = 4
            encode_nop(),
            encode_nop(),
            encode_nop(),
            encode_add(1, 1, 2),   # R1 = 5
            encode_nop(),
            encode_nop(),
            encode_nop(),
            encode_nop(),
            encode_halt(),
        )
        c.load_program(program, 0)
        c.run(500)
        assert c.halted
        assert c.read_register(1) == 5


# =========================================================================
# Statistics Tests
# =========================================================================


class TestStatistics:
    """Verify CoreStats computation."""

    def test_ipc_calculation(self) -> None:
        c = make_simple_core()
        program = encode_program(
            encode_addi(1, 0, 1),
            encode_addi(2, 0, 2),
            encode_addi(3, 0, 3),
            encode_halt(),
        )
        c.load_program(program, 0)
        stats = c.run(200)
        assert stats.instructions_completed > 0
        assert stats.total_cycles > 0
        expected_ipc = stats.instructions_completed / stats.total_cycles
        assert abs(stats.ipc() - expected_ipc) < 1e-9
        # CPI should be inverse.
        if stats.instructions_completed > 0:
            expected_cpi = stats.total_cycles / stats.instructions_completed
            assert abs(stats.cpi() - expected_cpi) < 1e-9

    def test_aggregate_stats(self) -> None:
        c = make_simple_core()
        program = encode_program(
            encode_addi(1, 0, 10),
            encode_addi(2, 0, 20),
            encode_halt(),
        )
        c.load_program(program, 0)
        stats = c.run(200)
        # Pipeline stats populated.
        assert stats.pipeline_stats.total_cycles > 0
        # Predictor stats exist.
        assert stats.predictor_stats is not None
        # Cache stats have L1I and L1D.
        assert "L1I" in stats.cache_stats
        assert "L1D" in stats.cache_stats
        # L1I should have been accessed.
        assert stats.cache_stats["L1I"].total_accesses > 0

    def test_stats_string(self) -> None:
        c = make_simple_core()
        program = encode_program(encode_addi(1, 0, 1), encode_halt())
        c.load_program(program, 0)
        stats = c.run(100)
        s = str(stats)
        assert len(s) > 0
        assert "Core Statistics" in s

    def test_ipc_zero_cycles(self) -> None:
        stats = CoreStats()
        assert stats.ipc() == 0.0

    def test_cpi_zero_instructions(self) -> None:
        stats = CoreStats(total_cycles=10)
        assert stats.cpi() == 0.0


# =========================================================================
# ISA Decoder Tests
# =========================================================================


class TestMockDecoder:
    """Verify MockDecoder decode and execute methods."""

    def test_instruction_size(self) -> None:
        d = MockDecoder()
        assert d.instruction_size() == 4

    def test_decode_all_types(self) -> None:
        from cpu_pipeline import new_token

        d = MockDecoder()
        tests = [
            ("NOP", encode_nop(), "NOP", -1, -1, -1),
            ("ADD", encode_add(3, 1, 2), "ADD", 3, 1, 2),
            ("SUB", encode_sub(3, 1, 2), "SUB", 3, 1, 2),
            ("ADDI", encode_addi(1, 0, 42), "ADDI", 1, 0, -1),
            ("LOAD", encode_load(1, 2, 100), "LOAD", 1, 2, -1),
            ("STORE", encode_store(2, 3, 100), "STORE", -1, 2, 3),
            ("BRANCH", encode_branch(1, 2, 4), "BRANCH", -1, 1, 2),
            ("HALT", encode_halt(), "HALT", -1, -1, -1),
        ]
        for name, raw, opcode, rd, rs1, rs2 in tests:
            token = new_token()
            result = d.decode(raw, token)
            assert result.opcode == opcode, f"{name}: opcode"
            assert result.rd == rd, f"{name}: rd"
            assert result.rs1 == rs1, f"{name}: rs1"
            assert result.rs2 == rs2, f"{name}: rs2"

    def test_decode_unknown_opcode(self) -> None:
        from cpu_pipeline import new_token

        d = MockDecoder()
        token = new_token()
        result = d.decode(0xFF << 24, token)
        assert result.opcode == "NOP"

    def test_execute_add(self) -> None:
        from cpu_pipeline import new_token

        d = MockDecoder()
        rf = RegisterFile(None)
        rf.write(1, 10)
        rf.write(2, 20)
        token = new_token()
        d.decode(encode_add(3, 1, 2), token)
        d.execute(token, rf)
        assert token.alu_result == 30

    def test_execute_sub(self) -> None:
        from cpu_pipeline import new_token

        d = MockDecoder()
        rf = RegisterFile(None)
        rf.write(1, 10)
        rf.write(2, 20)
        token = new_token()
        d.decode(encode_sub(3, 2, 1), token)
        d.execute(token, rf)
        assert token.alu_result == 10  # 20 - 10

    def test_execute_addi(self) -> None:
        from cpu_pipeline import new_token

        d = MockDecoder()
        rf = RegisterFile(None)
        rf.write(1, 10)
        token = new_token()
        d.decode(encode_addi(3, 1, 5), token)
        d.execute(token, rf)
        assert token.alu_result == 15

    def test_execute_load_effective_address(self) -> None:
        from cpu_pipeline import new_token

        d = MockDecoder()
        rf = RegisterFile(None)
        rf.write(1, 10)
        token = new_token()
        d.decode(encode_load(3, 1, 100), token)
        d.execute(token, rf)
        assert token.alu_result == 110

    def test_execute_branch_taken(self) -> None:
        from cpu_pipeline import new_token

        d = MockDecoder()
        rf = RegisterFile(None)
        rf.write(1, 10)
        token = new_token()
        token.pc = 100
        d.decode(encode_branch(1, 1, 3), token)  # Rs1==Rs1 always true
        d.execute(token, rf)
        assert token.branch_taken
        assert token.branch_target == 100 + 3 * 4

    def test_execute_branch_not_taken(self) -> None:
        from cpu_pipeline import new_token

        d = MockDecoder()
        rf = RegisterFile(None)
        rf.write(1, 10)
        rf.write(2, 20)
        token = new_token()
        token.pc = 100
        d.decode(encode_branch(1, 2, 3), token)  # 10 != 20
        d.execute(token, rf)
        assert not token.branch_taken

    def test_execute_store(self) -> None:
        from cpu_pipeline import new_token

        d = MockDecoder()
        rf = RegisterFile(None)
        rf.write(1, 100)
        rf.write(2, 42)
        token = new_token()
        d.decode(encode_store(1, 2, 16), token)
        d.execute(token, rf)
        assert token.alu_result == 116  # 100 + 16
        assert token.write_data == 42

    def test_execute_nop_halt(self) -> None:
        from cpu_pipeline import new_token

        d = MockDecoder()
        rf = RegisterFile(None)
        # NOP
        token = new_token()
        d.decode(encode_nop(), token)
        d.execute(token, rf)
        # HALT
        token = new_token()
        d.decode(encode_halt(), token)
        d.execute(token, rf)
        # Should not crash


# =========================================================================
# Register File Tests
# =========================================================================


class TestRegisterFile:
    """Verify RegisterFile operations."""

    def test_basic_read_write(self) -> None:
        rf = RegisterFile(None)
        rf.write(1, 42)
        assert rf.read(1) == 42
        rf.write(1, 100)
        assert rf.read(1) == 100

    def test_zero_register(self) -> None:
        cfg = RegisterFileConfig(count=16, width=32, zero_register=True)
        rf = RegisterFile(cfg)
        rf.write(0, 999)
        assert rf.read(0) == 0

    def test_no_zero_register(self) -> None:
        cfg = RegisterFileConfig(count=16, width=32, zero_register=False)
        rf = RegisterFile(cfg)
        rf.write(0, 999)
        assert rf.read(0) == 999

    def test_out_of_range(self) -> None:
        rf = RegisterFile(None)
        assert rf.read(100) == 0
        assert rf.read(-1) == 0
        # Should not crash.
        rf.write(100, 42)
        rf.write(-1, 42)

    def test_values(self) -> None:
        rf = RegisterFile(None)
        rf.write(1, 10)
        rf.write(2, 20)
        vals = rf.values()
        assert len(vals) == rf.count
        assert vals[1] == 10
        assert vals[2] == 20

    def test_reset(self) -> None:
        rf = RegisterFile(None)
        rf.write(1, 42)
        rf.write(5, 99)
        rf.reset()
        assert rf.read(1) == 0
        assert rf.read(5) == 0

    def test_string(self) -> None:
        rf = RegisterFile(None)
        rf.write(1, 42)
        s = str(rf)
        assert len(s) > 0

    def test_bit_width(self) -> None:
        cfg = RegisterFileConfig(count=4, width=8, zero_register=False)
        rf = RegisterFile(cfg)
        rf.write(1, 0xABCD)
        assert rf.read(1) == 0xCD  # only low 8 bits

    def test_properties(self) -> None:
        rf = RegisterFile(None)
        assert rf.count == 16
        assert rf.width == 32
        assert rf.config == default_register_file_config()

    def test_64_bit_width(self) -> None:
        cfg = RegisterFileConfig(count=4, width=64, zero_register=False)
        rf = RegisterFile(cfg)
        rf.write(1, 0x123456789ABCDEF0)
        assert rf.read(1) == 0x123456789ABCDEF0


# =========================================================================
# Memory Controller Tests
# =========================================================================


class TestMemoryController:
    """Verify MemoryController operations."""

    def test_read_write(self) -> None:
        mem = bytearray(4096)
        mc = MemoryController(mem, 10)
        mc.write_word(100, 0x1234ABCD)
        assert mc.read_word(100) == 0x1234ABCD

    def test_load_program(self) -> None:
        mem = bytearray(4096)
        mc = MemoryController(mem, 10)
        program = bytes([0x01, 0x02, 0x03, 0x04])
        mc.load_program(program, 0)
        word = mc.read_word(0)
        assert word == 0x04030201  # little-endian

    def test_pending_requests(self) -> None:
        mem = bytearray(4096)
        mc = MemoryController(mem, 3)  # 3-cycle latency
        mc.write_word(0, 42)
        mc.request_read(0, 4, 0)
        assert mc.pending_count == 1
        # Tick 1 and 2: not ready.
        assert len(mc.tick()) == 0
        assert len(mc.tick()) == 0
        # Tick 3: ready.
        result = mc.tick()
        assert len(result) == 1
        assert result[0].requester_id == 0

    def test_bounds_check(self) -> None:
        mem = bytearray(64)
        mc = MemoryController(mem, 1)
        # Should not crash.
        mc.read_word(1000)
        mc.write_word(1000, 42)
        mc.load_program(bytes([1, 2, 3, 4]), 1000)

    def test_memory_size(self) -> None:
        mem = bytearray(4096)
        mc = MemoryController(mem, 10)
        assert mc.memory_size == 4096

    def test_async_write(self) -> None:
        mem = bytearray(4096)
        mc = MemoryController(mem, 2)
        mc.request_write(0, bytes([0xAB, 0xCD, 0xEF, 0x12]), 0)
        assert mc.pending_count == 1
        mc.tick()  # 1
        mc.tick()  # 2 -- write completes
        assert mc.pending_count == 0
        assert mc.read_word(0) == 0x12EFCDAB


# =========================================================================
# Interrupt Controller Tests
# =========================================================================


class TestInterruptController:
    """Verify InterruptController operations."""

    def test_basic(self) -> None:
        ic = InterruptController(4)
        ic.raise_interrupt(1, 2)
        assert ic.pending_count == 1
        pending = ic.pending_for_core(2)
        assert len(pending) == 1
        assert pending[0].interrupt_id == 1
        ic.acknowledge(2, 1)
        assert ic.pending_count == 0
        assert ic.acknowledged_count == 1

    def test_default_routing(self) -> None:
        ic = InterruptController(4)
        ic.raise_interrupt(5, -1)  # should route to core 0
        pending = ic.pending_for_core(0)
        assert len(pending) == 1

    def test_reset(self) -> None:
        ic = InterruptController(4)
        ic.raise_interrupt(1, 0)
        ic.acknowledge(0, 1)
        ic.reset()
        assert ic.pending_count == 0
        assert ic.acknowledged_count == 0

    def test_overflow_routing(self) -> None:
        ic = InterruptController(4)
        ic.raise_interrupt(1, 10)  # target >= num_cores, routes to 0
        pending = ic.pending_for_core(0)
        assert len(pending) == 1


# =========================================================================
# Configuration Tests
# =========================================================================


class TestConfigurations:
    """Verify preset configurations."""

    def test_simple_config_fields(self) -> None:
        cfg = simple_config()
        assert cfg.name == "Simple"
        assert len(cfg.pipeline.stages) == 5
        assert cfg.branch_predictor_type == "static_always_not_taken"
        assert cfg.register_file is not None
        assert cfg.register_file.count == 16
        assert cfg.fp_unit is None
        assert cfg.l1i_cache is not None
        assert cfg.l1i_cache.total_size == 4096
        assert cfg.l1d_cache is not None
        assert cfg.l1d_cache.total_size == 4096
        assert cfg.l2_cache is None

    def test_cortex_a78_like_config_fields(self) -> None:
        cfg = cortex_a78_like_config()
        assert cfg.name == "CortexA78Like"
        assert len(cfg.pipeline.stages) == 13
        assert cfg.branch_predictor_type == "two_bit"
        assert cfg.branch_predictor_size == 4096
        assert cfg.register_file is not None
        assert cfg.register_file.count == 31
        assert cfg.fp_unit is not None
        assert cfg.l1i_cache is not None
        assert cfg.l1i_cache.total_size == 65536
        assert cfg.l2_cache is not None
        assert cfg.l2_cache.total_size == 262144

    def test_default_core_config_fields(self) -> None:
        cfg = default_core_config()
        assert cfg.name == "Default"
        assert cfg.hazard_detection is True
        assert cfg.forwarding is True

    def test_default_multi_core_config(self) -> None:
        cfg = default_multi_core_config()
        assert cfg.num_cores == 2
        assert cfg.memory_size == 1048576


# =========================================================================
# Branch Predictor Factory Tests
# =========================================================================


class TestBranchPredictorFactory:
    """Verify all predictor types can be created."""

    def test_all_types(self) -> None:
        types = [
            "static_always_taken",
            "static_always_not_taken",
            "static_btfnt",
            "one_bit",
            "two_bit",
            "unknown_type",
        ]
        for typ in types:
            p = create_branch_predictor(typ, 256)
            assert p is not None
            # Should implement predict/update.
            prediction = p.predict(0)
            assert hasattr(prediction, "taken")


# =========================================================================
# Multi-Core Tests
# =========================================================================


class TestMultiCore:
    """Verify multi-core CPU operations."""

    def test_construction(self) -> None:
        config = default_multi_core_config()
        decoders = [MockDecoder(), MockDecoder()]
        mc = MultiCoreCPU(config, decoders)
        assert len(mc.cores) == 2
        assert mc.interrupt_controller is not None
        assert mc.shared_memory_controller is not None

    def test_independent_programs(self) -> None:
        config = default_multi_core_config()
        decoders = [MockDecoder(), MockDecoder()]
        mc = MultiCoreCPU(config, decoders)

        # Core 0: R1 = 10
        prog0 = encode_program(
            encode_addi(1, 0, 10),
            encode_nop(),
            encode_nop(),
            encode_nop(),
            encode_nop(),
            encode_halt(),
        )
        # Core 1: R1 = 20
        prog1 = encode_program(
            encode_addi(1, 0, 20),
            encode_nop(),
            encode_nop(),
            encode_nop(),
            encode_nop(),
            encode_halt(),
        )
        mc.load_program(0, prog0, 0)
        mc.load_program(1, prog1, 4096)
        mc.run(200)
        assert mc.all_halted
        assert mc.cores[0].read_register(1) == 10
        assert mc.cores[1].read_register(1) == 20

    def test_shared_memory(self) -> None:
        config = default_multi_core_config()
        decoders = [MockDecoder(), MockDecoder()]
        mc = MultiCoreCPU(config, decoders)

        mc.shared_memory_controller.write_word(512, 0xCAFE)

        prog0 = encode_program(
            encode_addi(1, 0, 0),
            encode_nop(),
            encode_nop(),
            encode_load(2, 1, 512),
            encode_nop(),
            encode_nop(),
            encode_nop(),
            encode_nop(),
            encode_halt(),
        )
        mc.load_program(0, prog0, 0)

        prog1 = encode_program(encode_halt())
        mc.load_program(1, prog1, 4096)

        mc.run(200)
        assert mc.cores[0].read_register(2) == 0xCAFE

    def test_stats(self) -> None:
        config = default_multi_core_config()
        decoders = [MockDecoder(), MockDecoder()]
        mc = MultiCoreCPU(config, decoders)

        prog = encode_program(encode_addi(1, 0, 1), encode_halt())
        mc.load_program(0, prog, 0)
        mc.load_program(1, prog, 4096)

        stats = mc.run(200)
        assert len(stats) == 2
        for s in stats:
            assert s.total_cycles > 0

    def test_step(self) -> None:
        config = default_multi_core_config()
        decoders = [MockDecoder(), MockDecoder()]
        mc = MultiCoreCPU(config, decoders)

        prog = encode_program(encode_halt())
        mc.load_program(0, prog, 0)
        mc.load_program(1, prog, 4096)

        snapshots = mc.step()
        assert len(snapshots) == 2

    def test_core_count_scaling(self) -> None:
        for num_cores in [1, 2, 4]:
            config = default_multi_core_config()
            config.num_cores = num_cores
            decoders = [MockDecoder() for _ in range(num_cores)]
            mc = MultiCoreCPU(config, decoders)

            prog = encode_program(encode_halt())
            for i in range(num_cores):
                mc.load_program(i, prog, i * 4096)

            mc.run(200)
            assert mc.all_halted

    def test_invalid_core_id_load(self) -> None:
        config = default_multi_core_config()
        decoders = [MockDecoder(), MockDecoder()]
        mc = MultiCoreCPU(config, decoders)
        # Should not crash.
        mc.load_program(-1, b"", 0)
        mc.load_program(100, b"", 0)

    def test_cycle_count(self) -> None:
        config = default_multi_core_config()
        decoders = [MockDecoder(), MockDecoder()]
        mc = MultiCoreCPU(config, decoders)
        assert mc.cycle == 0
        mc.step()
        assert mc.cycle == 1


# =========================================================================
# Performance Comparison Tests
# =========================================================================


class TestPerformanceComparison:
    """Verify different configs produce different behavior."""

    def test_predictor_impact(self) -> None:
        prog = encode_program(
            encode_addi(1, 0, 1),
            encode_addi(2, 0, 2),
            encode_addi(3, 0, 3),
            encode_halt(),
        )

        configs = [
            ("always_not_taken", "static_always_not_taken"),
            ("two_bit", "two_bit"),
        ]

        stats_list = []
        for _name, predictor_type in configs:
            config = simple_config()
            config.branch_predictor_type = predictor_type
            c = Core(config, MockDecoder())
            c.load_program(prog, 0)
            stats = c.run(200)
            stats_list.append(stats)

        # Both should complete.
        for s in stats_list:
            assert s.instructions_completed > 0


# =========================================================================
# Encode Program Tests
# =========================================================================


class TestEncodeProgram:
    """Verify instruction encoding helpers."""

    def test_encode_program_roundtrip(self) -> None:
        """Encode instructions and verify they can be decoded back."""
        instructions = [
            encode_nop(),
            encode_add(3, 1, 2),
            encode_sub(3, 1, 2),
            encode_addi(1, 0, 42),
            encode_load(1, 2, 100),
            encode_store(2, 3, 100),
            encode_branch(1, 2, 4),
            encode_halt(),
        ]

        program = encode_program(*instructions)
        assert len(program) == len(instructions) * 4

        # Each instruction should decode correctly from bytes.
        for i, raw in enumerate(instructions):
            offset = i * 4
            word = (
                int(program[offset])
                | (int(program[offset + 1]) << 8)
                | (int(program[offset + 2]) << 16)
                | (int(program[offset + 3]) << 24)
            )
            assert word == raw


# =========================================================================
# Core Accessor Tests
# =========================================================================


class TestCoreAccessors:
    """Verify Core property accessors."""

    def test_cycle(self) -> None:
        c = make_simple_core()
        assert c.cycle == 0
        prog = encode_program(encode_halt())
        c.load_program(prog, 0)
        c.step()
        assert c.cycle == 1

    def test_config(self) -> None:
        c = make_simple_core()
        assert c.config.name == "Simple"

    def test_write_register(self) -> None:
        c = make_simple_core()
        c.write_register(5, 99)
        assert c.read_register(5) == 99

    def test_halted_before_run(self) -> None:
        c = make_simple_core()
        assert not c.halted

    def test_step_when_halted(self) -> None:
        c = make_simple_core()
        prog = encode_program(encode_halt())
        c.load_program(prog, 0)
        c.run(100)
        assert c.halted
        # Step should return snapshot without advancing.
        snap = c.step()
        assert snap is not None


# =========================================================================
# FPUnitConfig Tests
# =========================================================================


class TestFPUnitConfig:
    """Verify FPUnitConfig dataclass."""

    def test_defaults(self) -> None:
        cfg = FPUnitConfig()
        assert cfg.formats == ()
        assert cfg.pipeline_depth == 4

    def test_custom(self) -> None:
        cfg = FPUnitConfig(formats=("fp32", "fp64"), pipeline_depth=5)
        assert cfg.formats == ("fp32", "fp64")
        assert cfg.pipeline_depth == 5
