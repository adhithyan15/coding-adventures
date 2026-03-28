package = "coding-adventures-actor"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Actor model implementation — isolated actors, message passing, spawning, and supervised execution",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.actor"] = "src/coding_adventures/actor/init.lua",
    },
}
