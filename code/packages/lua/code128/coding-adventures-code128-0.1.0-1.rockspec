-- Rockspec for coding-adventures-code128
-- ======================================

package = "coding-adventures-code128"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary  = "Code 128 barcode encoder — normalize, encode, expand runs, emit paint scenes",
    detailed = [[
        A dependency-free Code 128 Code Set B barcode package. Encodes
        printable ASCII, computes the weighted checksum, emits alternating
        bar/space run sequences, and lays them out into backend-neutral
        paint scenes.
    ]],
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license  = "MIT",
}

dependencies = {
    "lua >= 5.4",
    "coding-adventures-barcode-layout-1d == 0.1.0-1",
}

build = {
    type    = "builtin",
    modules = {
        ["coding_adventures.code128"] =
            "src/coding_adventures/code128/init.lua",
    },
}
