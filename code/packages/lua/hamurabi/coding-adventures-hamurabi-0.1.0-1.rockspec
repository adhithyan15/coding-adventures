package = "coding-adventures-hamurabi"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Pure, deterministic simulation core for the seminal 1968 Hamurabi mainframe game",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.hamurabi"] = "src/coding_adventures/hamurabi/init.lua",
    },
}
