--- Error classification (VER05o section 4).
--
-- A Lua error can be ANY value: a string, a table, a function, a coroutine, or
-- `nil` (`error(nil)`).  A table error can carry a `__tostring` that raises or an
-- `__index` that loops.  So the engine classifies an error by `type()` ALONE: it
-- never calls `tostring` on it, never reads a field of it, never compares or
-- measures it with a metamethod-aware operator.  The only way a user error ever
-- reaches text is through `Canon.render`, which reads structurally.

-- Standard functions are captured at load so that a property that patches the
-- string metatable or a library table cannot make the engine raise (VER05o 4).
local type, error = type, error

local Canon = require("coding_adventures.test_bootstrap.canon")

local Safe = {}

--- The kind of an error value: the result of `type()`, nothing else.
function Safe.kind(err)
  return type(err)
end

--- Redacted description: the kind only (never any content).
function Safe.redacted(err)
  return Safe.kind(err)
end

--- Local (unredacted) description: kind and structural rendering, clipped.
function Safe.describe(err)
  return Safe.kind(err) .. ": " .. Canon.clip(Canon.render(err), 200)
end

return Safe
