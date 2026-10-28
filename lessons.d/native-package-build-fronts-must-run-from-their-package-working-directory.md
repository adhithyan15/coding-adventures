---
category: Cross-platform & Windows BUILD_windows
---

# Native package build fronts must run from their package working directory

Launching the C PowerShell build front from the repository root compiled its
binary after `Set-Location`, but later .NET relative-path resolution still used
the process's original current directory. Command tests then failed to find a
binary which had actually compiled and passed its library tests. This was a
local invocation error, not numerical drift or a compiler failure.

Run the package front with the package directory as the process working
directory, as the build tool and CI do. Do not assume PowerShell's provider
location and the process current directory are interchangeable. NN49's workflow
pins `working-directory: code/packages/c/neural-framework`; its POSIX front then
compiled both GCC and Clang and passed actual command and native resume tests.
