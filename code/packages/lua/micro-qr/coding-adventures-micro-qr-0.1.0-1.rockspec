package = "coding-adventures-micro-qr"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary = "Micro QR Code encoder (ISO/IEC 18004:2015 Annex E) — M1–M4 symbols, 4 mask patterns, RS ECC",
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license = "MIT",
}

dependencies = {
    "lua >= 5.4",
}

build = {
    type = "builtin",
    modules = {
        ["coding_adventures.micro_qr"] =
            "src/coding_adventures/micro_qr/init.lua",
    },
}
