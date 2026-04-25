-- coding-adventures-conduit-1.0.0-1.rockspec
--
-- LuaRocks specification for the Conduit Lua package.
--
-- Note: the Rust C extension (conduit_native.so / .dll) is NOT included in this
-- rockspec. It must be built separately with `cargo build --release` from the
-- ext/conduit_native/ directory, then copied to conduit/conduit_native.so (or
-- .dll on Windows). The BUILD file in this package automates that step.

package = "coding-adventures-conduit"
version = "1.0.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures",
    tag = "v1.0.0",
}

description = {
    summary  = "Conduit web framework for Lua 5.4 — backed by Rust web-core",
    detailed = [[
        Lua 5.4 port of the Conduit web framework. Handlers are plain Lua
        functions. Routing, lifecycle hooks (before/after filters), and all HTTP
        I/O run in Rust via the conduit_native C extension.

        The framework mirrors the Ruby (WEB02) and Python (WEB03) Conduit ports:
        the same web-core engine powers all three. Lua is used here because its
        single-threaded model makes the Rust integration the simplest of all
        target languages.
    ]],
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license  = "MIT",
}

dependencies = {
    "lua >= 5.4",
}

build = {
    type = "builtin",
    modules = {
        ["conduit"]                 = "conduit/init.lua",
        ["conduit.application"]     = "conduit/application.lua",
        ["conduit.halt"]            = "conduit/halt.lua",
        ["conduit.handler_context"] = "conduit/handler_context.lua",
        ["conduit.json"]            = "conduit/json.lua",
        ["conduit.request"]         = "conduit/request.lua",
        ["conduit.server"]          = "conduit/server.lua",
    },
}
