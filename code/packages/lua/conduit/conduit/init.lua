--- conduit/init.lua — Conduit web framework for Lua 5.4
---
--- This is the main entry point loaded by `require("conduit")`. It assembles
--- all sub-modules into a single module table and re-exports the public API.
---
--- ## Quick start
---
---   local conduit = require("conduit")
---   local html    = conduit.html
---   local json    = conduit.json
---   local halt    = conduit.halt
---
---   local app = conduit.Application.new()
---
---   app:before(function(ctx)
---       if ctx:path() == "/down" then halt(503, "Under maintenance") end
---   end)
---
---   app:get("/", function(ctx)
---       return html("<h1>Hello from Conduit!</h1>")
---   end)
---
---   app:get("/hello/:name", function(ctx)
---       return json({ message = "Hello " .. ctx:params()["name"] })
---   end)
---
---   app:not_found(function(ctx)
---       return html("<h1>Not Found</h1>", 404)
---   end)
---
---   app:error_handler(function(ctx, err)
---       return json({ error = "Internal Server Error" }, 500)
---   end)
---
---   local server = conduit.Server.new(app, { host="127.0.0.1", port=3000 })
---   server:serve()
---
--- ## Module-level response helpers
---
---   conduit.html(body [, status])           → {status, headers, body}
---   conduit.json(tbl  [, status])           → {status, headers, json_body}
---   conduit.text(body [, status])           → {status, headers, body}
---   conduit.redirect(location [, status])   → {status, headers, ""}
---   conduit.halt(status, body [, headers])  → raises HaltError
---
--- ## Classes
---
---   conduit.Application  — route registration, filters, settings
---   conduit.Server       — TCP socket, serve/stop

local hc_mod   = require("conduit.handler_context")
local halt_mod = require("conduit.halt")
local app_mod  = require("conduit.application")
local srv_mod  = require("conduit.server")

local M = {}

-- Module-level response helpers (plain functions, not methods).
M.html     = hc_mod.html
M.json     = hc_mod.json
M.text     = hc_mod.text
M.redirect = hc_mod.redirect

--- Raise a HaltError immediately.
---@param status  integer  HTTP status code
---@param body    string   Response body (default "")
---@param headers table    Optional header pairs {{name, value}, ...}
M.halt = halt_mod.raise

--- Check whether a pcall-captured error is a HaltError.
---@param err any
---@return boolean
M.is_halt_error = halt_mod.is_halt_error

-- Classes.
M.Application = app_mod.Application
M.Server      = srv_mod.Server

return M
