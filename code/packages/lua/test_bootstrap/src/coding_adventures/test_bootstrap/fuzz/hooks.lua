--- Sabotage hooks: the engine entry points the corpus runner calls, as swappable
-- functions.  These are the only production indirections added for the negative
-- controls (VER06p section 9): a test installs a broken version with `Hooks.with`,
-- runs the corpus, and the runner must report problems.  Production code never
-- writes them; the table is a documented test seam, restored on every exit
-- (including an error).

-- Standard functions are captured at load so that a target that patches the string
-- metatable or a library table cannot make the engine raise (VER06p section 1).
local t_pack, t_unpack = table.pack, table.unpack
local type, pcall, error, pairs, next = type, pcall, error, pairs, next

local Mutate = require("coding_adventures.test_bootstrap.fuzz.mutate")
local Bitmap = require("coding_adventures.test_bootstrap.fuzz.bitmap")
local Signature = require("coding_adventures.test_bootstrap.fuzz.signature")
local FailureIds = require("coding_adventures.test_bootstrap.fuzz.failure_ids")
local Tfv = require("coding_adventures.test_bootstrap.tfv")

local Hooks = {}

local DEFAULTS = {
  bit_flip = Mutate.bit_flip,
  byte_flip = Mutate.byte_flip,
  byte_arithmetic = Mutate.byte_arithmetic,
  boundary_insert = Mutate.boundary_insert,
  block_delete = Mutate.block_delete,
  block_insert = Mutate.block_insert,
  block_duplicate = Mutate.block_duplicate,
  splice = Mutate.splice,
  mutate_sequence = Mutate.mutate_sequence_counted,
  corpus_admit = Bitmap.corpus_admit,
  duplicate_crash = Signature.duplicate_crash,
  failure_id = FailureIds.is_fuzz_failure_id,
  format_tap = Signature.format_tap_failure,
  tfv_valid = Tfv.valid,
}

Hooks.DEFAULTS = DEFAULTS

function Hooks.reset()
  for name, fn in pairs(DEFAULTS) do
    Hooks[name] = fn
  end
end

--- Runs `fn(...)` with the given overrides installed, restoring every hook on every
-- exit.  Returns what `fn` returns; rethrows what it raises.
function Hooks.with(overrides, fn, ...)
  local saved = {}
  for name in pairs(DEFAULTS) do
    saved[name] = Hooks[name]
  end
  for name, replacement in next, overrides do
    if DEFAULTS[name] == nil then
      error("unknown hook (" .. type(name) .. ")", 2)
    end
    Hooks[name] = replacement
  end
  local packed = t_pack(pcall(fn, ...))
  for name in pairs(DEFAULTS) do
    Hooks[name] = saved[name]
  end
  if not packed[1] then
    error(packed[2], 0)
  end
  return t_unpack(packed, 2, packed.n)
end

Hooks.reset()

return Hooks
