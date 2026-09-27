# Barcode symbologies v1 fixtures

This directory is the language-neutral executable contract for six pure 1D
barcode encoders: ITF, Code 39, Codabar, Code 128-B, UPC-A, and EAN-13.

- `schema.json` is the closed Draft 2020-12 schema.
- `cases.json` is the normative generated corpus.
- `generate_cases.py` contains the dependency-free tables and oracle. Run it
  with `--check` to reject drift or without flags to regenerate `cases.json`.
- `CHANGELOG.md` records contract changes.

Normal cases pin exact normalized text, binary modules, maximal alternating
run lengths, and symbology-specific metadata. Boundary cases use compact
repeat inputs plus SHA-256 digests of the normalized text, module string, and
canonical JSON run-length array so the 4,096-scalar limit is executable
without storing very large outputs. Error cases return one stable,
payload-blind ID and no partial result.

Boundary digests are SHA-256 over bytes with no terminator: normalized text is
UTF-8, module bits are ASCII, and run lengths are the UTF-8/ASCII compact JSON
array containing decimal integers separated by commas with no whitespace
(for example `[1,3,1]`).

The corpus is encoder-only and grants no host authority. Layout, quiet zones,
text, rendering, files, devices, and native backends are deliberately absent.

Regenerate or verify the committed corpus from the repository root:

```console
python code/specs/fixtures/barcode-symbologies-v1/generate_cases.py --check
```

Run the repository validation harness:

```console
python -m unittest discover -s code/scripts/tests -p 'test_barcode_symbologies_fixtures.py'
```
