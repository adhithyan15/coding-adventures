package = "coding-adventures-feature-normalization"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Feature scaling utilities for machine-learning examples",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.feature_normalization"] = "src/coding_adventures/feature_normalization/init.lua",
    },
}
