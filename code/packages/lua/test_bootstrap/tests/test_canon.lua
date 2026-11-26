-- Canonical rendering (hazards 3, 4, 5): total, bounded, never runs user code.
package.path = "../src/?.lua;../src/?/init.lua;" .. package.path
local H = require("helper")
local Canon, Int = H.tb.Canon, H.tb.Int

local function render(v)
  return Canon.render(v)
end

describe("scalars", function()
  it("renders nil, booleans and integers", function()
    assert.are.equal("nil", render(nil))
    assert.are.equal("true", render(true))
    assert.are.equal("false", render(false))
    assert.are.equal("-7", render(-7))
    assert.are.equal("9223372036854775807", render(math.maxinteger))
    assert.are.equal("18446744073709551616", render(Int.pow2(64)))
  end)

  it("tells 3 from 3.0 and spells the specials", function()
    assert.are.equal("3", render(3))
    assert.are.equal("3.0", render(3.0))
    assert.are.equal("0.1", render(0.1))
    assert.are.equal("-0.5", render(-0.5))
    assert.are.equal("nan", render(0 / 0))
    assert.are.equal("inf", render(1 / 0))
    assert.are.equal("-inf", render(-1 / 0))
    assert.are.equal("1e+100", render(1e100))
    assert.are_not.equal(render(3), render(3.0))
  end)

  it("renders other types as fixed placeholders", function()
    assert.are.equal("#Function", render(print))
    assert.are.equal("#Thread", render(coroutine.create(print)))
    assert.are.equal("#Userdata", render(io.stdout))
  end)

  it("renders a bigint over 100,000 bits as a size, not digits", function()
    assert.are.equal("<bigint 100001 bits>", render(Int.pow2(100000)))
  end)
end)

describe("strings are byte strings", function()
  it("quotes valid UTF-8 and hex-encodes anything else", function()
    assert.are.equal('"abc"', render("abc"))
    assert.are.equal('"h\195\169"', render("h\195\169"))
    assert.are.equal("0xff", render("\255"))
    assert.are.equal("0xc080", render("\192\128")) -- overlong NUL
    assert.are.equal("0xeda080", render("\237\160\128")) -- a surrogate
    assert.are.equal("0xf4908080", render("\244\144\128\128")) -- above U+10FFFF
    assert.are.equal('""', render(""))
  end)

  it("uses the short forms and escapes quote and backslash inside quotes only", function()
    assert.are.equal('"a\\nb\\rc\\td"', render("a\nb\rc\td"))
    assert.are.equal('"q\\"b\\\\"', render('q"b\\'))
    assert.are.equal('"\\u0000"', render("\0"))
    assert.are.equal('"\\u001b"', render("\27"))
    assert.are.equal('"\\u007f"', render("\127"))
  end)

  it("escapes every class of the Elixir Canon set exactly", function()
    local escaped = {}
    local function add(lo, hi)
      for c = lo, hi or lo do
        escaped[c] = true
      end
    end
    add(0x00, 0x1F)
    add(0x7F)
    add(0x80, 0x9F)
    add(0x2028, 0x2029)
    add(0x061C)
    add(0x200E, 0x200F)
    add(0x202A, 0x202E)
    add(0x2066, 0x2069)
    add(0xAD)
    add(0x34F)
    add(0x180E)
    add(0xFEFF)
    add(0x200B, 0x200D)
    add(0x2060, 0x2064)
    add(0xFFF9, 0xFFFB)
    add(0xE0000, 0xE007F)
    local probes = { 0x00, 0x1F, 0x20, 0x41, 0x7E, 0x7F, 0x80, 0x9F, 0xA0, 0xAC, 0xAD, 0xAE, 0x34E, 0x34F, 0x350, 0x61B, 0x61C, 0x61D,
      0x180D, 0x180E, 0x180F, 0x200A, 0x200B, 0x200D, 0x200E, 0x200F, 0x2010, 0x2027, 0x2028, 0x2029, 0x202A, 0x202E, 0x202F,
      0x205F, 0x2060, 0x2064, 0x2065, 0x2066, 0x2069, 0x206A, 0xFEFE, 0xFEFF, 0xFFF8, 0xFFF9, 0xFFFB, 0xFFFC, 0xFFFD,
      0x10FFFF, 0xE0000, 0xE007F, 0xE0080, 0xDFFFF, 0x10000, 0x1F600 }
    for _, c in ipairs(probes) do
      local out = render(utf8.char(c))
      local is_escaped = out:find("\\u", 1, true) ~= nil or (c == 10 or c == 13 or c == 9)
      assert.are.equal(escaped[c] == true, is_escaped, string.format("U+%04X rendered %s", c, out))
    end
    assert.are.equal('"\\u202e"', render("\u{202E}"))
    assert.are.equal('"\\u{e0041}"', render(utf8.char(0xE0041)))
    assert.are.equal('"\\u{e007f}"', render(utf8.char(0xE007F)))
    assert.are.equal('"\\u200b"', render("\u{200B}"))
    assert.are.equal('"\\u0085"', render("\u{85}"))
  end)

  it("cannot forge a log line", function()
    local out = render("ok\nFAIL: injected")
    assert.is_nil(out:find("\n", 1, true))
  end)

  it("text records render like strings and are not bytes", function()
    assert.are.equal('"x"', render(Canon.text("x")))
    assert.is_true(Canon.is_text(Canon.text("x")))
    assert.is_false(Canon.is_text("x"))
    assert.is_false(Canon.is_text({ s = "x" }))
  end)
end)

describe("tables", function()
  it("renders sequences, maps (sorted by rendered key) and nesting", function()
    assert.are.equal("[]", render({}))
    assert.are.equal("[1,2,[3]]", render({ 1, 2, { 3 } }))
    assert.are.equal('%{"a"=>1,"b"=>2}', render({ b = 2, a = 1 }))
    assert.are.equal('%{"a"=>1,"b"=>2}', render({ a = 1, b = 2 }))
    assert.are.equal("%{1=>1,3=>3}", render({ [1] = 1, [3] = 3 }))
  end)

  it("sorts map entries bytewise so the digest never depends on next() order", function()
    local a, b = {}, {}
    for i = 1, 60 do
      a["k" .. i] = i
    end
    for i = 60, 1, -1 do
      b["k" .. i] = i
    end
    assert.are.equal(render(a), render(b))
  end)

  it("terminates on a table that contains itself", function()
    local t = {}
    t[1] = t
    local out = render(t)
    assert.is_true(#out < 1000)
    assert.is_truthy(out:find("...", 1, true))
    local m = { name = "x" }
    m.self = m
    assert.is_true(#render(m) < 100000)
  end)

  it("caps nesting at depth 16", function()
    local t = {}
    local cur = t
    for _ = 1, 1000 do
      local n = {}
      cur[1] = n
      cur = n
    end
    local out = render(t)
    assert.is_true(#out < 100)
    assert.is_truthy(out:find("...", 1, true))
  end)

  it("truncates a million-entry table and a 5 MiB string at 1 MiB", function()
    local big = {}
    for i = 1, 1000000 do
      big[i] = i
    end
    local out = render(big)
    assert.is_true(#out <= Canon.MAX_OUTPUT + 4)
    assert.are.equal("...", out:sub(-3))
    local s = render(string.rep("a", 5 * 1024 * 1024))
    assert.is_true(#s <= Canon.MAX_OUTPUT + 8)
    assert.are.equal("...", s:sub(-3))
    -- cut inside a multi-byte character: still valid UTF-8 plus "..."
    local multi = render(string.rep("\195\169", 1024 * 1024))
    assert.is_truthy(utf8.len(multi))
    assert.is_true(#multi <= Canon.MAX_OUTPUT + 8)
  end)

  it("stays total on a 10 MiB string of control characters", function()
    local out = H.limited(5e8, render, string.rep("\1", 10 * 1024 * 1024))
    assert.is_true(#out < 2 * 1024 * 1024 + 100)
  end)
end)

describe("no user metamethod ever runs (hazard 3)", function()
  it("renders a trapped table without touching any metamethod", function()
    local log = {}
    local t = setmetatable({ 1, 2, x = 3 }, H.trap(log))
    local out = render(t)
    assert.is_string(out)
    assert.are.same({}, log)
    local nested = { t, setmetatable({}, H.trap(log)) }
    render(nested)
    assert.are.same({}, log)
  end)

  it("ignores __index, __len and __pairs that would lie or loop", function()
    local t = setmetatable({}, {
      __index = function()
        error("idx")
      end,
      __len = function()
        return 10 ^ 9
      end,
      __pairs = function()
        error("pairs")
      end,
    })
    assert.are.equal("[]", render(t))
    local lying = setmetatable({ 1, 2 }, {
      __len = function()
        return 99
      end,
    })
    assert.are.equal("[1,2]", render(lying))
  end)

  it("never calls __tostring or __name", function()
    local log = {}
    local t = setmetatable({}, { __tostring = function() log[1] = "ts"; error("ts") end, __name = "Evil" })
    render(t)
    assert.are.same({}, log)
  end)

  it("does not run __gc or __close when rendering", function()
    collectgarbage("stop") -- a finalizer would otherwise fire whenever the collector happens to run
    local log = {}
    local t = setmetatable({ 1 }, { __gc = function() log[#log + 1] = "gc" end, __close = function() log[#log + 1] = "close" end })
    render(t)
    render({ t, { t } })
    local seen = #log
    collectgarbage("restart")
    assert.are.equal(0, seen)
  end)

  it("a metatable that hides itself (__metatable) is harmless", function()
    local t = setmetatable({ 1 }, { __metatable = "locked" })
    assert.are.equal("[1]", render(t))
  end)
end)

describe("escape and clip", function()
  it("escape hex-encodes invalid UTF-8 and stops at the limit", function()
    assert.are.equal("0xff00", Canon.escape("\255\0"))
    assert.is_true(#Canon.escape(string.rep("\1", 1000), false, 50) < 400)
  end)

  it("clip cuts on a character boundary", function()
    assert.are.equal("abc", Canon.clip("abc", 5))
    assert.are.equal("ab...", Canon.clip("abcdef", 2))
    local clipped = Canon.clip("\195\169\195\169", 3)
    assert.is_truthy(utf8.len(clipped:sub(1, -4)))
  end)
end)
