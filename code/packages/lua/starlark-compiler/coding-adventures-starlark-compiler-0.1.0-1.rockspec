package = "coding-adventures-starlark-compiler"
version = "0.1.0-1"
source = {
  url = "git+https://github.com/adhithyan15/coding-adventures.git"
}
description = {
  summary = "Pure Starlark opcode provider and operator lookup tables for Lua",
  detailed = [[
    coding-adventures-starlark-compiler provides an in-memory, deterministic
    specification of Starlark bytecode opcodes, functional categories, and
    bidirectional operator resolution mappings conforming to STP01.
  ]],
  homepage = "https://github.com/adhithyan15/coding-adventures",
  license = "MIT"
}
dependencies = {
  "lua >= 5.1"
}
build = {
  type = "builtin",
  modules = {
    ["coding_adventures.starlark_compiler"] = "src/coding_adventures/starlark_compiler/init.lua",
    ["coding_adventures.starlark_compiler.constants"] = "src/coding_adventures/starlark_compiler/constants.lua",
    ["coding_adventures.starlark_compiler.messages"] = "src/coding_adventures/starlark_compiler/messages.lua",
    ["coding_adventures.starlark_compiler.errors"] = "src/coding_adventures/starlark_compiler/errors.lua",
    ["coding_adventures.starlark_compiler.opcodes"] = "src/coding_adventures/starlark_compiler/opcodes.lua"
  }
}
