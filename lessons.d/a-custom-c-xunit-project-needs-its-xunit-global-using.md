---
category: C#
---

# A custom C# xUnit project needs its Xunit global using

Copying the test package references alone leaves `[Fact]` unresolved when the
test project does not use the repository's shared `Directory.Build.props`.
Declare `<Using Include="Xunit" />` in the test project, or add an explicit
`using Xunit;` to each test source. Verify a freshly restored project rather
than relying on another project directory's imports.
