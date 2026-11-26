--- What one execution of the target did (VER06 section 5.1).
--
-- An outcome has a `kind` (`pass`, `crash`, `timeout`, `oom`), for a crash a
-- `crash_kind` (the `type()` of the error, or an engine-named protocol failure) and
-- its `frames`, and an `escaped` flag: true when the target escaped the execution
-- contract (it yielded or cleared the hook), which counts toward the leak cap
-- (VER06p 5.4).  Only this module builds one; `Outcome.is` recognises them by
-- IDENTITY, so a table a target or a test exec fabricates is never one.

-- Standard functions are captured at load so that a target that patches the string
-- metatable or a library table cannot make the engine raise (VER06p section 1).
local type, setmetatable = type, setmetatable

local Outcome = {}

local MADE = setmetatable({}, { __mode = "k" })
local META = {}
META.__index = META

--- No payload, no message: the kind and, for a crash, its kind only.
function META:__tostring()
  if self.kind == "crash" then
    return "#<Fuzz.Outcome crash " .. self.crash_kind .. ">"
  end
  return "#<Fuzz.Outcome " .. self.kind .. ">"
end

local function make(kind, crash_kind, frames, escaped)
  local o = setmetatable({ kind = kind, crash_kind = crash_kind, frames = frames or {}, escaped = escaped == true }, META)
  MADE[o] = true
  return o
end

function Outcome.pass(escaped)
  return make("pass", nil, nil, escaped)
end

function Outcome.timeout(escaped)
  return make("timeout", nil, nil, escaped)
end

function Outcome.oom(escaped)
  return make("oom", nil, nil, escaped)
end

function Outcome.crash(kind, frames, escaped)
  return make("crash", kind, frames, escaped)
end

--- True only for an outcome this module built.
function Outcome.is(x)
  return type(x) == "table" and MADE[x] == true
end

function Outcome.is_crash(x)
  return Outcome.is(x) and x.kind == "crash"
end

function Outcome.is_pass(x)
  return Outcome.is(x) and x.kind == "pass"
end

return Outcome
