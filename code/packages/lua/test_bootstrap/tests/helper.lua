-- Shared test helpers (not a test file: busted only runs files matching test_).
package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local tb = require("coding_adventures.test_bootstrap")

local H = { tb = tb }

--- Runs `fn(...)` with an instruction-count hook that raises after `limit`
-- instructions, so a runaway property FAILS a test instead of hanging the run
-- (hazard 8).  The hook is removed on every exit.
function H.limited(limit, fn, ...)
  -- luacov attributes a line to the frame two levels up from its hook, so chaining it
  -- under another hook would misattribute every line: when measuring coverage
  -- (LUA_COVERAGE=1) the bound is skipped (use `hard_limited` where code really loops).
  if os.getenv("LUA_COVERAGE") then
    return fn(...)
  end
  return H.hard_limited(limit, fn, ...)
end

--- Like `limited`, but the bound is always installed.
function H.hard_limited(limit, fn, ...)
  local prev, prev_mask, prev_count = debug.gethook()
  local function hook(event, line)
    if event == "count" then
      error("instruction limit exceeded: the code under test did not terminate", 2)
    elseif prev then
      prev(event, line) -- keep a line-coverage hook working underneath
    end
  end
  debug.sethook(hook, prev_mask or "", limit)
  local packed = table.pack(pcall(fn, ...))
  if prev then
    debug.sethook(prev, prev_mask, prev_count)
  else
    debug.sethook()
  end
  if not packed[1] then
    error(packed[2], 0)
  end
  return table.unpack(packed, 2, packed.n)
end

--- The repository root: walk up from the working directory until the fixture exists.
function H.repo_root()
  local dir = "."
  for _ = 1, 12 do
    local f = io.open(dir .. "/code/specs/fixtures/pbt-v1/cases.json", "rb")
    if f then
      f:close()
      return dir
    end
    dir = dir .. "/.."
  end
  error("could not find the repository root from " .. tostring(os.getenv("PWD")))
end

function H.read_file(path)
  local f = assert(io.open(path, "rb"))
  local data = f:read("a")
  f:close()
  return data
end

--- Decimal text -> Int.
function H.int(text)
  return tb.Int.parse(text)
end

function H.dec(x)
  return tb.Int.tostring(x)
end

--- A deep copy of a decoded JSON node tree.
function H.deep_copy(node)
  if type(node) ~= "table" then
    return node
  end
  local out = {}
  for k, v in pairs(node) do
    out[k] = H.deep_copy(v)
  end
  return out
end

--- JSON node constructors, to build corpus cases by hand.
function H.jint(text)
  return { kind = "object", keys = { "kind", "value" }, map = { kind = "int", value = text } }
end

function H.jbytes(hex)
  return { kind = "object", keys = { "kind", "hex" }, map = { kind = "bytes", hex = hex } }
end

--- A metatable whose every metamethod records its name and raises: installing it on
-- a user value proves the engine never triggers one.
function H.trap(log)
  local mt = {}
  for _, name in ipairs({
    "__index", "__newindex", "__len", "__eq", "__lt", "__le", "__concat", "__call", "__tostring",
    "__pairs", "__unm", "__add", "__sub", "__mul", "__div", "__mod", "__idiv",
  }) do
    mt[name] = function()
      log[#log + 1] = name
      error("trap " .. name)
    end
  end
  return mt
end

return H
