defmodule CodingAdventures.CompilerIr.IrOpTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.CompilerIr.IrOp

  # ── to_string/1 ─────────────────────────────────────────────────────────────

  describe "to_string/1" do
    test "constants group" do
      assert IrOp.to_string(:load_imm) == "LOAD_IMM"
      assert IrOp.to_string(:load_addr) == "LOAD_ADDR"
    end

    test "memory group" do
      assert IrOp.to_string(:load_byte) == "LOAD_BYTE"
      assert IrOp.to_string(:store_byte) == "STORE_BYTE"
      assert IrOp.to_string(:load_word) == "LOAD_WORD"
      assert IrOp.to_string(:store_word) == "STORE_WORD"
    end

    test "arithmetic group" do
      assert IrOp.to_string(:add) == "ADD"
      assert IrOp.to_string(:add_imm) == "ADD_IMM"
      assert IrOp.to_string(:sub) == "SUB"
      assert IrOp.to_string(:and) == "AND"
      assert IrOp.to_string(:and_imm) == "AND_IMM"
    end

    test "comparison group" do
      assert IrOp.to_string(:cmp_eq) == "CMP_EQ"
      assert IrOp.to_string(:cmp_ne) == "CMP_NE"
      assert IrOp.to_string(:cmp_lt) == "CMP_LT"
      assert IrOp.to_string(:cmp_gt) == "CMP_GT"
    end

    test "control flow group" do
      assert IrOp.to_string(:label) == "LABEL"
      assert IrOp.to_string(:jump) == "JUMP"
      assert IrOp.to_string(:branch_z) == "BRANCH_Z"
      assert IrOp.to_string(:branch_nz) == "BRANCH_NZ"
      assert IrOp.to_string(:call) == "CALL"
      assert IrOp.to_string(:ret) == "RET"
    end

    test "system group" do
      assert IrOp.to_string(:syscall) == "SYSCALL"
      assert IrOp.to_string(:halt) == "HALT"
    end

    test "meta group" do
      assert IrOp.to_string(:nop) == "NOP"
      assert IrOp.to_string(:comment) == "COMMENT"
    end

    test "unknown opcode returns UNKNOWN" do
      assert IrOp.to_string(:bogus) == "UNKNOWN"
      assert IrOp.to_string(:not_real) == "UNKNOWN"
    end
  end

  # ── parse/1 ─────────────────────────────────────────────────────────────────

  describe "parse/1" do
    test "parses all 25 opcodes" do
      pairs = [
        {"LOAD_IMM", :load_imm},
        {"LOAD_ADDR", :load_addr},
        {"LOAD_BYTE", :load_byte},
        {"STORE_BYTE", :store_byte},
        {"LOAD_WORD", :load_word},
        {"STORE_WORD", :store_word},
        {"ADD", :add},
        {"ADD_IMM", :add_imm},
        {"SUB", :sub},
        {"AND", :and},
        {"AND_IMM", :and_imm},
        {"CMP_EQ", :cmp_eq},
        {"CMP_NE", :cmp_ne},
        {"CMP_LT", :cmp_lt},
        {"CMP_GT", :cmp_gt},
        {"LABEL", :label},
        {"JUMP", :jump},
        {"BRANCH_Z", :branch_z},
        {"BRANCH_NZ", :branch_nz},
        {"CALL", :call},
        {"RET", :ret},
        {"SYSCALL", :syscall},
        {"HALT", :halt},
        {"NOP", :nop},
        {"COMMENT", :comment}
      ]

      for {name, atom} <- pairs do
        assert IrOp.parse(name) == {:ok, atom}, "Expected parse(#{name}) == #{atom}"
      end
    end

    test "returns error for unknown name" do
      assert IrOp.parse("BOGUS") == {:error, :unknown_opcode}
      assert IrOp.parse("load_imm") == {:error, :unknown_opcode}
      assert IrOp.parse("") == {:error, :unknown_opcode}
    end
  end

  # ── Roundtrip: to_string . parse ─────────────────────────────────────────────

  describe "roundtrip" do
    test "all opcodes roundtrip through to_string and parse" do
      for op <- IrOp.all() do
        name = IrOp.to_string(op)
        assert {:ok, ^op} = IrOp.parse(name), "Roundtrip failed for #{inspect(op)}"
      end
    end
  end

  # ── all/0 ────────────────────────────────────────────────────────────────────

  describe "all/0" do
    test "returns 25 opcodes" do
      assert length(IrOp.all()) == 25
    end

    test "all returned opcodes are atoms" do
      for op <- IrOp.all() do
        assert is_atom(op), "Expected atom, got #{inspect(op)}"
      end
    end
  end
end
