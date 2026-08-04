-- Shared application fixture for the Conduit server E2E tests.

local conduit = require("conduit")

local M = {}

function M.build_app()
    local app = conduit.Application.new()
    app:set("app_name", "Conduit E2E Test")

    app:before(function(ctx)
        if ctx:path() == "/down" then
            conduit.halt(503, "Under maintenance")
        end
    end)

    app:get("/", function()
        return conduit.html("<h1>Hello from Conduit!</h1>")
    end)

    app:get("/hello/:name", function(ctx)
        local name = ctx:params()["name"]
        return conduit.json({ message = "Hello " .. name })
    end)

    app:post("/echo", function(ctx)
        return conduit.json(ctx:json_body())
    end)

    app:get("/redirect", function()
        return conduit.redirect("/", 301)
    end)

    app:get("/halt", function()
        conduit.halt(403, "Forbidden — this route always halts")
    end)

    app:get("/down", function()
        return conduit.html("this should never be reached")
    end)

    app:get("/error", function()
        error("Intentional error for testing")
    end)

    app:not_found(function(ctx)
        return conduit.json({ message = "Not Found", path = ctx:path() }, 404)
    end)

    app:error_handler(function()
        return conduit.json({ error = "Internal Server Error" }, 500)
    end)

    return app
end

return M
