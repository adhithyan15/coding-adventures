# Barcode symbologies v1 portable contract

## Status and authority

This specification freezes the portable, encoder-only contract shared by the
ITF, Code 39, Codabar, Code 128-B, UPC-A, and EAN-13 packages. The executable
source of truth is the closed corpus in
`code/specs/fixtures/barcode-symbologies-v1/`.

The contract is pure. It accepts strings and small option records and returns
immutable semantic data. It grants no filesystem, process, environment,
network, credential, clock, randomness, font, raster, native-paint, or device
authority. Layout, quiet zones, human-readable text, SVG, and paint backends
belong to `barcode-layout-1d` and `barcode-1d`, not this contract.

## Common rules

- The portable input type is a sequence of Unicode scalar values, not arbitrary
  host code units. Boundary adapters must reject ill-formed host strings such
  as unpaired UTF surrogates before calling the encoder; those values are
  outside the v1 API domain and receive no v1 stable error identifier.
- Input is measured as Unicode scalar values before normalization. At most
  4,096 scalars are accepted. A larger input fails with `input-too-long`
  before symbol lookup, checksum work, or output allocation.
- The neutral JSON corpus is limited to 96 cases, 131,072 encoded bytes, eight
  document nesting levels, 65,567 module bits per result, and 40,979 runs.
- ASCII means code points U+0000 through U+007F. Unicode digits, Unicode case
  folding, locale-sensitive folding, compatibility characters, and lone UTF
  surrogates are never substitutes for ASCII input.
- Code 39 and Codabar fold only ASCII `a` through `z` to uppercase. The other
  symbologies perform no case folding.
- Validation is atomic. A failure returns exactly one stable error identifier
  and no normalized text, symbols, modules, run lengths, or partial checksum.
- Successful module strings contain only `0` and `1`, begin with a bar (`1`),
  and are represented by the maximal alternating run lengths in the corpus.
- Stable error identifiers are payload-blind. Human-facing messages may add
  context, but callers must branch only on the identifier.

## Stable errors and precedence

The v1 error set is listed below. This list defines membership, not global
precedence:

1. `input-too-long`
2. `invalid-length`
3. `invalid-character`
4. `reserved-character`
5. `invalid-guard`
6. `guard-conflict`
7. `invalid-check-digit`

For every symbology, `input-too-long` has highest precedence. ITF, UPC-A, and
EAN-13 then check structural length, then scan the alphabet left to right, then
validate a supplied checksum. Code 39 scans left to right after the limit and
reports `reserved-character` for `*` before the generic character failure.
Codabar checks guard form, guard options, and option conflicts before scanning
the body alphabet. Code 128-B scans the printable-ASCII domain left to right
after the limit. No encoder tries to repair an invalid input.

## ITF

- Input is a non-empty, even number of ASCII digits.
- Digit patterns are the table in `itf.md`.
- Start is `1010`; stop is `11101`.
- Each digit pair interleaves five bar widths from the first digit with five
  space widths from the second digit.
- The canonical fixture projection uses one module for `N` and three modules
  for `W`. ITF does not add or validate a checksum.

## Code 39

- Empty input is valid. ASCII lowercase is folded to uppercase.
- The data alphabet is `0-9`, `A-Z`, space, `-`, `.`, `$`, `/`, `+`, and `%`.
- `*` is reserved and fails with `reserved-character`; the encoder inserts it
  as both start and stop.
- The complete 43-character table and delimiter pattern are fixed by
  `code39.md` and the executable fixture generator.
- Every symbol has nine alternating bar/space widths followed, except after
  the final stop symbol, by one narrow space.
- The canonical fixture projection uses one module for `N` and three modules
  for `W`. This ratio is an encoder projection for conformance, not a layout
  option. V1 never computes or appends a modulo-43 checksum.

## Codabar

- ASCII lowercase in the input and guard option values is folded to uppercase.
- The body alphabet is `0-9`, `-`, `$`, `:`, `/`, `.`, and `+`.
- Guards are `A`, `B`, `C`, or `D`.
- Input may be a fully guarded string, or an unguarded body plus optional
  `start` and `stop` options. Missing options default independently to `A`.
- A string with only one edge guard, an interior guard, a non-guard option, or
  a guarded string whose explicit options disagree fails without output.
- Empty body input is valid and normalizes to `AA` with default options.
- Symbols use the exact binary table in the corpus and have a one-module
  inter-character gap. Those patterns canonically use a 2:1 wide:narrow
  projection. Codabar has no v1 checksum.

## Code 128-B

- Empty input is valid.
- Every input scalar must be printable ASCII U+0020 through U+007E.
- Data value is `code_point - 32`. Start B is value 104 and stop is 106.
- Checksum is `(104 + sum(data_value[i] * (i + 1))) mod 103`.
- Values 0 through 105 use the six-width, 11-module table; stop uses the
  seven-width, 13-module pattern. V1 never switches code sets and has no FNC
  or GS1 behavior.

## UPC-A

- Exactly 11 or 12 ASCII digits are accepted.
- Eleven digits receive a computed check digit. Twelve digits must already
  carry the correct digit or fail with `invalid-check-digit`.
- The checksum and L/R tables are exactly those in `upc-a.md`.
- Output is always 95 modules: `101`, six L-coded digits, `01010`, six R-coded
  digits, and `101`.

## EAN-13

- Exactly 12 or 13 ASCII digits are accepted.
- Twelve digits receive a computed check digit. Thirteen digits must already
  carry the correct digit or fail with `invalid-check-digit`.
- The first digit selects the exact six-character L/G parity row in
  `ean-13.md`; it is not directly emitted as modules.
- Output is always 95 modules: `101`, six L/G-coded digits, `01010`, six
  R-coded digits, and `101`.

## Executable corpus

`schema.json` closes the envelope and every result variant. `cases.json`
contains exact normalizations, checksums, symbol values, parity decisions,
module strings, maximal run lengths, boundary module and run-sequence digests,
and every error class.
`generate_cases.py` is a dependency-free oracle and deterministic generator.
Repository tests perform bounded strict JSON loading, reject duplicate names,
non-standard numbers, lone surrogates, and non-local schema references,
regenerate the corpus byte-for-byte, independently pin representative
published vectors, and prove all limits and errors are exercised.

Consumers must execute every case. They may adapt names and native error
types, but may not skip cases, accept extra characters, replace stable IDs,
silently pad ITF, add a Code 39 checksum, switch Code 128 sets, or delegate to
a host barcode library.
