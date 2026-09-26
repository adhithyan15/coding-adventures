package = "coding-adventures-der-asn1"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Bounded typed ASN.1 DER values",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-der-tlv >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.der_asn1"] = "src/coding_adventures/der_asn1/init.lua",
    },
}
