package = "coding-adventures-network-stack"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Layered network protocol stack — Ethernet/IP/TCP/UDP with routing and ARP",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.network_stack"] = "src/coding_adventures/network_stack/init.lua",
    },
}
