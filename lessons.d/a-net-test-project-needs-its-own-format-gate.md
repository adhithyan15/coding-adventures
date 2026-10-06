---
category: Testing & coverage
---

# A .NET test project needs its own format gate

`dotnet format --verify-no-changes` against the C# build-tool project alone did not
check the separate xUnit project. Two test-only initializer lines still failed
WHITESPACE when the test project was checked directly. Run the format gate for
each changed project, including test projects, before publishing a PR.
