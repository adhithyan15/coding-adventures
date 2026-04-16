package = "coding-adventures-content-addressable-storage"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Generic content-addressable storage with SHA-1 keying and LocalDiskStore backend",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-sha1",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.content_addressable_storage"] = "src/coding_adventures/content_addressable_storage/init.lua",
    },
}
