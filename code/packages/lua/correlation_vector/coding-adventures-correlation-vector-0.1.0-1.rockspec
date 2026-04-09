package = "coding-adventures-correlation-vector"
version = "0.1.0-1"
source  = { url = "." }
description = {
  summary  = "Correlation Vector — append-only provenance tracking for any entity",
  homepage = "https://github.com/adhithyan15/coding-adventures",
  license  = "MIT",
}
dependencies = {
  "lua >= 5.1",
  "coding-adventures-sha256",
  "coding-adventures-json-serializer",
}
build = {
  type    = "builtin",
  modules = {
    ["coding_adventures.correlation_vector"] = "src/coding_adventures/correlation_vector/init.lua",
  },
}
