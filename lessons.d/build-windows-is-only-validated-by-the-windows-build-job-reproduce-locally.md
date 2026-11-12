---
category: BUILD files & dependency management
---

# BUILD_windows is only validated by the Windows build job: reproduce locally by copying it to BUILD_linux

The build tool validates the build file the *current OS* would run (`BUILD_windows` on Windows, `BUILD_linux`/`BUILD` on Linux). My Linux-side `build-tool -validate-build-files` runs therefore never looked at any `BUILD_windows`, so a Perl batch passed locally and failed the Windows job: `neural-graph-vm`'s recipe referenced `../neural-network` without a `cpanfile` declaration, and `aes-modes`' older recipe ended in a trailing `\` (`t\`), which the validator rejects as a shell line continuation. Both only surfaced once a change pulled those packages into the affected set.

Fix: declare the dependency in the metadata file, and use the helper recipe form instead of a trailing-backslash command.

Next time: before pushing BUILD_windows changes, copy every changed `BUILD_windows` to `BUILD_linux` in the worktree, run `build-tool -root . -validate-build-files -emit-plan /tmp/plan.json`, grep for `validation failed`, then delete the temporary `BUILD_linux` files (they take priority over `BUILD` on Linux, so they must never be committed).
