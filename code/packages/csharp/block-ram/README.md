# CodingAdventures.BlockRam

A pure C# model of the memory hierarchy used by FPGA simulations: individual
SRAM cells, row-addressed arrays, synchronous single- and dual-port RAM, and a
Block RAM whose fixed capacity can be reconfigured into different width/depth
aspect ratios.

```csharp
using CodingAdventures.BlockRam;

var ram = new SinglePortRAM(256, 8, ReadMode.ReadFirst);
ram.Tick(0, 0, new[] { 1, 0, 1, 0, 1, 0, 1, 0 }, 1);
ram.Tick(1, 0, new[] { 1, 0, 1, 0, 1, 0, 1, 0 }, 1);
```

All operations are deterministic and in memory. RAM operations occur only on
the rising clock edge; dual-port writes to the same address are rejected.
