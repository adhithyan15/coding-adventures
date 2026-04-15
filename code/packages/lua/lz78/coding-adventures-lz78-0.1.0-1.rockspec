package = "coding-adventures-lz78"
version = "0.1.0-1"

source = {
  url = "https://github.com/adhithyan15/coding-adventures",
}

description = {
  summary  = "LZ78 lossless compression algorithm (1978)",
  detailed = "LZ78 (Lempel & Ziv, 1978) builds an explicit trie-based dictionary. CMP01 in the coding-adventures compression series.",
  license  = "MIT",
}

dependencies = {
  "lua >= 5.1",
}

build = {
  type    = "builtin",
  modules = {
    ["coding_adventures.lz78"] = "src/coding_adventures/lz78/init.lua",
  },
}
