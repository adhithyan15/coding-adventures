load("code/packages/starlark/library-rules/typescript_library.star", "ts_library")

_targets = [
    ts_library(
        name = "sir-runtime-symbolic",
        srcs = ["src/**/*.ts", "tests/**/*.ts"],
        # file: deps must ALL be listed (npm install does not recursively
        # install file deps' own file deps): cas-pattern-matching (the
        # matcher/rewriter this package builds on) plus its own transitive
        # dep symbolic-ir (the term-tree type this package operates over).
        deps = ["typescript/cas-pattern-matching", "typescript/symbolic-ir"],
        test_runner = "vitest",
    ),
]
