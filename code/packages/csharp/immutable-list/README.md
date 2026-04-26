# CodingAdventures.ImmutableList.CSharp

Pure C# persistent list. `Push`, `Set`, and `Pop` return new list instances while the original list remains unchanged.

```csharp
var empty = ImmutableList<string>.Empty;
var one = empty.Push("hello");
var two = one.Push("world");

var updated = two.Set(0, "hi");
```
