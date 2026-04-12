load("code/packages/starlark/library-rules/go_library.star", "go_library")

_targets = [
    go_library(
        name = "irc-server",
        srcs = ["**/*.go"],
        deps = ["//code/packages/go/irc-proto"],
    ),
]
