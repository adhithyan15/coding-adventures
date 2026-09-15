### Swift build-tool Windows front

- Made the Swift `BUILD_windows` front skip successfully only when Swift is
  absent. When Swift is present, native test failures now retain their nonzero
  exit status instead of being mislabeled as a missing toolchain.

