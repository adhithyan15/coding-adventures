package = "coding-adventures-rpc"
version = "0.1.0-1"
source  = { url = "." }
description = {
  summary  = "Codec-agnostic RPC primitive — abstract server and client over pluggable codec and framer",
  homepage = "https://github.com/example/coding-adventures",
  license  = "MIT",
}
dependencies = { "lua >= 5.1" }
build = {
  type    = "builtin",
  modules = { ["coding_adventures.rpc"] = "src/coding_adventures/rpc/init.lua" },
}
