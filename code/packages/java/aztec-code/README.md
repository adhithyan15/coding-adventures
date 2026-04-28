# coding-adventures/java/aztec-code

Aztec Code encoder — ISO/IEC 24778:2008 compliant.

## What is Aztec Code?

Aztec Code was invented in 1995 by Andrew Longacre Jr. at Welch Allyn. Unlike QR Code (three square finder patterns at three corners), Aztec Code uses a single **bullseye finder pattern at the center**. Benefits: no large quiet zone, rotation-invariant.

Where it's used:
- **IATA boarding passes** — the barcode on every airline boarding pass
- **Eurostar / Amtrak rail tickets** — printed and on-screen
- **PostNL, Deutsche Post, La Poste** — European postal routing
- **US military ID cards**

## Quick Start

```java
import com.codingadventures.aztec.AztecCode;
import com.codingadventures.barcode2d.ModuleGrid;

// Simple encode
ModuleGrid grid = AztecCode.encode("IATA BP DATA");
System.out.println(grid.rows() + "×" + grid.cols());  // e.g. 19×19

// With options
AztecCode.AztecOptions opts = new AztecCode.AztecOptions();
opts.minEccPercent = 33;
ModuleGrid grid2 = AztecCode.encode("Hello", opts);
```

## API

| Method | Description |
|--------|-------------|
| `encode(String)` | Encode with default 23% ECC |
| `encode(String, AztecOptions)` | Encode with custom options |
| `encode(byte[])` | Encode raw bytes with defaults |
| `encode(byte[], AztecOptions)` | Encode raw bytes with options |

## In the Stack

```
paint-instructions ← PaintScene rendering target
barcode-2d         ← ModuleGrid type
       ↓
aztec-code         ← this package
```
