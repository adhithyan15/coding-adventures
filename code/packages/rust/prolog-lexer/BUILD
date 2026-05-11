load("code/packages/starlark/library-rules/rust_library.star", "rust_library")

_targets = [
    rust_library(
        name = "prolog-lexer",
        srcs = ["src/**/*.rs"],
        deps = [
            "//code/packages/rust/grammar-tools:grammar-tools",
            "//code/packages/rust/lexer:lexer",
        ],
    ),
]
