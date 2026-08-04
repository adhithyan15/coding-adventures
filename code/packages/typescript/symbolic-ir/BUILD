load("code/packages/starlark/library-rules/typescript_library.star", "ts_library")

_targets = [
    ts_library(
        name = "symbolic-ir",
        srcs = ["src/**/*.ts", "tests/**/*.ts"],
        deps = [],
        test_runner = "vitest",
    ),
]
