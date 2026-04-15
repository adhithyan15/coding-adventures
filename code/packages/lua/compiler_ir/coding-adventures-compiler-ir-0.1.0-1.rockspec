package = "coding-adventures-compiler-ir"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "General-purpose IR type library for the AOT compiler pipeline",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.compiler_ir"] = "src/coding_adventures/compiler_ir/init.lua",
    },
}
