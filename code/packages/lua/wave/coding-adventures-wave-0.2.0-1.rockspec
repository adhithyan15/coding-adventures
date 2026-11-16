package = "coding-adventures-wave"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Signal and waveform generation — sine, cosine, square, sawtooth, triangle waves",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-trig >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.wave"] = "src/coding_adventures/wave/init.lua",
    },
}
