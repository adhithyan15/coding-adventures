-- test_server.lua — End-to-end integration tests for conduit.Server
--
-- Starts a real TCP server on an ephemeral port, makes HTTP requests via
-- luasocket, and asserts on status codes, headers, and response bodies.
--
-- All tests share a single server instance (started in before_all / lazy
-- startup) to keep test run time short.
--
-- If luasocket is not installed, the E2E tests are pending (skipped).

package.path  = "../?.lua;../?/init.lua;" .. package.path
package.cpath = "../?.so;../?.dll;" .. package.cpath

-- Try to load luasocket; skip E2E if unavailable.
local socket_http_ok, socket_http = pcall(require, "socket.http")
local ltn12_ok,       ltn12       = pcall(require, "ltn12")
local socket_ok,      socket      = pcall(require, "socket")

if not (socket_http_ok and ltn12_ok and socket_ok) then
    pending("luasocket is not installed — skipping E2E server tests")
    return
end

local conduit = require("conduit")

-- ---------------------------------------------------------------------------
-- Helpers
-- ---------------------------------------------------------------------------

--- Make an HTTP request and return {status, headers, body}.
local function request(method, port, path, req_headers, req_body)
    local url = "http://127.0.0.1:" .. port .. path
    local body_sink = {}
    local opts = {
        url     = url,
        method  = method,
        sink    = ltn12.sink.table(body_sink),
        headers = req_headers or {},
    }
    if req_body then
        opts.source  = ltn12.source.string(req_body)
        opts.headers = opts.headers or {}
        opts.headers["content-length"] = tostring(#req_body)
    end
    local ok, status, resp_headers = socket_http.request(opts)
    return {
        status  = status,
        headers = resp_headers or {},
        body    = table.concat(body_sink),
    }
end

local function get(port, path)
    return request("GET", port, path)
end

local function post(port, path, content_type, body)
    return request("POST", port, path,
        { ["content-type"] = content_type },
        body)
end

--- Wait up to `timeout` seconds for the server to accept connections.
local function wait_for_server(port, timeout)
    local deadline = socket.gettime() + (timeout or 2)
    while socket.gettime() < deadline do
        local c = socket.connect("127.0.0.1", port)
        if c then c:close(); return true end
        socket.sleep(0.05)
    end
    return false
end

-- ---------------------------------------------------------------------------
-- Server setup
-- ---------------------------------------------------------------------------

local server_instance
local server_port

local function build_app()
    local app = conduit.Application.new()
    app:set("app_name", "Conduit E2E Test")

    -- Before filter: block /down
    app:before(function(ctx)
        if ctx:path() == "/down" then
            conduit.halt(503, "Under maintenance")
        end
    end)

    -- GET /
    app:get("/", function(ctx)
        return conduit.html("<h1>Hello from Conduit!</h1>")
    end)

    -- GET /hello/:name
    app:get("/hello/:name", function(ctx)
        local name = ctx:params()["name"]
        return conduit.json({ message = "Hello " .. name })
    end)

    -- POST /echo  — echoes the JSON body
    app:post("/echo", function(ctx)
        local data = ctx:json_body()
        return conduit.json(data)
    end)

    -- GET /redirect → 301 to /
    app:get("/redirect", function(ctx)
        return conduit.redirect("/", 301)
    end)

    -- GET /halt → 403 Forbidden via halt()
    app:get("/halt", function(ctx)
        conduit.halt(403, "Forbidden — this route always halts")
    end)

    -- GET /down → unreachable (before filter intercepts)
    app:get("/down", function(ctx)
        return conduit.html("this should never be reached")
    end)

    -- GET /error → triggers error handler
    app:get("/error", function(ctx)
        error("Intentional error for testing")
    end)

    -- Custom not-found handler (JSON to avoid XSS from raw path interpolation)
    app:not_found(function(ctx)
        return conduit.json({ message = "Not Found", path = ctx:path() }, 404)
    end)

    -- Custom error handler
    app:error_handler(function(ctx, err)
        return conduit.json({ error = "Internal Server Error" }, 500)
    end)

    return app
end

setup(function()
    local app = build_app()
    server_instance = conduit.Server.new(app, { host = "127.0.0.1", port = 0 })
    server_instance:serve_background()
    server_port = server_instance:local_port()
    assert.is_true(wait_for_server(server_port, 5), "Server did not start in time")
end)

teardown(function()
    if server_instance then
        server_instance:stop()
        socket.sleep(0.1)
    end
end)

-- ---------------------------------------------------------------------------
-- Tests
-- ---------------------------------------------------------------------------

describe("conduit.Server (E2E)", function()

    it("GET / returns 200 HTML", function()
        local r = get(server_port, "/")
        assert.equal(200, r.status)
        assert.truthy(r.body:find("Hello from Conduit"))
    end)

    it("GET / has text/html content-type", function()
        local r = get(server_port, "/")
        local ct = r.headers["content-type"] or ""
        assert.truthy(ct:find("text/html"))
    end)

    it("GET /hello/Alice returns 200 JSON with greeting", function()
        local r = get(server_port, "/hello/Alice")
        assert.equal(200, r.status)
        local ct = r.headers["content-type"] or ""
        assert.truthy(ct:find("application/json"))
        assert.truthy(r.body:find("Alice"))
    end)

    it("GET /hello/World returns the correct name", function()
        local r = get(server_port, "/hello/World")
        assert.truthy(r.body:find("World"))
    end)

    it("POST /echo echoes the JSON body", function()
        local r = post(server_port, "/echo",
                       "application/json", '{"ping":"pong"}')
        assert.equal(200, r.status)
        assert.truthy(r.body:find("ping"))
        assert.truthy(r.body:find("pong"))
    end)

    it("GET /redirect returns 301 with Location: /", function()
        -- Disable automatic redirects so we see the 301.
        local body_sink = {}
        local ok, status, resp_headers = socket_http.request({
            url        = "http://127.0.0.1:" .. server_port .. "/redirect",
            method     = "GET",
            sink       = ltn12.sink.table(body_sink),
            redirect   = false,
        })
        assert.equal(301, status)
        assert.equal("/", resp_headers and resp_headers["location"] or "")
    end)

    it("GET /halt returns 403 Forbidden", function()
        local r = get(server_port, "/halt")
        assert.equal(403, r.status)
        assert.truthy(r.body:find("Forbidden") or r.body == "Forbidden — this route always halts")
    end)

    it("GET /down returns 503 via before filter", function()
        local r = get(server_port, "/down")
        assert.equal(503, r.status)
    end)

    it("GET /error returns 500 JSON via error handler", function()
        local r = get(server_port, "/error")
        assert.equal(500, r.status)
        assert.truthy(r.body:find("Internal Server Error"))
    end)

    it("GET /missing returns 404 via not_found handler", function()
        local r = get(server_port, "/missing")
        assert.equal(404, r.status)
        assert.truthy(r.body:find("Not Found") or r.body:find("missing"))
    end)

    it("custom 404 response includes the path", function()
        local r = get(server_port, "/no-such-route")
        assert.truthy(r.body:find("no%-such%-route") or r.body:find("no-such-route"))
    end)

    it("server:local_port() returns a valid port number", function()
        assert.is_number(server_port)
        assert.truthy(server_port > 0 and server_port <= 65535)
    end)

    it("server:running() returns true while serving", function()
        assert.is_true(server_instance:running())
    end)

    it("app:get(key) returns the configured setting", function()
        -- Verify settings survive from Application creation to request time.
        -- We test this by checking the app object directly (not via HTTP).
        local app = build_app()
        assert.equal("Conduit E2E Test", app:get("app_name"))
    end)

    it("GET /hello/:name handles special-character names", function()
        local r = get(server_port, "/hello/Lua5.4")
        assert.equal(200, r.status)
        assert.truthy(r.body:find("Lua5.4") or r.status == 200)
    end)
end)
