--- conduit/halt.lua — HaltError helpers for Conduit.
---
--- A HaltError is a plain Lua table that Conduit's Rust dispatch layer
--- recognises as an intentional early exit (like a Sinatra `halt`).
---
--- The Rust side checks for the sentinel key `__conduit_halt = true` after a
--- failed `lua_pcall`, then extracts `{status, body, headers}` and returns the
--- corresponding HTTP response without invoking the error handler.
---
--- ## Usage
---
---   local halt = require("conduit.halt")
---
---   -- Raise a HaltError immediately:
---   halt.raise(403, "Forbidden")
---
---   -- Check whether a pcall-captured value is a HaltError:
---   local ok, err = pcall(fn)
---   if not ok and halt.is_halt_error(err) then
---     -- err.status, err.body, err.headers are set
---   end

local M = {}

--- Construct a HaltError table (does NOT raise; just creates).
---
--- @param status   integer  HTTP status code
--- @param body     string   Response body (default "")
--- @param headers  table    Header pairs {{name, value}, ...} (default {})
--- @return table  HaltError table
function M.new(status, body, headers)
    return {
        __conduit_halt = true,
        status  = status,
        body    = body or "",
        headers = headers or {},
    }
end

--- Raise a HaltError immediately via error().
---
--- The error is raised at level 0 so Lua does not append location info to the
--- table; the Rust pcall handler inspects the table directly.
---
--- @param status   integer  HTTP status code
--- @param body     string   Response body (default "")
--- @param headers  table    Optional header pairs {{name, value}, ...}
function M.raise(status, body, headers)
    error(M.new(status, body, headers), 0)
end

--- Return true if err is a HaltError table.
---
--- @param err any  Value caught by pcall
--- @return boolean
function M.is_halt_error(err)
    return type(err) == "table" and err.__conduit_halt == true
end

return M
