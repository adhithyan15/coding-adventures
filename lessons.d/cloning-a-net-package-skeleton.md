---
category: Testing & coverage
---

# Cloning a .NET package skeleton

requires renaming `.csproj` files and setting explicit `AssemblyName`/`RootNamespace`, not just changing `PackageId`. MSBuild treats same-filename copies as the same project identity in `.artifacts`.
