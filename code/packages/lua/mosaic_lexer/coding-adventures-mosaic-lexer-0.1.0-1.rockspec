package = "coding-adventures-mosaic-lexer"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Tokenizes .mosaic source using the grammar-driven lexer",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-grammar-tools >= 0.1.0",
    "coding-adventures-lexer >= 0.1.0",
    "coding-adventures-directed-graph >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.mosaic_lexer"] = "src/coding_adventures/mosaic_lexer/init.lua",
    },
}
