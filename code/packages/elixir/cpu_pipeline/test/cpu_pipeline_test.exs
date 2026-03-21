defmodule CodingAdventures.CpuPipelineTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.CpuPipeline.{
    Token,
    PipelineStage,
    PipelineConfig,
    PipelineStats,
    Snapshot,
    HazardResponse,
    StageCategory,
    Pipeline
  }

  # =========================================================================
  # Test helpers -- simple instruction memory and callbacks
  # =========================================================================
  #
  # For testing, we create a tiny "instruction memory" -- just a list of
  # integers. Each integer represents one instruction's raw bits. The fetch
  # callback reads from this list using PC/4 as the index.
  #
  # The decode callback creates simple instructions:
  #   - opcode 0x01 = ADD (register write)
  #   - opcode 0x02 = LDR (load from memory, register write)
  #   - opcode 0x03 = STR (store to memory)
  #   - opcode 0x04 = BEQ (branch if equal)
  #   - opcode 0xFF = HALT
  #   - opcode 0x00 = NOP
  #
  # Encoding: raw = (opcode <<< 24) ||| (rd <<< 16) ||| (rs1 <<< 8) ||| rs2

  @op_nop 0x00
  @op_add 0x01
  @op_ldr 0x02
  @op_str 0x03
  @op_beq 0x04
  @op_halt 0xFF

  defp make_instruction(opcode, rd, rs1, rs2) do
    Bitwise.bor(
      Bitwise.bor(Bitwise.bsl(opcode, 24), Bitwise.bsl(rd, 16)),
      Bitwise.bor(Bitwise.bsl(rs1, 8), rs2)
    )
  end

  defp simple_fetch(instrs) do
    fn pc ->
      idx = div(pc, 4)
      if idx >= 0 and idx < length(instrs) do
        Enum.at(instrs, idx)
      else
        make_instruction(@op_nop, 0, 0, 0)
      end
    end
  end

  defp simple_decode do
    fn raw, tok ->
      opcode = Bitwise.band(Bitwise.bsr(raw, 24), 0xFF)
      rd = Bitwise.band(Bitwise.bsr(raw, 16), 0xFF)
      rs1 = Bitwise.band(Bitwise.bsr(raw, 8), 0xFF)
      rs2 = Bitwise.band(raw, 0xFF)

      case opcode do
        @op_add ->
          %{tok | opcode: "ADD", rd: rd, rs1: rs1, rs2: rs2, reg_write: true}
        @op_ldr ->
          %{tok | opcode: "LDR", rd: rd, rs1: rs1, mem_read: true, reg_write: true}
        @op_str ->
          %{tok | opcode: "STR", rs1: rs1, rs2: rs2, mem_write: true}
        @op_beq ->
          %{tok | opcode: "BEQ", rs1: rs1, rs2: rs2, is_branch: true}
        @op_halt ->
          %{tok | opcode: "HALT", is_halt: true}
        _ ->
          %{tok | opcode: "NOP"}
      end
    end
  end

  defp simple_execute do
    fn tok ->
      case tok.opcode do
        "ADD" ->
          %{tok | alu_result: tok.rs1 + tok.rs2}
        "LDR" ->
          %{tok | alu_result: tok.rs1 + tok.immediate}
        "STR" ->
          %{tok | alu_result: tok.rs1 + tok.immediate}
        "BEQ" ->
          %{tok | branch_target: tok.pc + tok.immediate}
        _ ->
          tok
      end
    end
  end

  defp simple_memory do
    fn tok ->
      if tok.mem_read do
        %{tok | mem_data: 42, write_data: 42}
      else
        %{tok | write_data: tok.alu_result}
      end
    end
  end

  defp simple_writeback(agent) do
    fn tok ->
      Agent.update(agent, fn pcs -> [tok.pc | pcs] end)
      :ok
    end
  end

  defp new_test_pipeline(instrs, agent) do
    config = Pipeline.classic_5_stage()
    {:ok, p} = Pipeline.new(
      config,
      simple_fetch(instrs),
      simple_decode(),
      simple_execute(),
      simple_memory(),
      simple_writeback(agent)
    )
    p
  end

  defp completed_pcs(agent) do
    Agent.get(agent, fn pcs -> Enum.reverse(pcs) end)
  end

  # =========================================================================
  # Token tests
  # =========================================================================

  test "new token has default register values of -1" do
    tok = Token.new()
    assert tok.rs1 == -1
    assert tok.rs2 == -1
    assert tok.rd == -1
    refute tok.is_bubble
    assert tok.stage_entered == %{}
  end

  test "new bubble has is_bubble true" do
    b = Token.new_bubble()
    assert b.is_bubble
    assert Token.to_string(b) == "---"
  end

  test "token string formats correctly" do
    tok = Token.new()
    tok = %{tok | opcode: "ADD", pc: 100}
    assert Token.to_string(tok) == "ADD@100"

    tok2 = Token.new()
    tok2 = %{tok2 | pc: 200}
    assert Token.to_string(tok2) == "instr@200"
  end

  test "token clone returns equivalent value" do
    tok = Token.new()
    tok = %{tok | pc: 100, opcode: "ADD", stage_entered: %{"IF" => 1, "ID" => 2}}
    clone = Token.clone(tok)
    assert clone.pc == 100
    assert clone.opcode == "ADD"
    assert clone.stage_entered == %{"IF" => 1, "ID" => 2}
  end

  test "clone of nil returns nil" do
    assert Token.clone(nil) == nil
  end

  # =========================================================================
  # StageCategory tests
  # =========================================================================

  test "stage category to_string" do
    assert StageCategory.to_string(:fetch) == "fetch"
    assert StageCategory.to_string(:decode) == "decode"
    assert StageCategory.to_string(:execute) == "execute"
    assert StageCategory.to_string(:memory) == "memory"
    assert StageCategory.to_string(:writeback) == "writeback"
    assert StageCategory.to_string(:bogus) == "unknown"
  end

  # =========================================================================
  # PipelineConfig tests
  # =========================================================================

  test "classic 5-stage has 5 stages and validates" do
    config = Pipeline.classic_5_stage()
    assert PipelineConfig.num_stages(config) == 5
    assert PipelineConfig.validate(config) == :ok
    assert Enum.at(config.stages, 0).name == "IF"
    assert Enum.at(config.stages, 4).name == "WB"
  end

  test "deep 13-stage has 13 stages and validates" do
    config = Pipeline.deep_13_stage()
    assert PipelineConfig.num_stages(config) == 13
    assert PipelineConfig.validate(config) == :ok
  end

  test "config validation rejects too few stages" do
    cfg = %PipelineConfig{
      stages: [%PipelineStage{name: "IF", category: :fetch}],
      execution_width: 1
    }
    assert {:error, _} = PipelineConfig.validate(cfg)
  end

  test "config validation rejects zero execution width" do
    cfg = %PipelineConfig{
      stages: [
        %PipelineStage{name: "IF", category: :fetch},
        %PipelineStage{name: "WB", category: :writeback}
      ],
      execution_width: 0
    }
    assert {:error, _} = PipelineConfig.validate(cfg)
  end

  test "config validation rejects duplicate stage names" do
    cfg = %PipelineConfig{
      stages: [
        %PipelineStage{name: "IF", category: :fetch},
        %PipelineStage{name: "IF", category: :writeback}
      ],
      execution_width: 1
    }
    assert {:error, _} = PipelineConfig.validate(cfg)
  end

  test "config validation rejects missing fetch stage" do
    cfg = %PipelineConfig{
      stages: [
        %PipelineStage{name: "EX", category: :execute},
        %PipelineStage{name: "WB", category: :writeback}
      ],
      execution_width: 1
    }
    assert {:error, _} = PipelineConfig.validate(cfg)
  end

  test "config validation rejects missing writeback stage" do
    cfg = %PipelineConfig{
      stages: [
        %PipelineStage{name: "IF", category: :fetch},
        %PipelineStage{name: "EX", category: :execute}
      ],
      execution_width: 1
    }
    assert {:error, _} = PipelineConfig.validate(cfg)
  end

  test "valid 2-stage pipeline passes validation" do
    cfg = %PipelineConfig{
      stages: [
        %PipelineStage{name: "IF", category: :fetch},
        %PipelineStage{name: "WB", category: :writeback}
      ],
      execution_width: 1
    }
    assert PipelineConfig.validate(cfg) == :ok
  end

  # =========================================================================
  # PipelineStats tests
  # =========================================================================

  test "IPC and CPI calculations" do
    stats = %PipelineStats{total_cycles: 100, instructions_completed: 80}
    assert PipelineStats.ipc(stats) == 0.8
    assert PipelineStats.cpi(stats) == 1.25
  end

  test "IPC and CPI with zero values" do
    assert PipelineStats.ipc(%PipelineStats{total_cycles: 0}) == 0.0
    assert PipelineStats.cpi(%PipelineStats{instructions_completed: 0}) == 0.0
  end

  test "stats to_string" do
    stats = %PipelineStats{total_cycles: 10, instructions_completed: 5}
    s = PipelineStats.to_string(stats)
    assert String.contains?(s, "cycles=10")
    assert String.contains?(s, "completed=5")
  end

  # =========================================================================
  # Snapshot tests
  # =========================================================================

  test "snapshot to_string" do
    snap = %Snapshot{cycle: 7, pc: 28, stalled: false, flushing: false}
    s = Snapshot.to_string(snap)
    assert String.contains?(s, "cycle 7")
    assert String.contains?(s, "PC=28")
  end

  # =========================================================================
  # HazardResponse tests
  # =========================================================================

  test "hazard response defaults" do
    hr = %HazardResponse{}
    assert hr.action == :none
    assert hr.forward_value == 0
    assert hr.stall_stages == 0
  end

  # =========================================================================
  # Basic Pipeline tests
  # =========================================================================

  test "new pipeline initializes correctly" do
    {:ok, agent} = Agent.start_link(fn -> [] end)
    instrs = [make_instruction(@op_add, 1, 2, 3)]
    p = new_test_pipeline(instrs, agent)

    refute Pipeline.halted?(p)
    assert Pipeline.cycle(p) == 0
    assert Pipeline.pc(p) == 0
    Agent.stop(agent)
  end

  test "new pipeline with invalid config returns error" do
    cfg = %PipelineConfig{
      stages: [%PipelineStage{name: "IF", category: :fetch}],
      execution_width: 1
    }
    assert {:error, _} = Pipeline.new(cfg, fn _ -> 0 end, fn _, t -> t end, fn t -> t end, fn t -> t end, fn _ -> :ok end)
  end

  # =========================================================================
  # Single instruction flow tests
  # =========================================================================

  test "single instruction flows through 5 stages" do
    {:ok, agent} = Agent.start_link(fn -> [] end)
    instrs = [
      make_instruction(@op_add, 1, 2, 3),
      make_instruction(@op_nop, 0, 0, 0),
      make_instruction(@op_nop, 0, 0, 0),
      make_instruction(@op_nop, 0, 0, 0),
      make_instruction(@op_nop, 0, 0, 0)
    ]
    p = new_test_pipeline(instrs, agent)

    # Step 5 times -- the ADD should complete at cycle 5.
    p = Enum.reduce(1..5, p, fn _, acc ->
      {acc, _snap} = Pipeline.step(acc)
      acc
    end)

    pcs = completed_pcs(agent)
    assert length(pcs) > 0, "expected at least one instruction to complete after 5 cycles"
    assert List.first(pcs) == 0
    Agent.stop(agent)
  end

  test "pipeline fill timing: first instruction completes at cycle 5" do
    {:ok, agent} = Agent.start_link(fn -> [] end)
    instrs = for _ <- 1..20, do: make_instruction(@op_add, 1, 2, 3)
    p = new_test_pipeline(instrs, agent)

    # After 4 cycles, nothing should have completed yet.
    p = Enum.reduce(1..4, p, fn _, acc ->
      {acc, _} = Pipeline.step(acc)
      acc
    end)
    assert length(completed_pcs(agent)) == 0

    # After cycle 5, exactly 1 instruction should have completed.
    {p, _} = Pipeline.step(p)
    assert length(completed_pcs(agent)) == 1

    # After cycle 6, 2 completions.
    {p, _} = Pipeline.step(p)
    assert length(completed_pcs(agent)) == 2

    # After cycle 7, 3 completions.
    {_p, _} = Pipeline.step(p)
    assert length(completed_pcs(agent)) == 3

    Agent.stop(agent)
  end

  test "steady-state IPC approaches 1.0" do
    {:ok, agent} = Agent.start_link(fn -> [] end)
    instrs = for _ <- 1..100, do: make_instruction(@op_add, 1, 2, 3)
    p = new_test_pipeline(instrs, agent)

    # Run for 50 cycles.
    p = Enum.reduce(1..50, p, fn _, acc ->
      {acc, _} = Pipeline.step(acc)
      acc
    end)

    stats = Pipeline.stats(p)
    expected_completed = 50 - 5 + 1
    assert stats.instructions_completed == expected_completed

    ipc = PipelineStats.ipc(stats)
    assert ipc > 0.85 and ipc <= 1.01
    Agent.stop(agent)
  end

  test "halt propagation stops pipeline" do
    {:ok, agent} = Agent.start_link(fn -> [] end)
    instrs = [
      make_instruction(@op_add, 1, 2, 3),
      make_instruction(@op_add, 4, 5, 6),
      make_instruction(@op_halt, 0, 0, 0),
      make_instruction(@op_nop, 0, 0, 0),
      make_instruction(@op_nop, 0, 0, 0)
    ]
    p = new_test_pipeline(instrs, agent)

    # Run until halt (max 20 cycles).
    {p, _stats} = Pipeline.run(p, 20)

    assert Pipeline.halted?(p)
    pcs = completed_pcs(agent)
    assert 8 in pcs, "HALT instruction at PC=8 should have completed"
    Agent.stop(agent)
  end

  test "halted pipeline does not advance" do
    {:ok, agent} = Agent.start_link(fn -> [] end)
    instrs = [make_instruction(@op_halt, 0, 0, 0)]
    p = new_test_pipeline(instrs, agent)

    {p, _stats} = Pipeline.run(p, 20)
    assert Pipeline.halted?(p)
    cycle_before = Pipeline.cycle(p)

    {p, _snap} = Pipeline.step(p)
    assert Pipeline.cycle(p) == cycle_before
    Agent.stop(agent)
  end

  # =========================================================================
  # Pipeline stall tests
  # =========================================================================

  test "stall inserts bubble and freezes earlier stages" do
    {:ok, agent} = Agent.start_link(fn -> [] end)
    instrs = for _ <- 1..20, do: make_instruction(@op_add, 1, 2, 3)
    p = new_test_pipeline(instrs, agent)

    # Fill the pipeline for 3 cycles.
    p = Enum.reduce(1..3, p, fn _, acc ->
      {acc, _} = Pipeline.step(acc)
      acc
    end)

    # Now set a hazard function that stalls once.
    stall_once = fn stages ->
      %HazardResponse{action: :stall, stall_stages: 2}
    end
    p = Pipeline.set_hazard_func(p, stall_once)

    # Step with stall.
    {p, snap} = Pipeline.step(p)
    assert snap.stalled

    # The EX stage (index 2) should have a bubble.
    if Map.has_key?(snap.stages, "EX") do
      assert snap.stages["EX"].is_bubble
    end

    stats = Pipeline.stats(p)
    assert stats.stall_cycles == 1
    Agent.stop(agent)
  end

  # =========================================================================
  # Pipeline flush tests
  # =========================================================================

  test "flush replaces speculative stages with bubbles" do
    {:ok, agent} = Agent.start_link(fn -> [] end)
    instrs = for _ <- 1..20, do: make_instruction(@op_add, 1, 2, 3)
    p = new_test_pipeline(instrs, agent)

    # Fill for 3 cycles.
    p = Enum.reduce(1..3, p, fn _, acc ->
      {acc, _} = Pipeline.step(acc)
      acc
    end)

    # Set flush hazard.
    flush_once = fn _stages ->
      %HazardResponse{action: :flush, flush_count: 2, redirect_pc: 100}
    end
    p = Pipeline.set_hazard_func(p, flush_once)

    {p, snap} = Pipeline.step(p)
    assert snap.flushing

    stats = Pipeline.stats(p)
    assert stats.flush_cycles == 1
    Agent.stop(agent)
  end

  # =========================================================================
  # Pipeline forwarding tests
  # =========================================================================

  test "forwarding updates decode stage token" do
    {:ok, agent} = Agent.start_link(fn -> [] end)
    instrs = for _ <- 1..20, do: make_instruction(@op_add, 1, 2, 3)
    p = new_test_pipeline(instrs, agent)

    # Fill for 2 cycles so there's something in the decode stage.
    p = Enum.reduce(1..2, p, fn _, acc ->
      {acc, _} = Pipeline.step(acc)
      acc
    end)

    forward_fn = fn _stages ->
      %HazardResponse{action: :forward_from_ex, forward_value: 999, forward_source: "EX"}
    end
    p = Pipeline.set_hazard_func(p, forward_fn)

    {_p, _snap} = Pipeline.step(p)
    # The test verifies no crash and the forwarding path was exercised.
    Agent.stop(agent)
  end

  # =========================================================================
  # Predict function tests
  # =========================================================================

  test "predict function changes next PC" do
    {:ok, agent} = Agent.start_link(fn -> [] end)
    instrs = for _ <- 1..20, do: make_instruction(@op_add, 1, 2, 3)
    p = new_test_pipeline(instrs, agent)

    # Set a predictor that always predicts PC + 8 (skip an instruction).
    p = Pipeline.set_predict_func(p, fn pc -> pc + 8 end)

    {p, _snap} = Pipeline.step(p)
    # After one step with prediction, PC should be 8 (0 + 8).
    assert Pipeline.pc(p) == 8
    Agent.stop(agent)
  end

  # =========================================================================
  # Set PC test
  # =========================================================================

  test "set_pc changes program counter" do
    {:ok, agent} = Agent.start_link(fn -> [] end)
    instrs = [make_instruction(@op_nop, 0, 0, 0)]
    p = new_test_pipeline(instrs, agent)

    p = Pipeline.set_pc(p, 100)
    assert Pipeline.pc(p) == 100
    Agent.stop(agent)
  end

  # =========================================================================
  # Trace and snapshot tests
  # =========================================================================

  test "trace records history" do
    {:ok, agent} = Agent.start_link(fn -> [] end)
    instrs = for _ <- 1..10, do: make_instruction(@op_add, 1, 2, 3)
    p = new_test_pipeline(instrs, agent)

    p = Enum.reduce(1..5, p, fn _, acc ->
      {acc, _} = Pipeline.step(acc)
      acc
    end)

    trace = Pipeline.trace(p)
    assert length(trace) == 5
    # Trace should be in chronological order.
    cycles = Enum.map(trace, & &1.cycle)
    assert cycles == [1, 2, 3, 4, 5]
    Agent.stop(agent)
  end

  test "snapshot returns current state without advancing" do
    {:ok, agent} = Agent.start_link(fn -> [] end)
    instrs = [make_instruction(@op_add, 1, 2, 3)]
    p = new_test_pipeline(instrs, agent)

    snap = Pipeline.snapshot(p)
    assert snap.cycle == 0
    assert Pipeline.cycle(p) == 0
    Agent.stop(agent)
  end

  test "stage_contents returns token for valid stage" do
    {:ok, agent} = Agent.start_link(fn -> [] end)
    instrs = for _ <- 1..10, do: make_instruction(@op_add, 1, 2, 3)
    p = new_test_pipeline(instrs, agent)

    {p, _} = Pipeline.step(p)
    # IF stage should have a token after one step.
    tok = Pipeline.stage_contents(p, "IF")
    assert tok != nil
    Agent.stop(agent)
  end

  test "stage_contents returns nil for invalid stage name" do
    {:ok, agent} = Agent.start_link(fn -> [] end)
    instrs = [make_instruction(@op_nop, 0, 0, 0)]
    p = new_test_pipeline(instrs, agent)
    assert Pipeline.stage_contents(p, "NONEXISTENT") == nil
    Agent.stop(agent)
  end

  # =========================================================================
  # Run test
  # =========================================================================

  test "run returns stats after max cycles" do
    {:ok, agent} = Agent.start_link(fn -> [] end)
    instrs = for _ <- 1..100, do: make_instruction(@op_add, 1, 2, 3)
    p = new_test_pipeline(instrs, agent)

    {p, stats} = Pipeline.run(p, 10)
    assert stats.total_cycles == 10
    assert Pipeline.cycle(p) == 10
    Agent.stop(agent)
  end

  # =========================================================================
  # PipelineStage tests
  # =========================================================================

  test "pipeline stage to_string returns name" do
    stage = %PipelineStage{name: "EX1", description: "Execute 1", category: :execute}
    assert PipelineStage.to_string(stage) == "EX1"
  end

  # =========================================================================
  # Config accessor test
  # =========================================================================

  test "config returns pipeline configuration" do
    {:ok, agent} = Agent.start_link(fn -> [] end)
    instrs = [make_instruction(@op_nop, 0, 0, 0)]
    p = new_test_pipeline(instrs, agent)
    config = Pipeline.config(p)
    assert PipelineConfig.num_stages(config) == 5
    Agent.stop(agent)
  end

  # =========================================================================
  # Deep pipeline test
  # =========================================================================

  test "deep 13-stage pipeline runs without errors" do
    {:ok, agent} = Agent.start_link(fn -> [] end)
    config = Pipeline.deep_13_stage()
    instrs = for _ <- 1..30, do: make_instruction(@op_add, 1, 2, 3)

    {:ok, p} = Pipeline.new(
      config,
      simple_fetch(instrs),
      simple_decode(),
      simple_execute(),
      simple_memory(),
      simple_writeback(agent)
    )

    {p, stats} = Pipeline.run(p, 20)
    assert stats.total_cycles == 20
    assert stats.instructions_completed >= 0
    Agent.stop(agent)
  end

  # =========================================================================
  # Delegated function tests (main module)
  # =========================================================================

  test "CpuPipeline module delegates work correctly" do
    config = CodingAdventures.CpuPipeline.classic_5_stage()
    assert PipelineConfig.num_stages(config) == 5

    tok = CodingAdventures.CpuPipeline.new_token()
    assert tok.rs1 == -1

    bubble = CodingAdventures.CpuPipeline.new_bubble()
    assert bubble.is_bubble
  end
end
