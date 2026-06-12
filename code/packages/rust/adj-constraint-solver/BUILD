load("code/packages/starlark/library-rules/rust_library.star", "rust_library")

_targets = [
    rust_library(
        name = "adj-constraint-solver",
        srcs = ["src/**/*.rs", "tests/**/*.rs"],
        deps = [
            "adj-lang",
            "logic-engine",
            "cas-solve",
            "symbolic-ir",
            "constraint-core",
            "constraint-engine",
        ],
    ),
]
