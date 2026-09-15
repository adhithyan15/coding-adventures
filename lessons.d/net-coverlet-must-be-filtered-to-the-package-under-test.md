---
category: Testing & coverage
---

# .NET coverlet must be filtered to the package under test

: `/p:Include=[CodingAdventures.PaintInstructions]*`. Otherwise referenced assemblies' coverage drags down the threshold.
