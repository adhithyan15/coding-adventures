--- The production runner for the shared pbt-v1 corpus
-- (`code/specs/fixtures/pbt-v1/cases.json`, VER05 section 6, VER05o section 7).
--
-- It takes the raw JSON, strictly decodes it, evaluates every case with THIS
-- package's own public engine functions and returns human-readable problems.  An
-- empty list means "conforms".  It does no file I/O (the package is
-- capability-free); the test that reads the committed fixture lives in the test
-- tree.
--
-- Why a runner that can fail matters: a conformance check that cannot report a
-- problem is worse than none.  So the runner is strict:
--
--   * a case must have exactly the fields of its op, with canonical encodings;
--   * an unknown op, unknown property name or malformed value is a PROBLEM, never a
--     silent skip;
--   * every op and all four failure ids must appear, an unregistered id must be
--     rejected, and the number of cases must equal PBTV1_CASE_COUNT, so deleting
--     cases is itself a problem;
--   * every problem about a case starts with that case's id.

-- Standard functions are captured at load so that a property that patches the
-- string metatable or a library table cannot make the engine raise (VER05o 4).
local m_type, s_find, s_format, t_concat, t_sort = math.type, string.find, string.format, table.concat, table.sort
local type, pcall, rawget, rawlen, ipairs, pairs = type, pcall, rawget, rawlen, ipairs, pairs

local Int = require("coding_adventures.test_bootstrap.int")
local Canon = require("coding_adventures.test_bootstrap.canon")
local StrictJson = require("coding_adventures.test_bootstrap.strict_json")
local Values = require("coding_adventures.test_bootstrap.values")
local Tfv = require("coding_adventures.test_bootstrap.tfv")
local FailureIds = require("coding_adventures.test_bootstrap.failure_ids")
local Gens = require("coding_adventures.test_bootstrap.gens")
local Shrink = require("coding_adventures.test_bootstrap.shrink")
local Runner = require("coding_adventures.test_bootstrap.runner")

local Corpus = {}

Corpus.PBTV1_CASE_COUNT = 25
Corpus.MAX_CASES = 10000
Corpus.MAX_PROBLEMS = 1000
-- Bounds the total bytes one gen case may generate and hold (CWE-400).
Corpus.MAX_GEN_BYTES = 1 << 20

local show = Values.show
local same = Values.same

--- Strictly decodes a pbt-v1 document into its case list: exactly
-- {"schema_version": 1, "cases": [...]}.  Cases are returned undecoded.  Returns
-- `cases` or `nil, message`.
function Corpus.parse(data)
  local root, err = StrictJson.parse(data)
  if root == nil then
    return nil, err
  end
  if type(root) ~= "table" or root.kind ~= "object" or #root.keys ~= 2 then
    return nil, 'pbt corpus: want exactly {"schema_version": 1, "cases": [...]}'
  end
  local version = root.map.schema_version
  if type(version) ~= "table" or version.kind ~= "number" or version.text ~= "1" then
    return nil, "pbt corpus: schema_version must be 1"
  end
  local cases = root.map.cases
  if type(cases) ~= "table" or cases.kind ~= "array" then
    return nil, 'pbt corpus: "cases" must be an array'
  end
  return cases
end

local function int_le(x, n)
  return Int.le(x, n)
end

-- Predicates named by the corpus.  They return exactly `true` when the property
-- HOLDS.
local PROPERTIES = {
  int_le_100 = function(x)
    return int_le(x, 100)
  end,
  int_in_range_neg10_50 = function(x)
    return Int.le(-10, x) and Int.le(x, 50)
  end,
  int_is_even = function(x)
    return Int.is_even(x)
  end,
  int_abs_le_10 = function(x)
    return Int.le(Int.abs(x), 10)
  end,
  refinement_x_le_50 = function(x)
    return int_le(x, 50)
  end,
  list_no_consecutive_duplicates = function(l)
    for i = 1, rawlen(l) - 1 do
      if Int.eq(rawget(l, i), rawget(l, i + 1)) then
        return false
      end
    end
    return true
  end,
  list_sum_le_20 = function(l)
    local sum = 0
    for i = 1, rawlen(l) do
      sum = Int.add(sum, rawget(l, i))
    end
    return Int.le(sum, 20)
  end,
  bytes_len_lt_5 = function(b)
    return #b < 5
  end,
  bytes_no_zero_byte = function(b)
    return s_find(b, "\0", 1, true) == nil
  end,
}
local INT_PROPS = { "int_le_100", "int_in_range_neg10_50", "int_is_even", "int_abs_le_10", "refinement_x_le_50" }
local LIST_PROPS = { "list_no_consecutive_duplicates", "list_sum_le_20" }
local BYTES_PROPS = { "bytes_len_lt_5", "bytes_no_zero_byte" }

local function set_of(list)
  local s = {}
  for _, v in ipairs(list) do
    s[v] = true
  end
  return s
end

-- The closed field set of each op (besides id and op).
local SHAPES = {
  gen_primitive = { "type", "seed", "count", "outputs" },
  gen_bounded = { "min", "max", "seed", "count", "outputs" },
  gen_bytes = { "min_len", "max_len", "seed", "count", "outputs" },
  shrink_int = { "initial", "target", "min", "max", "property", "expected_minimal", "expected_steps" },
  shrink_refinement = { "initial", "target", "min", "max", "property", "expected_minimal", "expected_steps" },
  shrink_list = { "initial", "elem_target", "property", "expected_minimal", "expected_steps" },
  shrink_bytes = { "initial", "property", "expected_minimal", "expected_steps" },
  replay_consistency = { "seed", "property", "initial_fail", "expected_minimal" },
  filter_exhausted = { "max_retries", "verdict", "failure_id" },
  failure_id_valid = { "input", "valid" },
  corpus_reject = { "text" },
}

local function sorted_op_names()
  local names = {}
  for k in pairs(SHAPES) do
    names[#names + 1] = k
  end
  t_sort(names)
  return names
end

local function int_of(value, what)
  if not Int.is(value) then
    Values.fail(what .. " must be an int")
  end
  return value
end

-- An int that drives a loop or an allocation: 0 <= n <= MAX_SIZE.
local function size_of(value, what)
  local n = int_of(value, what)
  if m_type(n) ~= "integer" or n < 0 or n > Values.MAX_SIZE then
    Values.fail(what .. " outside [0, " .. Values.MAX_SIZE .. "]")
  end
  return n
end

local function text_of(value, what)
  if not Canon.is_text(value) then
    Values.fail(what .. " must be a string")
  end
  return Canon.text_value(value)
end

local function prop_of(name, allowed, op)
  local text = Canon.is_text(name) and Canon.text_value(name) or nil
  if text == nil or not set_of(allowed)[text] then
    Values.fail("unknown property " .. show(name) .. " for " .. op)
  end
  return PROPERTIES[text]
end

local function check_gen(cid, op, v)
  local seed = int_of(v.seed, "seed")
  local count = size_of(v.count, "count")
  -- The expected outputs are in hand already: a case whose list length is not count
  -- cannot conform, so refuse it before generating anything.
  if type(v.outputs) ~= "table" or Canon.is_text(v.outputs) or Int.is_big(v.outputs) or rawlen(v.outputs) ~= count then
    Values.fail("outputs must be a list of exactly count (" .. count .. ") values")
  end
  local got
  if op == "gen_primitive" then
    got = Gens.gen_primitive(seed, text_of(v.type, "type"), count)
  elseif op == "gen_bounded" then
    got = Gens.gen_bounded(seed, int_of(v.min, "min"), int_of(v.max, "max"), count)
  else
    local min_len = size_of(v.min_len, "min_len")
    local max_len = size_of(v.max_len, "max_len")
    if count * max_len > Corpus.MAX_GEN_BYTES then
      Values.fail("count " .. count .. " x max_len " .. max_len .. " exceeds the generation budget of " .. Corpus.MAX_GEN_BYTES .. " bytes")
    end
    got = Gens.gen_bytes(seed, min_len, max_len, count)
  end
  if same(got, v.outputs) then
    return {}
  end
  return { cid .. ": " .. op .. " outputs mismatch" }
end

local function check_shrink_int(cid, op, v)
  local prop = prop_of(v.property, INT_PROPS, op)
  local initial = int_of(v.initial, "initial")
  local target = int_of(v.target, "target")
  local lo = int_of(v.min, "min")
  local hi = int_of(v.max, "max")
  local minimal, steps = Shrink.shrink_int(initial, target, lo, hi, prop, 1000)
  local problems = {}
  if not same(minimal, v.expected_minimal) then
    problems[#problems + 1] = s_format("%s: %s minimal mismatch: got %s, corpus %s", cid, op, show(minimal), show(v.expected_minimal))
  end
  if not same(steps, v.expected_steps) then
    problems[#problems + 1] = s_format("%s: %s step count mismatch: got %s, corpus %s", cid, op, show(steps), show(v.expected_steps))
  end
  -- Engine invariants, independent of the corpus's expected values.
  if prop(minimal) == true then
    problems[#problems + 1] = s_format("%s: %s minimal %s does not falsify the property", cid, op, show(minimal))
  end
  if not (Int.le(lo, minimal) and Int.le(minimal, hi)) then
    problems[#problems + 1] = s_format("%s: %s minimal %s left [%s, %s]", cid, op, show(minimal), Int.tostring(lo), Int.tostring(hi))
  end
  return problems
end

local function check_shrink_list(cid, v)
  local prop = prop_of(v.property, LIST_PROPS, "shrink_list")
  if type(v.initial) ~= "table" or Canon.is_text(v.initial) or Int.is_big(v.initial) then
    Values.fail("initial must be a list of ints")
  end
  local initial = {}
  for i = 1, rawlen(v.initial) do
    initial[i] = int_of(rawget(v.initial, i), "element")
  end
  local elem_target = int_of(v.elem_target, "elem_target")
  local minimal, steps = Shrink.shrink_list(initial, elem_target, prop, 1000)
  local problems = {}
  if not same(minimal, v.expected_minimal) then
    problems[#problems + 1] = s_format("%s: shrink_list minimal mismatch: got %s, corpus %s", cid, show(minimal), show(v.expected_minimal))
  end
  if not same(steps, v.expected_steps) then
    problems[#problems + 1] = s_format("%s: shrink_list steps mismatch: got %s, corpus %s", cid, show(steps), show(v.expected_steps))
  end
  if prop(minimal) == true then
    problems[#problems + 1] = s_format("%s: shrink_list minimal %s does not falsify the property", cid, show(minimal))
  end
  return problems
end

local function check_shrink_bytes(cid, v)
  local prop = prop_of(v.property, BYTES_PROPS, "shrink_bytes")
  if type(v.initial) ~= "string" then
    Values.fail("initial must be bytes")
  end
  local minimal, steps = Shrink.shrink_bytes(v.initial, prop, 1000)
  local problems = {}
  if not same(minimal, v.expected_minimal) then
    problems[#problems + 1] = s_format("%s: shrink_bytes minimal mismatch: got %s, corpus %s", cid, show(minimal), show(v.expected_minimal))
  end
  if not same(steps, v.expected_steps) then
    problems[#problems + 1] = s_format("%s: shrink_bytes steps mismatch: got %s, corpus %s", cid, show(steps), show(v.expected_steps))
  end
  if prop(minimal) == true then
    problems[#problems + 1] = s_format("%s: shrink_bytes minimal %s does not falsify the property", cid, show(minimal))
  end
  return problems
end

local function check_replay(cid, v)
  local seed = int_of(v.seed, "seed")
  local prop = prop_of(v.property, INT_PROPS, "replay_consistency")
  local initial_fail, minimal = Shrink.simulate_replay(seed, prop)
  local problems = {}
  if not same(initial_fail, v.initial_fail) then
    problems[#problems + 1] = s_format("%s: replay initial_fail mismatch: got %s, corpus %s", cid, show(initial_fail), show(v.initial_fail))
  end
  if not same(minimal, v.expected_minimal) then
    problems[#problems + 1] = s_format("%s: replay minimal mismatch: got %s, corpus %s", cid, show(minimal), show(v.expected_minimal))
  end
  return problems
end

local function check_filter_exhausted(cid, v)
  local max_retries = size_of(v.max_retries, "max_retries")
  if max_retries < 1 then
    Values.fail("max_retries must be at least 1")
  end
  local calls = 0
  local gen = Gens.filter(Gens.integer(0, 10), function()
    calls = calls + 1
    return false
  end, max_retries)
  local verdict = Runner.check(0, gen, function()
    return true
  end, Runner.Options.new({ num_tests = 1, max_shrink_steps = 1000, ci = "on" }))
  local problems = {}
  if not (same(v.verdict, Canon.text("fail")) and not verdict.passed) then
    problems[#problems + 1] = cid .. ": an unsatisfiable filter must be a fail verdict"
  end
  local got_id = verdict.failure_id and Canon.text(verdict.failure_id) or nil
  if not same(got_id, v.failure_id) then
    problems[#problems + 1] = s_format("%s: failure id %s, corpus %s", cid, show(got_id), show(v.failure_id))
  end
  if calls ~= max_retries then
    problems[#problems + 1] = s_format("%s: filter sampled %d times, expected %d", cid, calls, max_retries)
  end
  return problems
end

-- Decodes and checks one case.  Any error (a malformed case or an engine that
-- raised) is reported as a problem.
local function evaluate_case(cid, op, raw)
  local v = {}
  for _, k in ipairs(raw.keys) do
    if k ~= "id" and k ~= "op" then
      v[k] = Values.decode(raw.map[k])
    end
  end
  local found, candidate, registered = {}, "", false
  if op == "gen_primitive" or op == "gen_bounded" or op == "gen_bytes" then
    found = check_gen(cid, op, v)
  elseif op == "shrink_int" or op == "shrink_refinement" then
    found = check_shrink_int(cid, op, v)
  elseif op == "shrink_list" then
    found = check_shrink_list(cid, v)
  elseif op == "shrink_bytes" then
    found = check_shrink_bytes(cid, v)
  elseif op == "replay_consistency" then
    found = check_replay(cid, v)
  elseif op == "filter_exhausted" then
    found = check_filter_exhausted(cid, v)
  elseif op == "failure_id_valid" then
    local input = Canon.is_text(v.input) and Canon.text_value(v.input) or nil
    local got = input ~= nil and FailureIds.is_failure_id(input)
    if not same(got, v.valid) then
      found[#found + 1] = s_format("%s: failure id membership disagrees for %s", cid, show(v.input))
    end
    candidate = input or ""
    registered = got
  else -- corpus_reject
    local text = text_of(v.text, "text")
    if Tfv.valid(text) then
      found[#found + 1] = cid .. ": corpus_reject text was accepted by the strict parser"
    end
  end
  return found, candidate, registered
end

local function safe_evaluate_case(cid, op, raw)
  local ok, found, candidate, registered = pcall(evaluate_case, cid, op, raw)
  if ok then
    return found, true, candidate, registered
  end
  return { s_format("%s: %s could not be evaluated: %s", cid, op, Values.error_message(found)) }, false, "", false
end

local function sorted_keys(list)
  local out = {}
  for i, k in ipairs(list) do
    out[i] = k
  end
  t_sort(out)
  return out
end

local function same_key_set(have, want)
  have, want = sorted_keys(have), sorted_keys(want)
  if #have ~= #want then
    return false
  end
  for i = 1, #have do
    if have[i] ~= want[i] then
      return false
    end
  end
  return true
end

--- Evaluates every case; returns `{evaluated = n, problems = {...}}`.  `cases` is the
-- decoded case list (an array of JSON nodes).  A nil `expected` skips the
-- case-count check (ad-hoc case lists in tests); the default is 25.
function Corpus.evaluate(cases_in, expected)
  if expected == nil then
    expected = Corpus.PBTV1_CASE_COUNT
  elseif expected == false then
    expected = nil
  end
  local problems = {}
  local evaluated = 0
  local cases = cases_in
  if expected and #cases ~= expected then
    problems[#problems + 1] = s_format("pbt-v1: expected %d cases, got %d", expected, #cases)
  end
  if #cases > Corpus.MAX_CASES then
    problems[#problems + 1] = s_format("pbt-v1: %d cases exceeds the cap of %d", #cases, Corpus.MAX_CASES)
    local cut = {}
    for i = 1, Corpus.MAX_CASES do
      cut[i] = cases[i]
    end
    cases = cut
  end

  local seen_ids = {}
  for _, raw in ipairs(cases) do
    local id = type(raw) == "table" and raw.kind == "object" and type(raw.map.id) == "string" and raw.map.id or nil
    if id == nil or id == "" or seen_ids[id] then
      problems[#problems + 1] = "duplicate or missing case id"
      break
    end
    seen_ids[id] = true
  end

  local ops_seen, registered_seen, rejected_id_seen = {}, {}, false

  for _, raw in ipairs(cases) do
    if type(raw) ~= "table" or raw.kind ~= "object" then
      problems[#problems + 1] = "case is not an object: " .. show(raw)
    elseif type(raw.map.id) ~= "string" or raw.map.id == "" then
      problems[#problems + 1] = "case missing a string id"
    else
      local cid = Canon.escape(Canon.clip(raw.map.id, 100))
      local op = type(raw.map.op) == "string" and raw.map.op or ""
      if not SHAPES[op] then
        problems[#problems + 1] = cid .. ": unknown op " .. show(raw.map.op)
      else
        local want = { "id", "op" }
        for _, f in ipairs(SHAPES[op]) do
          want[#want + 1] = f
        end
        if not same_key_set(raw.keys, want) then
          problems[#problems + 1] = s_format("%s: fields do not match %s", cid, t_concat(sorted_keys(want), ","))
        else
          local found, ran_ok, candidate, registered = safe_evaluate_case(cid, op, raw)
          for _, p in ipairs(found) do
            problems[#problems + 1] = p
          end
          if ran_ok then
            if op == "failure_id_valid" then
              if registered then
                registered_seen[candidate] = true
              else
                rejected_id_seen = true
              end
            end
            ops_seen[op] = true
            evaluated = evaluated + 1
          end
        end
      end
    end
  end

  local missing_ops = {}
  for _, name in ipairs(sorted_op_names()) do
    if not ops_seen[name] then
      missing_ops[#missing_ops + 1] = name
    end
  end
  if #missing_ops > 0 then
    problems[#problems + 1] = "pbt-v1: operations never exercised: " .. t_concat(missing_ops, ",")
  end
  local missing_ids = {}
  for _, id in ipairs(FailureIds.ALL) do
    if not registered_seen[id] then
      missing_ids[#missing_ids + 1] = id
    end
  end
  if #missing_ids > 0 then
    problems[#problems + 1] = "pbt-v1: registered failure ids missing: " .. t_concat(missing_ids, ",")
  end
  if not rejected_id_seen then
    problems[#problems + 1] = "pbt-v1: failure_id_valid needs an unregistered-id rejection"
  end

  local out = {}
  for i, p in ipairs(problems) do
    if i > Corpus.MAX_PROBLEMS then
      break
    end
    out[i] = Canon.clip(p, Values.MAX_PROBLEM_LEN)
  end
  return { evaluated = evaluated, problems = out }
end

--- Parses `data` strictly and evaluates it.  Returns the report; a document that
-- fails to parse is one problem.
function Corpus.evaluate_document(data)
  local cases, err = Corpus.parse(data)
  if cases == nil then
    return { evaluated = 0, problems = { "pbt-v1: " .. err } }
  end
  return Corpus.evaluate(cases)
end

return Corpus
