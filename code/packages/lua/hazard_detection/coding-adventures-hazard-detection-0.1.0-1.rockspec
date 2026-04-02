package = "coding-adventures-hazard-detection"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Pipeline hazard detection — data, control, and structural hazards",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.hazard_detection"] = "src/coding_adventures/hazard_detection/init.lua",
    },
}
