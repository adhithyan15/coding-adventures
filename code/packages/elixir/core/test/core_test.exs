defmodule CodingAdventures.CoreTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Core.{
    Config,
    Config.RegisterFileConfig,
    Config.FPUnitConfig,
    MultiCoreConfig,
    RegisterFile,
    MemoryController,
    InterruptController,
    MockDecoder,
    Stats
  }

  alias CodingAdventures.Core.Core, as: CoreModule
  alias CodingAdventures.Core.MultiCore
  alias CodingAdventures.CpuPipeline.{Pipeline, PipelineConfig, Token}

  # =========================================================================
  # Test Helpers
  # =========================================================================

  defp make_simple_core do
    {:ok, core_tuple} = CoreModule.new(Config.simple_config(), MockDecoder)
    core_tuple
  end

  defp make_default_core do
    {:ok, core_tuple} = CoreModule.new(Config.default_config(), MockDecoder)
    core_tuple
  end

  defp encode_prog(instructions) do
    MockDecoder.encode_program(instructions)
  end

  # Clean up agents after tests
  setup do
    on_exit(fn -> :ok end)
    :ok
  end

  # =========================================================================
  # Config Tests
  # =========================================================================

  test "default config has sensible values" do
    cfg = Config.default_config()
    assert cfg.name == "Default"
    assert cfg.hazard_detection == true
    assert cfg.forwarding == true
  end

  test "simple config fields" do
    cfg = Config.simple_config()
    assert cfg.name == "Simple"
    assert PipelineConfig.num_stages(cfg.pipeline) == 5
    assert cfg.register_file.count == 16
    assert cfg.fp_unit == nil
  end

  test "cortex A78-like config fields" do
    cfg = Config.cortex_a78_like_config()
    assert cfg.name == "CortexA78Like"
    assert PipelineConfig.num_stages(cfg.pipeline) == 13
    assert cfg.register_file.count == 31
    assert cfg.register_file.width == 64
    assert cfg.fp_unit != nil
    assert "fp32" in cfg.fp_unit.formats
  end

  test "default multi-core config" do
    cfg = MultiCoreConfig.default_config()
    assert cfg.num_cores == 2
    assert cfg.memory_size == 1_048_576
  end

  # =========================================================================
  # RegisterFile Tests
  # =========================================================================

  test "register file basic read/write" do
    rf = RegisterFile.new()
    rf = RegisterFile.write(rf, 1, 42)
    assert RegisterFile.read(rf, 1) == 42

    rf = RegisterFile.write(rf, 1, 100)
    assert RegisterFile.read(rf, 1) == 100
  end

  test "register file zero register" do
    cfg = %RegisterFileConfig{count: 16, width: 32, zero_register: true}
    rf = RegisterFile.new(cfg)
    rf = RegisterFile.write(rf, 0, 999)
    assert RegisterFile.read(rf, 0) == 0
  end

  test "register file no zero register" do
    cfg = %RegisterFileConfig{count: 16, width: 32, zero_register: false}
    rf = RegisterFile.new(cfg)
    rf = RegisterFile.write(rf, 0, 999)
    assert RegisterFile.read(rf, 0) == 999
  end

  test "register file out of range" do
    rf = RegisterFile.new()
    assert RegisterFile.read(rf, 100) == 0
    assert RegisterFile.read(rf, -1) == 0

    # Should not crash
    rf = RegisterFile.write(rf, 100, 42)
    rf = RegisterFile.write(rf, -1, 42)
    assert rf != nil
  end

  test "register file values" do
    rf = RegisterFile.new()
    rf = RegisterFile.write(rf, 1, 10)
    rf = RegisterFile.write(rf, 2, 20)

    vals = RegisterFile.values(rf)
    assert length(vals) == RegisterFile.count(rf)
    assert Enum.at(vals, 1) == 10
    assert Enum.at(vals, 2) == 20
  end

  test "register file reset" do
    rf = RegisterFile.new()
    rf = RegisterFile.write(rf, 1, 42)
    rf = RegisterFile.write(rf, 5, 99)
    rf = RegisterFile.reset(rf)
    assert RegisterFile.read(rf, 1) == 0
    assert RegisterFile.read(rf, 5) == 0
  end

  test "register file to_string" do
    rf = RegisterFile.new()
    rf = RegisterFile.write(rf, 1, 42)
    s = RegisterFile.to_string(rf)
    assert s != ""
    assert String.contains?(s, "R1=42")
  end

  test "register file bit width masking" do
    cfg = %RegisterFileConfig{count: 4, width: 8, zero_register: false}
    rf = RegisterFile.new(cfg)
    rf = RegisterFile.write(rf, 1, 0xABCD)
    assert RegisterFile.read(rf, 1) == 0xCD
  end

  test "register file count and width" do
    rf = RegisterFile.new()
    assert RegisterFile.count(rf) == 16
    assert RegisterFile.width(rf) == 32
  end

  test "register file config" do
    rf = RegisterFile.new()
    cfg = RegisterFile.config(rf)
    assert cfg.count == 16
  end

  test "register file 64-bit" do
    cfg = %RegisterFileConfig{count: 32, width: 64, zero_register: true}
    rf = RegisterFile.new(cfg)
    assert RegisterFile.width(rf) == 64
    assert RegisterFile.count(rf) == 32

    rf = RegisterFile.write(rf, 1, 0x80000000)
    assert RegisterFile.read(rf, 1) == 0x80000000
  end

  # =========================================================================
  # MemoryController Tests
  # =========================================================================

  test "memory controller read/write word" do
    mc = MemoryController.new(4096, 10)
    mc = MemoryController.write_word(mc, 100, 0x1234ABCD)
    assert MemoryController.read_word(mc, 100) == 0x1234ABCD
  end

  test "memory controller load program" do
    mc = MemoryController.new(4096, 10)
    mc = MemoryController.load_program(mc, [0x01, 0x02, 0x03, 0x04], 0)

    word = MemoryController.read_word(mc, 0)
    # Little-endian: 0x04030201
    assert word == 0x04030201
  end

  test "memory controller pending requests" do
    mc = MemoryController.new(4096, 3)
    mc = MemoryController.write_word(mc, 0, 42)
    mc = MemoryController.request_read(mc, 0, 4, 0)

    assert MemoryController.pending_count(mc) == 1

    # Tick 1 and 2: not ready yet.
    {mc, result1} = MemoryController.tick(mc)
    assert length(result1) == 0
    {mc, result2} = MemoryController.tick(mc)
    assert length(result2) == 0

    # Tick 3: ready.
    {_mc, result3} = MemoryController.tick(mc)
    assert length(result3) == 1
    assert hd(result3).requester_id == 0
  end

  test "memory controller bounds check" do
    mc = MemoryController.new(64, 1)
    # Should not crash
    assert MemoryController.read_word(mc, 1000) == 0
    mc = MemoryController.write_word(mc, 1000, 42)
    mc = MemoryController.load_program(mc, [1, 2, 3, 4], 1000)
    assert mc != nil
  end

  test "memory controller size" do
    mc = MemoryController.new(4096, 10)
    assert MemoryController.memory_size(mc) == 4096
  end

  test "memory controller write request" do
    mc = MemoryController.new(4096, 2)
    mc = MemoryController.request_write(mc, 100, <<0xAA, 0xBB, 0xCC, 0xDD>>, 0)

    # Tick 1: not committed.
    {mc, _} = MemoryController.tick(mc)
    assert MemoryController.read_word(mc, 100) == 0

    # Tick 2: committed.
    {mc, _} = MemoryController.tick(mc)
    word = MemoryController.read_word(mc, 100)
    assert Bitwise.band(word, 0xFF) == 0xAA
  end

  # =========================================================================
  # InterruptController Tests
  # =========================================================================

  test "interrupt controller basic" do
    ic = InterruptController.new(4)
    ic = InterruptController.raise_interrupt(ic, 1, 2)
    assert InterruptController.pending_count(ic) == 1

    pending = InterruptController.pending_for_core(ic, 2)
    assert length(pending) == 1
    assert hd(pending).interrupt_id == 1

    ic = InterruptController.acknowledge(ic, 2, 1)
    assert InterruptController.pending_count(ic) == 0
    assert InterruptController.acknowledged_count(ic) == 1
  end

  test "interrupt controller default routing" do
    ic = InterruptController.new(4)
    ic = InterruptController.raise_interrupt(ic, 5, -1)
    pending = InterruptController.pending_for_core(ic, 0)
    assert length(pending) == 1
  end

  test "interrupt controller reset" do
    ic = InterruptController.new(4)
    ic = InterruptController.raise_interrupt(ic, 1, 0)
    ic = InterruptController.acknowledge(ic, 0, 1)
    ic = InterruptController.reset(ic)
    assert InterruptController.pending_count(ic) == 0
    assert InterruptController.acknowledged_count(ic) == 0
  end

  test "interrupt controller overflow routing" do
    ic = InterruptController.new(2)
    ic = InterruptController.raise_interrupt(ic, 1, 99)
    pending = InterruptController.pending_for_core(ic, 0)
    assert length(pending) == 1
  end

  # =========================================================================
  # MockDecoder Tests
  # =========================================================================

  test "mock decoder instruction size" do
    assert MockDecoder.instruction_size() == 4
  end

  test "mock decoder decode all instruction types" do
    cases = [
      {MockDecoder.encode_nop(), "NOP", -1, -1, -1},
      {MockDecoder.encode_add(3, 1, 2), "ADD", 3, 1, 2},
      {MockDecoder.encode_sub(3, 1, 2), "SUB", 3, 1, 2},
      {MockDecoder.encode_addi(1, 0, 42), "ADDI", 1, 0, -1},
      {MockDecoder.encode_load(1, 2, 100), "LOAD", 1, 2, -1},
      {MockDecoder.encode_store(2, 3, 100), "STORE", -1, 2, 3},
      {MockDecoder.encode_branch(1, 2, 4), "BRANCH", -1, 1, 2},
      {MockDecoder.encode_halt(), "HALT", -1, -1, -1}
    ]

    for {raw, expected_opcode, expected_rd, expected_rs1, expected_rs2} <- cases do
      token = Token.new()
      result = MockDecoder.decode(raw, token)
      assert result.opcode == expected_opcode, "Expected #{expected_opcode}, got #{result.opcode}"
      assert result.rd == expected_rd, "#{expected_opcode}: Rd expected #{expected_rd}, got #{result.rd}"
      assert result.rs1 == expected_rs1, "#{expected_opcode}: Rs1 expected #{expected_rs1}, got #{result.rs1}"
      assert result.rs2 == expected_rs2, "#{expected_opcode}: Rs2 expected #{expected_rs2}, got #{result.rs2}"
    end
  end

  test "mock decoder execute ADD" do
    rf = RegisterFile.new()
    rf = RegisterFile.write(rf, 1, 10)
    rf = RegisterFile.write(rf, 2, 20)

    token = Token.new()
    token = MockDecoder.decode(MockDecoder.encode_add(3, 1, 2), token)
    token = MockDecoder.execute(token, rf)
    assert token.alu_result == 30
  end

  test "mock decoder execute SUB" do
    rf = RegisterFile.new()
    rf = RegisterFile.write(rf, 1, 10)
    rf = RegisterFile.write(rf, 2, 20)

    token = Token.new()
    token = MockDecoder.decode(MockDecoder.encode_sub(3, 2, 1), token)
    token = MockDecoder.execute(token, rf)
    assert token.alu_result == 10
  end

  test "mock decoder execute ADDI" do
    rf = RegisterFile.new()
    rf = RegisterFile.write(rf, 1, 10)

    token = Token.new()
    token = MockDecoder.decode(MockDecoder.encode_addi(3, 1, 5), token)
    token = MockDecoder.execute(token, rf)
    assert token.alu_result == 15
  end

  test "mock decoder execute LOAD effective address" do
    rf = RegisterFile.new()
    rf = RegisterFile.write(rf, 1, 10)

    token = Token.new()
    token = MockDecoder.decode(MockDecoder.encode_load(3, 1, 100), token)
    token = MockDecoder.execute(token, rf)
    assert token.alu_result == 110
  end

  test "mock decoder execute BRANCH taken" do
    rf = RegisterFile.new()
    rf = RegisterFile.write(rf, 1, 10)

    token = Token.new()
    token = %{token | pc: 100}
    token = MockDecoder.decode(MockDecoder.encode_branch(1, 1, 3), token)
    token = MockDecoder.execute(token, rf)
    assert token.branch_taken == true
    assert token.branch_target == 100 + 3 * 4
  end

  test "mock decoder execute BRANCH not taken" do
    rf = RegisterFile.new()
    rf = RegisterFile.write(rf, 1, 10)
    rf = RegisterFile.write(rf, 2, 20)

    token = Token.new()
    token = %{token | pc: 100}
    token = MockDecoder.decode(MockDecoder.encode_branch(1, 2, 3), token)
    token = MockDecoder.execute(token, rf)
    assert token.branch_taken == false
  end

  test "negative immediate sign extension" do
    token = Token.new()
    raw = MockDecoder.encode_addi(1, 0, 0xFFF)
    token = MockDecoder.decode(raw, token)
    assert token.immediate < 0
  end

  test "unknown opcode decodes as NOP" do
    token = Token.new()
    raw = 0xFF <<< 24
    token = MockDecoder.decode(raw, token)
    assert token.opcode == "NOP"
  end

  test "encode_program produces correct bytes" do
    prog = MockDecoder.encode_program([0x01020304, 0x05060708])
    assert length(prog) == 8
    # First instruction: 0x01020304 in little-endian = 04, 03, 02, 01
    assert Enum.at(prog, 0) == 0x04
    assert Enum.at(prog, 1) == 0x03
    assert Enum.at(prog, 2) == 0x02
    assert Enum.at(prog, 3) == 0x01
  end

  # =========================================================================
  # Core Assembly Tests
  # =========================================================================

  test "core construction succeeds" do
    core_tuple = make_simple_core()
    {core, _agent} = core_tuple
    assert core.pipeline != nil
    assert core.reg_file != nil
    assert core.mem_ctrl != nil
    refute CoreModule.halted?(core_tuple)
    assert CoreModule.cycle(core_tuple) == 0
    CoreModule.stop(core_tuple)
  end

  test "simple config runs" do
    ct = make_simple_core()
    prog = encode_prog([MockDecoder.encode_halt()])
    ct = CoreModule.load_program(ct, prog, 0)
    {ct, stats} = CoreModule.run(ct, 100)
    assert stats.total_cycles > 0
    CoreModule.stop(ct)
  end

  test "HALT stops pipeline" do
    ct = make_simple_core()
    prog = encode_prog([MockDecoder.encode_halt()])
    ct = CoreModule.load_program(ct, prog, 0)
    {ct, _stats} = CoreModule.run(ct, 100)
    assert CoreModule.halted?(ct)
    CoreModule.stop(ct)
  end

  test "ADDI produces correct register value" do
    ct = make_simple_core()
    prog = encode_prog([
      MockDecoder.encode_addi(1, 0, 42),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_halt()
    ])
    ct = CoreModule.load_program(ct, prog, 0)
    {ct, _stats} = CoreModule.run(ct, 100)
    assert CoreModule.halted?(ct)
    assert CoreModule.read_register(ct, 1) == 42
    CoreModule.stop(ct)
  end

  test "ADD produces correct register value" do
    ct = make_simple_core()
    prog = encode_prog([
      MockDecoder.encode_addi(1, 0, 10),
      MockDecoder.encode_addi(2, 0, 20),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_add(3, 1, 2),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_halt()
    ])
    ct = CoreModule.load_program(ct, prog, 0)
    {ct, _stats} = CoreModule.run(ct, 200)
    assert CoreModule.halted?(ct)
    assert CoreModule.read_register(ct, 1) == 10
    assert CoreModule.read_register(ct, 2) == 20
    assert CoreModule.read_register(ct, 3) == 30
    CoreModule.stop(ct)
  end

  test "SUB produces correct register value" do
    ct = make_simple_core()
    prog = encode_prog([
      MockDecoder.encode_addi(1, 0, 50),
      MockDecoder.encode_addi(2, 0, 20),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_sub(3, 1, 2),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_halt()
    ])
    ct = CoreModule.load_program(ct, prog, 0)
    {ct, _stats} = CoreModule.run(ct, 200)
    assert CoreModule.halted?(ct)
    assert CoreModule.read_register(ct, 3) == 30
    CoreModule.stop(ct)
  end

  test "NOP does not modify registers" do
    ct = make_simple_core()
    prog = encode_prog([
      MockDecoder.encode_nop(),
      MockDecoder.encode_halt()
    ])
    ct = CoreModule.load_program(ct, prog, 0)
    {ct, _stats} = CoreModule.run(ct, 100)
    assert CoreModule.halted?(ct)

    for i <- 0..15 do
      assert CoreModule.read_register(ct, i) == 0
    end
    CoreModule.stop(ct)
  end

  test "STORE writes to memory" do
    ct = make_simple_core()
    prog = encode_prog([
      MockDecoder.encode_addi(1, 0, 0),
      MockDecoder.encode_addi(2, 0, 0x42),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_store(1, 2, 512),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_halt()
    ])
    ct = CoreModule.load_program(ct, prog, 0)
    {ct, _stats} = CoreModule.run(ct, 200)
    assert CoreModule.halted?(ct)

    mc = CoreModule.memory_controller(ct)
    assert MemoryController.read_word(mc, 512) == 0x42
    CoreModule.stop(ct)
  end

  test "LOAD reads from memory" do
    ct = make_simple_core()
    # Write test data to memory
    {_core, agent} = ct
    Agent.update(agent, fn c ->
      %{c | mem_ctrl: MemoryController.write_word(c.mem_ctrl, 512, 0xDEAD)}
    end)

    prog = encode_prog([
      MockDecoder.encode_addi(1, 0, 0),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_load(2, 1, 512),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_halt()
    ])
    ct = CoreModule.load_program(ct, prog, 0)
    {ct, _stats} = CoreModule.run(ct, 200)
    assert CoreModule.halted?(ct)
    assert CoreModule.read_register(ct, 2) == 0xDEAD
    CoreModule.stop(ct)
  end

  # =========================================================================
  # Statistics Tests
  # =========================================================================

  test "IPC calculation" do
    ct = make_simple_core()
    prog = encode_prog([
      MockDecoder.encode_addi(1, 0, 1),
      MockDecoder.encode_addi(2, 0, 2),
      MockDecoder.encode_addi(3, 0, 3),
      MockDecoder.encode_halt()
    ])
    ct = CoreModule.load_program(ct, prog, 0)
    {ct, stats} = CoreModule.run(ct, 200)
    assert stats.instructions_completed > 0
    assert stats.total_cycles > 0

    expected_ipc = stats.instructions_completed / stats.total_cycles
    assert Stats.ipc(stats) == expected_ipc
    CoreModule.stop(ct)
  end

  test "stats with zero values" do
    stats = %Stats{}
    assert Stats.ipc(stats) == 0.0
    assert Stats.cpi(stats) == 0.0
  end

  test "stats to_string" do
    ct = make_simple_core()
    prog = encode_prog([MockDecoder.encode_addi(1, 0, 1), MockDecoder.encode_halt()])
    ct = CoreModule.load_program(ct, prog, 0)
    {ct, stats} = CoreModule.run(ct, 100)
    s = Stats.to_string(stats)
    assert s != ""
    assert String.contains?(s, "Core Statistics")
    CoreModule.stop(ct)
  end

  # =========================================================================
  # Core Accessor Tests
  # =========================================================================

  test "core read/write register" do
    ct = make_simple_core()
    ct = CoreModule.write_register(ct, 5, 123)
    assert CoreModule.read_register(ct, 5) == 123
    CoreModule.stop(ct)
  end

  test "step-by-step execution" do
    ct = make_simple_core()
    prog = encode_prog([MockDecoder.encode_halt()])
    ct = CoreModule.load_program(ct, prog, 0)

    ct = Enum.reduce_while(1..20, ct, fn _, acc ->
      {acc, _snap} = CoreModule.step(acc)
      if CoreModule.halted?(acc), do: {:halt, acc}, else: {:cont, acc}
    end)

    assert CoreModule.halted?(ct)
    assert CoreModule.cycle(ct) > 0
    CoreModule.stop(ct)
  end

  test "step after halt is no-op" do
    ct = make_simple_core()
    prog = encode_prog([MockDecoder.encode_halt()])
    ct = CoreModule.load_program(ct, prog, 0)
    {ct, _stats} = CoreModule.run(ct, 100)

    cycle_before = CoreModule.cycle(ct)
    {ct, _snap} = CoreModule.step(ct)
    assert CoreModule.cycle(ct) == cycle_before
    CoreModule.stop(ct)
  end

  test "core config accessor" do
    ct = make_simple_core()
    {core, _} = ct
    assert CoreModule.config(core).name == "Simple"
    CoreModule.stop(ct)
  end

  # =========================================================================
  # Multi-Core Tests
  # =========================================================================

  test "multi-core construction" do
    config = MultiCoreConfig.default_config()
    {:ok, mc} = MultiCore.new(config, [MockDecoder, MockDecoder])
    assert length(MultiCore.cores(mc)) == 2
    assert MultiCore.interrupt_controller(mc) != nil
    assert MultiCore.shared_memory_controller(mc) != nil
    MultiCore.stop(mc)
  end

  test "multi-core independent programs" do
    config = MultiCoreConfig.default_config()
    {:ok, mc} = MultiCore.new(config, [MockDecoder, MockDecoder])

    prog0 = encode_prog([
      MockDecoder.encode_addi(1, 0, 10),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_halt()
    ])
    prog1 = encode_prog([
      MockDecoder.encode_addi(1, 0, 20),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_halt()
    ])

    mc = MultiCore.load_program(mc, 0, prog0, 0)
    mc = MultiCore.load_program(mc, 1, prog1, 4096)

    {mc, _stats} = MultiCore.run(mc, 200)
    assert MultiCore.all_halted?(mc)

    [ct0, ct1] = MultiCore.cores(mc)
    assert CoreModule.read_register(ct0, 1) == 10
    assert CoreModule.read_register(ct1, 1) == 20
    MultiCore.stop(mc)
  end

  test "multi-core stats" do
    config = MultiCoreConfig.default_config()
    {:ok, mc} = MultiCore.new(config, [MockDecoder, MockDecoder])

    prog = encode_prog([MockDecoder.encode_addi(1, 0, 1), MockDecoder.encode_halt()])
    mc = MultiCore.load_program(mc, 0, prog, 0)
    mc = MultiCore.load_program(mc, 1, prog, 4096)

    {mc, stats} = MultiCore.run(mc, 200)
    assert length(stats) == 2

    for s <- stats do
      assert s.total_cycles > 0
    end
    MultiCore.stop(mc)
  end

  test "multi-core step returns snapshots" do
    config = MultiCoreConfig.default_config()
    {:ok, mc} = MultiCore.new(config, [MockDecoder, MockDecoder])

    prog = encode_prog([MockDecoder.encode_halt()])
    mc = MultiCore.load_program(mc, 0, prog, 0)
    mc = MultiCore.load_program(mc, 1, prog, 4096)

    {_mc, snapshots} = MultiCore.step(mc)
    assert length(snapshots) == 2
    MultiCore.stop(mc)
  end

  test "multi-core cycle counter" do
    config = MultiCoreConfig.default_config()
    {:ok, mc} = MultiCore.new(config, [MockDecoder, MockDecoder])
    assert MultiCore.cycle(mc) == 0

    prog = encode_prog([MockDecoder.encode_halt()])
    mc = MultiCore.load_program(mc, 0, prog, 0)
    mc = MultiCore.load_program(mc, 1, prog, 4096)

    {mc, _snaps} = MultiCore.step(mc)
    assert MultiCore.cycle(mc) == 1
    MultiCore.stop(mc)
  end

  # =========================================================================
  # Counting program test
  # =========================================================================

  test "counting program" do
    ct = make_simple_core()
    prog = encode_prog([
      MockDecoder.encode_addi(1, 0, 0),   # R1 = 0
      MockDecoder.encode_addi(2, 0, 1),   # R2 = 1
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_add(1, 1, 2),    # R1 = 1
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_add(1, 1, 2),    # R1 = 2
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_add(1, 1, 2),    # R1 = 3
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_add(1, 1, 2),    # R1 = 4
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_add(1, 1, 2),    # R1 = 5
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_nop(),
      MockDecoder.encode_halt()
    ])
    ct = CoreModule.load_program(ct, prog, 0)
    {ct, _stats} = CoreModule.run(ct, 500)
    assert CoreModule.halted?(ct)
    assert CoreModule.read_register(ct, 1) == 5
    CoreModule.stop(ct)
  end
end
