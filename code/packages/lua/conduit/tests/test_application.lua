-- test_application.lua — Tests for conduit.application (Application class)
--
-- Covers: route registration (get/post/put/delete/patch), before/after filters,
-- not_found, error_handler, settings (set/get), app:routes() introspection.
--
-- Note: these tests verify the Lua-side DSL layer only. They do not start a
-- real server. Integration with the Rust dispatch layer is tested in test_server.lua.

package.path  = "../?.lua;../?/init.lua;" .. package.path
package.cpath = "../?.so;../?.dll;" .. package.cpath

local app_mod = require("conduit.application")
local Application = app_mod.Application

describe("conduit.application.Application", function()

    -- -----------------------------------------------------------------------
    -- Constructor
    -- -----------------------------------------------------------------------

    describe("new()", function()
        it("creates an Application with no routes", function()
            local app = Application.new()
            assert.not_nil(app)
            assert.same({}, app:routes())
        end)

        it("creates independent applications", function()
            local a = Application.new()
            local b = Application.new()
            a:get("/foo", function() end)
            assert.equal(1, #a:routes())
            assert.equal(0, #b:routes())
        end)
    end)

    -- -----------------------------------------------------------------------
    -- Route registration
    -- -----------------------------------------------------------------------

    describe("get(pattern, fn) — route registration", function()
        it("registers a GET route", function()
            local app = Application.new()
            app:get("/hello", function(ctx) return ctx:html("hi") end)
            local routes = app:routes()
            assert.equal(1, #routes)
            assert.equal("GET",    routes[1].method)
            assert.equal("/hello", routes[1].pattern)
        end)

        it("registers multiple GET routes", function()
            local app = Application.new()
            app:get("/a", function() end)
            app:get("/b", function() end)
            assert.equal(2, #app:routes())
        end)
    end)

    describe("post()", function()
        it("registers a POST route", function()
            local app = Application.new()
            app:post("/echo", function(ctx) return ctx:html("") end)
            local r = app:routes()
            assert.equal("POST",  r[1].method)
            assert.equal("/echo", r[1].pattern)
        end)
    end)

    describe("put()", function()
        it("registers a PUT route", function()
            local app = Application.new()
            app:put("/item/:id", function() end)
            local r = app:routes()
            assert.equal("PUT",       r[1].method)
            assert.equal("/item/:id", r[1].pattern)
        end)
    end)

    describe("delete()", function()
        it("registers a DELETE route", function()
            local app = Application.new()
            app:delete("/item/:id", function() end)
            local r = app:routes()
            assert.equal("DELETE",    r[1].method)
            assert.equal("/item/:id", r[1].pattern)
        end)
    end)

    describe("patch()", function()
        it("registers a PATCH route", function()
            local app = Application.new()
            app:patch("/item/:id", function() end)
            local r = app:routes()
            assert.equal("PATCH",     r[1].method)
            assert.equal("/item/:id", r[1].pattern)
        end)
    end)

    describe("multiple methods on the same pattern", function()
        it("can register GET and POST on the same path", function()
            local app = Application.new()
            app:get("/form",  function() end)
            app:post("/form", function() end)
            local routes = app:routes()
            assert.equal(2, #routes)
            assert.equal("GET",  routes[1].method)
            assert.equal("POST", routes[2].method)
        end)
    end)

    -- -----------------------------------------------------------------------
    -- Before/after filters
    -- -----------------------------------------------------------------------

    describe("before()", function()
        it("accepts a before filter without error", function()
            local app = Application.new()
            assert.has_no_error(function()
                app:before(function(ctx)
                    if ctx:path() == "/down" then
                        ctx:halt(503, "Down")
                    end
                end)
            end)
        end)

        it("accepts multiple before filters", function()
            local app = Application.new()
            assert.has_no_error(function()
                app:before(function() end)
                app:before(function() end)
            end)
        end)
    end)

    describe("after()", function()
        it("accepts an after filter without error", function()
            local app = Application.new()
            assert.has_no_error(function()
                app:after(function(ctx) end)
            end)
        end)
    end)

    -- -----------------------------------------------------------------------
    -- Special handlers
    -- -----------------------------------------------------------------------

    describe("not_found()", function()
        it("registers a not-found handler without error", function()
            local app = Application.new()
            assert.has_no_error(function()
                app:not_found(function(ctx)
                    return ctx:html("404", 404)
                end)
            end)
        end)

        it("replaces the previous not-found handler", function()
            local app = Application.new()
            -- Second call should not error even if a handler is already set.
            assert.has_no_error(function()
                app:not_found(function(ctx) return ctx:html("first") end)
                app:not_found(function(ctx) return ctx:html("second") end)
            end)
        end)
    end)

    describe("error_handler()", function()
        it("registers an error handler without error", function()
            local app = Application.new()
            assert.has_no_error(function()
                app:error_handler(function(ctx, err)
                    return ctx:json({ error = err }, 500)
                end)
            end)
        end)
    end)

    -- -----------------------------------------------------------------------
    -- Settings
    -- -----------------------------------------------------------------------

    describe("set() / get()", function()
        it("set() stores a string value", function()
            local app = Application.new()
            app:set("app_name", "Test App")
            assert.equal("Test App", app:get("app_name"))
        end)

        it("set() converts numbers to strings", function()
            local app = Application.new()
            app:set("max_connections", 100)
            assert.equal("100", app:get("max_connections"))
        end)

        it("get() returns nil for an unknown key", function()
            local app = Application.new()
            assert.is_nil(app:get("nonexistent_key"))
        end)

        it("set() overwrites an existing value", function()
            local app = Application.new()
            app:set("version", "1.0")
            app:set("version", "2.0")
            assert.equal("2.0", app:get("version"))
        end)

        it("settings are independent per application", function()
            local a = Application.new()
            local b = Application.new()
            a:set("name", "App A")
            b:set("name", "App B")
            assert.equal("App A", a:get("name"))
            assert.equal("App B", b:get("name"))
        end)
    end)

    -- -----------------------------------------------------------------------
    -- routes() introspection
    -- -----------------------------------------------------------------------

    describe("routes()", function()
        it("returns an empty list initially", function()
            assert.same({}, Application.new():routes())
        end)

        it("returns all registered routes in order", function()
            local app = Application.new()
            app:get("/a",  function() end)
            app:post("/b", function() end)
            app:put("/c",  function() end)
            local r = app:routes()
            assert.equal(3, #r)
            assert.equal("/a", r[1].pattern)
            assert.equal("/b", r[2].pattern)
            assert.equal("/c", r[3].pattern)
        end)
    end)
end)
