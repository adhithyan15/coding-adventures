if [ "$(uname -s)" = "Darwin" ]; then
  xcrun swift test --enable-code-coverage
  xcrun swift build -c release
else
  swift test --enable-code-coverage
  swift build -c release
fi
