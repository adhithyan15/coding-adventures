# Arithmetic Visualizer

**Interactive arithmetic circuit visualizer** — from half adders to ALUs to CPU execution.

## Central Theme: Everything Reduces to Addition

- **Addition**: Ripple-carry adder (chain of full adders)
- **Subtraction**: `A + NOT(B) + 1` — same adder, just flip B and set carry-in to 1
- **Multiplication**: Shift-and-add — conditionally add shifted copies of the multiplicand

The adder is the CPU's workhorse. Everything else piggybacks on it.

## 4 Tabs

| Tab | What it shows |
|-----|--------------|
| **Binary Adders** | Half adder → full adder → ripple-carry adder with carry propagation |
| **Everything is Addition** | Subtraction via two's complement, multiplication via shift-and-add |
| **The ALU** | All 6 operations (ADD/SUB/AND/OR/XOR/NOT) + condition flags |
| **CPU Step-Through** | Load a program, step through, watch registers/ALU/memory change |

## Development

```bash
npm install
npm run dev       # Start dev server
npm run build     # Production build
npm test          # Run tests
```

## Where it fits

```
Transistors → Logic Gates → [Arithmetic] → CPU → Assembler → Compiler → VM
```

## Deployment

Deployed to GitHub Pages at `https://adhithyan15.github.io/coding-adventures/arithmetic/` via the `deploy-arithmetic.yml` workflow.
