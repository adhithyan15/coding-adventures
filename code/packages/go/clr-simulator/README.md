# CLR Simulator (Go Port)

**Layer 4-f of the computing stack** — simulates Microsoft's .NET Common Language Runtime Intermediate Language (CIL).

## Overview
Where the JVM has native `iadd`, `ladd`, `fadd` encoding the specific Primitive boundaries explicitly within the operational codes inherently, Microsoft chose an inferencing logic minimizing payload boundaries significantly. Operations like `ADD` function purely relative to the top components on the Evaluation Stack, executing type logic dynamically.
In addition, CLR aggressively maps `short` variants (`ldc.i4.0` thru 8!) removing bytes across common operations like `1` or `0` declarations entirely, alongside enabling `0xFE` extended payload indexing.

## Usage
```go
import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/clr-simulator"
)

sim := clrsimulator.NewCLRSimulator()

// Note the usage of 'OpAdd' generic rather than JVM's OpIadd typed explicitly. 
program := clrsimulator.AssembleClr([][]byte{
    clrsimulator.EncodeLdcI4(1), // explicit Int-1 constant push onto Stack
    clrsimulator.EncodeLdcI4(2), // explicit Int-2 constant push onto Stack
    {clrsimulator.OpAdd},        // evaluates generic boundaries inferring types across parameters dynamically
    clrsimulator.EncodeStloc(0), // pops value explicitly allocating against native Local array
    {clrsimulator.OpRet},        // Exits scope contexts returning back out correctly.
})

sim.Load(program, 16)
traces := sim.Run(100)
```
