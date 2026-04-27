-- Rockspec for coding-adventures-itf
-- ======================================

package = "coding-adventures-itf"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary  = "ITF barcode encoder — normalize, encode, expand runs, emit paint scenes",
    detailed = [[
        A dependency-free Interleaved 2 of 5 barcode package. Encodes
        digit pairs into interleaved bar/space run sequences and lays them
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
        ["coding_adventures.itf"] =
            "src/coding_adventures/itf/init.lua",
    },
}
