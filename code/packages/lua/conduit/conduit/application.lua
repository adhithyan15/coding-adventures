--- conduit/application.lua — Application class for Conduit.
---
--- The Application is the central object you configure before creating a
--- Server. You register route handlers, lifecycle filters, a not-found handler,
--- and an error handler, then pass the Application to Server.new().
---
--- All handlers receive a HandlerContext as their first argument (called `ctx`
--- in the examples below). Filters may return nil (no response) or a response
--- table to short-circuit the request.
---
--- ## Routing
---
---   app:get("/",           function(ctx) return ctx:html("Hello!") end)
---   app:post("/echo",      function(ctx) return ctx:json(ctx:json_body()) end)
---   app:put("/item/:id",   function(ctx) ... end)
---   app:delete("/item/:id", function(ctx) ... end)
---   app:patch("/item/:id", function(ctx) ... end)
---
--- Route patterns support named captures with a leading colon:
---   "/hello/:name"  →  ctx:params().name = "Alice"
---
--- ## Filters
---
---   app:before(function(ctx) ... end)  -- runs before every route
---   app:after(function(ctx)  ... end)  -- runs after every route
---
--- ## Special handlers
---
---   app:not_found(function(ctx)       ... end)
---   app:error_handler(function(ctx, err) ... end)
---
--- ## Settings
---
---   app:set("app_name", "My App")
---   app:get("app_name")   -- returns "My App"   (1-arg form = getter)
---   app:get("/path", fn)  -- registers GET route  (2-arg form = route)

local native = require("conduit.conduit_native")
local hc_mod  = require("conduit.handler_context")

local Application = {}
Application.__index = Application

--- Create a new Application.
function Application.new()
    local self = setmetatable({}, Application)
    self._app    = native.new_app()
    self._routes = {}         -- {method, pattern} pairs for app:routes()
    return self
end

-- ---------------------------------------------------------------------------
-- Internal helpers
-- ---------------------------------------------------------------------------

--- Wrap a user handler so it receives a HandlerContext instead of a raw env.
local function wrap(fn)
    return function(env)
        local ctx = hc_mod.HandlerContext.new(env)
        return fn(ctx)
    end
end

--- Wrap an error handler (receives ctx + error message string).
local function wrap_error(fn)
    return function(env, err_msg)
        local ctx = hc_mod.HandlerContext.new(env)
        return fn(ctx, err_msg or "")
    end
end

--- Register a route with a given HTTP method.
local function add_route(self, method, pattern, fn)
    native.app_add_route(self._app, method, pattern, wrap(fn))
    self._routes[#self._routes + 1] = { method = method, pattern = pattern }
end

-- ---------------------------------------------------------------------------
-- Route registration
-- ---------------------------------------------------------------------------

--- Register a GET route, OR (1-arg form) retrieve a setting.
---
--- Two-arg form:  app:get(pattern, handler)  → registers a GET route
--- One-arg form:  app:get(key)               → returns app:get_setting(key)
function Application:get(pattern_or_key, fn)
    if fn ~= nil then
        add_route(self, "GET", pattern_or_key, fn)
    else
        return native.app_get_setting(self._app, pattern_or_key)
    end
end

--- Register a POST route.
function Application:post(pattern, fn)
    add_route(self, "POST", pattern, fn)
end

--- Register a PUT route.
function Application:put(pattern, fn)
    add_route(self, "PUT", pattern, fn)
end

--- Register a DELETE route.
function Application:delete(pattern, fn)
    add_route(self, "DELETE", pattern, fn)
end

--- Register a PATCH route.
function Application:patch(pattern, fn)
    add_route(self, "PATCH", pattern, fn)
end

-- ---------------------------------------------------------------------------
-- Filters
-- ---------------------------------------------------------------------------

--- Register a before-filter. Runs before every route handler.
--- Return a response table to short-circuit (skip the route handler).
--- Return nil to continue to the next handler.
function Application:before(fn)
    native.app_add_before(self._app, wrap(fn))
end

--- Register an after-filter. Runs after every route handler.
--- Return a replacement response table to override, or nil to keep the
--- original response.
function Application:after(fn)
    native.app_add_after(self._app, wrap(fn))
end

-- ---------------------------------------------------------------------------
-- Special handlers
-- ---------------------------------------------------------------------------

--- Register the custom not-found handler (called when no route matches).
function Application:not_found(fn)
    native.app_set_not_found(self._app, wrap(fn))
end

--- Register the custom error handler (called on unhandled Lua errors).
--- The handler receives (ctx, err_message_string).
function Application:error_handler(fn)
    native.app_set_error_handler(self._app, wrap_error(fn))
end

-- ---------------------------------------------------------------------------
-- Settings
-- ---------------------------------------------------------------------------

--- Store a configuration value.
---@param key   string  Setting name
---@param value any     Value (converted to string)
function Application:set(key, value)
    native.app_set_setting(self._app, key, tostring(value))
end

-- ---------------------------------------------------------------------------
-- Introspection
-- ---------------------------------------------------------------------------

--- Return the list of registered routes as {method, pattern} tables.
--- Useful for debugging and testing.
function Application:routes()
    return self._routes
end

return { Application = Application }
