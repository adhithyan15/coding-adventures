load("code/packages/starlark/library-rules/typescript_library.star", "ts_library")

_targets = [
    ts_library(
        name = "sir-runtime-array",
        srcs = ["src/**/*.ts", "tests/**/*.ts"],
        # No in-repo `file:` deps — this package is a self-contained numeric
        # wrapper over Float64Array, unlike sir-runtime-symbolic/-core which
        # build on sibling term-tree/pairs packages.
        deps = [],
        test_runner = "vitest",
    ),
]
