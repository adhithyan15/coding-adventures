-- Run the conduit-hello demo in a dedicated Lua process for the E2E suite.
--
-- A Lua state cannot be driven by the test runner while native worker threads
-- invoke callbacks on that same state. Keeping the foreground server in this
-- child process leaves its main Lua thread blocked in Server:serve(), which is
-- the production execution model, while the parent process drives HTTP.

package.path = "../../../../packages/lua/conduit/?.lua;"
    .. "../../../../packages/lua/conduit/?/init.lua;"
    .. package.path
package.cpath = "../../../../packages/lua/conduit/?.so;"
    .. "../../../../packages/lua/conduit/?.dll;"
    .. package.cpath

local conduit = require("conduit")
local demo = dofile("../hello.lua")
local app = demo.build_app()
local server

app:get("/__test_shutdown", function()
    server:stop()
    return conduit.json({ status = "stopping" })
end)

server = conduit.Server.new(app, { host = "127.0.0.1", port = 0 })

io.write("CONDUIT_READY " .. server:local_port() .. "\n")
io.flush()
server:serve()
