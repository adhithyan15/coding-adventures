load("code/packages/starlark/library-rules/typescript_library.star", "ts_library")

_targets = [
    ts_library(
        name = "sir-runtime-core",
        srcs = ["src/**/*.ts", "tests/**/*.ts"],
        # file: deps must ALL be listed (npm install does not recursively
        # install file deps' own file deps): the pairs runtime plus the
        # exceptions runtime (T2 — the div helper raises typed SirErrors through
        # sir-runtime-exceptions' raiseError).
        deps = ["typescript/sir-runtime-exceptions", "typescript/sir-runtime-pairs"],
        test_runner = "vitest",
    ),
]
