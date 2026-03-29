package = "coding-adventures-event-loop"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Event emitter and tick-based scheduler — on, emit, once, off, tick, run",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.event_loop"] = "src/coding_adventures/event_loop/init.lua",
    },
}
