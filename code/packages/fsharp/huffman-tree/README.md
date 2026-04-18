# huffman-tree

Pure F# Huffman tree construction and canonical code generation for the DT27 compression foundation.

## What It Includes

- Deterministic greedy tree construction over the existing F# `heap` package
- Prefix-code lookup, canonical code-table generation, and bit-string decoding helpers
- Inspection helpers for weight, depth, symbol count, leaves, and structural invariants

## Example

```fsharp
open CodingAdventures.HuffmanTree.FSharp

let tree = HuffmanTree.Build [ 65, 3; 66, 2; 67, 1 ]
let codes = tree.CodeTable()

printfn "%s" codes.[65] // 0
printfn "%A" (tree.DecodeAll("001011", 4))
```

## Development

```bash
bash BUILD
```
