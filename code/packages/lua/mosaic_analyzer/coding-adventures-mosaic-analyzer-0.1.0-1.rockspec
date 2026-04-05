package = "coding-adventures-mosaic-analyzer"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Validates the Mosaic AST and produces a typed MosaicIR",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-mosaic-parser >= 0.1.0",
    "coding-adventures-mosaic-lexer >= 0.1.0",





}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.mosaic_analyzer"] = "src/coding_adventures/mosaic_analyzer/init.lua",
    },
}
