--- The closed failure vocabulary (VER06 section 6.2).  Nothing else is ever printed
-- in a public failure line.

-- Standard functions are captured at load so that a target that patches the string
-- metatable or a library table cannot make the engine raise (VER06p section 1).
local type, ipairs = type, ipairs

local FailureIds = {
  CRASH = "fuzz-crash",
  TIMEOUT = "fuzz-timeout",
  OOM = "fuzz-oom",
  LEAK = "fuzz-leak",
  PANIC = "fuzz-panic",
}
FailureIds.ALL = { FailureIds.CRASH, FailureIds.TIMEOUT, FailureIds.OOM, FailureIds.LEAK, FailureIds.PANIC }

--- True only for a string equal to one of the five registered ids.
--
--   is_fuzz_failure_id("fuzz-crash")           --> true
--   is_fuzz_failure_id("fuzz-arbitrary-error") --> false
--   is_fuzz_failure_id(nil)                    --> false (not a string at all)
function FailureIds.is_fuzz_failure_id(candidate)
  if type(candidate) ~= "string" then
    return false
  end
  for _, id in ipairs(FailureIds.ALL) do
    if candidate == id then
      return true
    end
  end
  return false
end

return FailureIds
