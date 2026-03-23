defmodule CodingAdventures.WasmModuleParserTest do
  use ExUnit.Case

  alias CodingAdventures.WasmModuleParser

  alias CodingAdventures.WasmTypes.{
    Export,
    FuncType,
    FunctionBody,
    GlobalType,
    Limits,
    MemoryType,
    TableType,
    WasmModule
  }

  # ── Helpers ───────────────────────────────────────────────────────────────────

  # The 8-byte header every .wasm file starts with.
  defp minimal_module, do: <<0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00>>

  # Encode a u32 as unsigned LEB128.
  defp encode_u32leb(val), do: do_encode_u32leb(val, [])

  defp do_encode_u32leb(val, acc) when val < 128, do: :erlang.list_to_binary(Enum.reverse([val | acc]))

  defp do_encode_u32leb(val, acc) do
    byte = Bitwise.bor(Bitwise.band(val, 0x7F), 0x80)
    do_encode_u32leb(Bitwise.bsr(val, 7), [byte | acc])
  end

  # Encode a length-prefixed UTF-8 string.
  defp encode_str(s) do
    bytes = String.to_charlist(s) |> :erlang.list_to_binary()
    encode_u32leb(byte_size(bytes)) <> bytes
  end

  # Build one section: id byte + u32leb(size) + payload.
  defp make_section(id, payload) do
    <<id::8>> <> encode_u32leb(byte_size(payload)) <> payload
  end

  # Build a complete WASM module binary: header + sections.
  defp wasm_with_sections(sections) do
    minimal_module() <> Enum.reduce(sections, <<>>, fn s, acc -> acc <> s end)
  end

  # ── Test 1: Minimal module (header only) ─────────────────────────────────────

  test "minimal module — header only" do
    assert {:ok, %WasmModule{} = m} = WasmModuleParser.parse(minimal_module())
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

  # ── Test 2: Type section — (i32, i32) → i32 ──────────────────────────────────
  #
  # Binary:
  #   01        count = 1
  #   60        func type tag
  #   02        param count = 2
  #   7F 7F     params: i32, i32
  #   01        result count = 1
  #   7F        result: i32

  test "type section — (i32, i32) → i32" do
    payload =
      <<0x01,  # count = 1
        0x60,  # func type tag
        0x02, 0x7F, 0x7F,  # 2 params: i32, i32
        0x01, 0x7F>>       # 1 result: i32

    data = wasm_with_sections([make_section(1, payload)])
    assert {:ok, m} = WasmModuleParser.parse(data)
    assert length(m.types) == 1
    assert m.types == [%FuncType{params: [:i32, :i32], results: [:i32]}]
  end

  # ── Test 3: Function section — type index list ────────────────────────────────

  test "function section — type index list" do
    payload = <<0x02>> <> encode_u32leb(0) <> encode_u32leb(1)
    data = wasm_with_sections([make_section(3, payload)])
    assert {:ok, m} = WasmModuleParser.parse(data)
    assert m.functions == [0, 1]
  end

  # ── Test 4: Export section — function export ──────────────────────────────────

  test "export section — function export" do
    payload =
      <<0x01>>
      <> encode_str("main")
      <> <<0x00>>   # ExternalKind::Function
      <> encode_u32leb(0)

    data = wasm_with_sections([make_section(7, payload)])
    assert {:ok, m} = WasmModuleParser.parse(data)
    assert length(m.exports) == 1
    assert m.exports == [%Export{name: "main", kind: :function, index: 0}]
  end

  # ── Test 5: Code section — function with locals ───────────────────────────────
  #
  # Body: 1 local decl group (2 × i32), code = [end]

  test "code section — function with locals" do
    body =
      encode_u32leb(1)    # 1 local decl
      <> encode_u32leb(2) # 2 locals
      <> <<0x7F>>         # type: i32
      <> <<0x0B>>         # end opcode

    payload = encode_u32leb(1) <> encode_u32leb(byte_size(body)) <> body
    data = wasm_with_sections([make_section(10, payload)])
    assert {:ok, m} = WasmModuleParser.parse(data)
    assert length(m.code) == 1
    assert m.code == [%FunctionBody{locals: [:i32, :i32], code: <<0x0B>>}]
  end

  # ── Test 6: Import section — function import ──────────────────────────────────

  test "import section — function import" do
    payload =
      <<0x01>>              # count = 1
      <> encode_str("env")
      <> encode_str("abort")
      <> <<0x00>>           # Function
      <> encode_u32leb(0)   # type index = 0

    data = wasm_with_sections([make_section(2, payload)])
    assert {:ok, m} = WasmModuleParser.parse(data)
    assert length(m.imports) == 1
    imp = hd(m.imports)
    assert imp.module_name == "env"
    assert imp.name == "abort"
    assert imp.kind == :function
    assert imp.type_info == {:function, 0}
  end

  # ── Test 7: Memory section ────────────────────────────────────────────────────
  #
  # One memory: min=1 page, no max.
  #   01  count = 1
  #   00  flags = 0 (no max)
  #   01  min = 1

  test "memory section" do
    payload = <<0x01, 0x00, 0x01>>
    data = wasm_with_sections([make_section(5, payload)])
    assert {:ok, m} = WasmModuleParser.parse(data)
    assert length(m.memories) == 1
    assert m.memories == [%MemoryType{limits: %Limits{min: 1, max: nil}}]
  end

  # ── Test 8: Table section ─────────────────────────────────────────────────────
  #
  # One table: funcref, min=0, max=100.

  test "table section" do
    payload =
      <<0x01>>        # count = 1
      <> <<0x70>>     # funcref
      <> <<0x01>>     # flags: has max
      <> encode_u32leb(0)   # min = 0
      <> encode_u32leb(100) # max = 100

    data = wasm_with_sections([make_section(4, payload)])
    assert {:ok, m} = WasmModuleParser.parse(data)
    assert length(m.tables) == 1
    t = hd(m.tables)
    assert t.element_type == 0x70
    assert t.limits == %Limits{min: 0, max: 100}
  end

  # ── Test 9: Global section — immutable i32 const ─────────────────────────────
  #
  # global i32 (i32.const 42):
  #   01      count = 1
  #   7F      i32
  #   00      immutable
  #   41 2A 0B   i32.const 42; end

  test "global section — immutable i32 const" do
    payload = <<0x01, 0x7F, 0x00, 0x41, 0x2A, 0x0B>>
    data = wasm_with_sections([make_section(6, payload)])
    assert {:ok, m} = WasmModuleParser.parse(data)
    assert length(m.globals) == 1
    g = hd(m.globals)
    assert g.global_type == %GlobalType{value_type: :i32, mutable: false}
    assert g.init_expr == <<0x41, 0x2A, 0x0B>>
  end

  # ── Test 10: Data section ─────────────────────────────────────────────────────

  test "data section" do
    payload =
      <<0x01>>              # count = 1
      <> <<0x00>>           # mem_idx = 0
      <> <<0x41, 0x00, 0x0B>> # i32.const 0; end
      <> <<0x02, 0xDE, 0xAD>> # 2 bytes

    data = wasm_with_sections([make_section(11, payload)])
    assert {:ok, m} = WasmModuleParser.parse(data)
    assert length(m.data) == 1
    seg = hd(m.data)
    assert seg.memory_index == 0
    assert seg.offset_expr == <<0x41, 0x00, 0x0B>>
    assert seg.data == <<0xDE, 0xAD>>
  end

  # ── Test 11: Element section ──────────────────────────────────────────────────

  test "element section" do
    payload =
      <<0x01>>                  # count = 1
      <> encode_u32leb(0)       # table_idx = 0
      <> <<0x41, 0x00, 0x0B>>   # i32.const 0; end
      <> <<0x02>>               # func_count = 2
      <> encode_u32leb(0)       # func 0
      <> encode_u32leb(1)       # func 1

    data = wasm_with_sections([make_section(9, payload)])
    assert {:ok, m} = WasmModuleParser.parse(data)
    assert length(m.elements) == 1
    elem = hd(m.elements)
    assert elem.table_index == 0
    assert elem.offset_expr == <<0x41, 0x00, 0x0B>>
    assert elem.function_indices == [0, 1]
  end

  # ── Test 12: Start section ────────────────────────────────────────────────────

  test "start section" do
    payload = encode_u32leb(5)
    data = wasm_with_sections([make_section(8, payload)])
    assert {:ok, m} = WasmModuleParser.parse(data)
    assert m.start == 5
  end

  # ── Test 13: Custom section ───────────────────────────────────────────────────

  test "custom section" do
    payload = encode_str("name") <> <<0x01, 0x02, 0x03>>
    data = wasm_with_sections([make_section(0, payload)])
    assert {:ok, m} = WasmModuleParser.parse(data)
    assert length(m.customs) == 1
    c = hd(m.customs)
    assert c.name == "name"
    assert c.data == <<0x01, 0x02, 0x03>>
  end

  # ── Test 14: Multi-section module ─────────────────────────────────────────────

  test "multi-section module" do
    # Type: (i32) -> i32
    type_payload = <<0x01, 0x60, 0x01, 0x7F, 0x01, 0x7F>>

    # Function: func 0 → type 0
    func_payload = encode_u32leb(1) <> encode_u32leb(0)

    # Export: func 0 as "add"
    exp_payload = <<0x01>> <> encode_str("add") <> <<0x00>> <> encode_u32leb(0)

    # Code: empty body (0 locals, end)
    body = encode_u32leb(0) <> <<0x0B>>
    code_payload = encode_u32leb(1) <> encode_u32leb(byte_size(body)) <> body

    data =
      wasm_with_sections([
        make_section(1, type_payload),
        make_section(3, func_payload),
        make_section(7, exp_payload),
        make_section(10, code_payload)
      ])

    assert {:ok, m} = WasmModuleParser.parse(data)
    assert length(m.types) == 1
    assert m.functions == [0]
    assert hd(m.exports).name == "add"
    assert length(m.code) == 1
    assert hd(m.code).locals == []
  end

  # ── Test 15: Error — bad magic ────────────────────────────────────────────────

  test "error — bad magic bytes" do
    data = <<0xFF, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00>>
    assert {:error, msg} = WasmModuleParser.parse(data)
    assert String.contains?(msg, "magic")
  end

  # ── Test 16: Error — wrong version ────────────────────────────────────────────

  test "error — wrong version" do
    data = <<0x00, 0x61, 0x73, 0x6D, 0x02, 0x00, 0x00, 0x00>>
    assert {:error, msg} = WasmModuleParser.parse(data)
    assert String.contains?(msg, "version")
  end

  # ── Test 17: Error — empty data / truncated header ────────────────────────────

  test "error — empty data" do
    assert {:error, msg} = WasmModuleParser.parse(<<>>)
    assert String.contains?(msg, "too short") or String.contains?(msg, "8 bytes")
  end

  test "error — truncated header (3 bytes)" do
    assert {:error, msg} = WasmModuleParser.parse(<<0x00, 0x61, 0x73>>)
    assert String.contains?(msg, "too short") or String.contains?(msg, "8 bytes")
  end

  # ── Test 18: Error — truncated section payload ────────────────────────────────

  test "error — truncated section payload" do
    # Type section claiming 10 bytes, only 1 present
    data =
      minimal_module()
      <> <<0x01>>             # section id = type
      <> encode_u32leb(10)    # size = 10 bytes
      <> <<0x01>>             # only 1 byte

    assert {:error, _msg} = WasmModuleParser.parse(data)
  end

  # ── Test 19: Round-trip — build binary manually, parse, verify ───────────────

  test "round-trip — build binary, parse, verify fields" do
    # () -> ()
    type_payload = <<0x01, 0x60, 0x00, 0x00>>
    func_payload = encode_u32leb(1) <> encode_u32leb(0)
    exp_payload = <<0x01>> <> encode_str("nop") <> <<0x00>> <> encode_u32leb(0)

    body = encode_u32leb(0) <> <<0x0B>>
    code_payload = encode_u32leb(1) <> encode_u32leb(byte_size(body)) <> body

    wasm =
      wasm_with_sections([
        make_section(1, type_payload),
        make_section(3, func_payload),
        make_section(7, exp_payload),
        make_section(10, code_payload)
      ])

    assert {:ok, m} = WasmModuleParser.parse(wasm)
    assert m.types == [%FuncType{params: [], results: []}]
    assert m.functions == [0]

    assert hd(m.exports) == %Export{name: "nop", kind: :function, index: 0}

    assert hd(m.code) == %FunctionBody{locals: [], code: <<0x0B>>}
  end

  # ── Additional coverage tests ─────────────────────────────────────────────────

  test "import — table kind" do
    payload =
      <<0x01>>
      <> encode_str("host")
      <> encode_str("tbl")
      <> <<0x01>>           # Table
      <> <<0x70>>           # funcref
      <> <<0x00>>           # no max
      <> encode_u32leb(10)  # min = 10

    data = wasm_with_sections([make_section(2, payload)])
    assert {:ok, m} = WasmModuleParser.parse(data)
    imp = hd(m.imports)
    assert imp.kind == :table
    assert {:table, %TableType{element_type: 0x70, limits: %Limits{min: 10}}} = imp.type_info
  end

  test "import — memory kind" do
    payload =
      <<0x01>>
      <> encode_str("env")
      <> encode_str("memory")
      <> <<0x02>>           # Memory
      <> <<0x01>>           # has max
      <> encode_u32leb(1)   # min = 1
      <> encode_u32leb(4)   # max = 4

    data = wasm_with_sections([make_section(2, payload)])
    assert {:ok, m} = WasmModuleParser.parse(data)
    imp = hd(m.imports)
    assert imp.kind == :memory
    assert {:memory, %MemoryType{limits: %Limits{min: 1, max: 4}}} = imp.type_info
  end

  test "import — global kind" do
    payload =
      <<0x01>>
      <> encode_str("env")
      <> encode_str("sp")
      <> <<0x03>>   # Global
      <> <<0x7F>>   # i32
      <> <<0x01>>   # mutable

    data = wasm_with_sections([make_section(2, payload)])
    assert {:ok, m} = WasmModuleParser.parse(data)
    imp = hd(m.imports)
    assert imp.kind == :global
    assert {:global, %GlobalType{value_type: :i32, mutable: true}} = imp.type_info
  end

  test "multiple type entries" do
    # Two types: (i32)->() and ()->(f64)
    payload =
      <<0x02,
        0x60, 0x01, 0x7F, 0x00,   # (i32) -> ()
        0x60, 0x00, 0x01, 0x7C>>  # () -> (f64)

    data = wasm_with_sections([make_section(1, payload)])
    assert {:ok, m} = WasmModuleParser.parse(data)
    assert length(m.types) == 2
    [t0, t1] = m.types
    assert t0 == %FuncType{params: [:i32], results: []}
    assert t1 == %FuncType{params: [], results: [:f64]}
  end

  test "custom section can appear before type section" do
    custom_payload = encode_str("debug") <> "hello"

    type_payload = <<0x01, 0x60, 0x00, 0x00>>

    after_custom = encode_str("after") <> <<0xFF>>

    data =
      wasm_with_sections([
        make_section(0, custom_payload),
        make_section(1, type_payload),
        make_section(0, after_custom)
      ])

    assert {:ok, m} = WasmModuleParser.parse(data)
    assert length(m.types) == 1
    assert length(m.customs) == 2
    assert Enum.at(m.customs, 0).name == "debug"
    assert Enum.at(m.customs, 1).name == "after"
  end

  test "memory with max" do
    payload =
      <<0x01>>
      <> <<0x01>>           # flags: has max
      <> encode_u32leb(2)   # min = 2
      <> encode_u32leb(8)   # max = 8

    data = wasm_with_sections([make_section(5, payload)])
    assert {:ok, m} = WasmModuleParser.parse(data)
    assert hd(m.memories).limits == %Limits{min: 2, max: 8}
  end

  test "code section — multiple local decl groups" do
    # 2 groups: (2 × i32) and (1 × f64)
    body =
      encode_u32leb(2)      # 2 local decls
      <> encode_u32leb(2) <> <<0x7F>>  # 2 × i32
      <> encode_u32leb(1) <> <<0x7C>>  # 1 × f64
      <> <<0x0B>>                       # end

    payload = encode_u32leb(1) <> encode_u32leb(byte_size(body)) <> body
    data = wasm_with_sections([make_section(10, payload)])
    assert {:ok, m} = WasmModuleParser.parse(data)
    assert hd(m.code).locals == [:i32, :i32, :f64]
  end

  test "value types f32 and i64 in type section" do
    # (f32, i64) -> f64
    payload = <<0x01, 0x60, 0x02, 0x7D, 0x7E, 0x01, 0x7C>>
    data = wasm_with_sections([make_section(1, payload)])
    assert {:ok, m} = WasmModuleParser.parse(data)
    assert hd(m.types) == %FuncType{params: [:f32, :i64], results: [:f64]}
  end

  test "global mutable" do
    payload = <<0x01, 0x7F, 0x01, 0x41, 0x00, 0x0B>>
    data = wasm_with_sections([make_section(6, payload)])
    assert {:ok, m} = WasmModuleParser.parse(data)
    g = hd(m.globals)
    assert g.global_type.mutable == true
  end

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.WasmModuleParser)
  end

  # Coverage boost: error paths and read_expr opcode variants

  test "error — bad function type tag in type section" do
    # type section with tag 0x61 instead of 0x60
    payload = <<0x01, 0x61, 0x00, 0x00>>
    data = wasm_with_sections([make_section(1, payload)])
    assert {:error, msg} = WasmModuleParser.parse(data)
    assert String.contains?(msg, "0x60")
  end

  test "error — unknown import kind" do
    payload =
      <<0x01>>
      <> encode_str("env")
      <> encode_str("x")
      <> <<0x99>>

    data = wasm_with_sections([make_section(2, payload)])
    assert {:error, _msg} = WasmModuleParser.parse(data)
  end

  test "error — unknown value type in type section" do
    # type section with invalid param type byte 0x01
    payload = <<0x01, 0x60, 0x01, 0x01, 0x00>>
    data = wasm_with_sections([make_section(1, payload)])
    assert {:error, msg} = WasmModuleParser.parse(data)
    assert String.contains?(msg, "value type")
  end

  test "unknown section id is silently ignored" do
    payload = <<0xAA, 0xBB>>
    data = wasm_with_sections([make_section(99, payload)])
    assert {:ok, m} = WasmModuleParser.parse(data)
    assert m.types == []
  end

  test "export — table kind" do
    payload =
      <<0x01>>
      <> encode_str("mytable")
      <> <<0x01>>
      <> encode_u32leb(0)

    data = wasm_with_sections([make_section(7, payload)])
    assert {:ok, m} = WasmModuleParser.parse(data)
    assert hd(m.exports).kind == :table
  end

  test "export — memory kind" do
    payload =
      <<0x01>>
      <> encode_str("mem")
      <> <<0x02>>
      <> encode_u32leb(0)

    data = wasm_with_sections([make_section(7, payload)])
    assert {:ok, m} = WasmModuleParser.parse(data)
    assert hd(m.exports).kind == :memory
  end

  test "export — global kind" do
    payload =
      <<0x01>>
      <> encode_str("g")
      <> <<0x03>>
      <> encode_u32leb(0)

    data = wasm_with_sections([make_section(7, payload)])
    assert {:ok, m} = WasmModuleParser.parse(data)
    assert hd(m.exports).kind == :global
  end

  test "global section — i64 with i64.const init_expr" do
    payload = <<0x01, 0x7E, 0x00, 0x42, 0x01, 0x0B>>
    data = wasm_with_sections([make_section(6, payload)])
    assert {:ok, m} = WasmModuleParser.parse(data)
    g = hd(m.globals)
    assert g.global_type.value_type == :i64
    assert g.init_expr == <<0x42, 0x01, 0x0B>>
  end

  test "global section — f32 with f32.const init_expr" do
    # 0x43 + 4 bytes (f32 bits) + 0x0B
    payload = <<0x01, 0x7D, 0x00, 0x43, 0x00, 0x00, 0x80, 0x3F, 0x0B>>
    data = wasm_with_sections([make_section(6, payload)])
    assert {:ok, m} = WasmModuleParser.parse(data)
    assert hd(m.globals).global_type.value_type == :f32
  end

  test "global section — f64 with f64.const init_expr" do
    # 0x44 + 8 bytes (f64 bits) + 0x0B
    payload =
      <<0x01, 0x7C, 0x00,
        0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F,
        0x0B>>

    data = wasm_with_sections([make_section(6, payload)])
    assert {:ok, m} = WasmModuleParser.parse(data)
    assert hd(m.globals).global_type.value_type == :f64
  end

  test "element section with global.get offset_expr" do
    # global.get 0 = 0x23 0x00; end = 0x0B
    payload =
      <<0x01>>
      <> encode_u32leb(0)
      <> <<0x23, 0x00, 0x0B>>
      <> <<0x01>>
      <> encode_u32leb(0)

    data = wasm_with_sections([make_section(9, payload)])
    assert {:ok, m} = WasmModuleParser.parse(data)
    assert hd(m.elements).offset_expr == <<0x23, 0x00, 0x0B>>
  end
end
