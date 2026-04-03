package = "coding-adventures-device-driver-framework"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Device driver abstraction — character, block, and network devices with registry",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.device_driver_framework"] = "src/coding_adventures/device_driver_framework/init.lua",
    },
}
