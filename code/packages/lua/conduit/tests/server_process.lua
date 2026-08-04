-- Run the Conduit E2E fixture in a dedicated foreground Lua process.

package.path = "../?.lua;../?/init.lua;" .. package.path
package.cpath = "../?.so;../?.dll;" .. package.cpath

local conduit = require("conduit")
local fixture = dofile("server_fixture.lua")
local app = fixture.build_app()
local server

app:get("/__test_running", function()
    return conduit.json({ running = server:running() })
end)

app:get("/__test_shutdown", function()
    server:stop()
    return conduit.json({ status = "stopping" })
end)

server = conduit.Server.new(app, { host = "127.0.0.1", port = 0 })

io.write("CONDUIT_READY " .. server:local_port() .. "\n")
io.flush()
server:serve()
