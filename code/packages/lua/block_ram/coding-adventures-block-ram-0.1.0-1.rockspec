package = "coding-adventures-block-ram"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Block RAM memory built from logic gates — addressable read/write storage",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-logic-gates >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.block_ram"] = "src/coding_adventures/block_ram/init.lua",
    },
}
