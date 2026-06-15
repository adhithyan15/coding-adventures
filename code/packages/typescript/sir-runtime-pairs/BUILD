load("code/packages/starlark/library-rules/typescript_library.star", "ts_library")

_targets = [
    ts_library(
        name = "sir-runtime-pairs",
        srcs = ["src/**/*.ts", "tests/**/*.ts"],
        deps = [],
        test_runner = "vitest",
    ),
]
