-- Rockspec for coding-adventures-ean-13
-- ======================================

package = "coding-adventures-ean-13"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary  = "EAN-13 barcode encoder — normalize, encode, expand runs, emit paint scenes",
    detailed = [[
        A dependency-free EAN-13 barcode package. Validates or computes the
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
        ["coding_adventures.ean_13"] =
            "src/coding_adventures/ean_13/init.lua",
    },
}
