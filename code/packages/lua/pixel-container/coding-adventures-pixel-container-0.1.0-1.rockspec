package = "coding-adventures-pixel-container"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "IC00: Fixed RGBA8 pixel buffer — row-major, 0-indexed coordinates, codec interface",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.pixel_container"] = "src/coding_adventures/pixel_container/init.lua",
    },
}
