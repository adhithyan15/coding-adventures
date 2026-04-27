# CodingAdventures.ImmutableList

Pure F# persistent list. `Push`, `Set`, and `Pop` return new list instances while the original list remains unchanged.

```fsharp
let empty = ImmutableList<string>.Empty
let one = empty.Push "hello"
let two = one.Push "world"

let updated = two.Set(0, "hi")
```
