package = "coding-adventures-bitset"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Compact boolean bitset packed into 64-bit integers — supports AND, OR, XOR, popcount, and set iteration",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.bitset"] = "src/coding_adventures/bitset/init.lua",
    },
}
