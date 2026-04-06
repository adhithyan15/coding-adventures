# Arithmetic

This package implements binary arithmetic from fundamental logic gates.

## Overview

As Layer 2 of the `coding-adventures` stack, the `arithmetic` package constructs:

- **Half Adders**: Computes the sum of 2 bits safely using XOR and AND gates.
- **Full Adders**: Integrates a `carryIn` bit to allow sequential addition chaining.
- **Ripple Carry Adders**: Sequentially bridges Full Adders to sum multi-bit sequences.
- **ALU (Arithmetic Logic Unit)**: Provides comprehensive operations including `.add`, `.sub`, `.and`, `.or`, `.xor`, and `.not` with respective `zero`, `carry`, `negative`, and `overflow` flags.

## Dependencies

- Layer 1: `logic-gates` (for `andGate`, `orGate`, `xorGate`, `notGate`)

## Usage

```swift
import Arithmetic

let alu = ALU(bitWidth: 8)
let a = [1, 0, 0, 0, 0, 0, 0, 0] // 1
let b = [0, 1, 0, 0, 0, 0, 0, 0] // 2

let result = try alu.execute(op: .add, a: a, b: b)
// result.value == [1, 1, 0, 0, 0, 0, 0, 0] (3)
```
