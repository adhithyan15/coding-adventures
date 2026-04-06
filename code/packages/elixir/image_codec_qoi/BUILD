load("code/packages/starlark/library-rules/elixir_library.star", "elixir_library")

_targets = [
    elixir_library(
        name = "image_codec_qoi",
        srcs = ["lib/**/*.ex", "test/**/*.exs"],
        deps = ["//code/packages/elixir/pixel_container"],
    ),
]
