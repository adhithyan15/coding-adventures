category: Testing & coverage
---

# Validate the same byte-list snapshot that the key stores

A byte-key constructor validated a caller-supplied `List` and then copied it in
another pass. A custom list can return different values on successive reads,
so the stored payload could contain a byte that validation never saw. Copy once,
validate the owned snapshot in order, and retain that exact snapshot. Test with
a stateful `ListBase` as well as an ordinary caller list mutated after key
construction.
