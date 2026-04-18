package = "coding-adventures-nib-type-checker"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Nib semantic checker for Lua",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-nib-parser >= 0.1.0",
    "coding-adventures-type-checker-protocol >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.nib_type_checker"] = "src/coding_adventures/nib_type_checker/init.lua",
    },
}
