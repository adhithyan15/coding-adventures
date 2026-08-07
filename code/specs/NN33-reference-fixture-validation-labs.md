# NN33: Reference Fixture Validation Catalog

## Status

Implemented.

## Purpose

NN33 closes the first cross-language-consumer roadmap item by making the full
neural-learning fixture roster executable through one strict reference gate.
It covers NN03 through NN32: 30 fixture families, 30 Python reference
validators, and 33 lab documents.

## Contract

The language-neutral fixture lives at
`code/specs/fixtures/reference-validation-v1/` and contains:

- `catalog.json`: the ordered spec, fixture-root, validator, lab-count, track,
  and oracle mapping;
- `schema.json`: a closed Draft 2020-12 interchange shape;
- `README.md` and `CHANGELOG.md`: execution and evolution notes.

The catalog is valid only when:

1. its file roster and top-level keys are exact;
2. orders are the complete sorted integer range 3 through 32;
3. every registered path is normalized, repository-relative, exists, and
   resolves below its fixed `code/specs` or `code/scripts` root;
4. specs, fixture roots, validators, and family identifiers are unique;
5. the spec and validator sets exactly match repository discovery;
6. every fixture root contains a schema and its declared number of lab JSON
   files;
7. each validator source names the fixture root it is registered to execute;
8. the total is exactly 30 families and 33 lab documents;
9. the hand-check error and tolerance result recompute honestly.

## Execution boundary

`code/scripts/validate_reference_fixture_catalog.py` validates the complete
catalog before selecting any optional diagnostic family. It then invokes each
registered validator as:

```text
[current_python_executable, absolute_validated_script_path]
```

The runner uses no shell, fixes the working directory to the repository root,
captures strict UTF-8 output, limits output size, applies a 60-second
per-validator timeout, rejects silent success, and propagates non-zero exits.

Run all families:

```bash
python code/scripts/validate_reference_fixture_catalog.py
```

Run one family after complete catalog validation:

```bash
python code/scripts/validate_reference_fixture_catalog.py --family neural-learning
```

## Interactive trace

The Reference Catalog workbench loads the same catalog, verifies the fixed
NN03-NN32 roster in TypeScript, recomputes the hand tolerance example, and lets
the learner inspect each family mapping. It labels families as registered; only
the Python CLI and CI may claim validator execution.

## Cross-language and Rust direction

Non-Python consumers must load the family fixtures directly and reproduce
their declared oracles. Future high-performance bindings may execute through a
Rust C ABI, but the catalog remains the source of fixture coverage and the
stored expectations remain the correctness oracle.

The next tranche should add thin consumers in representative language families
and record whether each lane is native or backed by the future Rust core.
