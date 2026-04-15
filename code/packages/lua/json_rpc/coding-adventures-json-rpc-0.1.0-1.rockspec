package = "coding-adventures-json-rpc"
version = "0.1.0-1"
source  = { url = "." }
description = {
  summary  = "JSON-RPC 2.0 server — Content-Length framed messages over stdin/stdout",
  homepage = "https://github.com/example/coding-adventures",
  license  = "MIT",
}
dependencies = { "lua >= 5.1" }
build = {
  type    = "builtin",
  modules = { ["coding_adventures.json_rpc"] = "src/coding_adventures/json_rpc/init.lua" },
}
