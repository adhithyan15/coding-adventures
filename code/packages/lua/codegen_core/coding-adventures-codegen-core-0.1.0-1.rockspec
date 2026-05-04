package = "coding-adventures-codegen-core"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Text backend registry for LANG VM targets",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-interpreter-ir >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.codegen_core"] = "src/coding_adventures/codegen_core/init.lua",
    },
}
