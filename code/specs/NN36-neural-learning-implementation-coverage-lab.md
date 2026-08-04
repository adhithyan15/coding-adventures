# NN36: Native and Rust-core implementation coverage

## Status

Implemented.

## Purpose

NN36 records who owns the arithmetic behind each verified language lane. It
keeps the three NN34 native implementations visible beside the first NN35
Rust-core binding instead of flattening both designs into one ambiguous
"language supported" claim.

## Contract

`code/specs/fixtures/neural-learning-implementation-coverage-v1/` contains:

- `catalog.json`: the source fixture, upstream contracts, four coverage lanes,
  hand calculation, and classification rules;
- `schema.json`: the closed Draft 2020-12 interchange shape;
- `README.md` and `CHANGELOG.md`: execution and evolution notes.

Every lane still answers the NN03 weighted-neuron fixture:

```text
2 * 0.5 + (-1) * (-0.25) + 0.1 = 1.35
```

## Coverage matrix

| Lane | Language | Classification | Arithmetic owner | Boundary |
| --- | --- | --- | --- | --- |
| `go-native` | Go | native | Go | fixture JSON |
| `ruby-native` | Ruby | native | Ruby | fixture JSON |
| `rust-native` | Rust | native | Rust | fixture JSON |
| `python-ctypes-rust-core` | Python | Rust-core binding | Rust | `ctypes` to C ABI v1 |

The hand-sized coverage count is therefore:

```text
native implementations = 3
Rust-core bindings      = 1
total verified lanes    = 3 + 1 = 4
```

This is an inventory. It is not a code-quality score, performance comparison,
or claim that four separate neural-network curricula are complete.

## Executable gate

Run the complete coverage contract from the repository root:

```bash
python code/scripts/validate_neural_learning_implementation_coverage.py
```

The validator fails closed on path, lane, ownership, count, or schema drift.
It then runs all three NN34 native consumers and the real NN35 dynamic-library
call. The four-lane result is printed only after both underlying gates pass.

## Interactive trace

The **Implementation Coverage** workbench preserves the paper arithmetic while
the learner switches among lanes. For each lane it exposes the classification,
arithmetic owner, interface, source evidence, and executable validator.

The browser validates registered metadata and recomputes the hand example. It
does not claim to execute Go, Ruby, Python, or the native shared library.

## Extension rule

Add a lane only with a real executable gate. A future native consumer must own
the arithmetic named by its classification. A future binding must cross the
stable Rust ABI and prove the Rust result. Missing work should remain absent or
explicitly missing rather than being inferred from a language name.
