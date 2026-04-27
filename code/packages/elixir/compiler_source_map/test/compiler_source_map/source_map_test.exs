defmodule CodingAdventures.CompilerSourceMap.SourceMapTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.CompilerSourceMap.{
    SourcePosition,
    SourceToAst,
    AstToIr,
    IrToIr,
    IrToMachineCode,
    SourceMapChain
  }

  # ── SourcePosition ───────────────────────────────────────────────────────────

  describe "SourcePosition" do
    test "to_string produces file:line:col (len=N) format" do
      sp = %SourcePosition{file: "hello.bf", line: 1, column: 3, length: 1}
      assert SourcePosition.to_string(sp) == "hello.bf:1:3 (len=1)"
    end

    test "struct fields" do
      sp = %SourcePosition{file: "prog.bas", line: 10, column: 5, length: 5}
      assert sp.file == "prog.bas"
      assert sp.line == 10
      assert sp.column == 5
      assert sp.length == 5
    end
  end

  # ── SourceToAst ─────────────────────────────────────────────────────────────

  describe "SourceToAst" do
    test "starts with no entries" do
      seg = %SourceToAst{}
      assert seg.entries == []
    end

    test "add/3 appends entry" do
      seg = %SourceToAst{}
      pos = %SourcePosition{file: "hello.bf", line: 1, column: 1, length: 1}
      seg2 = SourceToAst.add(seg, pos, 42)
      assert length(seg2.entries) == 1
    end

    test "lookup_by_node_id returns position for known ID" do
      seg = %SourceToAst{}
      pos = %SourcePosition{file: "hello.bf", line: 1, column: 3, length: 1}
      seg = SourceToAst.add(seg, pos, 42)
      result = SourceToAst.lookup_by_node_id(seg, 42)
      assert result == pos
    end

    test "lookup_by_node_id returns nil for unknown ID" do
      seg = %SourceToAst{}
      assert SourceToAst.lookup_by_node_id(seg, 99) == nil
    end

    test "multiple entries" do
      seg = %SourceToAst{}
      p1 = %SourcePosition{file: "a.bf", line: 1, column: 1, length: 1}
      p2 = %SourcePosition{file: "a.bf", line: 1, column: 2, length: 1}
      seg = seg |> SourceToAst.add(p1, 0) |> SourceToAst.add(p2, 1)

      assert SourceToAst.lookup_by_node_id(seg, 0) == p1
      assert SourceToAst.lookup_by_node_id(seg, 1) == p2
    end
  end

  # ── AstToIr ─────────────────────────────────────────────────────────────────

  describe "AstToIr" do
    test "starts with no entries" do
      seg = %AstToIr{}
      assert seg.entries == []
    end

    test "add/3 appends entry" do
      seg = %AstToIr{}
      seg2 = AstToIr.add(seg, 42, [7, 8, 9, 10])
      assert length(seg2.entries) == 1
    end

    test "lookup_by_ast_node_id returns IR IDs" do
      seg = %AstToIr{}
      seg = AstToIr.add(seg, 42, [7, 8, 9, 10])
      result = AstToIr.lookup_by_ast_node_id(seg, 42)
      assert result == [7, 8, 9, 10]
    end

    test "lookup_by_ast_node_id returns nil for unknown" do
      seg = %AstToIr{}
      assert AstToIr.lookup_by_ast_node_id(seg, 99) == nil
    end

    test "lookup_by_ir_id returns AST node ID" do
      seg = %AstToIr{}
      seg = AstToIr.add(seg, 42, [7, 8, 9, 10])
      assert AstToIr.lookup_by_ir_id(seg, 7) == 42
      assert AstToIr.lookup_by_ir_id(seg, 9) == 42
    end

    test "lookup_by_ir_id returns -1 for unknown IR ID" do
      seg = %AstToIr{}
      assert AstToIr.lookup_by_ir_id(seg, 999) == -1
    end

    test "one-to-many: one AST node → multiple IR IDs" do
      seg = %AstToIr{}
      seg = AstToIr.add(seg, 0, [0, 1, 2, 3])
      assert AstToIr.lookup_by_ast_node_id(seg, 0) == [0, 1, 2, 3]
    end
  end

  # ── IrToIr ───────────────────────────────���──────────────────────────────────

  describe "IrToIr" do
    test "new/1 creates segment with pass name" do
      seg = IrToIr.new("contraction")
      assert seg.pass_name == "contraction"
      assert seg.entries == []
    end

    test "add_mapping/3 records transformation" do
      seg = IrToIr.new("identity")
      seg = IrToIr.add_mapping(seg, 7, [100])
      result = IrToIr.lookup_by_original_id(seg, 7)
      assert result == [100]
    end

    test "add_deletion/2 records deletion" do
      seg = IrToIr.new("dead_store")
      seg = IrToIr.add_deletion(seg, 7)
      # Deleted → nil
      assert IrToIr.lookup_by_original_id(seg, 7) == nil
    end

    test "lookup_by_original_id returns nil for unknown" do
      seg = IrToIr.new("pass")
      assert IrToIr.lookup_by_original_id(seg, 99) == nil
    end

    test "lookup_by_new_id returns original ID" do
      seg = IrToIr.new("contraction")
      seg = IrToIr.add_mapping(seg, 7, [100])
      assert IrToIr.lookup_by_new_id(seg, 100) == 7
    end

    test "lookup_by_new_id returns -1 for unknown" do
      seg = IrToIr.new("pass")
      assert IrToIr.lookup_by_new_id(seg, 99) == -1
    end

    test "many-to-one: multiple originals → same new ID (contraction)" do
      seg = IrToIr.new("contraction")
      seg = seg |> IrToIr.add_mapping(7, [100]) |> IrToIr.add_mapping(8, [100])
      # Returns first match
      original = IrToIr.lookup_by_new_id(seg, 100)
      assert original in [7, 8]
    end
  end

  # ── IrToMachineCode ──────────────────────────────────────────────────────────

  describe "IrToMachineCode" do
    test "starts with no entries" do
      seg = %IrToMachineCode{}
      assert seg.entries == []
    end

    test "add/4 appends entry" do
      seg = %IrToMachineCode{}
      seg2 = IrToMachineCode.add(seg, 5, 0x14, 8)
      assert length(seg2.entries) == 1
    end

    test "lookup_by_ir_id returns offset and length" do
      seg = %IrToMachineCode{}
      seg = IrToMachineCode.add(seg, 5, 0x14, 8)
      assert IrToMachineCode.lookup_by_ir_id(seg, 5) == {0x14, 8}
    end

    test "lookup_by_ir_id returns {-1, 0} for unknown" do
      seg = %IrToMachineCode{}
      assert IrToMachineCode.lookup_by_ir_id(seg, 99) == {-1, 0}
    end

    test "lookup_by_mc_offset returns IR ID for offset within range" do
      seg = %IrToMachineCode{}
      seg = IrToMachineCode.add(seg, 5, 0x10, 8)
      # 0x10 = 16, range is [16, 24)
      assert IrToMachineCode.lookup_by_mc_offset(seg, 16) == 5
      assert IrToMachineCode.lookup_by_mc_offset(seg, 20) == 5
      assert IrToMachineCode.lookup_by_mc_offset(seg, 23) == 5
    end

    test "lookup_by_mc_offset returns -1 for offset outside range" do
      seg = %IrToMachineCode{}
      seg = IrToMachineCode.add(seg, 5, 0x10, 8)
      assert IrToMachineCode.lookup_by_mc_offset(seg, 0x18) == -1
      assert IrToMachineCode.lookup_by_mc_offset(seg, 0x0F) == -1
    end
  end

  # ── SourceMapChain ───────────────────────────────────────────────────────────

  describe "SourceMapChain.new/0" do
    test "creates chain with empty segments" do
      chain = SourceMapChain.new()
      assert chain.source_to_ast != nil
      assert chain.ast_to_ir != nil
      assert chain.ir_to_ir == []
      assert chain.ir_to_machine_code == nil
    end
  end

  describe "SourceMapChain.add_optimizer_pass/2" do
    test "appends IrToIr segment" do
      chain = SourceMapChain.new()
      pass = IrToIr.new("identity")
      chain2 = SourceMapChain.add_optimizer_pass(chain, pass)
      assert length(chain2.ir_to_ir) == 1
    end

    test "appends multiple passes in order" do
      chain = SourceMapChain.new()
      p1 = IrToIr.new("pass1")
      p2 = IrToIr.new("pass2")
      chain = chain |> SourceMapChain.add_optimizer_pass(p1) |> SourceMapChain.add_optimizer_pass(p2)
      assert length(chain.ir_to_ir) == 2
      assert Enum.at(chain.ir_to_ir, 0).pass_name == "pass1"
      assert Enum.at(chain.ir_to_ir, 1).pass_name == "pass2"
    end
  end

  # ── Composite queries (source_to_mc, mc_to_source) ───────────────────────────

  # Helper: build a fully-populated chain for testing composite queries.
  defp build_full_chain do
    chain = SourceMapChain.new()

    # Source: "+" at file "test.bf", line 1, column 1
    pos = %SourcePosition{file: "test.bf", line: 1, column: 1, length: 1}

    chain = %{
      chain
      | source_to_ast: SourceToAst.add(chain.source_to_ast, pos, 0)
    }

    # AST node 0 → IR instructions [2, 3, 4, 5]
    chain = %{
      chain
      | ast_to_ir: AstToIr.add(chain.ast_to_ir, 0, [2, 3, 4, 5])
    }

    # Machine code: IR 2 → offset 0, 4 bytes
    mc = %IrToMachineCode{}
    mc = IrToMachineCode.add(mc, 2, 0, 4)
    chain = %{chain | ir_to_machine_code: mc}

    {chain, pos}
  end

  describe "SourceMapChain.source_to_mc/2" do
    test "returns [] when ir_to_machine_code is nil" do
      chain = SourceMapChain.new()
      pos = %SourcePosition{file: "t.bf", line: 1, column: 1, length: 1}
      assert SourceMapChain.source_to_mc(chain, pos) == []
    end

    test "returns [] for unknown source position" do
      {chain, _pos} = build_full_chain()
      unknown_pos = %SourcePosition{file: "other.bf", line: 99, column: 1, length: 1}
      assert SourceMapChain.source_to_mc(chain, unknown_pos) == []
    end

    test "returns machine code entries for known source position" do
      {chain, pos} = build_full_chain()
      results = SourceMapChain.source_to_mc(chain, pos)
      assert length(results) == 1
      [{ir_id, mc_offset, mc_length}] = results
      assert ir_id == 2
      assert mc_offset == 0
      assert mc_length == 4
    end

    test "follows IrToIr pass when present" do
      chain = SourceMapChain.new()
      pos = %SourcePosition{file: "test.bf", line: 1, column: 1, length: 1}

      chain = %{
        chain
        | source_to_ast: SourceToAst.add(chain.source_to_ast, pos, 0)
      }

      chain = %{
        chain
        | ast_to_ir: AstToIr.add(chain.ast_to_ir, 0, [7])
      }

      # Optimiser maps 7 → 100
      pass = IrToIr.new("contraction") |> IrToIr.add_mapping(7, [100])
      chain = SourceMapChain.add_optimizer_pass(chain, pass)

      mc = %IrToMachineCode{}
      mc = IrToMachineCode.add(mc, 100, 0x10, 4)
      chain = %{chain | ir_to_machine_code: mc}

      results = SourceMapChain.source_to_mc(chain, pos)
      assert length(results) == 1
      [{ir_id, _, _}] = results
      assert ir_id == 100
    end

    test "returns [] when instruction deleted in optimiser pass" do
      chain = SourceMapChain.new()
      pos = %SourcePosition{file: "test.bf", line: 1, column: 1, length: 1}

      chain = %{chain | source_to_ast: SourceToAst.add(chain.source_to_ast, pos, 0)}
      chain = %{chain | ast_to_ir: AstToIr.add(chain.ast_to_ir, 0, [7])}

      # Optimiser deletes instruction 7
      pass = IrToIr.new("dead_store") |> IrToIr.add_deletion(7)
      chain = SourceMapChain.add_optimizer_pass(chain, pass)

      mc = %IrToMachineCode{}
      chain = %{chain | ir_to_machine_code: mc}

      results = SourceMapChain.source_to_mc(chain, pos)
      assert results == []
    end
  end

  describe "SourceMapChain.mc_to_source/2" do
    test "returns nil when ir_to_machine_code is nil" do
      chain = SourceMapChain.new()
      assert SourceMapChain.mc_to_source(chain, 0) == nil
    end

    test "returns nil for unknown MC offset" do
      {chain, _pos} = build_full_chain()
      assert SourceMapChain.mc_to_source(chain, 9999) == nil
    end

    test "returns source position for known MC offset" do
      {chain, pos} = build_full_chain()
      result = SourceMapChain.mc_to_source(chain, 0)
      assert result == pos
    end

    test "follows IrToIr passes in reverse" do
      chain = SourceMapChain.new()
      pos = %SourcePosition{file: "test.bf", line: 1, column: 2, length: 1}

      chain = %{chain | source_to_ast: SourceToAst.add(chain.source_to_ast, pos, 0)}
      chain = %{chain | ast_to_ir: AstToIr.add(chain.ast_to_ir, 0, [7])}

      # Optimiser maps 7 → 100
      pass = IrToIr.new("contraction") |> IrToIr.add_mapping(7, [100])
      chain = SourceMapChain.add_optimizer_pass(chain, pass)

      mc = %IrToMachineCode{}
      mc = IrToMachineCode.add(mc, 100, 0x10, 4)
      chain = %{chain | ir_to_machine_code: mc}

      # MC offset 0x10 should trace back to pos
      result = SourceMapChain.mc_to_source(chain, 0x10)
      assert result == pos
    end

    test "returns nil when trace fails at IrToIr reverse step" do
      chain = SourceMapChain.new()
      pos = %SourcePosition{file: "test.bf", line: 1, column: 1, length: 1}

      chain = %{chain | source_to_ast: SourceToAst.add(chain.source_to_ast, pos, 0)}
      chain = %{chain | ast_to_ir: AstToIr.add(chain.ast_to_ir, 0, [7])}

      # Pass maps 7 → 100 but MC has instruction 200 (not 100)
      pass = IrToIr.new("pass") |> IrToIr.add_mapping(7, [100])
      chain = SourceMapChain.add_optimizer_pass(chain, pass)

      mc = %IrToMachineCode{}
      mc = IrToMachineCode.add(mc, 200, 0, 4)
      chain = %{chain | ir_to_machine_code: mc}

      # MC offset 0 → IR 200 → can't trace back through pass (no mapping for 200)
      result = SourceMapChain.mc_to_source(chain, 0)
      assert result == nil
    end
  end
end
