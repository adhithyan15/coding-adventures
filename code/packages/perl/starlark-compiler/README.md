# CodingAdventures::StarlarkCompiler (Perl)

Pure Perl in-memory Starlark opcode provider and operator lookup tables, complying with the Standalone Opcode Provider Specification (STP01).

## Overview

`CodingAdventures::StarlarkCompiler` provides the complete specification of Starlark bytecode opcodes, functional categories, and operator resolution mappings without taking runtime dependencies or binding to a parser or virtual machine.

### Key Capabilities

- **61 Canonical Opcodes**: Full coverage of Starlark bytecode operations across all 12 architectural categories.
- **256-Byte Decode Matrix**: Precomputed O(1) table mapping 8-bit unsigned values to opcodes, preserving 195 unassigned byte holes.
- **34 Operator Mappings**: Bidirectional lookup for 12 binary, 8 comparison, 12 augmented assignment, and 2 unary operators.
- **Fresh-Map Ownership**: Public mapping accessors return defensive, mutable copies, preventing caller mutations from polluting shared tables.
- **Standing Rule #20 Compliance**: 100% centralized localizable diagnostic catalog in `Messages.pm`, closed error classes, and strict payload-blindness.

---

## 5-Class Security Resilience Showcase

| Class | Vulnerability Vector | Defense Mechanism | Invariant Guaranteed |
| :--- | :--- | :--- | :--- |
| **Class 1: Bounded Memory & Input Envelope** | Denial of Service via unbounded corpus buffers or deeply recursive JSON trees | Strict `131,072` byte limit (`MAX_CORPUS_BYTES`) and recursion depth limit of `8` (`MAX_JSON_NESTING_DEPTH`) | Prevents heap exhaustion and stack overflow attacks |
| **Class 2: Lexical, Character & Encoding Integrity** | UTF-8 desynchronization, illegal surrogate injection, or UTF-8 BOM evasion | Pre-decode UTF-8 scalar inspection validating paired surrogates (`0xD800`..`0xDFFF`) and rejecting BOM | Ensures uniform Unicode interpretation across platforms |
| **Class 3: Deterministic Fresh-State Isolation** | Shared state corruption or cross-call pollution via mutable collection modification | `binary_op_map()`, `compare_op_map()`, `augmented_assign_map()`, `unary_op_map()`, and `all_ops()` return fresh copies | Prevents cross-request data poisoning |
| **Class 4: Closed Diagnostic Catalog & Payload Blindness** | Sensitive payload exfiltration or format string injection via error channels | Closed error classes with static messages from `Messages.pm` and zero caller string reflection | Eliminates payload echoing in error messages |
| **Class 5: Adversarial Transport & Mutation Resilience** | Inconsistent opcode mapping under byte or schema corruption | Strict profile and schema version gating rejecting unknown formats | Guarantees deterministic, reproducible decoding |

---

## Usage

```perl
use strict;
use warnings;
use CodingAdventures::StarlarkCompiler qw(:all);

# Inspect canonical opcodes
my $op = op_from_byte(0x01);
print $op->{symbol}, "\n";       # "LoadConst"
print $op->{category}{id}, "\n"; # "stack"

# Resolve operator mappings
my $add_op = binary_opcode('+');
print $add_op->{symbol}, "\n";   # "Add"

# Fresh map copies prevent pollution
my $map = binary_op_map();
delete $map->{'+'}; # Local mutation has zero effect on subsequent queries
```

---

## Verification & Testing

Validate syntax, test suites, and line coverage:

```bash
sh BUILD
```
