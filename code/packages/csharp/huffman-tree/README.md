# huffman-tree

Pure C# Huffman tree construction and canonical code generation for the DT27 compression foundation.

## What It Includes

- Deterministic greedy tree construction over the existing C# `heap` package
- Prefix-code lookup, canonical code-table generation, and bit-string decoding helpers
- Inspection helpers for weight, depth, symbol count, leaves, and structural invariants

## Example

```csharp
using CodingAdventures.HuffmanTree;

var tree = HuffmanTree.Build(new[] { (65, 3), (66, 2), (67, 1) });
var codes = tree.CodeTable();

Console.WriteLine(codes[65]); // 0
Console.WriteLine(tree.DecodeAll("001011", 4).Count); // 4
```

## Development

```bash
bash BUILD
```
