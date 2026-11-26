# Unix History Simulator (Lua)

The independent [HOS00](../../../specs/HOS00-historical-os-simulator.md)
filesystem, descriptor table and kernel are available from `require("unix_history")`.
Requires Lua 5.3 or newer; the repository runner is pinned to Lua 5.4.7. From
here, run `lua cli.lua cat /hello` for the JSON trace or `bash BUILD` for
component tests and cross-language replay.
