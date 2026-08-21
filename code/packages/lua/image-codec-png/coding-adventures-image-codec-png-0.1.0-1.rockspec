package = "coding-adventures-image-codec-png"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "IC18 bounded portable PNG encoder and decoder for Lua",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-pixel-container",
    "coding-adventures-zip",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.image_codec_png"] = "src/coding_adventures/image_codec_png/init.lua",
    },
}
