# ENIAC Visualizer

**Interactive visualizer showing how ENIAC did decimal arithmetic with vacuum tubes (1945).**

## 4 Tabs

| Tab | What it shows |
|-----|--------------|
| **The Triode Switch** | How a vacuum tube acts as a digital on/off switch |
| **Decade Ring Counter** | 10 tubes in a ring = one decimal digit (0-9) |
| **ENIAC Accumulator** | Chained ring counters for multi-digit decimal addition |
| **ENIAC vs Binary** | Side-by-side comparison: decimal pulse counting vs binary gate logic |

## Development

```bash
npm install
npm run dev       # Start dev server
npm run build     # Production build
npm test          # Run tests
```

## Where it fits

```
[ENIAC (1945)] → Transistors → Logic Gates → Arithmetic → CPU
```
