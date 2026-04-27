# CodingAdventures.SuffixTree.CSharp

Pure C# suffix tree facade with substring search, occurrence counting, all-suffix enumeration, longest repeated substring, and longest common substring helpers.

```csharp
var tree = SuffixTree.Build("banana");
var positions = tree.Search("ana");
var repeated = tree.LongestRepeatedSubstring();
```
