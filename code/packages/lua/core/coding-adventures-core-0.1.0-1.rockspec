package = "coding-adventures-core"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Complete CPU core: pipeline + memory + register file with ISA decoder injection",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-cpu-pipeline >= 0.1.0",
    "coding-adventures-cpu-simulator >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.core"] = "src/coding_adventures/core/init.lua",
    },
}
