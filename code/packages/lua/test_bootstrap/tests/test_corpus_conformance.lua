-- The shared pbt-v1 corpus, replayed through this package's own engine, with
-- per-operation negative controls and sabotage hooks (VER05o section 7).
package.path = "../src/?.lua;../src/?/init.lua;" .. package.path
local H = require("helper")
local tb = H.tb
local Corpus, Int, Hooks, Prng, Canon = tb.Corpus, tb.Int, tb.Hooks, tb.Prng, tb.Canon

local root = H.repo_root()
local DATA = H.read_file(root .. "/code/specs/fixtures/pbt-v1/cases.json")
local CASES = assert(Corpus.parse(DATA))

local function fresh()
  return H.deep_copy(CASES)
end

local function evaluate(cases, expected)
  return H.limited(5e8, Corpus.evaluate, cases, expected)
end

-- ---- corruptions ---------------------------------------------------------

local function bump_int(node)
  node.map.value = Int.tostring(Int.add(Int.parse(node.map.value), 1))
end

local function corrupt_value(node)
  if type(node) == "boolean" then
    return not node
  elseif node.kind == "array" then
    if #node == 0 then
      node[1] = H.jint("0")
    else
      node[1] = corrupt_value(node[1])
    end
    return node
  elseif node.map and node.map.kind == "int" then
    bump_int(node)
    return node
  elseif node.map and node.map.kind == "bytes" then
    local hex = node.map.hex
    node.map.hex = (hex == "") and "00" or ((hex:sub(1, 1) == "0" and "1" or "0") .. hex:sub(2))
    return node
  end
  error("cannot corrupt " .. tostring(node.kind))
end

-- Each op defines the corruptions that MUST be detected.
local CORRUPTIONS = {
  gen_primitive = { { "flipped output", function(m) m.outputs = corrupt_value(m.outputs) end } },
  gen_bounded = { { "flipped output", function(m) m.outputs = corrupt_value(m.outputs) end } },
  gen_bytes = { { "flipped output", function(m) m.outputs = corrupt_value(m.outputs) end } },
  shrink_int = {
    { "wrong minimal", function(m) bump_int(m.expected_minimal) end },
    { "steps + 1", function(m) bump_int(m.expected_steps) end },
  },
  shrink_refinement = {
    { "wrong minimal", function(m) bump_int(m.expected_minimal) end },
    { "steps + 1", function(m) bump_int(m.expected_steps) end },
  },
  shrink_list = {
    { "wrong minimal", function(m) m.expected_minimal = corrupt_value(m.expected_minimal) end },
    { "steps + 1", function(m) bump_int(m.expected_steps) end },
  },
  shrink_bytes = {
    { "wrong minimal", function(m) m.expected_minimal = corrupt_value(m.expected_minimal) end },
    { "steps + 1", function(m) bump_int(m.expected_steps) end },
  },
  replay_consistency = {
    { "wrong initial_fail", function(m) bump_int(m.initial_fail) end },
    { "wrong minimal", function(m) bump_int(m.expected_minimal) end },
  },
  filter_exhausted = {
    { "wrong failure id", function(m) m.failure_id = "property-panic" end },
    { "verdict pass", function(m) m.verdict = "pass" end },
  },
  failure_id_valid = { { "flipped validity", function(m) m.valid = not m.valid end } },
  corpus_reject = { { "a valid TFV in place of a reject text", function(m) m.text = "tfv 1\nok R1 Ka T\n" end } },
}

describe("the committed pbt-v1 corpus", function()
  it("is exactly 25 cases and every one is evaluated with zero problems", function()
    assert.are.equal(25, #CASES)
    local report = evaluate(fresh())
    assert.are.same({}, report.problems)
    assert.are.equal(25, report.evaluated)
    print("pbt-v1: " .. report.evaluated .. "/25")
  end)

  it("evaluates straight from the document text, and the TFV twin is valid", function()
    local report = H.limited(5e8, Corpus.evaluate_document, DATA)
    assert.are.equal(25, report.evaluated)
    assert.are.equal(0, #report.problems)
    assert.is_true(tb.Tfv.valid(H.read_file(root .. "/code/specs/fixtures/pbt-v1/cases.tfv")))
  end)

  it("covers every op, so every op has a negative control below", function()
    local ops = {}
    for _, c in ipairs(CASES) do
      ops[c.map.op] = (ops[c.map.op] or 0) + 1
    end
    for op in pairs(CORRUPTIONS) do
      assert.is_truthy(ops[op], "no case for op " .. op)
    end
    for op in pairs(ops) do
      assert.is_truthy(CORRUPTIONS[op], "no corruption defined for op " .. op)
    end
  end)
end)

describe("per-operation negative controls", function()
  for index, case in ipairs(CASES) do
    local op, id = case.map.op, case.map.id
    for _, corruption in ipairs(CORRUPTIONS[op]) do
      it(id .. ": " .. corruption[1], function()
        local cases = fresh()
        corruption[2](cases[index].map)
        local report = evaluate(cases)
        assert.is_true(#report.problems > 0, "the corruption was not detected")
        for _, p in ipairs(report.problems) do
          assert.are.equal(id .. ":", p:sub(1, #id + 1), "problem does not start with the case id: " .. p)
          for j, other in ipairs(CASES) do
            if j ~= index then
              assert.is_nil(p:find(other.map.id .. ":", 1, true), "problem names another case: " .. p)
            end
          end
        end
      end)
    end
  end
end)

describe("whole-corpus corruptions", function()
  it("a dropped case, a truncated corpus and an empty one are problems", function()
    local cases = fresh()
    table.remove(cases, 3)
    local r = evaluate(cases)
    assert.is_true(#r.problems > 0)
    assert.is_truthy(r.problems[1]:find("expected 25 cases, got 24", 1, true))
    local half = fresh()
    for i = #half, 11, -1 do half[i] = nil end
    assert.is_true(#evaluate(half).problems > 0)
    assert.is_true(#evaluate({}).problems > 0)
  end)

  it("dropping every case of an op reports the op as never exercised", function()
    local cases = {}
    for _, c in ipairs(fresh()) do
      if c.map.op ~= "shrink_bytes" then cases[#cases + 1] = c end
    end
    local r = evaluate(cases, false)
    assert.is_truthy(table.concat(r.problems, "\n"):find("never exercised: shrink_bytes", 1, true))
  end)

  it("an unknown op is a problem naming that case, never a skip", function()
    local cases = fresh()
    cases[2].map.op = "teleport"
    local r = evaluate(cases)
    assert.is_true(#r.problems > 0)
    assert.are.equal(cases[2].map.id .. ": unknown op", r.problems[1]:sub(1, #cases[2].map.id + 12))
  end)

  it("a missing, an extra or a renamed field is a problem", function()
    local cases = fresh()
    cases[1].keys[#cases[1].keys + 1] = "extra"
    cases[1].map.extra = true
    assert.is_true(#evaluate(cases).problems > 0)
    cases = fresh()
    cases[1].keys[#cases[1].keys] = nil
    assert.is_true(#evaluate(cases).problems > 0)
  end)

  it("a missing, non-string or duplicate id is a problem, not a crash", function()
    local cases = fresh()
    cases[2].map.id = 7
    assert.is_true(#evaluate(cases).problems > 0)
    cases = fresh()
    cases[2].map.id = cases[1].map.id
    assert.is_truthy(table.concat(evaluate(cases).problems, "\n"):find("duplicate or missing case id", 1, true))
    cases = fresh()
    cases[2] = "not a case"
    assert.is_true(#evaluate(cases).problems > 0)
    cases = fresh()
    cases[2].map.id = ""
    assert.is_true(#evaluate(cases).problems > 0)
  end)

  it("an unregistered failure id must be rejected and all four registered ones present", function()
    local cases = fresh()
    for i, c in ipairs(cases) do
      if c.map.id == "pbt/failure_id/unregistered" then table.remove(cases, i) break end
    end
    assert.is_truthy(table.concat(evaluate(cases, false).problems, "\n"):find("unregistered-id rejection", 1, true))
    cases = fresh()
    for i, c in ipairs(cases) do
      if c.map.id == "pbt/failure_id/timeout" then table.remove(cases, i) break end
    end
    assert.is_truthy(table.concat(evaluate(cases, false).problems, "\n"):find("property-timeout", 1, true))
  end)

  it("a malformed value is a problem for that case: bad int, bad hex, huge count", function()
    local function first_of(op)
      for i, c in ipairs(fresh()) do
        if c.map.op == op then return i end
      end
    end
    local cases = fresh()
    local i = first_of("shrink_int")
    cases[i].map.initial = H.jint("0123")
    local r = evaluate(cases)
    assert.is_truthy(r.problems[1]:find(cases[i].map.id .. ": shrink_int could not be evaluated", 1, true))
    cases = fresh()
    i = first_of("gen_bytes")
    cases[i].map.count = H.jint("100000")
    assert.is_true(#evaluate(cases).problems > 0)
    cases = fresh()
    i = first_of("gen_primitive")
    cases[i].map.outputs = { kind = "array" } -- wrong length: refused before generating
    assert.is_truthy(evaluate(cases).problems[1]:find("exactly count", 1, true))
    cases = fresh()
    cases[i].map.type = "bool" -- a JSON string decodes to text: fine ...
    cases[i].map.type = H.jint("5") -- ... but an int is not a type name
    assert.is_true(#evaluate(cases).problems > 0)
  end)

  it("an unknown property name is a problem naming the case", function()
    local cases = fresh()
    for _, c in ipairs(cases) do
      if c.map.op == "shrink_int" then
        c.map.property = "int_is_prime"
        break
      end
    end
    local r = evaluate(cases)
    assert.is_truthy(r.problems[1]:find("unknown property", 1, true))
  end)

  it("a hostile case count is capped before any allocation", function()
    local many = {}
    for i = 1, Corpus.MAX_CASES + 5 do
      many[i] = { kind = "object", keys = { "id" }, map = { id = "c" .. i } }
    end
    local r = H.limited(2e8, Corpus.evaluate, many)
    assert.is_true(#r.problems > 0)
    assert.is_true(#r.problems <= Corpus.MAX_PROBLEMS)
  end)

  it("fragments and problems are clipped", function()
    local cases = fresh()
    cases[1].map.id = string.rep("x", 5000)
    local r = evaluate(cases)
    for _, p in ipairs(r.problems) do
      assert.is_true(#p <= 400 + 3)
    end
  end)
end)

describe("document-level strictness", function()
  local function bad(text)
    local cases, err = Corpus.parse(text)
    assert.is_nil(cases)
    assert.is_string(err)
    local report = Corpus.evaluate_document(text)
    assert.are.equal(0, report.evaluated)
    assert.is_true(#report.problems > 0)
  end

  it("refuses a truncated document, a wrong version, extra top-level keys and a bad shape", function()
    bad(DATA:sub(1, #DATA // 2))
    bad(DATA:sub(1, #DATA - 3))
    bad((DATA:gsub('"schema_version": 1', '"schema_version": 2', 1)))
    bad((DATA:gsub('"schema_version": 1', '"schema_version": 1, "extra": 1', 1)))
    bad('{"schema_version": 1, "cases": {}}')
    bad('{"schema_version": "1", "cases": []}')
    bad("[]")
    bad("")
    bad(DATA .. "x")
    bad("\239\187\191" .. DATA)
    bad((DATA:gsub('"id": "pbt/gen_primitive/bool_seed0"', '"id": "pbt/gen_primitive/bool_seed0", "id": "dup"', 1)))
  end)
end)

describe("sabotaged engines fail the corpus", function()
  local function problems_with(over)
    return #Hooks.with(over, function()
      return Corpus.evaluate(fresh()).problems
    end)
  end

  it("a changed bisection", function()
    assert.is_true(problems_with({ bisect = function(c) return Int.sub(c, 1) end }) > 0)
    assert.is_true(problems_with({ bisect = function(c, d) return Int.sub(c, Int.sub(d, 1)) end }) > 0)
  end)

  it("a ceiling instead of a floored half (the negative-bisection bug)", function()
    local ceil_half = function(c, d)
      local half = Int.add(Int.floor_half(d), (Int.is_even(d) and 0 or 1))
      return Int.sub(c, half)
    end
    assert.is_true(problems_with({ bisect = ceil_half }) > 0)
  end)

  it("a changed unit step", function()
    assert.is_true(problems_with({ unit = function(c, d) return Int.sub(c, Int.add(Int.sign(d), Int.sign(d))) end }) > 0)
  end)

  it("a skipped deletion phase", function()
    assert.is_true(problems_with({ skip_deletion = true }) > 0)
  end)

  it("a shifted RNG", function()
    assert.is_true(problems_with({
      new_rng = function(seed)
        local r = Prng.new(seed)
        r.state = r.state + 1
        return r
      end,
    }) > 0)
  end)

  it("a wrong rejection threshold is NOT caught by the corpus (the documented blind spot)", function()
    assert.are.equal(0, problems_with({ threshold = function() return 0 end }))
  end)

  it("hooks are restored afterwards", function()
    problems_with({ skip_deletion = true })
    assert.is_false(Hooks.skip_deletion)
    assert.are.equal(0, #Corpus.evaluate(fresh()).problems)
  end)
end)
