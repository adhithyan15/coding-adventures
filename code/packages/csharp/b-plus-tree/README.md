# CodingAdventures.BPlusTree.CSharp

Pure C# B+ tree package with sorted point lookup, inclusive range scans, full scans via linked leaves, deletion, and structural validation.

```csharp
var tree = new BPlusTree<int, string>(3);
tree.Insert(10, "ten");
tree.Insert(5, "five");

var value = tree.Search(10);
var range = tree.RangeScan(5, 10);
```
