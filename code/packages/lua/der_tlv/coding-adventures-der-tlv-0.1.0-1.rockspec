package = "coding-adventures-der-tlv"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Bounded DER identifier and definite-length framing",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.der_tlv"] = "src/coding_adventures/der_tlv/init.lua",
    },
}
