package = "coding-adventures-scrypt"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "scrypt key derivation function (RFC 7914) — implemented from scratch",
    detailed = [[
        scrypt is a memory-hard password hashing and key derivation function
        designed to resist brute-force attacks using specialised hardware
        (ASICs, FPGAs, GPUs). It builds on PBKDF2-HMAC-SHA256 and the
        Salsa20/8 stream cipher core via BlockMix and ROMix.

        Implements RFC 7914 including the two official test vectors.
        All RFC 7914 §11 test vectors pass.
    ]],
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-hmac >= 0.1",
    "coding-adventures-sha256 >= 0.1",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.scrypt"] = "src/coding_adventures/scrypt/init.lua",
    },
}
