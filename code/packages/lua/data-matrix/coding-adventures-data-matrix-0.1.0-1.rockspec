package = "coding-adventures-data-matrix"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary = "Data Matrix ECC200 encoder (ISO/IEC 16022:2006) — 30 squares + 6 rectangles, GF(256)/0x12D",
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license = "MIT",
}

dependencies = {
    "lua >= 5.4",
}

build = {
    type = "builtin",
    modules = {
        ["coding_adventures.data_matrix"] =
            "src/coding_adventures/data_matrix/init.lua",
    },
}
