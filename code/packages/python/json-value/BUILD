load("code/packages/starlark/library-rules/python_library.star", "py_library")

_targets = [
    py_library(
        name = "json-value",
        srcs = ["src/**/*.py", "tests/**/*.py"],
        deps = [
            "python/grammar-tools",
            "python/directed-graph",
            "python/state-machine",
            "python/lexer",
            "python/parser",
            "python/json-lexer",
            "python/json-parser",
        ],
        test_runner = "pytest",
    ),
]
