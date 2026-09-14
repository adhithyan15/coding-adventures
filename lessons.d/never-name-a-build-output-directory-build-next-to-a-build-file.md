---
category: BUILD files & dependency management
---

# NEVER name a build-output directory `build/` next to a `BUILD` file on macOS/Windows

HFS+/NTFS are case-insensitive — `build` and `BUILD` collide, so `rm -rf build` from inside the BUILD script deletes the script itself (mid-execution, with no way to recover except `git checkout HEAD -- BUILD`). Use `_build/`, `.cmake-out/`, `gradle-build/`, or any name whose case-insensitive folding doesn't match `BUILD`. This has now bitten the repo twice: Gradle (lesson #48) and CMake/C++ (mosaic-flux-qt cycle 8).
