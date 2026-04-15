package = "coding-adventures-ls00"
version = "0.1.0-1"
source  = { url = "." }
description = {
  summary  = "Generic LSP server framework — language bridges plug in via Lua tables",
  homepage = "https://github.com/example/coding-adventures",
  license  = "MIT",
}
dependencies = {
  "lua >= 5.1",
  "coding-adventures-json-rpc >= 0.1.0",
}
build = {
  type    = "builtin",
  modules = { ["coding_adventures.ls00"] = "src/coding_adventures/ls00/init.lua" },
}
