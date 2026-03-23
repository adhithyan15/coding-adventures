defmodule CodingAdventures.WasmOpcodesTest do
  use ExUnit.Case
  doctest CodingAdventures.WasmOpcodes

  alias CodingAdventures.WasmOpcodes

  # ── 1. Module loads ──────────────────────────────────────────────────────────

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.WasmOpcodes)
  end

  # ── 2. Total opcode count ────────────────────────────────────────────────────
  #
  # WASM 1.0 MVP defines 172 instructions in the byte range 0x00–0xBF.
  # Gaps (e.g. 0x06–0x0A, 0x12–0x1F, 0x25–0x27) are reserved/unassigned.

  test "total opcode count is at least 172" do
    count = length(WasmOpcodes.all_opcodes())
    assert count >= 172,
      "Expected >= 172 WASM 1.0 MVP opcodes, got #{count}"
  end

  # ── 3. get_opcode(0x6A) returns i32.add ─────────────────────────────────────

  test "get_opcode 0x6A returns i32.add" do
    assert {:ok, op} = WasmOpcodes.get_opcode(0x6A)
    assert op.name == "i32.add"
  end

  # ── 4. get_opcode_by_name("i32.add") returns correct entry ──────────────────

  test "get_opcode_by_name i32.add returns correct entry" do
    assert {:ok, op} = WasmOpcodes.get_opcode_by_name("i32.add")
    assert op.opcode == 0x6A
    assert op.category == "numeric_i32"
  end

  # ── 5. i32.add: stack_pop=2, stack_push=1 ───────────────────────────────────

  test "i32.add has stack_pop=2 and stack_push=1" do
    assert {:ok, op} = WasmOpcodes.get_opcode(0x6A)
    assert op.stack_pop == 2
    assert op.stack_push == 1
  end

  # ── 6. i32.const has immediates=["i32"] ─────────────────────────────────────

  test "i32.const has immediates [i32]" do
    assert {:ok, op} = WasmOpcodes.get_opcode(0x41)
    assert op.name == "i32.const"
    assert op.immediates == ["i32"]
  end

  # ── 7. i32.load has immediates=["memarg"] ───────────────────────────────────

  test "i32.load has immediates [memarg]" do
    assert {:ok, op} = WasmOpcodes.get_opcode(0x28)
    assert op.name == "i32.load"
    assert op.immediates == ["memarg"]
  end

  # ── 8. Unknown byte → {:error, :unknown_opcode} ─────────────────────────────

  test "unknown byte returns error" do
    # 0x06–0x0A are unassigned in WASM 1.0
    assert WasmOpcodes.get_opcode(0x06) == {:error, :unknown_opcode}
    assert WasmOpcodes.get_opcode(0xFF) == {:error, :unknown_opcode}
  end

  # ── 9. Unknown name → {:error, :unknown_opcode} ─────────────────────────────

  test "unknown name returns error" do
    assert WasmOpcodes.get_opcode_by_name("i32.banana") == {:error, :unknown_opcode}
    assert WasmOpcodes.get_opcode_by_name("") == {:error, :unknown_opcode}
  end

  # ── 10. All bytes unique ─────────────────────────────────────────────────────

  test "all opcode bytes are unique" do
    opcodes = WasmOpcodes.all_opcodes()
    bytes = Enum.map(opcodes, & &1.opcode)
    unique_bytes = Enum.uniq(bytes)
    assert length(bytes) == length(unique_bytes),
      "Duplicate opcode bytes detected"
  end

  # ── 11. All names unique ─────────────────────────────────────────────────────

  test "all opcode names are unique" do
    opcodes = WasmOpcodes.all_opcodes()
    names = Enum.map(opcodes, & &1.name)
    unique_names = Enum.uniq(names)
    assert length(names) == length(unique_names),
      "Duplicate opcode names detected"
  end

  # ── 12. all_opcodes and lookup maps same count ───────────────────────────────

  test "all_opcodes and name lookups return same count" do
    opcodes = WasmOpcodes.all_opcodes()

    # Every name in all_opcodes should be findable via get_opcode_by_name
    successful_lookups =
      opcodes
      |> Enum.filter(fn op ->
        match?({:ok, _}, WasmOpcodes.get_opcode_by_name(op.name))
      end)
      |> length()

    assert successful_lookups == length(opcodes),
      "Every opcode name in all_opcodes() should be findable by get_opcode_by_name"
  end

  # ── Additional: category spot checks ────────────────────────────────────────

  test "category spot checks" do
    assert {:ok, %{category: "control"}}     = WasmOpcodes.get_opcode(0x00)   # unreachable
    assert {:ok, %{category: "parametric"}}  = WasmOpcodes.get_opcode(0x1A)   # drop
    assert {:ok, %{category: "variable"}}    = WasmOpcodes.get_opcode(0x20)   # local.get
    assert {:ok, %{category: "memory"}}      = WasmOpcodes.get_opcode(0x28)   # i32.load
    assert {:ok, %{category: "numeric_i32"}} = WasmOpcodes.get_opcode(0x41)   # i32.const
    assert {:ok, %{category: "numeric_i64"}} = WasmOpcodes.get_opcode(0x42)   # i64.const
    assert {:ok, %{category: "numeric_f32"}} = WasmOpcodes.get_opcode(0x43)   # f32.const
    assert {:ok, %{category: "numeric_f64"}} = WasmOpcodes.get_opcode(0x44)   # f64.const
    assert {:ok, %{category: "conversion"}}  = WasmOpcodes.get_opcode(0xA7)   # i32.wrap_i64
  end

  # ── Additional: call_indirect has two immediates ─────────────────────────────

  test "call_indirect has typeidx and tableidx immediates" do
    assert {:ok, op} = WasmOpcodes.get_opcode(0x11)
    assert op.name == "call_indirect"
    assert op.immediates == ["typeidx", "tableidx"]
    assert op.stack_pop == 1
  end

  # ── Additional: select stack effects ────────────────────────────────────────

  test "select has stack_pop=3 and stack_push=1" do
    assert {:ok, op} = WasmOpcodes.get_opcode_by_name("select")
    assert op.stack_pop == 3
    assert op.stack_push == 1
  end

  # ── Additional: memory.grow ──────────────────────────────────────────────────

  test "memory.grow has stack_pop=1 and stack_push=1" do
    assert {:ok, op} = WasmOpcodes.get_opcode(0x40)
    assert op.name == "memory.grow"
    assert op.stack_pop == 1
    assert op.stack_push == 1
  end

  # ── Additional: all conversion instructions have pop=1, push=1 ───────────────

  test "all conversion instructions have stack_pop=1 and stack_push=1 and no immediates" do
    conversions =
      WasmOpcodes.all_opcodes()
      |> Enum.filter(fn op -> op.category == "conversion" end)

    assert length(conversions) == 25,
      "Expected 25 conversion instructions (0xA7–0xBF), got #{length(conversions)}"

    Enum.each(conversions, fn op ->
      assert op.stack_pop == 1,  "#{op.name} should pop 1, got #{op.stack_pop}"
      assert op.stack_push == 1, "#{op.name} should push 1, got #{op.stack_push}"
      assert op.immediates == [], "#{op.name} should have no immediates"
    end)
  end

  # ── Additional: f64.reinterpret_i64 at 0xBF ──────────────────────────────────

  test "f64.reinterpret_i64 is at opcode 0xBF" do
    assert {:ok, op} = WasmOpcodes.get_opcode(0xBF)
    assert op.name == "f64.reinterpret_i64"
  end

  # ── Additional: all memory stores have stack_pop=2, stack_push=0 ─────────────

  test "all memory store instructions pop 2 and push 0" do
    stores =
      WasmOpcodes.all_opcodes()
      |> Enum.filter(fn op ->
        op.category == "memory" and String.contains?(op.name, ".store")
      end)

    assert length(stores) == 9, "Expected 9 store instructions"

    Enum.each(stores, fn op ->
      assert op.stack_pop == 2,  "#{op.name} should pop 2"
      assert op.stack_push == 0, "#{op.name} should push 0"
    end)
  end

  # ── Additional: i64.const has immediates ["i64"] ─────────────────────────────

  test "i64.const has immediates [i64]" do
    assert {:ok, op} = WasmOpcodes.get_opcode(0x42)
    assert op.name == "i64.const"
    assert op.immediates == ["i64"]
  end

  # ── Additional: opcode map keys ──────────────────────────────────────────────

  test "every opcode map has the required keys" do
    required_keys = [:name, :opcode, :category, :immediates, :stack_pop, :stack_push]

    WasmOpcodes.all_opcodes()
    |> Enum.each(fn op ->
      Enum.each(required_keys, fn key ->
        assert Map.has_key?(op, key),
          "Opcode #{inspect(op)} is missing key :#{key}"
      end)
    end)
  end
end
