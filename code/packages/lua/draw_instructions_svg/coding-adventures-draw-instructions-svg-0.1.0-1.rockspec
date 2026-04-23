package = "coding-adventures-draw-instructions-svg"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "SVG renderer for Lua draw instruction scenes",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-draw-instructions >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.draw_instructions_svg"] = "src/coding_adventures/draw_instructions_svg/init.lua",
    },
}
