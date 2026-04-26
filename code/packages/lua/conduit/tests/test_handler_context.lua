-- test_handler_context.lua — Tests for conduit.handler_context
--
-- Covers: all response helpers, request delegation, json_body error handling.

package.path  = "../?.lua;../?/init.lua;" .. package.path
package.cpath = "../?.so;../?.dll;" .. package.cpath

local hc_mod   = require("conduit.handler_context")
local halt_mod = require("conduit.halt")

local HandlerContext = hc_mod.HandlerContext

-- Build a synthetic env table for test contexts.
local function make_env(overrides)
    local base = {
        REQUEST_METHOD            = "GET",
        PATH_INFO                 = "/test",
        ["conduit.route_params"]  = { id = "7", name = "Bob" },
        ["conduit.query_params"]  = { search = "lua" },
        ["conduit.headers"]       = { ["x-custom"] = "header-value",
                                      ["content-type"] = "application/json" },
        ["conduit.body"]          = '{"msg":"hello"}',
        ["conduit.content_type"]  = "application/json",
    }
    for k, v in pairs(overrides or {}) do base[k] = v end
    return base
end

local function make_ctx(overrides)
    return HandlerContext.new(make_env(overrides))
end

describe("conduit.handler_context.HandlerContext", function()

    -- -----------------------------------------------------------------------
    -- Request-inspection methods (inherited from Request)
    -- -----------------------------------------------------------------------

    describe("request delegation", function()
        it("method() returns the HTTP method", function()
            assert.equal("GET", make_ctx():method())
        end)

        it("path() returns PATH_INFO", function()
            assert.equal("/test", make_ctx():path())
        end)

        it("params() returns route params", function()
            local p = make_ctx():params()
            assert.equal("7",   p.id)
            assert.equal("Bob", p.name)
        end)

        it("query() returns query params", function()
            assert.equal("lua", make_ctx():query().search)
        end)

        it("headers() returns headers table (lowercase keys)", function()
            assert.equal("header-value", make_ctx():headers()["x-custom"])
        end)

        it("body() returns the raw body", function()
            assert.equal('{"msg":"hello"}', make_ctx():body())
        end)

        it("content_type() returns content type", function()
            assert.equal("application/json", make_ctx():content_type())
        end)
    end)

    -- -----------------------------------------------------------------------
    -- json_body()
    -- -----------------------------------------------------------------------

    describe("json_body()", function()
        it("parses valid JSON body", function()
            local result = make_ctx():json_body()
            assert.equal("hello", result.msg)
        end)

        it("raises 400 HaltError on invalid JSON", function()
            local ctx = make_ctx({ ["conduit.body"] = "}{invalid" })
            local ok, err = pcall(function() ctx:json_body() end)
            assert.is_false(ok)
            assert.is_true(halt_mod.is_halt_error(err))
            assert.equal(400, err.status)
        end)
    end)

    -- -----------------------------------------------------------------------
    -- html()
    -- -----------------------------------------------------------------------

    describe("html()", function()
        it("returns a 200 response table by default", function()
            local r = make_ctx():html("<p>Hello</p>")
            assert.equal(200, r[1])
            assert.same({{"content-type", "text/html"}}, r[2])
            assert.equal("<p>Hello</p>", r[3])
        end)

        it("accepts a custom status code", function()
            local r = make_ctx():html("<p>Not Found</p>", 404)
            assert.equal(404, r[1])
        end)

        it("defaults body to empty string when nil", function()
            local r = make_ctx():html(nil, 200)
            assert.equal("", r[3])
        end)
    end)

    -- -----------------------------------------------------------------------
    -- json()
    -- -----------------------------------------------------------------------

    describe("json()", function()
        it("returns a 200 response with application/json content-type", function()
            local r = make_ctx():json({ a = 1 })
            assert.equal(200, r[1])
            assert.same({{"content-type", "application/json"}}, r[2])
        end)

        it("serialises the table to a JSON string", function()
            local r = make_ctx():json({ key = "val" })
            -- The JSON body must contain "key":"val" (key order may vary).
            assert.truthy(r[3]:find('"key"'))
            assert.truthy(r[3]:find('"val"'))
        end)

        it("accepts a custom status code", function()
            local r = make_ctx():json({ err = "not found" }, 404)
            assert.equal(404, r[1])
        end)

        it("serialises arrays correctly", function()
            local r = make_ctx():json({10, 20, 30})
            assert.equal("[10,20,30]", r[3])
        end)
    end)

    -- -----------------------------------------------------------------------
    -- text()
    -- -----------------------------------------------------------------------

    describe("text()", function()
        it("returns a 200 text/plain response", function()
            local r = make_ctx():text("hello world")
            assert.equal(200, r[1])
            assert.same({{"content-type", "text/plain"}}, r[2])
            assert.equal("hello world", r[3])
        end)

        it("accepts a custom status code", function()
            local r = make_ctx():text("error", 500)
            assert.equal(500, r[1])
        end)
    end)

    -- -----------------------------------------------------------------------
    -- redirect()
    -- -----------------------------------------------------------------------

    describe("redirect()", function()
        it("returns a 301 response with Location header by default", function()
            local r = make_ctx():redirect("/home")
            assert.equal(301, r[1])
            assert.same({{"location", "/home"}}, r[2])
            assert.equal("", r[3])
        end)

        it("accepts a custom 302 status", function()
            local r = make_ctx():redirect("/home", 302)
            assert.equal(302, r[1])
        end)

        it("stores the redirect location correctly", function()
            local r = make_ctx():redirect("/other/page")
            assert.equal("/other/page", r[2][1][2])
        end)
    end)

    -- -----------------------------------------------------------------------
    -- halt()
    -- -----------------------------------------------------------------------

    describe("halt()", function()
        it("raises a HaltError via pcall", function()
            local ok, err = pcall(function()
                make_ctx():halt(403, "Forbidden")
            end)
            assert.is_false(ok)
            assert.is_true(halt_mod.is_halt_error(err))
        end)

        it("sets the correct status on the HaltError", function()
            local ok, err = pcall(function()
                make_ctx():halt(503, "Down")
            end)
            assert.is_false(ok)
            assert.equal(503, err.status)
        end)

        it("sets the correct body on the HaltError", function()
            local ok, err = pcall(function()
                make_ctx():halt(403, "Access denied")
            end)
            assert.is_false(ok)
            assert.equal("Access denied", err.body)
        end)

        it("accepts custom headers", function()
            local ok, err = pcall(function()
                make_ctx():halt(401, "", {{"www-authenticate", "Basic"}})
            end)
            assert.is_false(ok)
            assert.same({{"www-authenticate", "Basic"}}, err.headers)
        end)
    end)

    -- -----------------------------------------------------------------------
    -- Module-level helpers (exported from handler_context module)
    -- -----------------------------------------------------------------------

    describe("module-level html()", function()
        it("works as a standalone function", function()
            local r = hc_mod.html("<b>test</b>")
            assert.equal(200, r[1])
            assert.equal("<b>test</b>", r[3])
        end)
    end)

    describe("module-level json()", function()
        it("works as a standalone function", function()
            local r = hc_mod.json({ x = 1 })
            assert.equal(200, r[1])
            assert.truthy(r[3]:find('"x"'))
        end)
    end)

    describe("module-level redirect()", function()
        it("works as a standalone function", function()
            local r = hc_mod.redirect("/foo")
            assert.equal(301, r[1])
            assert.equal("/foo", r[2][1][2])
        end)
    end)
end)
