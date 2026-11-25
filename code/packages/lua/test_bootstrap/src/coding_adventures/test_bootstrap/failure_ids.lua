--- The closed failure vocabulary (VER05 5.1).  `property-timeout` is registered
-- but never produced: there is no wall-clock timeout, so a property that loops
-- forever hangs the run.

-- Standard functions are captured at load so that a property that patches the
-- string metatable or a library table cannot make the engine raise (VER05o 4).
local type, ipairs = type, ipairs

local FailureIds = {
  COUNTEREXAMPLE = "property-counterexample",
  EXHAUSTED = "property-exhausted",
  TIMEOUT = "property-timeout",
  PANIC = "property-panic",
}
FailureIds.ALL = { FailureIds.COUNTEREXAMPLE, FailureIds.EXHAUSTED, FailureIds.TIMEOUT, FailureIds.PANIC }

--- True only for a string equal to one of the four registered ids.
function FailureIds.is_failure_id(candidate)
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
