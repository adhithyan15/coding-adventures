# CodingAdventures.BPlusTree

Pure F# B+ tree package with sorted point lookup, inclusive range scans, full scans via linked leaves, deletion, and validation.

```fsharp
let tree = BPlusTree<int, string>(3)
tree.Insert(10, "ten")
tree.Insert(5, "five")

let value = tree.Search 10
let range = tree.RangeScan(5, 10)
```
