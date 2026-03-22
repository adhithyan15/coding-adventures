# fpga

A Go package implementing a simplified but structurally accurate FPGA (Field-Programmable Gate Array) model: LUTs, Slices, CLBs, switch matrices, I/O blocks, bitstream configuration, and the top-level fabric.

## Where this fits in the stack

```
Layer 0: Logic Gates (combinational + sequential)
Layer 1: Block RAM (SRAM cells, arrays, RAM modules)
Layer 2: FPGA                <-- you are here
Layer 3: Bitstream tools (place & route, synthesis)
```

An FPGA combines logic gates and block RAM into a programmable chip. By loading a bitstream, the same physical chip becomes any digital circuit.

## What's included

### Look-Up Table (`lut.go`)

| Component | Description |
|-----------|-------------|
| `LUT` | K-input Look-Up Table storing a truth table in SRAM, evaluated via MUX tree |

### Slice (`slice.go`)

| Component | Description |
|-----------|-------------|
| `Slice` | 2 LUTs + 2 flip-flops + carry chain + output MUXes |
| `SliceOutput` | Result struct with OutputA, OutputB, CarryOut |

### Configurable Logic Block (`clb.go`)

| Component | Description |
|-----------|-------------|
| `CLB` | 2 slices with inter-slice carry chain |
| `CLBOutput` | Result struct with Slice0 and Slice1 outputs |

### Switch Matrix (`switch_matrix.go`)

| Component | Description |
|-----------|-------------|
| `SwitchMatrix` | Programmable routing crossbar with Connect/Disconnect/Route |

### I/O Block (`io_block.go`)

| Component | Description |
|-----------|-------------|
| `IOBlock` | Bidirectional pad with INPUT/OUTPUT/TRISTATE modes |
| `IOMode` | Mode constants: `IOInput`, `IOOutput`, `IOTristate` |

### Bitstream (`bitstream.go`)

| Component | Description |
|-----------|-------------|
| `Bitstream` | FPGA configuration data (CLBs, routing, I/O) |
| `FromJSON` | Load bitstream from JSON file |
| `FromJSONBytes` | Parse bitstream from JSON bytes |
| `FromMap` | Create bitstream programmatically |

### Fabric (`fabric.go`)

| Component | Description |
|-----------|-------------|
| `FPGA` | Top-level fabric: creates and configures all elements from a bitstream |

## Usage

```go
import fpga "github.com/adhithyan15/coding-adventures/code/packages/go/fpga"

// Create an AND gate LUT
andTT := make([]int, 16)
andTT[3] = 1 // I0=1, I1=1 → output=1

// Build a bitstream programmatically
bs := fpga.FromMap(
    map[string]fpga.CLBConfig{
        "clb_0": {
            Slice0: fpga.SliceConfig{LutA: andTT, LutB: make([]int, 16)},
            Slice1: fpga.SliceConfig{LutA: make([]int, 16), LutB: make([]int, 16)},
        },
    },
    nil, // no routing
    map[string]fpga.IOConfig{
        "in_a": {Mode: "input"},
        "out":  {Mode: "output"},
    },
    4,
)

// Create and use the FPGA
f := fpga.NewFPGA(bs)
f.SetInput("in_a", 1)
out := f.EvaluateCLB("clb_0",
    []int{1, 1, 0, 0}, []int{0, 0, 0, 0},
    []int{0, 0, 0, 0}, []int{0, 0, 0, 0},
    0, 0,
)
// out.Slice0.OutputA == 1 (AND(1,1))
```

## Testing

```bash
go test ./... -v -cover
```

## Literate programming

All source files use Knuth-style literate programming with extensive comments explaining:
- How LUTs implement any boolean function via truth tables
- MUX tree architecture for LUT evaluation
- Slice architecture with flip-flops and carry chains
- CLB structure following Xilinx-style design
- Switch matrix crossbar routing model
- I/O block tri-state buffer operation
- Bitstream configuration format
