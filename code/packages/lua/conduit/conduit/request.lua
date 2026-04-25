--- conduit/request.lua — Request object for Conduit.
---
--- Wraps the env table that the Rust dispatch layer passes to handlers.
--- Provides typed accessors for all standard request properties.
---
--- The env table keys mirror the Ruby/Python Conduit env convention:
---
---   REQUEST_METHOD           string  "GET", "POST", etc.
---   PATH_INFO                string  "/hello/world"
---   conduit.route_params     table   { name = "Alice" }
---   conduit.query_params     table   { q = "foo" }
---   conduit.headers          table   { ["content-type"] = "application/json" }
---   conduit.body             string  Raw request body
---   conduit.content_type     string  Content-Type header value (or nil)
---   conduit.content_length   integer Content-Length value (or nil)

local json_mod = require("conduit.json")
local halt_mod = require("conduit.halt")

local Request = {}
Request.__index = Request

--- Construct a Request from a raw env table.
---@param env table  The Rust-provided environment table
---@return Request
function Request.new(env)
    return setmetatable({ _env = env }, Request)
end

--- HTTP method string ("GET", "POST", etc.).
function Request:method()
    return self._env["REQUEST_METHOD"]
end

--- Request path, e.g. "/hello/world".
function Request:path()
    return self._env["PATH_INFO"]
end

--- Named route parameters captured by the URL pattern.
--- e.g. for pattern "/hello/:name" with URL "/hello/Alice": {name="Alice"}
function Request:params()
    return self._env["conduit.route_params"] or {}
end

--- Parsed query-string parameters as a flat string→string table.
--- e.g. for "?q=foo&page=2": {q="foo", page="2"}
function Request:query()
    return self._env["conduit.query_params"] or {}
end

--- Request headers as a table with lowercase string keys.
--- e.g. {["content-type"]="application/json", ["accept"]="*/*"}
function Request:headers()
    return self._env["conduit.headers"] or {}
end

--- Raw request body string (empty string if no body).
function Request:body()
    return self._env["conduit.body"] or ""
end

--- Content-Type header value, or nil if not present.
function Request:content_type()
    return self._env["conduit.content_type"]
end

--- Parse the request body as JSON and return the decoded Lua value.
--- Raises a 400 HaltError if the body is not valid JSON.
function Request:json_body()
    local ok, result = pcall(json_mod.decode, self:body())
    if not ok then
        halt_mod.raise(400, "Invalid JSON body: " .. tostring(result))
    end
    return result
end

return { Request = Request }
