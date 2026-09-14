---
category: Testing & coverage
---

# C# package referencing a type with the same name as its namespace

needs an explicit alias: `using FieldMath = CodingAdventures.Gf256.Gf256;`. Otherwise `Gf256.*` binds to the namespace.
