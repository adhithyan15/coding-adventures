-- SplitMix64, FNV-1a, bounded draws and sub-seeds (VER05 2, VER05o 3), with the
-- pinned tests for what the pbt-v1 corpus cannot catch.
package.path = "../src/?.lua;../src/?/init.lua;" .. package.path
local H = require("helper")
local Int, Prng, Hooks, Gens = H.tb.Int, H.tb.Prng, H.tb.Hooks, H.tb.Gens

local function words(rng, n)
  local out = {}
  for i = 1, n do
    out[i] = H.dec(rng:next_u64())
  end
  return out
end

describe("SplitMix64", function()
  it("matches the reference vector for seed 0", function()
    assert.are.same({ "16294208416658607535", "7960286522194355700", "487617019471545679" }, words(Prng.new(0), 3))
    assert.are.equal(0xe220a8397b1dcdaf, Prng.new(0):next_word())
  end)

  it("reduces any seed modulo 2^64, two's complement", function()
    assert.are.equal("16490336266968443936", words(Prng.new(-1), 1)[1])
    assert.are.equal(words(Prng.new(Int.add(Int.pow2(64), 5)), 1)[1], words(Prng.new(5), 1)[1])
    assert.are.equal("7134611160154358618", words(Prng.new(5), 1)[1])
  end)

  it("refuses floats, booleans and strings as seeds", function()
    assert.has_error(function() Prng.new(3.0) end)
    assert.has_error(function() Prng.new(0.5) end)
    assert.has_error(function() Prng.new(true) end)
    assert.has_error(function() Prng.new("5") end)
    assert.has_error(function() Prng.new(nil) end)
    assert.has_error(function() Prng.new(0 / 0) end)
  end)

  it("hides its state when printed", function()
    assert.are.equal("#<Prng.SplitMix64>", tostring(Prng.new(1)))
  end)

  it("wraps at the 64-bit boundary instead of overflowing", function()
    local r = Prng.new(0)
    r.state = math.maxinteger
    assert.is_true(math.type(r:next_word()) == "integer")
    r.state = -1
    assert.is_true(math.type(r:next_word()) == "integer")
  end)
end)

describe("FNV-1a 64 and sub_seed", function()
  it("matches the reference", function()
    assert.are.equal("14695981039346656037", H.dec(Prng.fnv1a64("")))
    assert.are.equal("12638187200555641996", H.dec(Prng.fnv1a64("a")))
    assert.are.equal("9625390261332436968", H.dec(Prng.fnv1a64("foobar")))
    assert.are.equal("6152156688079238993", H.dec(Prng.fnv1a64("prop/\195\169"))) -- hashed as raw bytes
  end)

  it("rejects non-strings", function()
    assert.has_error(function() Prng.fnv1a64(1) end)
    assert.has_error(function() Prng.fnv1a64(nil) end)
  end)

  it("derives sub-seeds as SplitMix64(seed xor fnv(id)).next_u64", function()
    local seeds = { 0, 1, 12345, Int.sub(Int.pow2(64), 1) }
    local a = { "6857225946766476583", "13906159068427031964", "5869837554499256867", "15365624696913284058" }
    local b = { "9313204760084199774", "17665131612661548845", "9898446428729937406", "4245073340154229409" }
    for i, s in ipairs(seeds) do
      assert.are.equal(a[i], H.dec(Prng.sub_seed(s, "a")))
      assert.are.equal(b[i], H.dec(Prng.sub_seed(s, "prop/ct_compare_eq_reflexive")))
    end
    assert.has_error(function() Prng.sub_seed(1.0, "a") end)
    assert.has_error(function() Prng.sub_seed(1, 5) end)
  end)
end)

describe("bounded", function()
  it("agrees with the reference at every width", function()
    local r = Prng.new(11)
    local want = { "0", "1", "0", "0", "1963219396", "958081316009564174", "5164757140853396969", "6280598950732548470" }
    local ns = { 1, 2, 3, 10, 4294967296, Int.pow2(63), Int.add(Int.pow2(63), 1), Int.sub(Int.pow2(64), 1) }
    for i, n in ipairs(ns) do
      assert.are.equal(want[i], H.dec(r:bounded(n)))
    end
  end)

  it("treats 2^64 as one raw word and refuses everything outside [1, 2^64]", function()
    assert.are.equal(H.dec(Prng.new(5):next_u64()), H.dec(Prng.new(5):bounded(Int.pow2(64))))
    assert.has_error(function() Prng.new(5):bounded(0) end)
    assert.has_error(function() Prng.new(5):bounded(-3) end)
    assert.has_error(function() Prng.new(5):bounded(Int.add(Int.pow2(64), 1)) end)
    assert.has_error(function() Prng.new(5):bounded(2.0) end)
    assert.has_error(function() Prng.new(5):bounded("3") end)
  end)

  it("returns values below n, including above 2^63 (a Big)", function()
    local n = Int.sub(Int.pow2(64), 1)
    local r = Prng.new(3)
    for _ = 1, 50 do
      local v = r:bounded(n)
      assert.is_true(Int.cmp(v, n) < 0 and Int.sign(v) >= 0)
    end
  end)

  it("bounded_big uses k-word rejection above 2^64 and stays in range", function()
    local span = Int.add(Int.pow2(100), 12345)
    local r = Prng.new(8)
    for _ = 1, 50 do
      local v = r:bounded_big(span)
      assert.is_true(Int.cmp(v, span) < 0 and Int.sign(v) >= 0)
    end
    assert.has_error(function() Prng.new(1):bounded_big(0) end)
    assert.has_error(function() Prng.new(1):bounded_big(Int.pow2(16400)) end)
    -- a span of 2^64 or less is the plain bounded draw (same stream position)
    local a, b = Prng.new(9), Prng.new(9)
    assert.are.equal(H.dec(a:bounded(1000)), H.dec(b:bounded_big(1000)))
    assert.are.equal(H.dec(a:next_u64()), H.dec(b:next_u64()))
  end)
end)

describe("pinned tests for what the corpus cannot catch", function()
  it("the rejection draw: seed 7046029254386353131 starts with the word 0", function()
    local r = Prng.new(H.int("7046029254386353131"))
    assert.are.same({ "0", "16294208416658607535" }, words(r, 2))
    -- bounded(3) has threshold 1: the 0 MUST be rejected; the answer is 16294208416658607535 mod 3 = 1
    assert.are.equal(1, Prng.new(H.int("7046029254386353131")):bounded(3))
    assert.are.equal(1, Prng.threshold(3))
  end)

  it("a threshold of 0 still passes the whole corpus: only the pin above catches it", function()
    local zero = function()
      return 0
    end
    -- the corpus passes with the broken threshold ...
    local f = assert(io.open(H.repo_root() .. "/code/specs/fixtures/pbt-v1/cases.json", "rb"))
    local data = f:read("a")
    f:close()
    local report = Hooks.with({ threshold = zero }, function()
      return H.tb.Corpus.evaluate_document(data)
    end)
    assert.are.equal(0, #report.problems)
    assert.are.equal(25, report.evaluated)
    -- ... and the pin does not
    local got = Hooks.with({ threshold = zero }, function()
      return Prng.new(H.int("7046029254386353131")):bounded(3)
    end)
    assert.are.equal(0, got) -- the rejected 0 was accepted
  end)

  it("the 2^63+1 boundary: threshold 2^63-1, a draw below it is rejected", function()
    local n = Int.add(Int.pow2(63), 1)
    assert.are.equal(math.maxinteger, Prng.threshold(Int.to_u64(n)))
    -- seed 3's first word 2092789425003139053 < 2^63-1 -> rejected; answer from word 2
    assert.are.equal("2092789425003139053", words(Prng.new(3), 1)[1])
    assert.are.equal("3694763184872335752", H.dec(Prng.new(3):bounded(n)))
    -- seed 1's first word is above the threshold and is accepted
    assert.are.equal("1227844342346046656", H.dec(Prng.new(1):bounded(n)))
    -- a draw of exactly the threshold is KEPT, one below is rejected (forced stream)
    local function forced(list)
      local rng = Prng.new(0)
      local i = 0
      function rng:next_word()
        i = i + 1
        return list[i]
      end
      return rng
    end
    assert.are.equal(math.maxinteger, forced({ math.maxinteger }):bounded(n))
    assert.are.equal("9223372036854775808", H.dec(forced({ math.maxinteger - 1, math.mininteger }):bounded(n)))
  end)

  it("bounded(1) consumes a word", function()
    local r = Prng.new(7)
    assert.are.equal(0, r:bounded(1))
    assert.are.equal("309689372594955804", H.dec(r:next_u64()))
  end)
end)

describe("Hooks.with", function()
  it("restores every hook on an error and refuses unknown hooks", function()
    local before = Hooks.bisect
    assert.has_error(function()
      Hooks.with({ bisect = function() end }, function()
        error("boom")
      end)
    end)
    assert.are.equal(before, Hooks.bisect)
    assert.has_error(function() Hooks.with({ nonsense = 1 }, function() end) end)
    assert.are.equal(Hooks.default_unit, Hooks.unit)
  end)
end)
