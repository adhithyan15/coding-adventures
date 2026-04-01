package = "coding-adventures-font-parser"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary  = "Metrics-only OpenType/TrueType font parser — zero dependencies",
    detailed = [[
        Parses the tables needed for text layout (head, hhea, maxp, cmap Format 4,
        hmtx, kern Format 0, name, OS/2) from an OpenType or TrueType font binary.
        No C extensions, no external libraries.
    ]],
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license  = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.font_parser"] = "src/coding_adventures/font_parser/init.lua",
    },
}
