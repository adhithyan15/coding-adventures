package = "coding-adventures-mosaic-emit-react"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "React backend: emits TSX functional components from MosaicIR",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-mosaic-vm >= 0.1.0",
    "coding-adventures-mosaic-analyzer >= 0.1.0",
    "coding-adventures-mosaic-parser >= 0.1.0",
    "coding-adventures-mosaic-lexer >= 0.1.0",
    "coding-adventures-grammar-tools >= 0.1.0",
    "coding-adventures-lexer >= 0.1.0",
    "coding-adventures-directed-graph >= 0.1.0",
    "coding-adventures-parser >= 0.1.0",
    "coding-adventures-state-machine >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.mosaic_emit_react"] = "src/coding_adventures/mosaic_emit_react/init.lua",
    },
}
