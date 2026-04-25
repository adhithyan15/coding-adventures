# logic-gates — Java

The foundation of all digital computing: the seven fundamental logic gates, multi-input variants, and a complete proof that NAND is functionally complete (i.e., every gate can be built from NAND alone).

## What is a logic gate?

A logic gate takes one or two binary inputs (0 or 1) and produces a single binary output. Every computation a CPU performs — arithmetic, comparisons, memory addressing — ultimately reduces to combinations of these simple operations.

## Gates

| Gate | Inputs | Output rule |
|------|--------|-------------|
| `NOT(a)` | 1 | Flips the bit: 0→1, 1→0 |
| `AND(a,b)` | 2 | 1 only if BOTH are 1 |
| `OR(a,b)` | 2 | 1 if EITHER is 1 |
| `XOR(a,b)` | 2 | 1 if inputs are DIFFERENT |
| `NAND(a,b)` | 2 | NOT(AND) — only 0 when both are 1 |
| `NOR(a,b)` | 2 | NOT(OR) — only 1 when both are 0 |
| `XNOR(a,b)` | 2 | NOT(XOR) — 1 when inputs are the SAME |

## Multi-input gates

```java
LogicGates.AND_N(1, 1, 1, 0)  // → 0 (one zero kills it)
LogicGates.OR_N(0, 0, 0, 1)   // → 1 (one one saves it)
LogicGates.XOR_N(1, 1, 0, 1)  // → 1 (parity: 3 ones is odd)
```

`AND_N` and `OR_N` require at least 2 inputs. `XOR_N` accepts any number (0 inputs → 0).

## NAND functional completeness

```java
// These produce identical results to the originals:
LogicGates.nandNOT(a)      // = NAND(a, a)
LogicGates.nandAND(a, b)   // = NAND(NAND(a,b), NAND(a,b))  — 2 gates
LogicGates.nandOR(a, b)    // = NAND(NOT(a), NOT(b))          — 3 gates
LogicGates.nandXOR(a, b)   // = NAND(NAND(a,N), NAND(b,N))   — 4 gates
```

## Usage

```java
import com.codingadventures.logicgates.LogicGates;

int and = LogicGates.AND(1, 1);   // 1
int or  = LogicGates.OR(0, 1);    // 1
int xor = LogicGates.XOR(1, 1);   // 0
int not = LogicGates.NOT(0);      // 1

// Input validation
LogicGates.AND(2, 1);  // throws IllegalArgumentException: "a must be 0 or 1, got: 2"
```

## Running Tests

```bash
gradle test
```

60 tests covering all truth tables, NAND-derived gate verification, multi-input parity, input validation, and De Morgan's Law.

## Part of the Coding Adventures series

Java counterpart to the Python, Rust, Go, TypeScript, and Kotlin implementations.
