load("code/packages/starlark/library-rules/go_library.star", "go_library")

_targets = [
    go_library(
        name = "image-codec-ppm",
        srcs = ["**/*.go"],
        deps = ["//code/packages/go/pixel-container"],
    ),
]
