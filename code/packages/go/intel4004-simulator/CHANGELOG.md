# Changelog

## [0.1.0] - Unreleased

### Added
- Developed `Intel4004Simulator` disconnected structurally from explicit CPU generics due to inherent architecture differences (i.e constraints towards 4-bits solely).
- Implemented core Accumulator mapping arrays `LDM`, `XCH`, `ADD`, `SUB`.
- Traces fully reveal Accumulator manipulations alongside standard `Carry` indicators allowing observation of numeric rollovers.
- Documentation natively emphasizes how limiting logic boundaries dictated 6 unique execution steps comparative to more modern optimizations evaluating within single execution cycles for mathematical evaluations.
