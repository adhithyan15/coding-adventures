rust_library(
    name = "iir-codegen-adapters",
    srcs = glob(["src/**/*.rs"]),
    deps = [
        "//code/packages/rust/interpreter-ir",
        "//code/packages/rust/codegen-core",
        "//code/packages/rust/iir-to-beam",
        "//code/packages/rust/iir-to-wasm",
        "//code/packages/rust/iir-to-jvm-class-file",
        "//code/packages/rust/iir-to-cil-bytecode",
    ],
    visibility = ["//visibility:public"],
)

rust_test(
    name = "iir-codegen-adapters-test",
    srcs = glob(["tests/**/*.rs"]),
    deps = [
        ":iir-codegen-adapters",
        "//code/packages/rust/interpreter-ir",
        "//code/packages/rust/iir-to-beam",
        "//code/packages/rust/iir-to-wasm",
        "//code/packages/rust/iir-to-jvm-class-file",
        "//code/packages/rust/iir-to-cil-bytecode",
    ],
)
