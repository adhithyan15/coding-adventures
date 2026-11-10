package = "coding-adventures-ct-compare"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Constant-time byte comparison and selection (CT01)",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.ct_compare"] = "src/coding_adventures/ct_compare/init.lua",
    },
}
