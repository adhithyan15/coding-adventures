-- test_halt.lua — Tests for conduit.halt
--
-- Covers: HaltError table construction, raise(), is_halt_error().

-- Adjust the package path so modules are found when running from tests/.
package.path  = "../?.lua;../?/init.lua;" .. package.path
package.cpath = "../?.so;../?.dll;" .. package.cpath

local halt_mod = require("conduit.halt")

describe("conduit.halt", function()

    -- -----------------------------------------------------------------------
    -- halt.new()
    -- -----------------------------------------------------------------------

    describe("new()", function()
        it("returns a table with __conduit_halt = true", function()
            local e = halt_mod.new(503, "Under maintenance")
            assert.is_true(e.__conduit_halt)
        end)

        it("stores the status code", function()
            local e = halt_mod.new(403, "Forbidden")
            assert.equal(403, e.status)
        end)

        it("stores the body", function()
            local e = halt_mod.new(503, "Down for maintenance")
            assert.equal("Down for maintenance", e.body)
        end)

        it("defaults body to empty string when nil", function()
            local e = halt_mod.new(200)
            assert.equal("", e.body)
        end)

        it("defaults headers to empty table when nil", function()
            local e = halt_mod.new(200, "")
            assert.same({}, e.headers)
        end)

        it("stores custom headers", function()
            local hdrs = {{"location", "/"}}
            local e = halt_mod.new(301, "", hdrs)
            assert.same({{"location", "/"}}, e.headers)
        end)

        it("multiple halt errors are independent tables", function()
            local a = halt_mod.new(200, "a")
            local b = halt_mod.new(404, "b")
            assert.not_equal(a, b)
            assert.equal("a", a.body)
            assert.equal("b", b.body)
        end)
    end)

    -- -----------------------------------------------------------------------
    -- halt.raise()
    -- -----------------------------------------------------------------------

    describe("raise()", function()
        it("raises an error when called", function()
            local ok = pcall(halt_mod.raise, 403, "Forbidden")
            assert.is_false(ok)
        end)

        it("the raised error is a HaltError table", function()
            local ok, err = pcall(halt_mod.raise, 403, "Forbidden")
            assert.is_false(ok)
            assert.is_table(err)
            assert.is_true(err.__conduit_halt)
        end)

        it("raised HaltError has the correct status", function()
            local ok, err = pcall(halt_mod.raise, 503, "Maintenance")
            assert.is_false(ok)
            assert.equal(503, err.status)
        end)

        it("raised HaltError has the correct body", function()
            local ok, err = pcall(halt_mod.raise, 503, "Maintenance")
            assert.is_false(ok)
            assert.equal("Maintenance", err.body)
        end)

        it("raised HaltError has custom headers", function()
            local ok, err = pcall(halt_mod.raise, 301, "", {{"location", "/redirect"}})
            assert.is_false(ok)
            assert.same({{"location", "/redirect"}}, err.headers)
        end)

        it("raise with no body defaults to empty string", function()
            local ok, err = pcall(halt_mod.raise, 204)
            assert.is_false(ok)
            assert.equal("", err.body)
        end)
    end)

    -- -----------------------------------------------------------------------
    -- halt.is_halt_error()
    -- -----------------------------------------------------------------------

    describe("is_halt_error()", function()
        it("returns true for a HaltError table", function()
            local e = halt_mod.new(200, "")
            assert.is_true(halt_mod.is_halt_error(e))
        end)

        it("returns true for a raised HaltError caught by pcall", function()
            local ok, err = pcall(halt_mod.raise, 400, "Bad")
            assert.is_false(ok)
            assert.is_true(halt_mod.is_halt_error(err))
        end)

        it("returns false for a plain string error", function()
            local ok, err = pcall(error, "something went wrong")
            assert.is_false(ok)
            assert.is_false(halt_mod.is_halt_error(err))
        end)

        it("returns false for a plain table without __conduit_halt", function()
            assert.is_false(halt_mod.is_halt_error({ status = 200 }))
        end)

        it("returns false for nil", function()
            assert.is_false(halt_mod.is_halt_error(nil))
        end)

        it("returns false for a number", function()
            assert.is_false(halt_mod.is_halt_error(42))
        end)

        it("returns false for a table with __conduit_halt = false", function()
            assert.is_false(halt_mod.is_halt_error({ __conduit_halt = false, status = 200 }))
        end)
    end)
end)
