package = "coding-adventures-compiler-source-map"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Source map chain for the AOT compiler pipeline",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.compiler_source_map"] = "src/coding_adventures/compiler_source_map/init.lua",
    },
}
