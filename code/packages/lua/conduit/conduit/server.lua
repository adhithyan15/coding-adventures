--- conduit/server.lua — Server wrapper for Conduit.
---
--- The Server binds a TCP socket and serves HTTP requests through the given
--- Application. It wraps the native `conduit_native` server userdata.
---
--- ## Blocking usage
---
---   local server = conduit.Server.new(app, { host="127.0.0.1", port=3000 })
---   server:serve()  -- blocks until stopped
---
--- ## Non-blocking usage (tests)
---
---   local server = conduit.Server.new(app, { host="127.0.0.1", port=0 })
---   server:serve_background()   -- starts a Rust background thread
---   local port = server:local_port()   -- actual bound port
---   -- ... run tests ...
---   server:stop()

local native = require("conduit.conduit_native")

local Server = {}
Server.__index = Server

--- Create a new Server bound to the given application and options.
---
---@param app  Application  Conduit Application object
---@param opts table        Options: host (string), port (integer),
---                                  max_connections (integer, default 1024)
---@return Server
function Server.new(app, opts)
    opts = opts or {}
    local host     = opts.host            or "127.0.0.1"
    local port     = opts.port            or 0
    local max_conn = opts.max_connections or 1024

    local srv = native.new_server(app._app, host, port, max_conn)
    return setmetatable({ _server = srv, _host = host }, Server)
end

--- Start serving requests, blocking the calling thread until stopped.
--- For production use. Ctrl-C or server:stop() terminates the loop.
function Server:serve()
    native.server_serve(self._server)
end

--- Start serving requests in a background Rust thread (non-blocking).
--- Intended for use in tests so the Lua test runner is not blocked.
--- Call server:stop() when done and allow a brief settle time before
--- releasing the Server reference.
function Server:serve_background()
    native.server_serve_background(self._server)
end

--- Signal the server to stop accepting new connections and shut down.
function Server:stop()
    native.server_stop(self._server)
end

--- Return the actual port the server is listening on.
--- Useful when port = 0 was given (OS assigns an ephemeral port).
---@return integer
function Server:local_port()
    return native.server_local_port(self._server)
end

--- Return true if the server is currently running.
---@return boolean
function Server:running()
    return native.server_running(self._server)
end

--- Release server resources. The server must be stopped first.
function Server:dispose()
    native.server_dispose(self._server)
end

return { Server = Server }
