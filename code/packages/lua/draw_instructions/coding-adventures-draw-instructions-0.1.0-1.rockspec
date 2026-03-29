package = "coding-adventures-draw-instructions"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Backend-neutral drawing instruction set for visualizations — rect, text, line, circle, group, and scene",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.draw_instructions"] = "src/coding_adventures/draw_instructions/init.lua",
    },
}
