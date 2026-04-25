-- conduit-hello — Full Conduit Lua demo
--
-- Exercises every feature of the Conduit Lua DSL:
--
--   GET  /                   → HTML greeting
--   GET  /hello/:name        → JSON with name
--   POST /echo               → echoes JSON body
--   GET  /redirect           → 301 to /
--   GET  /halt               → 403 via halt()
--   GET  /down               → 503 via before filter
--   GET  /error              → 500 via custom error handler
--   GET  /missing            → 404 via custom not_found handler
--
-- Run:
--   lua hello.lua
-- Then test with curl:
--   curl http://127.0.0.1:3000/
--   curl http://127.0.0.1:3000/hello/Adhithya
--   curl -X POST http://127.0.0.1:3000/echo \
--        -H 'Content-Type: application/json' -d '{"ping":"pong"}'
--   curl -i http://127.0.0.1:3000/redirect
--   curl http://127.0.0.1:3000/halt
--   curl http://127.0.0.1:3000/down
--   curl http://127.0.0.1:3000/error
--   curl http://127.0.0.1:3000/missing

-- Adjust the package path so this program can be run from its own directory
-- or from the repo root. The conduit package directory is three levels up.
local script_dir = debug.getinfo(1, "S").source:match("@?(.*)/") or "."
package.path  = script_dir .. "/../../../packages/lua/conduit/?.lua;"
             .. script_dir .. "/../../../packages/lua/conduit/?/init.lua;"
             .. package.path
package.cpath = script_dir .. "/../../../packages/lua/conduit/?.so;"
             .. script_dir .. "/../../../packages/lua/conduit/?.dll;"
             .. package.cpath

local conduit = require("conduit")
local html     = conduit.html
local json     = conduit.json
local halt     = conduit.halt

local app = conduit.Application.new()

app:set("app_name", "Conduit Hello")

-- ── Before filter: block /down for maintenance ──────────────────────────────

app:before(function(ctx)
    if ctx:path() == "/down" then
        halt(503, "Under maintenance")
    end
end)

-- ── After filter: log every request to stdout ───────────────────────────────

app:after(function(ctx)
    io.write("[after] " .. ctx:method() .. " " .. ctx:path() .. "\n")
    io.flush()
end)

-- ── Routes ───────────────────────────────────────────────────────────────────

app:get("/", function(ctx)
    return html("<h1>Hello from Conduit!</h1><p>Try /hello/Adhithya</p>")
end)

app:get("/hello/:name", function(ctx)
    local name = ctx:params()["name"]
    return json({ message = "Hello " .. name, app = app:get("app_name") })
end)

app:post("/echo", function(ctx)
    local data = ctx:json_body()
    return json(data)
end)

app:get("/redirect", function(ctx)
    return conduit.redirect("/", 301)
end)

app:get("/halt", function(ctx)
    halt(403, "Forbidden — this route always halts")
end)

app:get("/down", function(ctx)
    -- Unreachable: the before filter halts 503 on /down
    return html("This should never be reached")
end)

app:get("/error", function(ctx)
    error("Intentional error for demo")
end)

-- ── Custom not-found handler ─────────────────────────────────────────────────

app:not_found(function(ctx)
    return html("<h1>404 Not Found</h1><p>No route for " .. ctx:path() .. "</p>", 404)
end)

-- ── Custom error handler ──────────────────────────────────────────────────────

app:error_handler(function(ctx, err)
    return json({ error = "Internal Server Error", detail = err }, 500)
end)

-- ── Start server ─────────────────────────────────────────────────────────────

local server = conduit.Server.new(app, { host = "127.0.0.1", port = 3000 })

io.write(app:get("app_name") .. " listening on http://127.0.0.1:3000\n")
io.write("Routes:\n")
io.write("  GET  /                 → HTML greeting\n")
io.write("  GET  /hello/:name      → JSON response\n")
io.write("  POST /echo             → echo JSON body\n")
io.write("  GET  /redirect         → 301 to /\n")
io.write("  GET  /halt             → 403 Forbidden\n")
io.write("  GET  /down             → 503 (before filter)\n")
io.write("  GET  /error            → 500 via custom error handler\n")
io.write("  GET  /missing          → 404 via custom not_found handler\n")
io.write("Press Ctrl-C to stop.\n")
io.flush()

server:serve()
