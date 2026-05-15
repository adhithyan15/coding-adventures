py_library(
    name = "z80-gatelevel",
    srcs = glob(["src/**/*.py"]),
    deps = [
        "//code/packages/python/logic-gates:logic-gates",
        "//code/packages/python/arithmetic:arithmetic",
        "//code/packages/python/simulator-protocol:simulator-protocol",
        "//code/packages/python/z80-simulator:z80-simulator",
    ],
)

py_test(
    name = "z80-gatelevel-test",
    srcs = glob(["tests/**/*.py"]),
    deps = [
        ":z80-gatelevel",
        "//code/packages/python/z80-simulator:z80-simulator",
    ],
)
