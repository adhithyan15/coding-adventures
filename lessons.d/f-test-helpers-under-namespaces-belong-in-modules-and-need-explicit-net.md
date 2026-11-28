---
category: C#
---

# F# test helpers under namespaces belong in modules and need explicit .NET imports

F# rejects top-level `let` bindings placed directly below a namespace. Put
shared test helpers inside a named module or test type. Framework-neutral test
projects also need explicit `open` declarations for .NET namespaces used by
helpers, including `System.Numerics` for `BitOperations`; do not assume another
project's imports are available.
---
category: C#
---

# F# test helpers under namespaces belong in modules and need explicit .NET imports

F# does not allow `let` value bindings directly under a namespace declaration;
place file-level test helpers in a named module or inside the test type. A
framework-neutral test project also does not inherit C# global imports, so open
the .NET namespaces that provide helpers such as `System.Numerics.BitOperations`
instead of assuming the type is globally available.
