package = "coding-adventures-json-serializer"
version = "0.1.0-1"
source  = { url = "." }
description = {
  summary  = "Schema-aware JSON serializer/deserializer",
  homepage = "https://github.com/example/coding-adventures",
  license  = "MIT",
}
dependencies = {
  "lua >= 5.1",
  "coding-adventures-json-value",
}
build = {
  type    = "builtin",
  modules = { ["coding_adventures.json_serializer"] = "src/coding_adventures/json_serializer/init.lua" },
}
