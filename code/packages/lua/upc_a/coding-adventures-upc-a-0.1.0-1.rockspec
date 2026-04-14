-- Rockspec for coding-adventures-upc-a
-- ======================================

package = "coding-adventures-upc-a"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary  = "UPC-A barcode encoder — normalize, encode, expand runs, emit paint scenes",
    detailed = [[
        A dependency-free UPC-A barcode package. Validates or computes the
        check digit, emits alternating bar/space run sequences, and lays them
        out into backend-neutral paint scenes.
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
        ["coding_adventures.upc_a"] =
            "src/coding_adventures/upc_a/init.lua",
    },
}
