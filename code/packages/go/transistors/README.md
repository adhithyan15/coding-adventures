# transistors

A Go package implementing transistor-level circuit simulation — the layer between raw semiconductor physics and digital logic gates. Port of the Python `transistors` package.

## Where this fits in the stack

```
Layer -1: Transistors    <-- you are here
Layer 0:  Logic Gates
Layer 1:  Arithmetic (adders, multipliers)
Layer 2:  ALU
Layer 3:  CPU / GPU control
```

Transistors are the physical devices that implement logic gates. This package simulates how MOSFET and BJT transistors work, how they combine into CMOS and TTL gates, and how to analyze their electrical properties.

## What's included

### MOSFET Transistors (`mosfet.go`)

NMOS and PMOS transistors with full operating region detection and current calculation:

```go
nmos := transistors.NewNMOS(nil) // default 180nm process
region := nmos.Region(1.5, 3.0)  // "saturation"
ids := nmos.DrainCurrent(1.5, 3.0)
gm := nmos.Transconductance(1.5, 3.0)
```

### BJT Transistors (`bjt.go`)

NPN and PNP transistors with Ebers-Moll current model:

```go
npn := transistors.NewNPN(nil) // default 2N2222-style
ic := npn.CollectorCurrent(0.7, 3.0)
ib := npn.BaseCurrent(0.7, 3.0) // ic/beta
```

### CMOS Logic Gates (`cmos_gates.go`)

Six gates built from MOSFET pairs with full electrical simulation:

| Gate | Transistors | Construction |
|------|-------------|--------------|
| NOT  | 2 | 1 NMOS + 1 PMOS |
| NAND | 4 | 2 NMOS series + 2 PMOS parallel |
| NOR  | 4 | 2 NMOS parallel + 2 PMOS series |
| AND  | 6 | NAND + NOT |
| OR   | 6 | NOR + NOT |
| XOR  | 6 | 4 NANDs |

### TTL/RTL Gates (`ttl_gates.go`)

Historical BJT-based logic demonstrating why CMOS replaced TTL.

### Analysis (`analysis.go`, `amplifier.go`)

- Noise margins (CMOS vs TTL)
- Power analysis (static + dynamic)
- Timing analysis (propagation delay, rise/fall time)
- CMOS technology scaling (180nm to 3nm)
- Common-source and common-emitter amplifier analysis

## Running tests

```bash
go test ./... -v -cover
```

105 tests, 95.2% coverage.
