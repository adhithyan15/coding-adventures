package = "coding-adventures-bloom-filter"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Space-efficient probabilistic set membership",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-hash-functions >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.bloom_filter"] = "src/coding_adventures/bloom_filter/init.lua",
    },
}
