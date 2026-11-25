--- The production runner for the shared fuzz-v1 corpus
-- (`code/specs/fixtures/fuzz-v1/cases.json`, VER06 section 8, VER06p section 8).
--
-- It takes the raw JSON, strictly decodes it, evaluates every case with THIS
-- package's own public engine functions and returns human-readable problems.  An
-- empty list means "conforms".  It does no file I/O (the package is
-- capability-free); the tests read the committed fixture.
--
-- A runner that cannot report a problem is worse than none, so this one is strict:
--
--   * a case has exactly the fields of its op, in canonical encodings;
--   * an unknown op or a malformed value is a PROBLEM, never a silent skip;
--   * every op appears, a failure id is accepted and one rejected, and the case count
--     equals FUZZV1_CASE_COUNT, so deleting cases is itself a problem;
--   * fixture-controlled sizes are capped (CWE-400): a hostile `steps = 10^9` is a
--     reported problem, not a hang;
--   * every problem about a case starts with that case's id, ESCAPED (an id comes from
--     the fixture and may hold a newline).

-- Standard functions are captured at load so that a target that patches the string
-- metatable or a library table cannot make the engine raise (VER06p section 1).
local m_type, s_find, s_format, s_gsub, s_byte, t_concat, t_sort = math.type, string.find, string.format, string.gsub, string.byte, table.concat, table.sort
local type, pcall, rawget, rawlen, ipairs, pairs, tostring = type, pcall, rawget, rawlen, ipairs, pairs, tostring

local Int = require("coding_adventures.test_bootstrap.int")
local Canon = require("coding_adventures.test_bootstrap.canon")
local StrictJson = require("coding_adventures.test_bootstrap.strict_json")
local Values = require("coding_adventures.test_bootstrap.values")
local Limits = require("coding_adventures.test_bootstrap.fuzz.limits")
local FailureIds = require("coding_adventures.test_bootstrap.fuzz.failure_ids")
local Hooks = require("coding_adventures.test_bootstrap.fuzz.hooks")

local Corpus = {}

-- The hooks of the run in progress: copied from `Hooks` when `evaluate` starts, so code that
-- changes `Hooks` while a run is going cannot change that run.
local HK = Hooks

local function snapshot_hooks()
  local copy = {}
  for name in pairs(Hooks.DEFAULTS) do
    copy[name] = Hooks[name]
  end
  return copy
end

Corpus.FUZZV1_CASE_COUNT = 25
Corpus.MAX_CASES = 10000
Corpus.MAX_PROBLEMS = 1000

local TWO64 = Int.pow2(64)
local show = Values.show

--- Strictly decodes a fuzz-v1 document into its case list: exactly
-- {"schema_version": 1, "cases": [...]}.  Cases are returned undecoded.  Returns
-- `cases` or `nil, message`.
function Corpus.parse(data)
  local root, err = StrictJson.parse(data)
  if root == nil then
    return nil, err
  end
  if type(root) ~= "table" or root.kind ~= "object" or #root.keys ~= 2 then
    return nil, 'fuzz corpus: want exactly {"schema_version": 1, "cases": [...]}'
  end
  local version = root.map.schema_version
  if type(version) ~= "table" or version.kind ~= "number" or version.text ~= "1" then
    return nil, "fuzz corpus: schema_version must be 1"
  end
  local cases = root.map.cases
  if type(cases) ~= "table" or cases.kind ~= "array" then
    return nil, 'fuzz corpus: "cases" must be an array'
  end
  return cases
end

-- The closed field set of each op (besides id and op).
local SHAPES = {
  mutate_bit_flip = { "input", "bit_offset", "count", "output" },
  mutate_byte_flip = { "input", "byte_offset", "count", "output" },
  mutate_arithmetic = { "input", "byte_offset", "delta", "width", "endian", "output" },
  mutate_boundary = { "input", "byte_offset", "width", "val", "endian", "output" },
  mutate_block_delete = { "input", "offset", "length", "output" },
  mutate_block_insert = { "input", "offset", "insert_bytes", "output" },
  mutate_block_duplicate = { "input", "src_offset", "length", "dst_offset", "output" },
  mutate_splice = { "target", "donor", "target_split", "donor_split", "output" },
  mutate_sequence = { "input", "seed", "steps", "output" },
  corpus_admit = { "existing_edges", "trace_edges", "admitted", "new_edges" },
  crash_dedup = { "crash_a_kind", "crash_a_frames", "crash_b_kind", "crash_b_frames", "is_duplicate" },
  failure_id_valid = { "input", "valid" },
  private_reporting = { "target", "failure_id", "raw_payload", "sig_hash", "tap_line", "contains_raw_payload" },
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

-- -- typed field access -----------------------------------------------------

local function bytes_of(v, what)
  if type(v) ~= "string" then
    Values.fail(what .. " must be bytes")
  end
  return v
end

local function text_of(v, what)
  if not Canon.is_text(v) then
    Values.fail(what .. " must be a string")
  end
  return Canon.text_value(v)
end

local function bool_of(v, what)
  if type(v) ~= "boolean" then
    Values.fail(what .. " must be a bool")
  end
  return v
end

local function int_of(v, what)
  if not Int.is(v) then
    Values.fail(what .. " must be an int")
  end
  return v
end

-- An int that drives a loop or an allocation: 0 <= n <= MAX_SIZE.
local function size_of(v, what)
  local n = int_of(v, what)
  if m_type(n) ~= "integer" or n < 0 or n > Values.MAX_SIZE then
    Values.fail(what .. " outside [0, " .. Values.MAX_SIZE .. "]")
  end
  return n
end

local function texts_of(v, what)
  if type(v) ~= "table" or Canon.is_text(v) or Int.is_big(v) then
    Values.fail(what .. " must be a list of strings")
  end
  local out = {}
  for i = 1, rawlen(v) do
    local item = rawget(v, i)
    if not Canon.is_text(item) then
      Values.fail(what .. " must be a list of strings")
    end
    out[i] = Canon.text_value(item)
  end
  return out
end

-- A list of edge ids, each within [0, 2^64).
local function edges_of(v, what)
  if type(v) ~= "table" or Canon.is_text(v) or Int.is_big(v) then
    Values.fail(what .. " must be a list of ints")
  end
  local out = {}
  for i = 1, rawlen(v) do
    local e = rawget(v, i)
    if not Int.is(e) then
      Values.fail(what .. " must be a list of ints")
    end
    if Int.sign(e) < 0 or Int.cmp(e, TWO64) >= 0 then
      Values.fail(what .. " element " .. show(e) .. " outside [0, 2^64)")
    end
    out[i] = e
  end
  return out
end

local function hex_of(s)
  return (s_gsub(s, ".", function(c)
    return s_format("%02x", s_byte(c))
  end))
end

-- -- per-op checks ----------------------------------------------------------

local function compare(cid, op, got, v)
  local want = v.output
  if type(want) ~= "string" then
    return { cid .. ": " .. op .. " output must be bytes" }
  end
  if got == want then
    return {}
  end
  return {
    s_format("%s: %s expected %s, got %s", cid, op, Canon.clip(hex_of(want), Values.MAX_FRAGMENT_LEN),
      type(got) == "string" and Canon.clip(hex_of(got), Values.MAX_FRAGMENT_LEN) or type(got)),
  }
end

-- Charges the REAL work: the call is handed what is left of the aggregate cap and
-- what it actually copied (inserts and duplicates grow the buffer, so steps x initial
-- length would undercount) is deducted afterwards.  A call that hits its cap is
-- charged too, the aggregate is then ZEROED, and an exhausted aggregate fails every
-- later case at once without running it.
local function sequence_result(v, work)
  local input = bytes_of(v.input, "input")
  local seed = int_of(v.seed, "seed")
  local steps = size_of(v.steps, "steps")
  local exhausted = "mutate_sequence work budget of " .. Limits.MAX_CORPUS_SEQUENCE_WORK .. " byte-steps exhausted"
  if work[1] <= 0 then
    Values.fail(exhausted)
  end
  local data, done = HK.mutate_sequence(input, seed, steps, Limits.MAX_INPUT_BYTES, work[1])
  if data == nil then
    work[1] = 0
    Values.fail(exhausted)
  end
  work[1] = work[1] - done
  return data
end

local function check_mutator(cid, op, v, work)
  local got
  if op == "mutate_bit_flip" then
    got = HK.bit_flip(bytes_of(v.input, "input"), int_of(v.bit_offset, "bit_offset"), int_of(v.count, "count"))
  elseif op == "mutate_byte_flip" then
    got = HK.byte_flip(bytes_of(v.input, "input"), int_of(v.byte_offset, "byte_offset"), int_of(v.count, "count"))
  elseif op == "mutate_arithmetic" then
    got = HK.byte_arithmetic(bytes_of(v.input, "input"), int_of(v.byte_offset, "byte_offset"),
      int_of(v.delta, "delta"), int_of(v.width, "width"), text_of(v.endian, "endian"))
  elseif op == "mutate_boundary" then
    got = HK.boundary_insert(bytes_of(v.input, "input"), int_of(v.byte_offset, "byte_offset"),
      int_of(v.width, "width"), int_of(v.val, "val"), text_of(v.endian, "endian"))
  elseif op == "mutate_block_delete" then
    got = HK.block_delete(bytes_of(v.input, "input"), int_of(v.offset, "offset"), int_of(v.length, "length"))
  elseif op == "mutate_block_insert" then
    got = HK.block_insert(bytes_of(v.input, "input"), int_of(v.offset, "offset"), bytes_of(v.insert_bytes, "insert_bytes"))
  elseif op == "mutate_block_duplicate" then
    got = HK.block_duplicate(bytes_of(v.input, "input"), int_of(v.src_offset, "src_offset"),
      int_of(v.length, "length"), int_of(v.dst_offset, "dst_offset"))
  elseif op == "mutate_splice" then
    got = HK.splice(bytes_of(v.target, "target"), bytes_of(v.donor, "donor"),
      int_of(v.target_split, "target_split"), int_of(v.donor_split, "donor_split"))
  else
    got = sequence_result(v, work)
  end
  local problems = compare(cid, op, got, v)
  if type(got) == "string" then
    if op == "mutate_bit_flip" or op == "mutate_byte_flip" or op == "mutate_arithmetic" or op == "mutate_boundary" then
      if #got ~= #v.input then
        problems[#problems + 1] = cid .. ": " .. op .. " changed the length of its input"
      end
    elseif op == "mutate_splice" then
      local function clamp(x, hi)
        if Int.sign(x) < 0 then
          return 0
        elseif m_type(x) ~= "integer" or x > hi then
          return hi
        end
        return x
      end
      local want = clamp(v.target_split, #v.target) + #v.donor - clamp(v.donor_split, #v.donor)
      if #got ~= want then
        problems[#problems + 1] = cid .. ": splice length is not prefix + suffix"
      end
    end
  end
  return problems
end

local function check_corpus_admit(cid, v)
  local existing = edges_of(v.existing_edges, "existing_edges")
  local trace = edges_of(v.trace_edges, "trace_edges")
  local admitted = bool_of(v.admitted, "admitted")
  local new_edges = int_of(v.new_edges, "new_edges")
  local got_admitted, got_new = HK.corpus_admit(existing, trace)
  local problems = {}
  if got_admitted ~= admitted then
    problems[#problems + 1] = s_format("%s: corpus_admit admitted %s, corpus %s", cid, tostring(got_admitted), tostring(admitted))
  end
  if not Int.is(got_new) or not Int.eq(got_new, new_edges) then
    problems[#problems + 1] = s_format("%s: corpus_admit new_edges %s, corpus %s", cid, show(got_new), show(new_edges))
  end
  -- Invariant: admission is exactly "some trace edge is unseen".
  local have = {}
  for _, e in ipairs(existing) do
    have[Int.tostring(e)] = true
  end
  local any = false
  for _, e in ipairs(trace) do
    if not have[Int.tostring(e)] then
      any = true
    end
  end
  if got_admitted ~= any then
    problems[#problems + 1] = cid .. ": corpus_admit disagrees with set difference"
  end
  return problems
end

local function check_crash_dedup(cid, v)
  local ka = text_of(v.crash_a_kind, "crash_a_kind")
  local fa = texts_of(v.crash_a_frames, "crash_a_frames")
  local kb = text_of(v.crash_b_kind, "crash_b_kind")
  local fb = texts_of(v.crash_b_frames, "crash_b_frames")
  local want = bool_of(v.is_duplicate, "is_duplicate")
  local got = HK.duplicate_crash(ka, fa, kb, fb)
  if got == want then
    return {}
  end
  return { s_format("%s: crash_dedup is_duplicate %s, corpus %s", cid, tostring(got), tostring(want)) }
end

local function check_private_reporting(cid, v)
  local raw = bytes_of(v.raw_payload, "raw_payload")
  local target = text_of(v.target, "target")
  local fid = text_of(v.failure_id, "failure_id")
  local sig = text_of(v.sig_hash, "sig_hash")
  local tap = text_of(v.tap_line, "tap_line")
  local want_leak = bool_of(v.contains_raw_payload, "contains_raw_payload")
  local ok, line = pcall(HK.format_tap, 1, target, fid, sig)
  if not ok then
    Values.fail("format_tap_failure refused: " .. Values.error_message(line))
  end
  local problems = {}
  if line ~= tap then
    problems[#problems + 1] = s_format("%s: tap_line expected %s, got %s", cid,
      Canon.clip(Canon.escape(tap), Values.MAX_FRAGMENT_LEN), Canon.clip(Canon.escape(line), Values.MAX_FRAGMENT_LEN))
  end
  local leaked = raw ~= "" and s_find(line, hex_of(raw), 1, true) ~= nil
  if leaked ~= want_leak then
    problems[#problems + 1] = s_format("%s: privacy verdict wrong, payload leaked: %s", cid, tostring(leaked))
  end
  if s_find(line, "[\r\n]") then
    problems[#problems + 1] = cid .. ": failure line spans more than one line"
  end
  return problems
end

-- Decodes and checks one case.  Any error (a malformed case or an engine that raised)
-- is reported as a problem.
local function evaluate_case(cid, op, raw, work)
  local v = {}
  for _, k in ipairs(raw.keys) do
    if k ~= "id" and k ~= "op" then
      v[k] = Values.decode(raw.map[k])
    end
  end
  local found, accepted = {}, false
  if op == "corpus_admit" then
    found = check_corpus_admit(cid, v)
  elseif op == "crash_dedup" then
    found = check_crash_dedup(cid, v)
  elseif op == "failure_id_valid" then
    local input = Canon.is_text(v.input) and Canon.text_value(v.input) or nil
    accepted = input ~= nil and HK.failure_id(input) == true
    if not Values.same(accepted, v.valid) then
      found[#found + 1] = s_format("%s: failure id membership disagrees for %s", cid, show(v.input))
    end
  elseif op == "private_reporting" then
    found = check_private_reporting(cid, v)
  elseif op == "corpus_reject" then
    local text = text_of(v.text, "text")
    if HK.tfv_valid(text) then
      found[#found + 1] = cid .. ": corpus_reject text was accepted by the strict parser"
    end
  else
    found = check_mutator(cid, op, v, work)
  end
  return found, accepted
end

local function safe_evaluate_case(cid, op, raw, work)
  local ok, found, accepted = pcall(evaluate_case, cid, op, raw, work)
  if ok then
    return found, true, accepted
  end
  return { s_format("%s: %s could not be evaluated: %s", cid, op, Values.error_message(found)) }, false, false
end

--- Evaluates every case; returns `{evaluated = n, problems = {...}}`.  `cases` is the
-- decoded case list (an array of JSON nodes).  A `false` `expected` skips the
-- case-count check (ad-hoc case lists in tests); the default is 25.
function Corpus.evaluate(cases_in, expected)
  if expected == nil then
    expected = Corpus.FUZZV1_CASE_COUNT
  elseif expected == false then
    expected = nil
  end
  HK = snapshot_hooks()
  local problems = {}
  local evaluated = 0
  local cases = cases_in
  if expected and #cases ~= expected then
    problems[#problems + 1] = s_format("fuzz-v1: expected %d cases, got %d", expected, #cases)
  end
  if #cases > Corpus.MAX_CASES then
    problems[#problems + 1] = s_format("fuzz-v1: %d cases exceeds the cap of %d", #cases, Corpus.MAX_CASES)
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

  -- the aggregate mutate_sequence work of one corpus, in bytes ACTUALLY copied
  local work = { Limits.MAX_CORPUS_SEQUENCE_WORK }
  local ops_seen, accepted_seen, rejected_seen = {}, false, false

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
          local found, ran_ok, accepted = safe_evaluate_case(cid, op, raw, work)
          for _, p in ipairs(found) do
            problems[#problems + 1] = p
          end
          if ran_ok then
            if op == "failure_id_valid" then
              if accepted then
                accepted_seen = true
              else
                rejected_seen = true
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
    problems[#problems + 1] = "fuzz-v1: operations never exercised: " .. t_concat(missing_ops, ",")
  end
  if not accepted_seen then
    problems[#problems + 1] = "fuzz-v1: failure_id_valid needs a registered-id acceptance"
  end
  if not rejected_seen then
    problems[#problems + 1] = "fuzz-v1: failure_id_valid needs an unregistered-id rejection"
  end
  local unregistered = {}
  for _, id in ipairs(FailureIds.ALL) do
    if not HK.failure_id(id) then
      unregistered[#unregistered + 1] = id
    end
  end
  if #unregistered > 0 then
    problems[#problems + 1] = "fuzz-v1: engine rejects registered ids " .. t_concat(unregistered, ",")
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

--- Parses `data` strictly and evaluates it.  Returns the report; a document that fails
-- to parse is one problem.
function Corpus.evaluate_document(data)
  local cases, err = Corpus.parse(data)
  if cases == nil then
    return { evaluated = 0, problems = { "fuzz-v1: " .. err } }
  end
  return Corpus.evaluate(cases)
end

return Corpus
