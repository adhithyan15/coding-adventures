package = "coding-adventures-trig"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Trigonometric functions computed from first principles — sine, cosine, tangent via Taylor series",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.trig"] = "src/coding_adventures/trig/init.lua",
    },
}
