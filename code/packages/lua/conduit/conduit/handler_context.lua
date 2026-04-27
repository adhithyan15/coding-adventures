--- conduit/handler_context.lua — Handler evaluation context for Conduit.
---
--- Every route handler, before/after filter, not-found handler, and error
--- handler receives a HandlerContext as its first argument (`ctx`).
---
--- HandlerContext extends Request with response-building helpers. All the
--- request-inspection methods (method, path, params, etc.) are inherited from
--- the Request metatable via Lua's metatable chain:
---
---   HandlerContext.__index = HandlerContext
---   HandlerContext's metatable.__index = Request
---
--- So `ctx:params()` looks up params in HandlerContext (not found) → falls
--- through to Request (found). And `ctx:html(...)` is found in HandlerContext.
---
--- ## Response helpers
---
--- All helpers return a three-element array table:
---
---   { status_code, {{header_name, header_value}, ...}, body_string }
---
--- The Rust dispatch layer inspects the return value: nil → no override;
--- a table → send this HTTP response.
---
---   return ctx:html("<h1>Hello</h1>")
---   -- returns {200, {{"content-type","text/html"}}, "<h1>Hello</h1>"}
---
--- ## Module-level helpers
---
--- This module also exports standalone functions (not methods) for use as
--- `conduit.html`, `conduit.json`, etc. from `conduit/init.lua`:
---
---   local html = require("conduit").html
---   return html("<h1>Hi</h1>", 200)

local Request  = require("conduit.request").Request
local halt_mod = require("conduit.halt")
local json_mod = require("conduit.json")

-- ---------------------------------------------------------------------------
-- HandlerContext class — inherits all Request methods
-- ---------------------------------------------------------------------------

local HandlerContext = {}
HandlerContext.__index = HandlerContext

-- Prototype chain: HandlerContext → Request
-- When a method is not found in HandlerContext, Lua looks in Request.
setmetatable(HandlerContext, { __index = Request })

--- Construct a HandlerContext from a raw env table.
---@param env table  The Rust-provided environment table
---@return HandlerContext
function HandlerContext.new(env)
    -- Reuse Request.new to initialise _env, then set the HandlerContext
    -- metatable so HC-specific methods take precedence.
    local req = Request.new(env)
    return setmetatable(req, HandlerContext)
end

-- ---------------------------------------------------------------------------
-- Response helpers (methods on ctx)
-- ---------------------------------------------------------------------------

--- Return an HTML response.
---@param body   string   Response body (HTML string)
---@param status integer  HTTP status (default 200)
---@return table  Response table {status, headers, body}
function HandlerContext:html(body, status)
    return { status or 200, {{"content-type", "text/html"}}, body or "" }
end

--- Encode tbl as JSON and return an application/json response.
---@param tbl    table    Value to encode
---@param status integer  HTTP status (default 200)
---@return table
function HandlerContext:json(tbl, status)
    return { status or 200, {{"content-type", "application/json"}}, json_mod.encode(tbl) }
end

--- Return a plain-text response.
---@param body   string   Response body
---@param status integer  HTTP status (default 200)
---@return table
function HandlerContext:text(body, status)
    return { status or 200, {{"content-type", "text/plain"}}, body or "" }
end

--- Return a redirect response.
---@param location string   Redirect URL
---@param status   integer  HTTP status (default 301)
---@return table
function HandlerContext:redirect(location, status)
    return { status or 301, {{"location", location}}, "" }
end

--- Raise a HaltError immediately, bypassing the rest of the handler chain.
--- Rust's dispatch catches it and returns the specified response.
---@param status  integer  HTTP status
---@param body    string   Response body (default "")
---@param headers table    Optional header pairs {{name, value}, ...}
function HandlerContext:halt(status, body, headers)
    halt_mod.raise(status, body, headers)
end

-- ---------------------------------------------------------------------------
-- Module-level response helpers (used as conduit.html, conduit.json, etc.)
-- These are plain functions, not methods — no `self` argument.
-- ---------------------------------------------------------------------------

local function html_helper(body, status)
    return { status or 200, {{"content-type", "text/html"}}, body or "" }
end

local function json_helper(tbl, status)
    return { status or 200, {{"content-type", "application/json"}}, json_mod.encode(tbl) }
end

local function text_helper(body, status)
    return { status or 200, {{"content-type", "text/plain"}}, body or "" }
end

local function redirect_helper(location, status)
    return { status or 301, {{"location", location}}, "" }
end

return {
    HandlerContext = HandlerContext,
    -- Module-level helpers exported for conduit/init.lua to re-export.
    html     = html_helper,
    json     = json_helper,
    text     = text_helper,
    redirect = redirect_helper,
}
