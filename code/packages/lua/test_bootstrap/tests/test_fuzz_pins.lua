-- Pinned values for the fuzz-v1 blind spots, cross-checked against the Ruby lane
-- (VER06i) and the specification's tables (VER06p sections 3, 4, 7 and 9).
package.path = "../src/?.lua;../src/?/init.lua;" .. package.path
local H = require("helper")
local tb = H.tb
local Fuzz = tb.Fuzz
local Mutate, Bitmap, Signature, Engine, Outcome, Prng, Int = Fuzz.Mutate, Fuzz.Bitmap, Fuzz.Signature, Fuzz.Engine, Fuzz.Outcome, tb.Prng, tb.Int
local PINS = require("fuzz_ruby_pins")

local function unhex(s)
  return (s:gsub("..", function(h) return string.char(tonumber(h, 16)) end))
end

local function hex(s)
  return (s:gsub(".", function(c) return string.format("%02x", c:byte()) end))
end

local I = Int.parse

describe("the Ruby lane's mutator pins", function()
  it("agree on all " .. #PINS.mutate .. " rows", function()
    local bad = {}
    local counts = {}
    for idx, row in ipairs(PINS.mutate) do
      local op, a, want = row[1], row[2], row[3]
      counts[op] = (counts[op] or 0) + 1
      local got
      if op == "splice" then got = Mutate.splice(unhex(a[1]), unhex(a[2]), I(a[3]), I(a[4]))
      elseif op == "boundary" then got = Mutate.boundary_insert(unhex(a[1]), I(a[2]), I(a[3]), I(a[4]), a[5])
      elseif op == "arithmetic" then got = Mutate.byte_arithmetic(unhex(a[1]), I(a[2]), I(a[3]), I(a[4]), a[5])
      elseif op == "bit_flip" then got = Mutate.bit_flip(unhex(a[1]), I(a[2]), I(a[3]))
      elseif op == "byte_flip" then got = Mutate.byte_flip(unhex(a[1]), I(a[2]), I(a[3]))
      elseif op == "block_delete" then got = Mutate.block_delete(unhex(a[1]), I(a[2]), I(a[3]))
      elseif op == "block_insert" then got = Mutate.block_insert(unhex(a[1]), I(a[2]), unhex(a[3]))
      elseif op == "block_duplicate" then got = Mutate.block_duplicate(unhex(a[1]), I(a[2]), I(a[3]), I(a[4]))
      elseif op == "sequence" then got = Mutate.mutate_sequence(unhex(a[1]), I(a[2]), I(a[3]), I(a[4]))
      else error("unknown pin op " .. op) end
      if hex(got) ~= want then
        bad[#bad + 1] = string.format("row %d %s %s: want %s got %s", idx, op, table.concat(a, ","), want, hex(got))
      end
    end
    assert.are.same({}, bad)
    for _, op in ipairs({ "splice", "boundary", "arithmetic", "bit_flip", "byte_flip", "block_delete", "block_insert", "block_duplicate", "sequence" }) do
      assert.is_truthy(counts[op], "no pin for " .. op)
    end
  end)

  it("the interest lists and deltas are exactly the specification's", function()
    assert.are.same({ 0, 1, 127, 128, 255 }, Mutate.INTEREST_8)
    assert.are.same({ 0, 1, 255, 256, 32767, 32768, 65535 }, Mutate.INTEREST_16)
    assert.are.same({ 0, 1, 65535, 65536, 2147483647, 2147483648, 4294967295 }, Mutate.INTEREST_32)
    local words = {}
    for i, w in ipairs(Mutate.INTEREST_64) do words[i] = Int.tostring(Int.from_u64(w)) end
    assert.are.same({ "0", "1", "4294967295", "4294967296", "9223372036854775807", "9223372036854775808", "18446744073709551615" }, words)
    assert.are.same({ -35, -8, -1, 1, 8, 35 }, Mutate.DELTAS)
  end)
end)

describe("calculate_energy", function()
  local rows = {
    { { 10, 4, 3, 6 }, 8 }, -- 7.5 rounds half away from zero
    { { 3, 4, 0, 0 }, 2 }, -- 1.5 -> 2
    { { 5, 4, 0, 0 }, 3 }, -- 2.5 -> 3 (banker's rounding would say 2)
    { { 25, 100, 0, 0 }, 3 }, -- 2.5 -> 3
    { { 1, 4, 0, 0 }, 1 }, -- 0.5 -> 1
    { { 10, 4, 0, 0 }, 5 },
    { { 10, 1, 0, 0 }, 10 },
    { { 0, 4, 0, 0 }, 1 }, -- base clamped to 1: 0.5 -> 1
    { { 10, 16, 1, 2 }, 4 },
    { { 7, 2, 1, 1 }, 10 },
    { { 10, 9, 2, 8 }, 4 },
    { { 10, 0, 0, 0 }, 10 }, -- an empty seed counts as length 1
    { { 1000, 100, 5, 5 }, 200 },
  }
  for _, row in ipairs(rows) do
    it(table.concat(row[1], ",") .. " -> " .. row[2], function()
      assert.are.equal(row[2], Bitmap.calculate_energy(table.unpack(row[1])))
    end)
  end
  it("returns an integer", function()
    assert.are.equal("integer", math.type(Bitmap.calculate_energy(10, 4, 3, 6)))
  end)
end)

describe("crash signatures (real sig_hash values)", function()
  local rows = {
    { "panic", { "eval", "step" }, "4eca7ae8" },
    { "segv", { "alloc", "write" }, "a0aa3aa2" },
    { "timeout", {}, "4e61ea76" },
    { "oom", {}, "7146a410" },
    { "stale-budget-sentinel", {}, "8a20b3bf" },
    { "ArgumentError", { "a.rb#x" }, "c93b2322" },
    { "executions-leaked", {}, "f0c5a137" },
    { "string", {}, "f909da86" },
    { "table", { "t.lua#3" }, "6626db37" },
    { "nil", {}, "b4b870e2" },
    { "yielded", {}, "67b6fc6f" },
    { "hook-cleared", {}, "e5ba272b" },
    { "panic", {}, "fadd156a" },
    { "", {}, "8601adad" },
  }
  for _, r in ipairs(rows) do
    it(r[1] .. " " .. table.concat(r[2], ":") .. " -> " .. r[3], function()
      assert.are.equal(r[3], Signature.crash_signature(r[1], r[2]))
    end)
  end
  it("rejects a non-string kind and non-string frames", function()
    assert.has_error(function() Signature.crash_signature(1, {}) end)
    assert.has_error(function() Signature.crash_signature("k", "x") end)
    assert.has_error(function() Signature.crash_signature("k", { 1 }) end)
  end)
  it("a colon is not escaped: one frame x:y and two frames x, y hash alike (VER06 9.4 item 6)", function()
    assert.are.equal(Signature.crash_signature("k", { "x:y" }), Signature.crash_signature("k", { "x", "y" }))
    assert.is_true(Signature.duplicate_crash("k", { "x:y" }, "k", { "x", "y" }))
  end)
end)

describe("format_tap_failure: cross-lane pins from the Ruby lane", function()
  local function tap(n, target) return Signature.format_tap_failure(n, target, "fuzz-crash", "3a8f19b2") end
  local e, rl = "\u{e9}", "\u{202e}"

  it("a # directive character becomes _", function()
    assert.are.equal("not ok 1 - a_b [fuzz-crash:3a8f19b2]", tap(1, "a#b"))
  end)
  it("spaces, newlines and carriage returns become _", function()
    assert.are.equal("not ok 2 - a_b_c [fuzz-crash:3a8f19b2]", tap(2, "a b c"))
    assert.are.equal("not ok 2 - a_b_c [fuzz-crash:3a8f19b2]", tap(2, "a\nb\rc"))
  end)
  it("a multi-byte character is one _ per BYTE; a bidi override is three", function()
    assert.are.equal("not ok 2 - _____x [fuzz-crash:3a8f19b2]", tap(2, e .. rl .. "x"))
    assert.are.equal("not ok 3 - " .. string.rep("_", 128) .. " [fuzz-crash:3a8f19b2]", tap(3, string.rep(e, 100)))
  end)
  it("more than 128 bytes is cut to 128 BYTES", function()
    assert.are.equal("not ok 2 - " .. string.rep("x", 128) .. " [fuzz-crash:3a8f19b2]", tap(2, string.rep("x", 200)))
  end)
  it("a cut in the middle of a character leaves a single _", function()
    assert.are.equal("not ok 2 - " .. string.rep("x", 127) .. "_ [fuzz-crash:3a8f19b2]", tap(2, string.rep("x", 127) .. e .. string.rep("y", 10)))
    assert.are.equal("not ok 4 - " .. string.rep("x", 127) .. "_ [fuzz-crash:3a8f19b2]", tap(4, string.rep("x", 127) .. rl .. "y"))
  end)
  it("an empty target leaves two spaces", function()
    assert.are.equal("not ok 3 -  [fuzz-crash:3a8f19b2]", tap(3, ""))
  end)
  it("an embedded NUL is a byte like any other", function()
    assert.are.equal("not ok 1 - a_b [fuzz-crash:3a8f19b2]", tap(1, "a\0b"))
  end)
  it("the grammar characters survive: letters, digits, dot, underscore, slash, dash", function()
    assert.are.equal("not ok 1 - Az09._/- [fuzz-crash:3a8f19b2]", tap(1, "Az09._/-"))
  end)
  it("refuses an unregistered failure id, a bad hash and a bad test number", function()
    local F = Signature.format_tap_failure
    assert.has_error(function() F(1, "t", "fuzz-bogus", "3a8f19b2") end)
    assert.has_error(function() F(1, "t", "fuzz-crash", "3A8F19B2") end)
    assert.has_error(function() F(1, "t", "fuzz-crash", "3a8f19b") end)
    assert.has_error(function() F(1, "t", "fuzz-crash", "3a8f19b2\n") end)
    assert.has_error(function() F(0, "t", "fuzz-crash", "3a8f19b2") end)
    assert.has_error(function() F(2147483648, "t", "fuzz-crash", "3a8f19b2") end)
    assert.has_error(function() F(1.0, "t", "fuzz-crash", "3a8f19b2") end)
    assert.has_error(function() F(1, 5, "fuzz-crash", "3a8f19b2") end)
    assert.has_error(function() F(I("9223372036854775808"), "t", "fuzz-crash", "3a8f19b2") end)
    assert.are.equal("not ok 2147483647 - t [fuzz-crash:3a8f19b2]", F(2147483647, "t", "fuzz-crash", "3a8f19b2"))
  end)
end)

describe("the rejection draw in bounded sampling (the corpus never reaches it)", function()
  local function draws(seed, n, count)
    local rng = Prng.new(seed)
    local out = {}
    for i = 1, count do out[i] = Int.tostring(rng:bounded(n)) end
    return out
  end
  local n63 = I("9223372036854775809") -- 2^63 + 1: threshold 2^63 - 1, so about half the raws are rejected

  it("seeds 3 and 4 have a first raw word BELOW the threshold, so the first draw is redrawn", function()
    local t = Prng.threshold(Int.to_u64(n63))
    assert.are.equal(math.maxinteger, t)
    assert.is_true(math.ult(Prng.new(3):next_word(), t))
    assert.is_true(math.ult(Prng.new(4):next_word(), t))
    assert.is_false(math.ult(Prng.new(0):next_word(), t))
  end)
  it("matches the Ruby lane's sequences", function()
    assert.are.same({ "3694763184872335752", "2084015055746161920", "2512858195355979526", "7170589470788784661", "7167102437399161713" }, draws(3, n63, 5))
    assert.are.same({ "7238628660928360495", "6624542149398201438", "1599671085479290336", "7760175111023239005", "694912874033826820" }, draws(4, n63, 5))
    assert.are.same({ "7070836379803831726", "8686239339925766635", "5009149828745571131", "8338494477124284581", "4815235170193628917" }, draws(0, n63, 5))
    assert.are.same({ "1227844342346046656", "4533873174211652710", "8688467253428114781", "4849545566009754239", "6960854651289091236" }, draws(1, n63, 5))
  end)
  it("n = 3, 7, 255, 1000 and 2^64 (one raw word)", function()
    assert.are.same({ "1", "0", "1", "1", "1" }, draws(0, 3, 5))
    assert.are.same({ "2", "1", "2", "4", "2" }, draws(0, 7, 5))
    assert.are.same({ "2", "1", "0", "2", "0" }, draws(1, 3, 5))
    assert.are.same({ "110", "226", "951", "236", "649" }, draws(2, 1000, 5))
    assert.are.same({ "13", "61", "108", "39", "250" }, draws(42, 255, 5))
    assert.are.same({ "16294208416658607535", "7960286522194355700", "487617019471545679", "17909611376780542444", "1961750202426094747" }, draws(0, Int.pow2(64), 5))
  end)
  it("the first words of seeds 0, 1, 2, 42", function()
    local got = {}
    for i, s in ipairs({ 0, 1, 2, 42 }) do got[i] = Int.tostring(Prng.new(s):next_u64()) end
    assert.are.same({ "16294208416658607535", "10451216379200822465", "10905525725756348110", "13679457532755275413" }, got)
  end)
  it("the engine draws through the sabotage seam: a shifted stream changes mutate_sequence", function()
    local base = Mutate.mutate_sequence("\0\0\0\0\0\0\0\0", 42, 4)
    assert.are.equal("03ff00000000bf080000", hex(base))
    local shifted = tb.Hooks.with({ new_rng = function(seed)
      local r = Prng.new(seed)
      r.state = r.state + 1
      return r
    end }, Mutate.mutate_sequence, "\0\0\0\0\0\0\0\0", 42, 4)
    assert.are_not.equal(base, shifted)
  end)
end)

describe("the minimiser's candidate sequences (bisection halves), from the Ruby lane", function()
  local SIG = Signature.crash_signature("k", {})

  -- runs the minimiser with a predicate; returns the executed candidates (hex), the result (hex), spent, finished
  local function run(data_hex, budget, crashes)
    local seen = {}
    local function test(c)
      seen[#seen + 1] = hex(c)
      return crashes(c) and Outcome.crash("k", {}) or Outcome.pass()
    end
    local out, spent, finished = Engine.minimize(unhex(data_hex), SIG, test, budget)
    return seen, hex(out), spent, finished
  end

  it("a 9-byte input whose crash needs byte 41: halves 4, 2, 1 then the zeroing pass", function()
    local seen, out, spent, finished = run("004142434445464748", 1000, function(c) return c:find("A", 1, true) ~= nil end)
    assert.are.same({ "4445464748", "0041424348", "00414243", "4243", "0041", "41", "", "00" }, seen)
    assert.are.equal("41", out)
    assert.are.equal(8, spent)
    assert.is_true(finished)
  end)
  it("a 1-byte crasher reaches the empty input (the first chunk is max(1, len/2))", function()
    local seen, out, spent, finished = run("41", 1000, function() return true end)
    assert.are.same({ "" }, seen)
    assert.are.equal("", out)
    assert.are.equal(1, spent)
    assert.is_true(finished)
  end)
  it("a 3-byte input that needs its first byte: deletion then zeroing", function()
    local seen, out, spent, finished = run("0a0b0c", 1000, function(c) return #c >= 1 and c:byte(1) == 0x0a end)
    assert.are.same({ "0b0c", "0a0c", "0a", "00" }, seen)
    assert.are.equal("0a", out)
    assert.are.equal(4, spent)
    assert.is_true(finished)
  end)
  it("a budget of 4 cuts the minimisation short and says so", function()
    local seen, out, spent, finished = run("0a0b0c0d0e0f1011", 4, function(c) return #c >= 1 and c:byte(1) == 0x0a end)
    assert.are.same({ "0e0f1011", "0a0b0c0d", "0c0d", "0a0b" }, seen)
    assert.are.equal("0a0b", out)
    assert.are.equal(4, spent)
    assert.is_false(finished)
  end)
  it("an empty input executes nothing", function()
    local seen, out, spent, finished = run("", 1000, function() return true end)
    assert.are.same({}, seen)
    assert.are.equal("", out)
    assert.are.equal(0, spent)
    assert.is_true(finished)
  end)
  it("a rejected candidate counts against the budget just like an accepted one", function()
    local seen, _, spent, finished = run("010203040506", 3, function() return false end)
    assert.are.equal(3, #seen)
    assert.are.equal(3, spent)
    assert.is_false(finished)
  end)
  it("a different crash is never accepted: the signature must match exactly", function()
    local seen = {}
    local out = Engine.minimize("\1\2\3", SIG, function(c)
      seen[#seen + 1] = c
      return Outcome.crash("other", {})
    end, 100)
    assert.are.equal("\1\2\3", out)
    assert.is_true(#seen > 0)
  end)
  it("a test that returns nothing (the campaign was aborted) stops the minimiser", function()
    local calls = 0
    local out, spent, finished = Engine.minimize("\1\2\3\4", SIG, function() calls = calls + 1 return nil end, 100)
    assert.are.equal(1, calls)
    assert.are.equal("\1\2\3\4", out)
    assert.are.equal(1, spent)
    assert.is_false(finished)
    -- ... in the zeroing pass too
    calls = 0
    out, spent, finished = Engine.minimize("\1", SIG, function(c)
      calls = calls + 1
      if c == "" then return Outcome.pass() end
      return nil
    end, 100)
    assert.are.equal(2, calls)
    assert.is_false(finished)
  end)
  it("minimize_crashing_input validates its arguments", function()
    local T = function() return Outcome.pass() end
    assert.are.equal("\1", Engine.minimize_crashing_input("\1", SIG, 5, T))
    assert.are.equal("\1", Engine.minimize_crashing_input("\1", SIG, nil, T))
    assert.has_error(function() Engine.minimize_crashing_input("\1", SIG, 5) end)
    assert.has_error(function() Engine.minimize_crashing_input(1, SIG, 5, T) end)
    assert.has_error(function() Engine.minimize_crashing_input("\1", SIG, -1, T) end)
    assert.has_error(function() Engine.minimize_crashing_input("\1", SIG, 100001, T) end)
    assert.has_error(function() Engine.minimize_crashing_input("\1", SIG, 1.5, T) end)
    assert.has_error(function() Engine.minimize_crashing_input(string.rep("x", (1 << 20) + 1), SIG, 5, T) end)
  end)
end)

describe("whole campaigns, driven by the same deterministic target as the Ruby lane", function()
  -- The target (identical in the Ruby script that produced the pins): edges from the
  -- length, the first byte and a long input; a timeout at 14 bytes, an oom when the byte
  -- sum is 5 mod 97, crash "k" (frame "f") when the sum is 0 mod 31, crash "j" when the
  -- second byte is 0x42.
  local function exec(data)
    local sum = 0
    for i = 1, #data do sum = sum + data:byte(i) end
    local edges = { #data % 5, 10 + (data:byte(1) or 0) % 6 }
    if #data > 5 then edges[#edges + 1] = 20 end
    if #data >= 14 then return Outcome.timeout(), edges end
    if sum % 97 == 5 then return Outcome.oom(), edges end
    if #data > 0 and sum % 31 == 0 then return Outcome.crash("k", { "f" }), edges end
    if #data >= 2 and data:byte(2) == 0x42 then return Outcome.crash("j", {}), edges end
    return Outcome.pass(), edges
  end

  for i, camp in ipairs(PINS.campaigns) do
    it("campaign " .. i .. " (seed " .. camp.seed .. ") is the Ruby lane's, report for report", function()
      local corpus = {}
      for j, h in ipairs(camp.corpus) do corpus[j] = unhex(h) end
      local o = Fuzz.Options.new({ max_iterations = camp.iters, seed = camp.seed, max_input_len = camp.mil, base_energy = 10, minimize_budget = 500 })
      local r = H.limited(5e8, Fuzz.run_outcomes, corpus, o, exec)
      assert.are.equal(camp.iterations, r.iterations)
      assert.are.equal(camp.admitted, r.admitted_seeds)
      assert.are.equal(camp.edges, r.total_edges)
      assert.are.equal(camp.timeouts, r.timeouts)
      assert.are.equal(camp.ooms, r.ooms)
      assert.are.equal(camp.dropped, r.crashes_dropped)
      assert.are.equal(#camp.crashes, #r.crashes)
      for j, c in ipairs(camp.crashes) do
        local rec = r.crashes[j]
        assert.are.equal(c[1], rec.signature)
        assert.are.equal(c[2], rec.kind)
        assert.are.equal(c[3], rec.hits)
        assert.are.equal(c[4], hex(rec.minimal_input))
        assert.are.equal(c[5], rec.minimized)
      end
      local seeds = {}
      for j, s in ipairs(r.corpus) do seeds[j] = hex(s) end
      assert.are.same(camp.seeds, seeds)
    end)
  end
end)
