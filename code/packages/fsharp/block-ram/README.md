# CodingAdventures.BlockRam.FSharp

A pure F# model of the memory hierarchy used by FPGA simulations: individual
SRAM cells, row-addressed arrays, synchronous single- and dual-port RAM, and a
Block RAM whose fixed capacity can be reconfigured into different width/depth
aspect ratios.

```fsharp
open CodingAdventures.BlockRam.FSharp

let ram = SinglePortRAM(256, 8, readMode = ReadFirst)
ram.Tick(0, 0, [| 1; 0; 1; 0; 1; 0; 1; 0 |], 1) |> ignore
ram.Tick(1, 0, [| 1; 0; 1; 0; 1; 0; 1; 0 |], 1) |> ignore
```

All operations are deterministic and in memory. RAM operations occur only on
the rising clock edge; dual-port writes to the same address are rejected.
