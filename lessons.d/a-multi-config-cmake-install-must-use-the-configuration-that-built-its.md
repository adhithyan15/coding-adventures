---
category: Cross-platform & Windows BUILD_windows
---

# A multi-config CMake install must use the configuration that built its library

Visual Studio and Ninja Multi-Config can build Debug by default while
`cmake --install` selects Release. The install then searches for a Release
artifact that was never produced, even though the build succeeded. Choose one
configuration explicitly for every dependency and consumer: set
`CMAKE_BUILD_TYPE` for single-config generators and pass `--config` to both
`cmake --build` and `cmake --install`. Also run the consumer from the
configuration subdirectory when the generator uses one, with a root-level
fallback for single-config outputs.
