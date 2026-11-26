-- The 820 differential vectors of the Go lane, computed by the independent Python
-- oracle `scripts/fuzz_conformance.py` (generator:
-- `go/test-bootstrap/fuzz/testdata/gen_reference_vectors.py`).  The Go table is a
-- committed source file, so it is READ from its committed path: there is no copy
-- (VER06p section 8).
package.path = "../src/?.lua;../src/?/init.lua;" .. package.path
local H = require("helper")
local Mutate = require("coding_adventures.test_bootstrap.fuzz.mutate")
local Int = H.tb.Int

local OPS = { "bit_flip", "byte_flip", "arithmetic", "boundary", "block_delete", "block_insert", "block_duplicate", "splice", "sequence" }

local function unhex(s)
  return (s:gsub("..", function(h) return string.char(tonumber(h, 16)) end))
end

local function hex(s)
  return (s:gsub(".", function(c) return string.format("%02x", c:byte()) end))
end

local function load_vectors()
  local path = H.repo_root() .. "/code/packages/go/test-bootstrap/fuzz/reference_vectors_test.go"
  local rows, started = {}, false
  for line in io.lines(path) do
    if started then
      if line == "}" then break end
      local op, args, out = line:match('^\t{"([a-z_]+)", %[%]string{(.*)}, "([0-9a-f]*)"},$')
      assert(op, "unparseable vector row: " .. line)
      local a = {}
      for item in args:gmatch('"([^"]*)"') do a[#a + 1] = item end
      rows[#rows + 1] = { op, a, out }
    elseif line == "var referenceVectors = []refVector{" then
      started = true
    end
  end
  assert(started, "the vector table moved")
  return rows
end

local function run(op, a)
  local i = Int.parse
  if op == "bit_flip" then return Mutate.bit_flip(unhex(a[1]), i(a[2]), i(a[3]))
  elseif op == "byte_flip" then return Mutate.byte_flip(unhex(a[1]), i(a[2]), i(a[3]))
  elseif op == "arithmetic" then return Mutate.byte_arithmetic(unhex(a[1]), i(a[2]), i(a[3]), i(a[4]), a[5])
  elseif op == "boundary" then return Mutate.boundary_insert(unhex(a[1]), i(a[2]), i(a[3]), i(a[4]), a[5])
  elseif op == "block_delete" then return Mutate.block_delete(unhex(a[1]), i(a[2]), i(a[3]))
  elseif op == "block_insert" then return Mutate.block_insert(unhex(a[1]), i(a[2]), unhex(a[3]))
  elseif op == "block_duplicate" then return Mutate.block_duplicate(unhex(a[1]), i(a[2]), i(a[3]), i(a[4]))
  elseif op == "splice" then return Mutate.splice(unhex(a[1]), unhex(a[2]), i(a[3]), i(a[4]))
  elseif op == "sequence" then return Mutate.mutate_sequence(unhex(a[1]), i(a[2]), i(a[3]), i(a[4]))
  end
  error("unknown vector op " .. op)
end

describe("the Go lane's reference vectors", function()
  it("all 820 vectors match", function()
    local vectors = load_vectors()
    assert.are.equal(820, #vectors)
    local counts, bad = {}, {}
    for idx, v in ipairs(vectors) do
      local op, a, out = v[1], v[2], v[3]
      counts[op] = (counts[op] or 0) + 1
      local ok, got = pcall(run, op, a)
      if not ok or hex(got) ~= out then
        bad[#bad + 1] = string.format("vector %d %s %s: want %s got %s", idx, op, table.concat(a, ","), out, tostring(ok and hex(got) or got))
        if #bad >= 5 then break end
      end
    end
    assert.are.same({}, bad)
    for _, op in ipairs(OPS) do
      assert.is_true((counts[op] or 0) >= 50, "only " .. tostring(counts[op]) .. " vectors for " .. op)
    end
    print("vectors: " .. #vectors .. "/" .. #vectors)
  end)
end)
