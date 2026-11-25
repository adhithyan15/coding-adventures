-- The shared fuzz-v1 corpus, replayed through this package's own engine, with
-- per-operation negative controls and sabotage hooks (VER06p sections 8 and 9).
package.path = "../src/?.lua;../src/?/init.lua;" .. package.path
local H = require("helper")
local tb = H.tb
local Fuzz = tb.Fuzz
local Corpus, Hooks, Int = Fuzz.Corpus, Fuzz.Hooks, tb.Int

local root = H.repo_root()
local DATA = H.read_file(root .. "/code/specs/fixtures/fuzz-v1/cases.json")
local CASES = assert(Corpus.parse(DATA))

local function fresh()
  return H.deep_copy(CASES)
end

local function evaluate(cases, expected)
  return H.limited(5e8, Corpus.evaluate, cases, expected)
end

local function index_of(id)
  for i, c in ipairs(CASES) do
    if c.map.id == id then
      return i
    end
  end
  error("no case " .. id)
end

-- ---- corruptions ---------------------------------------------------------

local function bump_int(node)
  node.map.value = Int.tostring(Int.add(Int.parse(node.map.value), 1))
end

local function flip_output(m)
  local hex = m.output.map.hex
  m.output.map.hex = (hex == "") and "00" or ((hex:sub(1, 1) == "0" and "1" or "0") .. hex:sub(2))
end

-- Each op defines the corruptions that MUST be detected.
local CORRUPTIONS = {
  mutate_bit_flip = { { "flipped output", flip_output } },
  mutate_byte_flip = { { "flipped output", flip_output } },
  mutate_arithmetic = { { "flipped output", flip_output } },
  mutate_boundary = { { "flipped output", flip_output } },
  mutate_block_delete = { { "flipped output", flip_output } },
  mutate_block_insert = { { "flipped output", flip_output } },
  mutate_block_duplicate = { { "flipped output", flip_output } },
  mutate_splice = { { "flipped output", flip_output } },
  mutate_sequence = { { "flipped output", flip_output } },
  corpus_admit = {
    { "flipped admission", function(m) m.admitted = not m.admitted end },
    { "wrong new_edges", function(m) bump_int(m.new_edges) end },
  },
  crash_dedup = { { "flipped is_duplicate", function(m) m.is_duplicate = not m.is_duplicate end } },
  failure_id_valid = { { "flipped validity", function(m) m.valid = not m.valid end } },
  private_reporting = {
    { "a tap line changed by a character", function(m) m.tap_line = m.tap_line:gsub("ct_compare", "ct_compara") end },
    { "flipped privacy verdict", function(m) m.contains_raw_payload = not m.contains_raw_payload end },
  },
  corpus_reject = { { "a valid TFV in place of a reject text", function(m) m.text = "tfv 1\nok R1 Ka T\n" end } },
}

describe("the committed fuzz-v1 corpus", function()
  it("is exactly 25 cases and every one is evaluated with zero problems", function()
    assert.are.equal(25, #CASES)
    local report = evaluate(fresh())
    assert.are.same({}, report.problems)
    assert.are.equal(25, report.evaluated)
    print("fuzz-v1: " .. report.evaluated .. "/25")
  end)

  it("evaluates straight from the document text, and the TFV twin is valid", function()
    local report = H.limited(5e8, Corpus.evaluate_document, DATA)
    assert.are.equal(25, report.evaluated)
    assert.are.equal(0, #report.problems)
    assert.is_true(tb.Tfv.valid(H.read_file(root .. "/code/specs/fixtures/fuzz-v1/cases.tfv")))
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
  it("a dropped case, a duplicated one, a truncated corpus and an empty one are problems", function()
    local cases = fresh()
    table.remove(cases, 3)
    local r = evaluate(cases)
    assert.is_true(#r.problems > 0)
    assert.is_truthy(r.problems[1]:find("expected 25 cases, got 24", 1, true))
    local dup = fresh()
    dup[#dup + 1] = H.deep_copy(dup[1])
    assert.is_true(#evaluate(dup).problems > 0)
    local half = fresh()
    for i = #half, 11, -1 do half[i] = nil end
    assert.is_true(#evaluate(half).problems > 0)
    assert.is_true(#evaluate({}).problems > 0)
  end)

  it("dropping every case of an op reports the op as never exercised", function()
    local cases = {}
    for _, c in ipairs(fresh()) do
      if c.map.op ~= "crash_dedup" then cases[#cases + 1] = c end
    end
    local r = evaluate(cases, false)
    assert.is_truthy(table.concat(r.problems, "\n"):find("never exercised: crash_dedup", 1, true))
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

  it("a registered id must be accepted and an unregistered one rejected", function()
    local cases = fresh()
    table.remove(cases, index_of("fuzz/failure_id/unregistered"))
    assert.is_truthy(table.concat(evaluate(cases, false).problems, "\n"):find("unregistered-id rejection", 1, true))
    cases = fresh()
    table.remove(cases, index_of("fuzz/failure_id/crash_valid"))
    assert.is_truthy(table.concat(evaluate(cases, false).problems, "\n"):find("registered-id acceptance", 1, true))
  end)

  it("a malformed value is a problem for that case: bad int, bad hex, huge steps, wrong types", function()
    local function first_of(op)
      for i, c in ipairs(fresh()) do
        if c.map.op == op then return i end
      end
    end
    local cases = fresh()
    local i = first_of("mutate_bit_flip")
    cases[i].map.bit_offset = H.jint("0123")
    local r = evaluate(cases)
    assert.is_truthy(r.problems[1]:find(cases[i].map.id .. ": mutate_bit_flip could not be evaluated", 1, true))
    cases = fresh()
    cases[i].map.input = H.jbytes("0G")
    assert.is_truthy(evaluate(cases).problems[1]:find("could not be evaluated", 1, true))
    cases = fresh()
    i = first_of("mutate_sequence")
    cases[i].map.steps = H.jint("1000000000")
    assert.is_truthy(evaluate(cases).problems[1]:find("steps outside", 1, true))
    cases = fresh()
    cases[i].map.steps = H.jbytes("00") -- not an int
    assert.is_truthy(evaluate(cases).problems[1]:find("steps must be an int", 1, true))
    cases = fresh()
    cases[i].map.input = H.jint("5") -- not bytes
    assert.is_truthy(evaluate(cases).problems[1]:find("input must be bytes", 1, true))
    cases = fresh()
    i = first_of("mutate_arithmetic")
    cases[i].map.endian = H.jint("1") -- not text
    assert.is_truthy(evaluate(cases).problems[1]:find("endian must be a string", 1, true))
    cases = fresh()
    cases[i].map.endian = "middle" -- text, but not an endianness: the mutator refuses
    assert.is_truthy(evaluate(cases).problems[1]:find("endian must be", 1, true))
    cases = fresh()
    i = first_of("corpus_admit")
    cases[i].map.admitted = H.jint("1")
    assert.is_truthy(evaluate(cases).problems[1]:find("admitted must be a bool", 1, true))
    cases = fresh()
    cases[i].map.trace_edges = { kind = "array", H.jint("-1") }
    assert.is_truthy(evaluate(cases).problems[1]:find("outside [0, 2^64)", 1, true))
    cases = fresh()
    cases[i].map.trace_edges = { kind = "array", H.jint("18446744073709551616") }
    assert.is_truthy(evaluate(cases).problems[1]:find("outside [0, 2^64)", 1, true))
    cases = fresh()
    cases[i].map.trace_edges = H.jint("3")
    assert.is_truthy(evaluate(cases).problems[1]:find("trace_edges must be a list of ints", 1, true))
    cases = fresh()
    cases[i].map.existing_edges = { kind = "array", true }
    assert.is_truthy(evaluate(cases).problems[1]:find("existing_edges must be a list of ints", 1, true))
    cases = fresh()
    i = first_of("crash_dedup")
    cases[i].map.crash_a_frames = { kind = "array", H.jint("1") }
    assert.is_truthy(evaluate(cases).problems[1]:find("must be a list of strings", 1, true))
    cases = fresh()
    cases[i].map.crash_a_frames = "eval"
    assert.is_truthy(evaluate(cases).problems[1]:find("must be a list of strings", 1, true))
    cases = fresh()
    i = first_of("private_reporting")
    cases[i].map.failure_id = "fuzz-arbitrary"
    assert.is_truthy(evaluate(cases).problems[1]:find("format_tap_failure refused", 1, true))
    cases = fresh()
    i = first_of("corpus_reject")
    cases[i].map.text = H.jint("1")
    assert.is_truthy(evaluate(cases).problems[1]:find("text must be a string", 1, true))
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

  it("a case that is not an object, and ids and fragments, are bounded", function()
    local cases = fresh()
    cases[1].map.id = string.rep("x", 5000)
    local r = evaluate(cases)
    for _, p in ipairs(r.problems) do
      assert.is_true(#p <= 400 + 3)
    end
    cases = fresh()
    cases[1] = true
    assert.is_truthy(table.concat(evaluate(cases).problems, "\n"):find("duplicate or missing case id", 1, true))
  end)

  it("every problem about a case starts with its id, escaped", function()
    local cases = fresh()
    cases[1].map.id = "evil\nid"
    cases[1].map.op = "teleport"
    local r = evaluate(cases)
    assert.is_truthy(r.problems[1]:find("evil\\", 1, true), r.problems[1])
    assert.is_nil(r.problems[1]:find("\n", 1, true))
  end)
end)

describe("the aggregate mutate_sequence work cap", function()
  local function sequence_case(steps, input_hex)
    return {
      kind = "object", keys = { "id", "op", "input", "seed", "steps", "output" },
      map = { id = "s" .. steps, op = "mutate_sequence", input = H.jbytes(input_hex), seed = H.jint("1"),
              steps = H.jint(tostring(steps)), output = H.jbytes("") },
    }
  end

  local function three()
    local cases = {}
    for i = 1, 3 do
      local c = sequence_case(4, "0000000000000000")
      c.map.id = "w" .. i
      cases[i] = c
    end
    return cases
  end

  it("a breach zeroes the allowance, so every later case fails at once without running", function()
    local calls = 0
    local r = Hooks.with({ mutate_sequence = function() calls = calls + 1 return nil, 0 end }, function()
      return Corpus.evaluate(three(), false)
    end)
    local text = table.concat(r.problems, "\n")
    for i = 1, 3 do
      assert.is_truthy(text:find("w" .. i .. ": mutate_sequence could not be evaluated", 1, true), text)
    end
    assert.is_truthy(text:find("work budget", 1, true))
    assert.are.equal(1, calls, "the later cases must not run the engine")
  end)

  it("the work a call reports is charged against the aggregate of 2^24, exactly", function()
    local seen = {}
    local r = Hooks.with({ mutate_sequence = function(_, _, _, _, cap)
      seen[#seen + 1] = cap
      return "", 1 << 23
    end }, function()
      return Corpus.evaluate(three(), false)
    end)
    -- the third call finds nothing left: it is refused before the engine runs
    assert.are.same({ 1 << 24, 1 << 23 }, seen)
    local text = table.concat(r.problems, "\n")
    assert.is_truthy(text:find("w3: mutate_sequence could not be evaluated", 1, true), text)
    assert.is_nil(text:find("w1: mutate_sequence could not", 1, true))
    assert.is_nil(text:find("w2: mutate_sequence could not", 1, true))
  end)

  it("a real call that fits is charged what it did, not what it might do", function()
    local r = H.limited(5e8, Corpus.evaluate, three(), false)
    -- the outputs are wrong ("") so there are mismatch problems, but never a budget one
    assert.is_nil(table.concat(r.problems, "\n"):find("work budget", 1, true))
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
    bad((DATA:gsub('"id": "fuzz/mutate_bit_flip/bit0"', '"id": "fuzz/mutate_bit_flip/bit0", "id": "dup"', 1)))
  end)
end)

describe("sabotaged engines fail the corpus", function()
  local function problems_with(over)
    return #Hooks.with(over, function()
      return Corpus.evaluate(fresh()).problems
    end)
  end

  local same = function(data) return data end
  local SABOTAGE = {
    bit_flip = same,
    byte_flip = same,
    byte_arithmetic = same,
    boundary_insert = same,
    block_delete = same,
    block_insert = same,
    block_duplicate = same,
    splice = same,
    mutate_sequence = function(data) return data, 0 end,
    corpus_admit = function() return true, 0 end,
    duplicate_crash = function() return true end,
    failure_id = function() return true end,
    format_tap = function() return "not ok 1 - ct_compare [fuzz-crash:3a8f19b2]deadbeef" end,
    tfv_valid = function() return true end,
  }

  for name, wrong in pairs(SABOTAGE) do
    it("a wrong " .. name, function()
      assert.is_true(problems_with({ [name] = wrong }) > 0)
    end)
  end

  it("a different PRNG stream changes mutate_sequence", function()
    local Prng = tb.Prng
    assert.is_true(#tb.Hooks.with({
      new_rng = function(seed)
        local r = Prng.new(seed)
        r.state = r.state + 1
        return r
      end,
    }, function()
      return Corpus.evaluate(fresh()).problems
    end) > 0)
  end)

  it("every hook of the table is sabotaged above", function()
    for name in pairs(Hooks.DEFAULTS) do
      assert.is_truthy(SABOTAGE[name], "no sabotage for " .. name)
    end
  end)

  it("an unknown hook is refused, and hooks are restored afterwards, even after an error", function()
    assert.has_error(function() Hooks.with({ nonsense = same }, function() end) end)
    assert.has_error(function() Hooks.with({ splice = same }, function() error("boom") end) end)
    assert.are.equal(Hooks.DEFAULTS.splice, Hooks.splice)
    assert.are.equal(0, #Corpus.evaluate(fresh()).problems)
  end)

  it("an engine function that raises is a problem for its case", function()
    local r = Hooks.with({ bit_flip = function() error("kaboom", 0) end }, function()
      return Corpus.evaluate(fresh())
    end)
    assert.is_truthy(table.concat(r.problems, "\n"):find("mutate_bit_flip could not be evaluated: kaboom", 1, true))
  end)
end)
