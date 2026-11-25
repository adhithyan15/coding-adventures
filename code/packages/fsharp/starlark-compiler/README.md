# CodingAdventures.StarlarkCompiler.FSharp

Pure F# Starlark bytecode opcode metadata provider conforming to the STP01 specification.

## Features
- Complete metadata for all 61 standard Starlark opcodes.
- 12 opcode categories with canonical IDs and nibble assignments.
- 34 operator mappings (binary, comparison, augmented assignment, unary).
- Standing Rule #20 sealed exception hierarchy and centralized error catalog.
- Fully bounded STP01 corpus reader and validator (`cases.json`).

## 5-Class Security Resilience Showcase
- **Class 1 (Memory Exhaustion / Payload Size Defense)**: Corpus readers enforce strict upper bounds (`<= 131,072` bytes) to prevent resource starvation.
- **Class 2 (Stack Overflow Defense)**: Table lookups and validation procedures run iteratively in $O(1)$ time without recursive descent.
- **Class 3 (Opcode Integrity & Byte Range Defense)**: Validates byte values (`0..255`) against the strict 256-element decode matrix containing 61 valid opcodes and 195 rejected holes.
- **Class 4 (Diagnostic Injection & Information Disclosure Defense)**: 100% compliant with Standing Rule #20; all diagnostics use static error IDs and static message templates without reflecting caller input.
- **Class 5 (State Mutation & Map Ownership Defense)**: All operator mapping getters (`binaryOpMap()`, `compareOpMap()`, `augmentedAssignMap()`, `unaryOpMap()`) return fresh mutable dictionary copies (or immutable maps), guaranteeing immutability of provider internals.
