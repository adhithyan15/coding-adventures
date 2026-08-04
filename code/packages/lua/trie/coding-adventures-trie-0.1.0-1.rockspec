package = "coding-adventures-trie"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Prefix trie with sorted enumeration and longest-prefix matching",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.trie"] = "src/coding_adventures/trie/init.lua",
    },
}
