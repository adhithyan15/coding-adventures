package = "coding-adventures-lunar-lander"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Pure, deterministic simulation core and ASCII telemetry for the classic discrete lunar landing flight simulation (GAME02)",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.lunar_lander"] = "src/coding_adventures/lunar_lander/init.lua",
    },
}
