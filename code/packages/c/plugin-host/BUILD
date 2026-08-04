# build-tool: deps=c/iso-harness c/platform-harness c/os-platform
# plugin-host (Unix: macOS + Linux). Builds the example plugin into a shared
# library, then builds & runs the host test (which loads it via os-platform's
# dynlib). Skips gracefully if a shared plugin cannot be built. Windows uses
# BUILD_windows (cl /LD).
sh tools/run.sh
