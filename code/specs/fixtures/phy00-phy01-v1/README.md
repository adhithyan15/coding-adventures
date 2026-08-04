# PHY00/PHY01 conformance fixtures v1

This directory is the language-neutral oracle for the first-principles
`trig` (PHY00) and simple-harmonic `wave` (PHY01) contracts. `schema.json`
uses JSON Schema Draft 2020-12. `cases/trig.json` and `cases/wave.json` are
closed, versioned corpus documents rather than executable inputs.

## Scalar encoding

JSON numbers cannot portably preserve every IEEE 754 binary64 value. In
particular, they cannot represent NaN or infinity and many parsers erase the
sign of zero. Every numeric input and expected value is therefore tagged:

```json
{"kind": "finite", "decimal": "-0"}
{"kind": "finite", "decimal": "4.9406564584124654e-324"}
{"kind": "positive-infinity"}
{"kind": "negative-infinity"}
{"kind": "nan"}
```

Consumers parse finite decimal strings directly into their IEEE 754 binary64
type and must preserve the sign of `-0`. A finite decimal must remain finite
and a mathematically nonzero decimal must not underflow to binary64 zero.
Symbols map to the corresponding non-finite value. Native JSON floating values
are forbidden throughout the corpus. Approximate tolerances must remain finite
and positive in binary64 and may not exceed `1e-10`.

## Outcomes

- `value` compares an observed scalar using `exact`, `absolute`, or `relative`
  comparison. Exact NaN means `isNaN`; exact signed zero includes its sign.
- `error` requires the operation to reject the input as `invalid-argument`.
- `accepted` applies only to a successful PHY01 construction case.
- `property` checks a bounded invariant when an exact cross-language decimal
  would be less stable than the contract. The v1 corpus uses it only for
  finite, amplitude-bounded extreme wave evaluation, including a valid
  subnormal frequency whose reciprocal rounds to positive infinity.

Case IDs are globally unique, stable, and namespaced by `phy00/` or `phy01/`.
The authoring validator rejects duplicate JSON keys, a UTF-8 BOM, non-finite
native JSON numbers, unknown properties, and unknown operations before
dispatch.

## Validation

Run the always-on contract validator:

```text
python code/scripts/tests/test_phy00_phy01_fixtures.py
```

The validator checks the schema, strict JSON shape, semantic invariants,
unique identities, a standard-library reference calculation, and exact sync
with the generated Dart representation. Dart's PHY00 and PHY01 tests decode
that canonical base64 representation rather than reading repository files.
Their loader independently checks version, suite, size, nesting, numeric
representation, scalar range, and tolerance bounds before dispatch. This keeps
both production packages and their test harnesses at zero filesystem
capability while eliminating parser ambiguity.

After editing either JSON document, regenerate the authority-free Dart input:

```text
python code/scripts/generate_phy00_phy01_dart_fixtures.py
```

The corpus now also carries exact cross-language `atan` identities for negative
zero and tiny/subnormal inputs. Package implementations reconcile those cases
before half-angle reduction, alongside the full-range square-root and
non-finite wave boundaries already represented here.

These fixtures carry data only. They grant no filesystem, process, network,
clock, environment, or publishing authority.
