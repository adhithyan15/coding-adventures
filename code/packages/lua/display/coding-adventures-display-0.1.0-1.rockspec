package = "coding-adventures-display"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Text-mode display driver — framebuffer, cursor management, scrolling, and screen snapshots",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.display"] = "src/coding_adventures/display/init.lua",
    },
}
