-- test_hello.lua — End-to-end tests for the conduit-hello Lua demo.
--
-- Starts the demo's production foreground server in a dedicated child process,
-- then drives every route over real HTTP via luasocket — mirroring the conduit
-- library's own `test_server`.
--
-- If luasocket is not installed the E2E tests are pending (skipped), exactly
-- like the library suite, so the demo still builds where the optional socket
-- library is unavailable.
--
-- Run (from this directory): `busted . --pattern=test_`

-- Find the conduit package: from this `tests/` dir it is four levels up under
-- `code/packages/lua/conduit`.  The demo's own `hello.lua` also extends the
-- path, but we set it here too so this test file can be loaded standalone.
package.path  = "../../../../packages/lua/conduit/?.lua;"
             .. "../../../../packages/lua/conduit/?/init.lua;"
             .. package.path
package.cpath = "../../../../packages/lua/conduit/?.so;"
             .. "../../../../packages/lua/conduit/?.dll;"
             .. package.cpath

-- Try to load luasocket; skip E2E if unavailable.
local socket_http_ok, socket_http = pcall(require, "socket.http")
local ltn12_ok,       ltn12       = pcall(require, "ltn12")
local socket_ok,      socket      = pcall(require, "socket")

if not (socket_http_ok and ltn12_ok and socket_ok) then
    pending("luasocket is not installed — skipping E2E demo tests")
    return
end

socket_http.TIMEOUT = 5

-- ---------------------------------------------------------------------------
-- HTTP helpers (same shape as the library's test_server.lua)
-- ---------------------------------------------------------------------------

local function request(method, port, path, req_headers, req_body)
    local url = "http://127.0.0.1:" .. port .. path
    local body_sink = {}
    local opts = {
        url     = url,
        method  = method,
        sink    = ltn12.sink.table(body_sink),
        headers = req_headers or {},
        redirect = false, -- assert the 3xx ourselves; do not follow
    }
    if req_body then
        opts.source  = ltn12.source.string(req_body)
        opts.headers = opts.headers or {}
        opts.headers["content-length"] = tostring(#req_body)
    end
    local _, status, resp_headers = socket_http.request(opts)
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
    return request("POST", port, path, { ["content-type"] = content_type }, body)
end

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
-- Server setup — load the DEMO's app factory and serve it in the background.
-- ---------------------------------------------------------------------------

local server_instance
local server_port

describe("conduit-hello demo (Lua)", function()
    setup(function()
        server_instance = assert(io.popen("lua server_process.lua 2>&1", "r"))
        local ready_line = server_instance:read("*l")
        assert.is_truthy(ready_line, "demo server process exited before startup")
        server_port = tonumber(ready_line:match("^CONDUIT_READY (%d+)$"))
        assert.is_truthy(
            server_port,
            "unexpected demo server startup output: " .. ready_line
        )
        assert.is_true(wait_for_server(server_port, 5), "demo server did not start in time")
    end)

    teardown(function()
        if server_instance and server_port then
            pcall(get, server_port, "/__test_shutdown")
        end
        if server_instance then
            server_instance:close()
        end
    end)

    it("GET / returns the HTML greeting", function()
        local res = get(server_port, "/")
        assert.are.equal(200, res.status)
        assert.is_truthy(res.body:find("Hello from Conduit!", 1, true))
    end)

    it("GET /hello/:name returns JSON with the name", function()
        local res = get(server_port, "/hello/Adhithya")
        assert.are.equal(200, res.status)
        assert.is_truthy(res.body:find("Hello Adhithya", 1, true))
    end)

    it("POST /echo echoes the JSON body", function()
        local res = post(server_port, "/echo", "application/json", '{"ping":"pong"}')
        assert.are.equal(200, res.status)
        assert.is_truthy(res.body:find("pong", 1, true))
    end)

    it("GET /redirect returns 301 with Location: /", function()
        local res = get(server_port, "/redirect")
        assert.are.equal(301, res.status)
        local loc = res.headers["location"] or res.headers["Location"]
        assert.are.equal("/", loc)
    end)

    it("GET /halt returns 403", function()
        local res = get(server_port, "/halt")
        assert.are.equal(403, res.status)
    end)

    it("GET /down returns 503 from the before filter", function()
        local res = get(server_port, "/down")
        assert.are.equal(503, res.status)
    end)

    it("GET /error returns 500 from the custom error handler", function()
        local res = get(server_port, "/error")
        assert.are.equal(500, res.status)
        assert.is_truthy(res.body:find("Internal Server Error", 1, true))
    end)

    it("GET /missing returns the custom 404 page", function()
        local res = get(server_port, "/missing")
        assert.are.equal(404, res.status)
        assert.is_truthy(res.body:find("404 Not Found", 1, true))
    end)
end)
