-- test_request.lua — Tests for conduit.request (Request class)
--
-- Covers: all request-inspection methods, json_body() parsing and error.

package.path  = "../?.lua;../?/init.lua;" .. package.path
package.cpath = "../?.so;../?.dll;" .. package.cpath

local request_mod = require("conduit.request")
local halt_mod    = require("conduit.halt")
local Request     = request_mod.Request

-- Build a synthetic env table (mirrors what the Rust dispatch layer sends).
local function make_env(overrides)
    local base = {
        REQUEST_METHOD         = "GET",
        PATH_INFO              = "/test/path",
        QUERY_STRING           = "q=hello&page=2",
        ["conduit.route_params"]  = { id = "42", name = "Alice" },
        ["conduit.query_params"]  = { q = "hello", page = "2" },
        ["conduit.headers"]       = { ["content-type"] = "application/json",
                                      ["accept"]        = "*/*" },
        ["conduit.body"]          = '{"key":"value"}',
        ["conduit.content_type"]  = "application/json",
        ["conduit.content_length"] = 15,
    }
    for k, v in pairs(overrides or {}) do base[k] = v end
    return base
end

describe("conduit.request.Request", function()

    describe("constructor", function()
        it("creates a Request from an env table", function()
            local req = Request.new(make_env())
            assert.not_nil(req)
        end)

        it("stores the env table internally", function()
            local env = make_env()
            local req = Request.new(env)
            assert.equal(env, req._env)
        end)
    end)

    describe("method()", function()
        it("returns GET for a GET request", function()
            assert.equal("GET", Request.new(make_env()):method())
        end)

        it("returns POST for a POST request", function()
            assert.equal("POST", Request.new(make_env({ REQUEST_METHOD = "POST" })):method())
        end)

        it("returns DELETE for a DELETE request", function()
            assert.equal("DELETE", Request.new(make_env({ REQUEST_METHOD = "DELETE" })):method())
        end)
    end)

    describe("path()", function()
        it("returns the PATH_INFO", function()
            assert.equal("/test/path", Request.new(make_env()):path())
        end)

        it("returns root path correctly", function()
            assert.equal("/", Request.new(make_env({ PATH_INFO = "/" })):path())
        end)
    end)

    describe("params()", function()
        it("returns the route params table", function()
            local params = Request.new(make_env()):params()
            assert.equal("42",    params.id)
            assert.equal("Alice", params.name)
        end)

        it("returns empty table when no route params", function()
            -- Use a minimal env with no conduit.route_params key.
            local params = Request.new({ REQUEST_METHOD="GET", PATH_INFO="/" }):params()
            assert.same({}, params)
        end)
    end)

    describe("query()", function()
        it("returns the query params table", function()
            local q = Request.new(make_env()):query()
            assert.equal("hello", q.q)
            assert.equal("2",     q.page)
        end)

        it("returns empty table when no query params", function()
            local q = Request.new({ REQUEST_METHOD="GET", PATH_INFO="/" }):query()
            assert.same({}, q)
        end)
    end)

    describe("headers()", function()
        it("returns the headers table", function()
            local h = Request.new(make_env()):headers()
            assert.equal("application/json", h["content-type"])
            assert.equal("*/*",              h["accept"])
        end)

        it("returns empty table when no headers", function()
            local h = Request.new({ REQUEST_METHOD="GET", PATH_INFO="/" }):headers()
            assert.same({}, h)
        end)
    end)

    describe("body()", function()
        it("returns the raw body string", function()
            assert.equal('{"key":"value"}', Request.new(make_env()):body())
        end)

        it("returns empty string when no body", function()
            assert.equal("", Request.new({ REQUEST_METHOD="GET", PATH_INFO="/" }):body())
        end)
    end)

    describe("content_type()", function()
        it("returns the content-type header value", function()
            assert.equal("application/json", Request.new(make_env()):content_type())
        end)

        it("returns nil when content-type is absent", function()
            assert.is_nil(Request.new({ REQUEST_METHOD="GET", PATH_INFO="/" }):content_type())
        end)
    end)

    describe("json_body()", function()
        it("parses valid JSON object", function()
            local result = Request.new(make_env()):json_body()
            assert.equal("value", result.key)
        end)

        it("parses valid JSON array", function()
            local req = Request.new(make_env({ ["conduit.body"] = "[1,2,3]" }))
            local result = req:json_body()
            assert.same({1, 2, 3}, result)
        end)

        it("parses nested JSON", function()
            local req = Request.new(make_env({ ["conduit.body"] = '{"a":{"b":1}}' }))
            assert.equal(1, req:json_body().a.b)
        end)

        it("raises HaltError 400 on invalid JSON", function()
            local req = Request.new(make_env({ ["conduit.body"] = "not json" }))
            local ok, err = pcall(function() req:json_body() end)
            assert.is_false(ok)
            assert.is_true(halt_mod.is_halt_error(err))
            assert.equal(400, err.status)
        end)

        it("raises HaltError 400 on empty body", function()
            local req = Request.new(make_env({ ["conduit.body"] = "" }))
            local ok, err = pcall(function() req:json_body() end)
            assert.is_false(ok)
            assert.is_true(halt_mod.is_halt_error(err))
            assert.equal(400, err.status)
        end)
    end)
end)
