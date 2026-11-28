---
category: Cross-platform & Windows BUILD_windows
---

# A POSIX install helper must locate config-specific consumer executables

The POSIX C install helper began passing an explicit `Release` configuration to
CMake build and install, but still tried to execute the consumer from the build
root. Ninja Multi-Config places that executable in `Release/`, so the helper
failed after a successful build. The fix checks the configuration-specific
location first, then supports single-config root output and `.exe` hosts.

When a CMake helper supports both generator families, verify not only that the
build and install selected the same configuration, but also that the test
helper locates the resulting executable in the generator's actual output
layout. Exercise both single- and multi-config generators.
