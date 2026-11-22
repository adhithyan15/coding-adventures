package = "coding-adventures-trig"
version = "0.3.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Trigonometric functions computed from first principles — sine, cosine, tangent via Cody-Waite reduction and Horner kernels (TRIG-REF-1)",
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
