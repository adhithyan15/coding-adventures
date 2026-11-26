--- Sabotage hooks: the only production indirections added for the negative
-- controls (VER05o section 7).  A test swaps one with `Hooks.with`, runs the
-- corpus, and the runner must report problems.  Production code never writes
-- them.  This table is the ONLY module-level mutable state of the package; it is
-- a documented test seam, restored on every exit (including an error).

-- Standard functions are captured at load so that a property that patches the
-- string metatable or a library table cannot make the engine raise (VER05o 4).
local t_pack, t_unpack = table.pack, table.unpack
local type, pcall, error, pairs, next = type, pcall, error, pairs, next

local Int = require("coding_adventures.test_bootstrap.int")

local Hooks = {}

-- floor(diff / 2) on negatives: `//` floors, `>>` would not.
function Hooks.default_bisect(current, diff)
  return Int.sub(current, Int.floor_half(diff))
end

function Hooks.default_unit(current, diff)
  return Int.sub(current, Int.sign(diff))
end

local KNOWN = {
  bisect = true,
  unit = true,
  skip_deletion = true,
  new_rng = true,
  threshold = true,
  default_max_work = true,
}

function Hooks.reset()
  Hooks.bisect = Hooks.default_bisect
  Hooks.unit = Hooks.default_unit
  Hooks.skip_deletion = false
  -- `new_rng(seed_word) -> rng` replaces the generator factory; nil = SplitMix64
  Hooks.new_rng = nil
  -- `threshold(n_word) -> word` replaces the rejection threshold; nil = (2^64-n) mod n
  Hooks.threshold = nil
  -- nil = Limits.MAX_SHRINK_WORK; read when a root scope is created
  Hooks.default_max_work = nil
end

--- Runs `fn(...)` with the given overrides installed, restoring every hook on
-- every exit.  Returns what `fn` returns; rethrows what it raises.
function Hooks.with(overrides, fn, ...)
  local saved = {}
  for k in pairs(KNOWN) do
    saved[k] = Hooks[k]
  end
  for k, v in next, overrides do
    if not KNOWN[k] then
      error("unknown hook (" .. type(k) .. ")", 2)
    end
    Hooks[k] = v
  end
  local packed = t_pack(pcall(fn, ...))
  for k in pairs(KNOWN) do
    Hooks[k] = saved[k]
  end
  if not packed[1] then
    error(packed[2], 0)
  end
  return t_unpack(packed, 2, packed.n)
end

Hooks.reset()

return Hooks
