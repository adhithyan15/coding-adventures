load("code/packages/starlark/library-rules/go_program.star", "go_program")

_targets = [
    go_program(
        name = "ircd",
        srcs = ["**/*.go"],
        deps = [
            "//code/packages/go/irc-framing",
            "//code/packages/go/irc-net-stdlib",
            "//code/packages/go/irc-proto",
            "//code/packages/go/irc-server",
        ],
    ),
]
