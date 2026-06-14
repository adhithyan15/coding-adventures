package = "coding-adventures-aztec-code"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary = "Aztec Code encoder (ISO/IEC 24778:2008) — compact + full symbols, GF(256)/0x12D RS ECC",
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license = "MIT",
}

dependencies = {
    "lua >= 5.4",
}

build = {
    type = "builtin",
    modules = {
        ["coding_adventures.aztec_code"] =
            "src/coding_adventures/aztec_code/init.lua",
    },
}
