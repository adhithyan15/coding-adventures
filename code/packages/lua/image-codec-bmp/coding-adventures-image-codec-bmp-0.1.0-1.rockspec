package = "coding-adventures-image-codec-bmp"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "IC01: BMP image encoder and decoder — 32-bit RGBA, top-down, BI_RGB",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-pixel-container >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.image_codec_bmp"] = "src/coding_adventures/image_codec_bmp/init.lua",
    },
}
