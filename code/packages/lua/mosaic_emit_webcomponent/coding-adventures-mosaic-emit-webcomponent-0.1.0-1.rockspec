package = "coding-adventures-mosaic-emit-webcomponent"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Web Components backend: emits Custom Element classes from MosaicIR",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-mosaic-vm >= 0.1.0",
    "coding-adventures-mosaic-analyzer >= 0.1.0",
    "coding-adventures-mosaic-parser >= 0.1.0",
    "coding-adventures-mosaic-lexer >= 0.1.0",





}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.mosaic_emit_webcomponent"] = "src/coding_adventures/mosaic_emit_webcomponent/init.lua",
    },
}
