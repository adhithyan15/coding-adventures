--- Bounds that keep every loop and allocation finite (CWE-400).  A caller or a
-- fixture can ask for more only by being refused.  (VER05o section 3, VER05h.)

-- Standard functions are captured at load so that a property that patches the
-- string metatable or a library table cannot make the engine raise (VER05o 4).
local m_type, s_format = math.type, string.format
local type, error = type, error

local Int = require("coding_adventures.test_bootstrap.int")

local Limits = {
  MAX_CASES = 100000,
  MAX_SHRINK_STEPS = 10000,
  MAX_INPUT_BYTES = 1 << 20,
  MAX_LIST_LEN = 100000,
  -- Most values any corpus entry point will generate.
  MAX_GEN_COUNT = 1000000,
  -- Attempt budget of one shrink = ATTEMPT_FACTOR * max_steps + ATTEMPT_SLACK:
  -- a rejected attempt copies its candidate but is not an accepted step.
  ATTEMPT_FACTOR = 16,
  ATTEMPT_SLACK = 64,
  -- count x width bound for gen_bytes / gen_bounded.
  MAX_GEN_BYTES_TOTAL = 1 << 20,
  MAX_FREQUENCY_TOTAL = 1 << 32,
  -- Most elements one whole shrink may copy, summed over every attempt of every
  -- nested shrinker (an attempt copies its candidate).
  MAX_SHRINK_WORK = 64 * 1024 * 1024,
  MAX_RETRIES = 10000,
  DEFAULT_MAX_RETRIES = 100,
  MAX_TUPLE_ARITY = 64,
  -- A generator span wider than this many bits is refused (k-word rejection
  -- sampling is linear in it).
  MAX_SPAN_BITS = 16384,
  -- Most digits of a fixture integer.
  MAX_INT_DIGITS = 1000,
}

function Limits.attempt_budget(max_steps)
  return Limits.ATTEMPT_FACTOR * max_steps + Limits.ATTEMPT_SLACK
end

--- `value` must be an Int inside [lo, hi] (native bounds); `name` is for the message.
function Limits.check_range(value, lo, hi, name)
  if m_type(value) ~= "integer" then
    if Int.is(value) then
      error(s_format("%s outside [%d, %d]", name, lo, hi), 3)
    end
    local what = m_type(value) == "float" and "a float" or type(value)
    error(s_format("%s must be an Integer, not %s", name, what), 3)
  end
  if value < lo or value > hi then
    error(s_format("%s %d outside [%d, %d]", name, value, lo, hi), 3)
  end
  return value
end

function Limits.check_budget(max_steps)
  return Limits.check_range(max_steps, 0, Limits.MAX_SHRINK_STEPS, "max_steps")
end

function Limits.check_count(count)
  return Limits.check_range(count, 0, Limits.MAX_GEN_COUNT, "count")
end

return Limits
