-- Rockspec for coding-adventures-code39
-- ======================================

package = "coding-adventures-code39"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary  = "Code 39 barcode encoder — normalize, encode, expand runs, render SVG",
    detailed = [[
        A dependency-free Code 39 barcode package. Encodes uppercase letters,
        digits, and special characters into Code 39's narrow/wide bar pattern,
        emits alternating bar/space run sequences, and renders SVG output.
        Optionally integrates with the draw-instructions package.
    ]],
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license  = "MIT",
}

dependencies = {
    "lua >= 5.4",
}

build = {
    type    = "builtin",
    modules = {
        ["coding_adventures.code39"] =
            "src/coding_adventures/code39/init.lua",
    },
}
