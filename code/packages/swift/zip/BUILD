set -eu
if command -v xcrun >/dev/null 2>&1; then
  xcrun swift format lint Tests/ZipTests/PortableConformanceTests.swift
  xcrun swift test --enable-code-coverage --verbose
  xcrun swift build -c release -Xswiftc -warnings-as-errors
else
  swift format lint Tests/ZipTests/PortableConformanceTests.swift
  swift test --enable-code-coverage --verbose
  swift build -c release -Xswiftc -warnings-as-errors
fi
