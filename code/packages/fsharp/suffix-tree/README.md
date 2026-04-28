# CodingAdventures.SuffixTree

Pure F# suffix tree facade with substring search, occurrence counting, all-suffix enumeration, longest repeated substring, and longest common substring helpers.

```fsharp
let tree = SuffixTree.Build("banana")
let positions = tree.Search "ana"
let repeated = tree.LongestRepeatedSubstring()
```
