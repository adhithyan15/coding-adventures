defmodule CodingAdventures.WasmTypesTest do
  use ExUnit.Case

  alias CodingAdventures.WasmTypes
  alias CodingAdventures.WasmTypes.{
    FuncType,
    Limits,
    MemoryType,
    TableType,
    GlobalType,
    Import,
    Export,
    Global,
    Element,
    DataSegment,
    FunctionBody,
    CustomSection,
    WasmModule
  }

  # ── Test 1: Module loads ────────────────────────────────────────────────────

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.WasmTypes)
  end

  # ── Test 2: ValueType byte values match WASM spec ───────────────────────────

  test "value_type/1 returns correct byte for i32" do
    assert WasmTypes.value_type(:i32) == 0x7F
  end

  test "value_type/1 returns correct byte for i64" do
    assert WasmTypes.value_type(:i64) == 0x7E
  end

  test "value_type/1 returns correct byte for f32" do
    assert WasmTypes.value_type(:f32) == 0x7D
  end

  test "value_type/1 returns correct byte for f64" do
    assert WasmTypes.value_type(:f64) == 0x7C
  end

  # ── Test 3: ExternalKind byte values ────────────────────────────────────────

  test "external_kind/1 returns 0 for function" do
    assert WasmTypes.external_kind(:function) == 0x00
  end

  test "external_kind/1 returns 1 for table" do
    assert WasmTypes.external_kind(:table) == 0x01
  end

  test "external_kind/1 returns 2 for memory" do
    assert WasmTypes.external_kind(:memory) == 0x02
  end

  test "external_kind/1 returns 3 for global" do
    assert WasmTypes.external_kind(:global) == 0x03
  end

  # ── Test 4: BLOCK_TYPE_EMPTY = 0x40 ─────────────────────────────────────────

  test "block_type_empty/0 returns 0x40" do
    assert WasmTypes.block_type_empty() == 0x40
  end

  # ── Test 5: FuncType construction and equality ──────────────────────────────

  test "FuncType construction and equality" do
    a = %FuncType{params: [:i32, :i64], results: [:f32]}
    b = %FuncType{params: [:i32, :i64], results: [:f32]}
    assert a == b
  end

  # ── Test 6: FuncType with empty params and results ──────────────────────────

  test "FuncType default has empty params and results" do
    ft = %FuncType{}
    assert ft.params == []
    assert ft.results == []
  end

  # ── Test 7: FuncType with multiple params and results ───────────────────────

  test "FuncType with multiple params and results" do
    ft = %FuncType{params: [:i32, :i32, :f64], results: [:i64, :f32]}
    assert length(ft.params) == 3
    assert length(ft.results) == 2
    assert Enum.at(ft.params, 2) == :f64
    assert Enum.at(ft.results, 0) == :i64
  end

  # ── Test 8: Limits with only min ────────────────────────────────────────────

  test "Limits with only min" do
    lim = %Limits{min: 1, max: nil}
    assert lim.min == 1
    assert lim.max == nil
  end

  # ── Test 9: Limits with min and max ─────────────────────────────────────────

  test "Limits with min and max" do
    lim = %Limits{min: 1, max: 4}
    assert lim.min == 1
    assert lim.max == 4
  end

  # ── Test 10: MemoryType construction ────────────────────────────────────────

  test "MemoryType construction" do
    mt = %MemoryType{limits: %Limits{min: 2, max: 8}}
    assert mt.limits.min == 2
    assert mt.limits.max == 8
  end

  # ── Test 11: TableType default element_type is FUNCREF (0x70) ───────────────

  test "TableType default element_type is 0x70 (funcref)" do
    tt = %TableType{limits: %Limits{min: 0, max: nil}}
    assert tt.element_type == 0x70
  end

  test "TableType explicit element_type 0x70" do
    tt = %TableType{element_type: 0x70, limits: %Limits{min: 1, max: 10}}
    assert tt.element_type == 0x70
    assert tt.limits.min == 1
  end

  # ── Test 12: GlobalType mutable and immutable ────────────────────────────────

  test "GlobalType mutable" do
    gt = %GlobalType{value_type: :i32, mutable: true}
    assert gt.mutable == true
    assert gt.value_type == :i32
  end

  test "GlobalType immutable" do
    gt = %GlobalType{value_type: :f64, mutable: false}
    assert gt.mutable == false
    assert gt.value_type == :f64
  end

  # ── Test 13: Import for each kind ───────────────────────────────────────────

  test "Import for function kind" do
    imp = %Import{
      module_name: "env",
      name: "abort",
      kind: :function,
      type_info: {:function, 0}
    }
    assert imp.kind == :function
    assert imp.type_info == {:function, 0}
    assert imp.module_name == "env"
    assert imp.name == "abort"
  end

  test "Import for table kind" do
    imp = %Import{
      module_name: "env",
      name: "table",
      kind: :table,
      type_info: {:table, %TableType{element_type: 0x70, limits: %Limits{min: 0, max: nil}}}
    }
    assert imp.kind == :table
    assert elem(imp.type_info, 0) == :table
  end

  test "Import for memory kind" do
    imp = %Import{
      module_name: "env",
      name: "memory",
      kind: :memory,
      type_info: {:memory, %MemoryType{limits: %Limits{min: 1, max: 2}}}
    }
    assert imp.kind == :memory
    assert elem(imp.type_info, 0) == :memory
  end

  test "Import for global kind" do
    imp = %Import{
      module_name: "env",
      name: "stack_ptr",
      kind: :global,
      type_info: {:global, %GlobalType{value_type: :i32, mutable: true}}
    }
    assert imp.kind == :global
    {:global, gt} = imp.type_info
    assert gt.mutable == true
  end

  # ── Test 14: Export construction ────────────────────────────────────────────

  test "Export construction" do
    exp = %Export{name: "main", kind: :function, index: 3}
    assert exp.name == "main"
    assert exp.kind == :function
    assert exp.index == 3
  end

  # ── Test 15: Global with init_expr ──────────────────────────────────────────

  test "Global with init_expr" do
    # i32.const 42; end → <<0x41, 0x2A, 0x0B>>
    g = %Global{
      global_type: %GlobalType{value_type: :i32, mutable: false},
      init_expr: <<0x41, 0x2A, 0x0B>>
    }
    assert g.init_expr == <<0x41, 0x2A, 0x0B>>
    assert g.global_type.value_type == :i32
    assert g.global_type.mutable == false
  end

  # ── Test 16: Element with function_indices ───────────────────────────────────

  test "Element with function_indices" do
    elem = %Element{
      table_index: 0,
      offset_expr: <<0x41, 0x00, 0x0B>>,
      function_indices: [1, 3, 5, 7]
    }
    assert elem.table_index == 0
    assert elem.function_indices == [1, 3, 5, 7]
    assert length(elem.function_indices) == 4
  end

  # ── Test 17: DataSegment ────────────────────────────────────────────────────

  test "DataSegment construction" do
    seg = %DataSegment{
      memory_index: 0,
      offset_expr: <<0x41, 0x80, 0x08, 0x0B>>,
      data: "hello"
    }
    assert seg.memory_index == 0
    assert seg.data == "hello"
  end

  # ── Test 18: FunctionBody ───────────────────────────────────────────────────

  test "FunctionBody construction" do
    body = %FunctionBody{
      locals: [:i32, :i32],
      code: <<0x41, 0x01, 0x0B>>
    }
    assert body.locals == [:i32, :i32]
    assert body.code == <<0x41, 0x01, 0x0B>>
  end

  # ── Test 19: CustomSection ──────────────────────────────────────────────────

  test "CustomSection construction" do
    sec = %CustomSection{name: "name", data: <<0x01, 0x02, 0x03>>}
    assert sec.name == "name"
    assert byte_size(sec.data) == 3
  end

  # ── Test 20: WasmModule has all required fields ──────────────────────────────

  test "WasmModule has all required fields" do
    m = %WasmModule{
      types: [%FuncType{params: [], results: [:i32]}],
      imports: [],
      functions: [0],
      tables: [],
      memories: [%MemoryType{limits: %Limits{min: 1, max: nil}}],
      globals: [],
      exports: [%Export{name: "main", kind: :function, index: 0}],
      start: 0,
      elements: [],
      code: [%FunctionBody{locals: [], code: <<0x0B>>}],
      data: [],
      customs: []
    }
    assert length(m.types) == 1
    assert m.functions == [0]
    assert m.start == 0
    assert hd(m.exports).name == "main"
  end

  # ── Test 21: WasmModule default is all-empty ─────────────────────────────────

  test "WasmModule default is all-empty" do
    m = %WasmModule{}
    assert m.types == []
    assert m.imports == []
    assert m.functions == []
    assert m.tables == []
    assert m.memories == []
    assert m.globals == []
    assert m.exports == []
    assert m.start == nil
    assert m.elements == []
    assert m.code == []
    assert m.data == []
    assert m.customs == []
  end

  # ── Additional: struct updates work (Elixir immutability pattern) ────────────

  test "WasmModule can be updated immutably" do
    original = %WasmModule{}
    updated = %{original | start: 5}
    assert updated.start == 5
    assert original.start == nil
  end

  # ── Additional: FuncType inequality ─────────────────────────────────────────

  test "FuncType with different params are not equal" do
    a = %FuncType{params: [:i32], results: []}
    b = %FuncType{params: [:i64], results: []}
    # Compare the params lists directly to avoid dialyzer warning about
    # distinct struct comparison
    refute a.params == b.params
  end

  # ── Additional: Limits default ───────────────────────────────────────────────

  test "Limits default values" do
    lim = %Limits{}
    assert lim.min == 0
    assert lim.max == nil
  end
end
