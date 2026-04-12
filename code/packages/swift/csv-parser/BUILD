load("@build_bazel_rules_swift//swift:swift.bzl", "swift_library", "swift_test")

swift_library(
    name = "CSVParser",
    srcs = glob(["Sources/CSVParser/**/*.swift"]),
    module_name = "CSVParser",
    visibility = ["//visibility:public"],
)

swift_test(
    name = "CSVParserTests",
    srcs = glob(["Tests/CSVParserTests/**/*.swift"]),
    deps = [
        ":CSVParser",
    ],
)
