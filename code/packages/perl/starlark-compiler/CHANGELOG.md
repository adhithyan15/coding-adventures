# Changelog

## 0.1.0

- Initial release of `CodingAdventures::StarlarkCompiler` for Perl.
- Pure in-memory Perl implementation of the Starlark opcode provider and operator lookup tables conforming to STP01.
- Complete support for 61 canonical opcodes, 12 categories, 34 operator mappings, and 195 byte holes.
- 100% compliance with Standing Rule #20 (centralized diagnostics, payload blindness).
- STP01 conformance validation with 5-class security showcase, PRNG property tests, and mutation fuzzing.
