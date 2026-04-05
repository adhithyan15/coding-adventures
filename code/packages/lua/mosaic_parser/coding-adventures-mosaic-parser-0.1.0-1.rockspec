package = "coding-adventures-mosaic-parser"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Parses Mosaic token stream into an ASTNode tree",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-mosaic-lexer >= 0.1.0",





}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.mosaic_parser"] = "src/coding_adventures/mosaic_parser/init.lua",
    },
}
