---
category: Testing & coverage
---

# F# `dict [...] :> IReadOnlyDictionary<string, obj>`

infers an intermediate `IDictionary<string, objnull>` that fails strict upcasts. Build a concrete `Dictionary<string, obj>` first, then upcast.
