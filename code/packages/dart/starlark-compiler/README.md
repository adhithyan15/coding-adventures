# coding_adventures_starlark_compiler

Pure Dart in-memory Starlark opcode provider and operator lookup tables, complying with the Standalone Opcode Provider Specification (STP01).

## Overview

`coding_adventures_starlark_compiler` provides the complete specification of Starlark bytecode opcodes, functional categories, and operator resolution mappings without taking runtime dependencies or binding to a parser or virtual machine.

### Key Capabilities

- **61 Canonical Opcodes**: Full coverage of Starlark bytecode operations across all 12 architectural categories.
- **256-Byte Decode Matrix**: Precomputed O(1) table mapping 8-bit unsigned values to opcodes, preserving 195 unassigned byte holes.
- **34 Operator Mappings**: Bidirectional lookup for 12 binary, 8 comparison, 12 augmented assignment, and 2 unary operators.
- **Fresh-Map Ownership**: Public mapping accessors return defensive, mutable copies, preventing caller mutations from polluting shared tables.
- **Standing Rule #20 Compliance**: 100% centralized localizable diagnostic catalog in `messages.dart`, sealed exceptions with closed constructors, and strict payload-blindness.

---

## 5-Class Security Resilience Showcase

| Class | Vulnerability Vector | Defense Mechanism | Invariant Guaranteed |
| :--- | :--- | :--- | :--- |
| **Class 1: Bounded Memory & Input Envelope** | Denial of Service via unbounded corpus buffers or deeply recursive JSON trees | Strict `131,072` byte limit (`maxCorpusBytes`) and recursion depth limit of `8` (`maxJsonNestingDepth`) | Prevents heap exhaustion and stack overflow attacks |
| **Class 2: Lexical, Character & Encoding Integrity** | UTF-8 desynchronization, illegal surrogate injection, or UTF-8 BOM evasion | Pre-decode UTF-8 scalar inspection validating paired surrogates (`0xD800`..`0xDFFF`) and rejecting BOM | Ensures uniform Unicode interpretation across platforms |
| **Class 3: Deterministic Fresh-State Isolation** | Shared state corruption or cross-call pollution via mutable collection modification | `binaryOpMap()`, `compareOpMap()`, `augmentedAssignMap()`, `unaryOpMap()`, and `allOps()` return fresh copies | Prevents cross-request data poisoning |
| **Class 4: Closed Diagnostic Catalog & Payload Blindness** | Sensitive payload exfiltration or format string injection via exception channels | Sealed `StarlarkCompilerException` with closed constructors and static messages from `messages.dart` | Eliminates payload echoing in error messages |
| **Class 5: Adversarial Transport & Mutation Resilience** | JSON key duplication, type confusion, schema drift, or out-of-range byte injection | Custom validating reader verifying exact closed object keys, monotonic ordering, and byte bounds | Guarantees deterministic rejection of corrupted corpora |

---

## Usage

```dart
import 'package:coding_adventures_starlark_compiler/starlark_compiler.dart';

void main() {
  // Decode byte value to opcode
  final op = opFromByte(0x01);
  print(op); // Op.loadConst

  // Access category information
  print(op?.category.id); // 'stack'
  print(op?.category.camelName); // 'stack'

  // Map operators to opcodes
  final binaryAdd = binaryOpcode('+');
  print(binaryAdd); // Op.add

  // Fresh mutable map ownership
  final ops = binaryOpMap();
  ops.clear(); // Does not affect internal tables or subsequent calls
  print(binaryOpcode('+')); // Op.add
}
```

---

## Verification & Testing

Validate formatting, static analysis, conformance, and test coverage:

```bash
sh BUILD
```
