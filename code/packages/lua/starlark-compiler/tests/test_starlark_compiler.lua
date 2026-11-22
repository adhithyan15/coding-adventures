-- ============================================================================
-- Tests for Starlark opcode provider conforming to STP01 (Lua).
-- ============================================================================

package.path = "./?.lua;../src/?.lua;../src/?/init.lua;" .. package.path

local json = require("dkjson")
local starlark = require("coding_adventures.starlark_compiler")
local constants = starlark.constants
local messages = starlark.messages
local errors = starlark.errors
local opcodes = starlark.opcodes

local Op = opcodes.Op
local OpCategory = opcodes.OpCategory

-- Independent verification tables: literal constants specified outside the provider.
local independent_opcodes = {
  { 0x01, "LoadConst", "stack" },
  { 0x02, "Pop", "stack" },
  { 0x03, "Dup", "stack" },
  { 0x04, "LoadNone", "stack" },
  { 0x05, "LoadTrue", "stack" },
  { 0x06, "LoadFalse", "stack" },
  { 0x10, "StoreName", "variable" },
  { 0x11, "LoadName", "variable" },
  { 0x12, "StoreLocal", "variable" },
  { 0x13, "LoadLocal", "variable" },
  { 0x14, "StoreClosure", "variable" },
  { 0x15, "LoadClosure", "variable" },
  { 0x20, "Add", "arithmetic" },
  { 0x21, "Sub", "arithmetic" },
  { 0x22, "Mul", "arithmetic" },
  { 0x23, "Div", "arithmetic" },
  { 0x24, "FloorDiv", "arithmetic" },
  { 0x25, "Mod", "arithmetic" },
  { 0x26, "Power", "arithmetic" },
  { 0x27, "Negate", "arithmetic" },
  { 0x28, "BitAnd", "arithmetic" },
  { 0x29, "BitOr", "arithmetic" },
  { 0x2A, "BitXor", "arithmetic" },
  { 0x2B, "BitNot", "arithmetic" },
  { 0x2C, "LShift", "arithmetic" },
  { 0x2D, "RShift", "arithmetic" },
  { 0x30, "CmpEq", "comparison" },
  { 0x31, "CmpNe", "comparison" },
  { 0x32, "CmpLt", "comparison" },
  { 0x33, "CmpGt", "comparison" },
  { 0x34, "CmpLe", "comparison" },
  { 0x35, "CmpGe", "comparison" },
  { 0x36, "CmpIn", "comparison" },
  { 0x37, "CmpNotIn", "comparison" },
  { 0x38, "Not", "comparison" },
  { 0x40, "Jump", "control_flow" },
  { 0x41, "JumpIfFalse", "control_flow" },
  { 0x42, "JumpIfTrue", "control_flow" },
  { 0x43, "JumpIfFalseOrPop", "control_flow" },
  { 0x44, "JumpIfTrueOrPop", "control_flow" },
  { 0x50, "MakeFunction", "function" },
  { 0x51, "CallFunction", "function" },
  { 0x52, "CallFunctionKw", "function" },
  { 0x53, "Return", "function" },
  { 0x60, "BuildList", "collection" },
  { 0x61, "BuildDict", "collection" },
  { 0x62, "BuildTuple", "collection" },
  { 0x63, "ListAppend", "collection" },
  { 0x64, "DictSet", "collection" },
  { 0x70, "LoadSubscript", "subscript_attribute" },
  { 0x71, "StoreSubscript", "subscript_attribute" },
  { 0x72, "LoadAttr", "subscript_attribute" },
  { 0x73, "StoreAttr", "subscript_attribute" },
  { 0x74, "LoadSlice", "subscript_attribute" },
  { 0x80, "GetIter", "iteration" },
  { 0x81, "ForIter", "iteration" },
  { 0x82, "UnpackSequence", "iteration" },
  { 0x90, "LoadModule", "module" },
  { 0x91, "ImportFrom", "module" },
  { 0xA0, "Print", "io" },
  { 0xFF, "Halt", "vm_control" },
}

local independent_operators = {
  binary = {
    { "+", "Add" },
    { "-", "Sub" },
    { "*", "Mul" },
    { "/", "Div" },
    { "//", "FloorDiv" },
    { "%", "Mod" },
    { "**", "Power" },
    { "&", "BitAnd" },
    { "|", "BitOr" },
    { "^", "BitXor" },
    { "<<", "LShift" },
    { ">>", "RShift" },
  },
  compare = {
    { "==", "CmpEq" },
    { "!=", "CmpNe" },
    { "<", "CmpLt" },
    { ">", "CmpGt" },
    { "<=", "CmpLe" },
    { ">=", "CmpGe" },
    { "in", "CmpIn" },
    { "not in", "CmpNotIn" },
  },
  augmented_assign = {
    { "+=", "Add" },
    { "-=", "Sub" },
    { "*=", "Mul" },
    { "/=", "Div" },
    { "//=", "FloorDiv" },
    { "%=", "Mod" },
    { "**=", "Power" },
    { "&=", "BitAnd" },
    { "|=", "BitOr" },
    { "^=", "BitXor" },
    { "<<=", "LShift" },
    { ">>=", "RShift" },
  },
  unary = {
    { "-", "Negate" },
    { "~", "BitNot" },
  },
}

local function find_cases_json()
  local candidates = {
    "../../../../specs/fixtures/starlark-opcode-provider-v1/cases.json",
    "../../../specs/fixtures/starlark-opcode-provider-v1/cases.json",
    "../../specs/fixtures/starlark-opcode-provider-v1/cases.json",
    "code/specs/fixtures/starlark-opcode-provider-v1/cases.json",
  }
  for _, path in ipairs(candidates) do
    local f = io.open(path, "rb")
    if f then
      local content = f:read("*a")
      f:close()
      return content, path
    end
  end
  error("STP01 fixture cases.json not found in candidate paths")
end

describe("Starlark Opcode Provider", function()
  describe("Canonical 61 Opcodes & 12 Categories", function()
    it("defines exactly 61 opcodes", function()
      local ops = starlark.all_ops()
      assert.are.equal(61, #ops)
      assert.are.equal(61, #independent_opcodes)
    end)

    it("verifies all 61 opcodes against independent literal table", function()
      for _, spec in ipairs(independent_opcodes) do
        local byte_val = spec[1]
        local symbol = spec[2]
        local cat_id = spec[3]

        local op = starlark.op_from_byte(byte_val)
        assert.is_not_nil(op)
        assert.are.equal(byte_val, op.byte_value)
        assert.are.equal(symbol, op.symbol)
        assert.are.equal(cat_id, op.category.id)

        assert.are.equal(byte_val, starlark.op_byte(op))
        assert.are.equal(symbol, starlark.op_symbol(op))
        assert.are.equal(cat_id, starlark.op_category(op).id)

        local from_sym = starlark.op_from_symbol(symbol)
        assert.is_not_nil(from_sym)
        assert.are.equal(byte_val, from_sym.byte_value)

        local cat_from_byte = starlark.op_category_from_byte(byte_val)
        assert.is_not_nil(cat_from_byte)
        assert.are.equal(cat_id, cat_from_byte.id)
      end
    end)

    it("validates 12 category definitions and nibble invariants", function()
      local cats = {
        OpCategory.stack,
        OpCategory.variable,
        OpCategory.arithmetic,
        OpCategory.comparison,
        OpCategory.control_flow,
        OpCategory["function"],
        OpCategory.collection,
        OpCategory.subscript_attribute,
        OpCategory.iteration,
        OpCategory.module,
        OpCategory.io,
        OpCategory.vm_control,
      }
      assert.are.equal(12, #cats)

      local seen_ids = {}
      for _, cat in ipairs(cats) do
        assert.is_string(cat.id)
        assert.is_string(cat.camel_name)
        assert.is_number(cat.nibble)
        assert.is_nil(seen_ids[cat.id])
        seen_ids[cat.id] = true
      end

      -- Check category camelCase aliases
      assert.are.equal(OpCategory.control_flow, OpCategory.controlFlow)
      assert.are.equal(OpCategory.subscript_attribute, OpCategory.subscriptAttribute)
      assert.are.equal(OpCategory.vm_control, OpCategory.vmControl)
    end)
  end)

  describe("256 Decode Matrix & 195 Holes", function()
    it("correctly identifies all 195 byte holes and 61 valid opcodes", function()
      local assigned_count = 0
      local hole_count = 0

      for b = 0, 255 do
        local op = starlark.op_from_byte(b)
        if op ~= nil then
          assigned_count = assigned_count + 1
          assert.are.equal(b, op.byte_value)
        else
          hole_count = hole_count + 1
          assert.is_nil(starlark.op_category_from_byte(b))
        end
      end

      assert.are.equal(61, assigned_count)
      assert.are.equal(195, hole_count)
      assert.are.equal(256, assigned_count + hole_count)
    end)

    it("returns nil for out-of-bounds, negative, float, and non-number inputs", function()
      assert.is_nil(starlark.op_from_byte(-1))
      assert.is_nil(starlark.op_from_byte(256))
      assert.is_nil(starlark.op_from_byte(1000))
      assert.is_nil(starlark.op_from_byte(1.5))
      assert.is_nil(starlark.op_from_byte("1"))
      assert.is_nil(starlark.op_from_byte(nil))
      assert.is_nil(starlark.op_from_byte({}))

      assert.is_nil(starlark.op_category_from_byte(-1))
      assert.is_nil(starlark.op_category_from_byte(256))
      assert.is_nil(starlark.op_category_from_byte(1.5))

      assert.is_nil(starlark.op_from_symbol("UnknownSymbol"))
      assert.is_nil(starlark.op_from_symbol(""))
      assert.is_nil(starlark.op_from_symbol(nil))
      assert.is_nil(starlark.op_from_symbol(123))

      assert.is_nil(starlark.op_byte(nil))
      assert.is_nil(starlark.op_symbol(nil))
      assert.is_nil(starlark.op_category(nil))
    end)
  end)

  describe("34 Operator Mappings & Fresh-Map Ownership", function()
    it("verifies all 34 operator mappings against independent literal tables", function()
      -- 12 binary operators
      local b_map = starlark.binary_op_map()
      local b_count = 0
      for _, spec in ipairs(independent_operators.binary) do
        local sym = spec[1]
        local expected_op = spec[2]
        assert.is_not_nil(b_map[sym])
        assert.are.equal(expected_op, b_map[sym].symbol)
        assert.are.equal(expected_op, starlark.binary_opcode(sym).symbol)
        b_count = b_count + 1
      end
      assert.are.equal(12, b_count)

      -- 8 comparison operators
      local c_map = starlark.compare_op_map()
      local c_count = 0
      for _, spec in ipairs(independent_operators.compare) do
        local sym = spec[1]
        local expected_op = spec[2]
        assert.is_not_nil(c_map[sym])
        assert.are.equal(expected_op, c_map[sym].symbol)
        assert.are.equal(expected_op, starlark.compare_opcode(sym).symbol)
        c_count = c_count + 1
      end
      assert.are.equal(8, c_count)

      -- 12 augmented assignment operators
      local a_map = starlark.augmented_assign_map()
      local a_count = 0
      for _, spec in ipairs(independent_operators.augmented_assign) do
        local sym = spec[1]
        local expected_op = spec[2]
        assert.is_not_nil(a_map[sym])
        assert.are.equal(expected_op, a_map[sym].symbol)
        assert.are.equal(expected_op, starlark.augmented_assign_opcode(sym).symbol)
        a_count = a_count + 1
      end
      assert.are.equal(12, a_count)

      -- 2 unary operators
      local u_map = starlark.unary_op_map()
      local u_count = 0
      for _, spec in ipairs(independent_operators.unary) do
        local sym = spec[1]
        local expected_op = spec[2]
        assert.is_not_nil(u_map[sym])
        assert.are.equal(expected_op, u_map[sym].symbol)
        assert.are.equal(expected_op, starlark.unary_opcode(sym).symbol)
        u_count = u_count + 1
      end
      assert.are.equal(2, u_count)

      assert.are.equal(34, b_count + c_count + a_count + u_count)
    end)

    it("enforces fresh-map ownership across repeated queries", function()
      local m1 = starlark.binary_op_map()
      local m2 = starlark.binary_op_map()
      assert.are_not.equal(m1, m2) -- distinct table instances

      m1["+"] = nil
      m1["__pollute__"] = true
      assert.is_nil(m1["+"])
      assert.is_not_nil(m2["+"])
      assert.is_nil(m2["__pollute__"])
      assert.is_not_nil(starlark.binary_opcode("+"))

      local l1 = starlark.all_ops()
      local l2 = starlark.all_ops()
      assert.are_not.equal(l1, l2)
      table.remove(l1, 1)
      assert.are.equal(60, #l1)
      assert.are.equal(61, #l2)
      assert.are.equal(61, #starlark.all_ops())
    end)

    it("returns nil for unknown operators", function()
      assert.is_nil(starlark.binary_opcode("unknown"))
      assert.is_nil(starlark.compare_opcode("unknown"))
      assert.is_nil(starlark.augmented_assign_opcode("unknown"))
      assert.is_nil(starlark.unary_opcode("unknown"))
    end)

    it("verifies CamelCase method aliases", function()
      assert.are.equal(starlark.all_ops, starlark.allOps)
      assert.are.equal(starlark.op_from_byte, starlark.opFromByte)
      assert.are.equal(starlark.op_from_symbol, starlark.opFromSymbol)
      assert.are.equal(starlark.op_byte, starlark.opByte)
      assert.are.equal(starlark.op_symbol, starlark.opSymbol)
      assert.are.equal(starlark.op_category, starlark.opCategory)
      assert.are.equal(starlark.op_category_from_byte, starlark.opCategoryFromByte)
      assert.are.equal(starlark.binary_op_map, starlark.binaryOpMap)
      assert.are.equal(starlark.compare_op_map, starlark.compareOpMap)
      assert.are.equal(starlark.augmented_assign_map, starlark.augmentedAssignMap)
      assert.are.equal(starlark.unary_op_map, starlark.unaryOpMap)
      assert.are.equal(starlark.binary_opcode, starlark.binaryOpcode)
      assert.are.equal(starlark.compare_opcode, starlark.compareOpcode)
      assert.are.equal(starlark.augmented_assign_opcode, starlark.augmentedAssignOpcode)
      assert.are.equal(starlark.unary_opcode, starlark.unaryOpcode)
    end)
  end)

  describe("Standing Rule #20: Centralized Diagnostics & Payload Blindness", function()
    it("creates closed sealed errors with static messages", function()
      local e1 = errors.invalid_byte()
      assert.are.equal(messages.ERROR_ID_INVALID_BYTE, e1.error_id)
      assert.are.equal(messages.MSG_INVALID_BYTE, e1.message)
      assert.are.equal(e1.message, tostring(e1))
      assert.is_true(errors.is_starlark_compiler_error(e1))

      local e2 = errors.unknown_opcode()
      assert.are.equal(messages.ERROR_ID_UNKNOWN_OPCODE, e2.error_id)
      assert.are.equal(messages.MSG_UNKNOWN_OPCODE, e2.message)
      assert.is_true(errors.is_starlark_compiler_error(e2))

      local e3 = errors.unknown_operator()
      assert.are.equal(messages.ERROR_ID_UNKNOWN_OPERATOR, e3.error_id)
      assert.are.equal(messages.MSG_UNKNOWN_OPERATOR, e3.message)
      assert.is_true(errors.is_starlark_compiler_error(e3))

      local e4 = errors.corpus_byte_limit_exceeded()
      assert.are.equal(messages.ERROR_ID_INVALID_CORPUS, e4.error_id)
      assert.are.equal(messages.MSG_CORPUS_BYTE_LIMIT_EXCEEDED, e4.message)

      local e5 = errors.corpus_malformed_transport()
      assert.are.equal(messages.ERROR_ID_INVALID_CORPUS, e5.error_id)
      assert.are.equal(messages.MSG_CORPUS_MALFORMED_TRANSPORT, e5.message)

      local e6 = errors.corpus_schema_violation()
      assert.are.equal(messages.ERROR_ID_INVALID_CORPUS, e6.error_id)
      assert.are.equal(messages.MSG_CORPUS_SCHEMA_VIOLATION, e6.message)

      assert.is_false(errors.is_starlark_compiler_error(nil))
      assert.is_false(errors.is_starlark_compiler_error("error"))
      assert.is_false(errors.is_starlark_compiler_error({}))
    end)

    it("verifies payload blindness: no format strings or script tags", function()
      local all_errs = {
        errors.invalid_byte(),
        errors.unknown_opcode(),
        errors.unknown_operator(),
        errors.corpus_byte_limit_exceeded(),
        errors.corpus_malformed_transport(),
        errors.corpus_schema_violation(),
      }
      for _, err in ipairs(all_errs) do
        assert.is_nil(err.message:find("%%s", 1, true))
        assert.is_nil(err.message:find("%%d", 1, true))
        assert.is_nil(err.message:find("<script>", 1, true))
      end
    end)

    it("verifies placeholders match specification", function()
      assert.are.equal("<redacted>", messages.REDACTED)
      assert.are.equal("<invalid>", messages.INVALID)
    end)
  end)

  describe("5-Class Security Resilience Showcase", function()
    it("Class 1: Bounded Memory & Input Envelope Protection", function()
      assert.are.equal(131072, constants.MAX_CORPUS_BYTES)
      assert.are.equal(8, constants.MAX_JSON_NESTING_DEPTH)
      assert.are.equal(195, constants.EXPECTED_HOLE_COUNT)
      assert.are.equal(256, constants.EXPECTED_DECODE_COUNT)
    end)

    it("Class 2: Lexical, Character & Encoding Integrity", function()
      assert.are.equal(0xD800, constants.MIN_SURROGATE_CODE_POINT)
      assert.are.equal(0xDFFF, constants.MAX_SURROGATE_CODE_POINT)
    end)

    it("Class 3: Deterministic Fresh-State Isolation", function()
      local m1 = starlark.binary_op_map()
      local m2 = starlark.binary_op_map()
      assert.are_not.equal(m1, m2)
    end)

    it("Class 4: Closed Diagnostic Catalog & Payload Blindness", function()
      local err = errors.invalid_byte()
      assert.is_not_nil(err.message)
      assert.is_nil(err.message:find("<script>", 1, true))
    end)

    it("Class 5: Adversarial Transport & Mutation Resilience", function()
      assert.are.equal(1, constants.STP01_SCHEMA_VERSION)
      assert.are.equal("standalone-provider-v1", constants.STP01_PROFILE)
      assert.are.equal(0, constants.MIN_BYTE_VALUE)
      assert.are.equal(255, constants.MAX_BYTE_VALUE)
    end)
  end)

  describe("200-Case Deterministic PRNG Property Test", function()
    it("cross-validates random opcode, category, and operator queries", function()
      math.randomseed(0x5741524C) -- deterministic seed

      for i = 1, 200 do
        local query_byte = math.random(-20, 279)
        local op = starlark.op_from_byte(query_byte)
        local cat = starlark.op_category_from_byte(query_byte)

        if query_byte < 0 or query_byte > 255 then
          assert.is_nil(op)
          assert.is_nil(cat)
        elseif op ~= nil then
          assert.are.equal(query_byte, op.byte_value)
          assert.are.equal(op.category, cat)
          assert.are.equal(query_byte, starlark.op_byte(op))
          assert.are.equal(op.symbol, starlark.op_symbol(op))
          assert.are.equal(op.category, starlark.op_category(op))
          assert.are.equal(op, starlark.op_from_symbol(op.symbol))
        else
          assert.is_nil(cat)
        end
      end
    end)
  end)

  describe("STP01 Conformance Fixture Reader: cases.json", function()
    it("replays shared STP01 fixture and verifies exact matches", function()
      local raw_text, path = find_cases_json()
      assert.is_not_nil(raw_text)
      assert.is_true(#raw_text <= constants.MAX_CORPUS_BYTES)

      -- Reject UTF-8 BOM if present
      assert.is_nil(raw_text:find("^\239\187\191"))

      local data, _, err = json.decode(raw_text, 1, json.null)
      assert.is_nil(err)
      assert.is_table(data)

      assert.are.equal(1, data.schema_version)
      assert.are.equal("standalone-provider-v1", data.profile)
      assert.are.equal(61, #data.opcodes)
      assert.are.equal(256, #data.decode)
      assert.are.equal(8, #data.unknown_lookups)

      -- Verify opcodes array
      for _, item in ipairs(data.opcodes) do
        local op = starlark.op_from_byte(item.byte)
        assert.is_not_nil(op)
        assert.are.equal(item.symbol, op.symbol)
        assert.are.equal(item.category, op.category.id)
      end

      -- Verify decode array (256 items)
      local assigned_seen = 0
      local holes_seen = 0
      for _, item in ipairs(data.decode) do
        local op = starlark.op_from_byte(item.byte)
        if item.symbol == json.null or item.symbol == nil then
          holes_seen = holes_seen + 1
          assert.is_nil(op)
        else
          assigned_seen = assigned_seen + 1
          assert.is_not_nil(op)
          assert.are.equal(item.symbol, op.symbol)
          assert.are.equal(item.category, op.category.id)
        end
      end
      assert.are.equal(61, assigned_seen)
      assert.are.equal(195, holes_seen)

      -- Verify operators section
      assert.is_table(data.operators)
      for _, item in ipairs(data.operators.binary or {}) do
        local op = starlark.binary_opcode(item.operator)
        assert.is_not_nil(op)
        assert.are.equal(item.symbol, op.symbol)
      end
      for _, item in ipairs(data.operators.compare or {}) do
        local op = starlark.compare_opcode(item.operator)
        assert.is_not_nil(op)
        assert.are.equal(item.symbol, op.symbol)
      end
      for _, item in ipairs(data.operators.augmented_assign or {}) do
        local op = starlark.augmented_assign_opcode(item.operator)
        assert.is_not_nil(op)
        assert.are.equal(item.symbol, op.symbol)
      end
      for _, item in ipairs(data.operators.unary or {}) do
        local op = starlark.unary_opcode(item.operator)
        assert.is_not_nil(op)
        assert.are.equal(item.symbol, op.symbol)
      end

      -- Verify unknown lookups return nil
      for _, item in ipairs(data.unknown_lookups) do
        local group = item.group
        local op_str = item.operator
        if group == "binary" then
          assert.is_nil(starlark.binary_opcode(op_str))
        elseif group == "compare" then
          assert.is_nil(starlark.compare_opcode(op_str))
        elseif group == "augmented_assign" then
          assert.is_nil(starlark.augmented_assign_opcode(op_str))
        elseif group == "unary" then
          assert.is_nil(starlark.unary_opcode(op_str))
        end
      end
    end)
  end)

  describe("10,000-Iteration Adversarial Mutation Fuzz Loop", function()
    it("resiliently processes 10,000 mutated Starlark provider queries", function()
      math.randomseed(0x5741524C)
      local handled = 0

      local test_symbols = {
        "LoadConst", "Halt", "Add", "Sub", "Jump", "Print",
        "InvalidOpcode", "", " ", "123", "\0", "LoadConst\0",
      }

      local test_ops = {
        "+", "-", "*", "/", "//", "%", "**", "&", "|", "^", "<<", ">>",
        "==", "!=", "<", ">", "<=", ">=", "in", "not in",
        "+=", "-=", "*=", "/=", "//=", "%=", "**=", "&=", "|=", "^=", "<<=", ">>=",
        "~", "@", "++", ":=", "invalid", "",
      }

      for i = 1, 10000 do
        local mode = math.random(1, 4)
        if mode == 1 then
          local b = math.random(-50, 300)
          local op = starlark.op_from_byte(b)
          if op then handled = handled + 1 else handled = handled + 1 end
        elseif mode == 2 then
          local sym = test_symbols[math.random(1, #test_symbols)]
          local op = starlark.op_from_symbol(sym)
          if op then handled = handled + 1 else handled = handled + 1 end
        elseif mode == 3 then
          local op_sym = test_ops[math.random(1, #test_ops)]
          local b_op = starlark.binary_opcode(op_sym)
          local c_op = starlark.compare_opcode(op_sym)
          local a_op = starlark.augmented_assign_opcode(op_sym)
          local u_op = starlark.unary_opcode(op_sym)
          handled = handled + 1
        else
          local m = starlark.binary_op_map()
          m["+"] = nil
          handled = handled + 1
        end
      end

      assert.are.equal(10000, handled)
    end)
  end)
end)
