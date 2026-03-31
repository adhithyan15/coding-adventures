package = "coding-adventures-intel4004-simulator"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Intel 4004 behavioral simulator — the world's first commercial microprocessor (1971)",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.intel4004_simulator"] = "src/coding_adventures/intel4004_simulator/init.lua",
    },
}
