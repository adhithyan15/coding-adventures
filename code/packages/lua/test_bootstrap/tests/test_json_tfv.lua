-- Strict JSON, TFV v1 and corpus value decoding (VER05o section 4).
package.path = "../src/?.lua;../src/?/init.lua;" .. package.path
local H = require("helper")
local J, Tfv, Values, Canon, Int = H.tb.StrictJson, H.tb.Tfv, H.tb.Values, H.tb.Canon, H.tb.Int

local function bad(text)
  local v, err = J.parse(text)
  assert.is_nil(v, "should have been refused: " .. tostring(text):sub(1, 60))
  assert.is_string(err)
  return err
end

describe("StrictJson accepts", function()
  it("the JSON the corpus uses", function()
    local v = J.parse('{"a": [1, -2.5e+3, true, false, null, "x\\n\\u00e9\\ud83d\\ude00"], "b": {}}')
    assert.are.equal("object", v.kind)
    assert.are.same({ "a", "b" }, v.keys)
    local arr = v.map.a
    assert.are.equal("array", arr.kind)
    assert.are.same({ kind = "number", text = "1" }, arr[1])
    assert.are.equal("-2.5e+3", arr[2].text)
    assert.is_true(arr[3])
    assert.is_false(arr[4])
    assert.are.equal(J.NULL, arr[5])
    assert.are.equal("x\n\195\169\240\159\152\128", arr[6])
  end)

  it("keeps numbers as text, however long", function()
    local digits = string.rep("9", 30)
    assert.are.equal(digits, J.parse(digits).text)
    assert.are.equal("0", J.parse("0").text)
    assert.are.equal("-0", J.parse("-0").text)
  end)

  it("allows a DEL byte, a BOM inside a string and surrounding whitespace", function()
    assert.are.equal("\127", J.parse('"\127"'))
    assert.are.equal("\239\187\191", J.parse('"\239\187\191"'))
    assert.are.equal("1", J.parse(" \t\r\n1 \n").text)
  end)

  it("allows nesting of exactly 64 containers", function()
    assert.is_truthy(J.parse(string.rep("[", 64) .. string.rep("]", 64)))
  end)
end)

describe("StrictJson refuses", function()
  it("the 65th nested container, before recursing", function()
    bad(string.rep("[", 65) .. string.rep("]", 65))
    bad(string.rep("[", 100000)) -- must not blow the stack
    bad(string.rep('{"a":', 65) .. "1" .. string.rep("}", 65))
  end)

  it("duplicate keys, compared by decoded bytes", function()
    bad('{"a":1,"a":2}')
    bad('{"a":1,"\\u0061":2}')
    assert.is_truthy(J.parse('{"a":1,"A":2}'))
  end)

  it("invalid UTF-8, a BOM, overlong forms, surrogates and too-large code points", function()
    bad('"\255"')
    bad("\239\187\191{}")
    bad('"\192\128"')
    bad('"\237\160\128"')
    bad('"\244\144\128\128"')
  end)

  it("raw control characters, lone and reversed surrogate escapes", function()
    bad('"a\nb"')
    bad('"a\0b"')
    bad('"\\ud800"')
    bad('"\\udc00"')
    bad('"\\ud800\\u0041"')
    bad('"\\udc00\\ud800"')
    bad('"\\u12"')
    bad('"\\x"')
    bad('"unterminated')
  end)

  it("malformed numbers, literals and structure", function()
    for _, t in ipairs({ "01", "+1", "1.", ".5", "-", "1e", "1e+", "NaN", "Infinity", "-Infinity", "tru", "nul", "[1,]", "[,1]", '{"a":1,}',
      '{"a"}', "{1:2}", "[1 2]", '{"a" 1}', "", "   ", "// c\n1", "/* c */ 1", "1 2", "[] []", "{}x", "[", "{", "'a'" }) do
      bad(t)
    end
  end)

  it("a document over 8 MiB, and one with more than 200,000 values", function()
    local err = bad(string.rep(" ", J.MAX_BYTES + 1))
    assert.is_truthy(err:find("exceeds", 1, true))
    bad("[" .. string.rep("1,", 200001) .. "1]")
    assert.is_truthy(J.parse("[" .. string.rep("1,", 1000) .. "1]"))
  end)

  it("non-string input", function()
    assert.is_nil((J.parse(nil)))
    assert.is_nil((J.parse(5)))
    assert.is_nil((J.parse({})))
  end)
end)

describe("Tfv accepts", function()
  it("the committed corpus text", function()
    local text = H.read_file(H.repo_root() .. "/code/specs/fixtures/pbt-v1/cases.tfv")
    assert.is_true(Tfv.valid(text))
  end)

  it("every value tag, comments and blank lines", function()
    local text = "tfv 1\n# c\n\nr R7 Ka N Kb T Kc F Kd I-5 Ke X00ff Kf S68 Kg L2 I1 I2\n"
    assert.is_true(Tfv.valid(text))
    assert.is_true(Tfv.valid("tfv 1\nr R2 Km M1 S6b I1 Kd D3ff00000000000000\n") == false) -- 17 hex digits
    assert.is_true(Tfv.valid("tfv 1\nr R2 Km M1 S6b I1 Kd D3ff0000000000000\n"))
    assert.is_true(Tfv.valid("tfv 1")) -- the header alone, no final newline
  end)
end)

describe("Tfv refuses", function()
  local cases = {
    { "no header", "r R1 Ka T\n" },
    { "bad header", "bad header\n" },
    { "header CR", "tfv 1\r\nr R1 Ka T\n" },
    { "empty", "" },
    { "uppercase hex", "tfv 1\nr R1 Ka XFF\n" },
    { "odd hex", "tfv 1\nr R1 Ka X0\n" },
    { "I-0", "tfv 1\nr R1 Ka I-0\n" },
    { "I007", "tfv 1\nr R1 Ka I007\n" },
    { "I empty", "tfv 1\nr R1 Ka I\n" },
    { "I minus", "tfv 1\nr R1 Ka I-\n" },
    { "count disagrees (too few)", "tfv 1\nr R2 Ka T\n" },
    { "trailing tokens", "tfv 1\nr R1 Ka T Kb T\n" },
    { "doubled space", "tfv 1\nr R1  Ka T\n" },
    { "trailing space", "tfv 1\nr R1 Ka T \n" },
    { "duplicate field", "tfv 1\nr R2 Ka T Ka F\n" },
    { "duplicate record id", "tfv 1\nr R1 Ka T\nr R1 Ka T\n" },
    { "bad S utf8", "tfv 1\nr R1 Ka Sff\n" },
    { "CR in record", "tfv 1\nr R1 Ka T\r\n" },
    { "bad id", "tfv 1\nr! R1 Ka T\n" },
    { "bad field name", "tfv 1\nr R1 KA T\n" },
    { "R leading zero", "tfv 1\nr R01 Ka T\n" },
    { "N with body", "tfv 1\nr R1 Ka Nx\n" },
    { "unknown tag", "tfv 1\nr R1 Ka Q1\n" },
    { "list count disagrees", "tfv 1\nr R1 Ka L2 I1\n" },
    { "duplicate map key", "tfv 1\nr R1 Ka M2 I1 I2 I1 I3\n" },
    { "float short", "tfv 1\nr R1 Ka Dff\n" },
    { "invalid utf8 document", "tfv 1\nr R1 Ka T \255\n" },
    { "a record that is only an id", "tfv 1\nr\n" },
  }
  for _, c in ipairs(cases) do
    it(c[1], function()
      local ok, msg = Tfv.validate(c[2])
      assert.is_false(ok)
      assert.is_truthy(msg:find("invalid TFV", 1, true))
    end)
  end

  it("nesting deeper than 64, integers over 1000 digits and inputs over 1 MiB", function()
    local deep = "tfv 1\nr R1 Ka " .. string.rep("L1 ", 65) .. "T\n"
    assert.is_false(Tfv.valid(deep))
    local ok = "tfv 1\nr R1 Ka " .. string.rep("L1 ", 64) .. "T\n"
    assert.is_true(Tfv.valid(ok))
    assert.is_true(Tfv.valid("tfv 1\nr R1 Ka I" .. string.rep("9", 1000) .. "\n"))
    assert.is_false(Tfv.valid("tfv 1\nr R1 Ka I" .. string.rep("9", 1001) .. "\n"))
    assert.is_false(Tfv.valid("tfv 1\n#" .. string.rep("x", Tfv.MAX_INPUT) .. "\n"))
    assert.is_false(Tfv.valid(nil))
    assert.is_false(Tfv.valid(5))
  end)

  it("treats NaN payloads as one value but +0 and -0 as two", function()
    assert.is_false(Tfv.valid("tfv 1\nr R1 Ka M2 D7ff8000000000000 I1 D7ff0000000000001 I2\n"))
    assert.is_true(Tfv.valid("tfv 1\nr R1 Ka M2 D0000000000000000 I1 D8000000000000000 I2\n"))
  end)

  it("never trusts a count for allocation", function()
    assert.is_false(Tfv.valid("tfv 1\nr R1 Ka L999999999 T\n"))
  end)
end)

describe("Values", function()
  it("decodes ints, bytes, lists, booleans and text", function()
    local node = J.parse('[{"kind":"int","value":"-12"},{"kind":"bytes","hex":"00ff"},true,"t",[]]')
    local v = Values.decode(node)
    assert.are.equal(-12, v[1])
    assert.are.equal("\0\255", v[2])
    assert.is_true(v[3])
    assert.is_true(Canon.is_text(v[4]))
    assert.are.same({}, v[5])
  end)

  it("accepts a 1,000-digit int and refuses a 1,001-digit one", function()
    local ok = H.jint(string.rep("7", 1000))
    assert.are.equal(string.rep("7", 1000), Int.tostring(Values.decode(ok)))
    assert.has_error(function() Values.decode(H.jint(string.rep("7", 1001))) end)
    assert.is_truthy(Int.tostring(Values.decode(H.jint("-" .. string.rep("7", 1000)))))
  end)

  it("refuses non-canonical ints and bytes", function()
    for _, t in ipairs({ "01", "-0", "+1", "1.0", "", "0x10", " 1" }) do
      assert.has_error(function() Values.decode(H.jint(t)) end)
    end
    for _, t in ipairs({ "FF", "0", "zz", "0G" }) do
      assert.has_error(function() Values.decode(H.jbytes(t)) end)
    end
    assert.has_error(function() Values.decode(J.parse("null")) end)
    assert.has_error(function() Values.decode(J.parse("5")) end)
    assert.has_error(function() Values.decode(J.parse('{"kind":"int"}')) end)
    assert.has_error(function() Values.decode(J.parse('{"kind":"int","value":"1","x":1}')) end)
  end)

  it("caps depth and list size", function()
    local deep = { kind = "array" }
    local cur = deep
    for _ = 1, 70 do
      local n = { kind = "array" }
      cur[1] = n
      cur = n
    end
    assert.has_error(function() Values.decode(deep) end)
    local big = { kind = "array" }
    for i = 1, Values.MAX_SIZE + 1 do
      big[i] = true
    end
    assert.has_error(function() Values.decode(big) end)
  end)

  it("compares by kind: true is not 1, bytes are not text, 3 is not 3.0", function()
    assert.is_false(Values.same(true, 1))
    assert.is_false(Values.same(false, 0))
    assert.is_false(Values.same(3, 3.0))
    assert.is_false(Values.same("a", Canon.text("a")))
    assert.is_true(Values.same(Canon.text("a"), Canon.text("a")))
    assert.is_false(Values.same({ 1 }, 1))
    assert.is_true(Values.same({ 1, { 2 } }, { 1, { 2 } }))
    assert.is_false(Values.same({ 1, { 2 } }, { 1, { 3 } }))
    assert.is_false(Values.same({ 1 }, { 1, 2 }))
    assert.is_true(Values.same(Int.pow2(70), Int.pow2(70)))
    assert.is_false(Values.same(Int.pow2(70), Int.pow2(71)))
    assert.is_true(Values.same(0 / 0, 0 / 0))
    assert.is_false(Values.same(0.0, -0.0))
  end)

  it("answers false past its depth and node caps and never runs a metamethod", function()
    local a, b = {}, {}
    local ca, cb = a, b
    for _ = 1, 100 do
      local na, nb = {}, {}
      ca[1], cb[1] = na, nb
      ca, cb = na, nb
    end
    assert.is_false(Values.same(a, b))
    local log = {}
    local t = setmetatable({ 1 }, H.trap(log))
    assert.is_true(Values.same(t, setmetatable({ 1 }, H.trap(log))))
    assert.are.same({}, log)
  end)

  it("error_message prints strings and case errors, and only the type of anything else", function()
    assert.are.equal("boom", Values.error_message("boom"))
    assert.are.equal("table", Values.error_message({ message = "secret" }))
    local ok, err = pcall(Values.fail, "bad case")
    assert.is_false(ok)
    assert.are.equal("bad case", Values.error_message(err))
    assert.are.equal("nil", Values.error_message(nil))
  end)
end)
