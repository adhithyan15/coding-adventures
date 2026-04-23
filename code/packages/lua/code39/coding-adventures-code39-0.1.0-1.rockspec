-- Rockspec for coding-adventures-code39
-- ======================================

package = "coding-adventures-code39"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary  = "Code 39 barcode encoder — normalize, encode, expand runs, emit paint scenes",
    detailed = [[
        A dependency-free Code 39 barcode package. Encodes uppercase letters,
        digits, and special characters into Code 39's narrow/wide bar pattern,
        emits alternating bar/space run sequences, and lays them out into
        backend-neutral paint scenes.
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
        ["coding_adventures.code39"] =
            "src/coding_adventures/code39/init.lua",
    },
}
