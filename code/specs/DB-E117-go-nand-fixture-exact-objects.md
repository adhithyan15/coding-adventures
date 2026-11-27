# DB-E117 — Enforce exact objects in the Go NAND fixture adapter

The exact DB01b test adapter at SHA-256
`2a2d0c18054ab26fd5779bc5eae8944757f2c3fbc6700de96a5adbdb06cfdc7e`
replays the canonical NAND vector. An independent test of the same
public package and adapter also admitted eight malformed variants.
They recased a root, geometry, state, operation or error key, added
`page: null` to erase, added unused read `data`, or added `error: null`
to a successful read. Go's typed JSON decoding accepts case-insensitive
field matches and pointer fields conflate missing values with null.
[Go NAND profile](DB01b-nand-flash-go.md)

Validate exact decoded object names and presence before typed replay.
Check the root header, geometry, initial and resulting states,
operations and nested error objects. A read has `op`, `block` and
`page`. A program also has `data`. An erase has `op` and `block`.
Each operation has exactly one of `result` and `error`. Keep the
existing required lowercase member spellings and the two intentional
null cases for the erase ceiling and successful program or erase
results. Unrecognized extras and conflicting result/error fields
receive input refusals. Preserve the bounded token walk, UTF-8,
decoded-key uniqueness, type checks and canonical fixture bytes.

Implement the repair in the package's test adapter and tests. Replay
the eight independent witnesses and add escaped exact-key controls,
recased aliases beside exact keys, canonical nulls, nested extra keys,
missing fields and conflicting outcomes. Check the fixture document
shape before executing any operations. Run both command lists, fixed
fixture-boundary mutations, native coverage, race and vet. Verify
architecture and the exact NAND-only fixture route before publishing
DB01b. The runtime digest remains
`2ed2ac4e93d4fa7321ca2477f14305d278ebbbcb0f8252ea24e2a0e70d0785fe`.

## Local verification

The repaired adapter SHA-256 is
`cba1724e304563dc16ff484838f247e3b4d34f1a3d42b9da223d838703d4ba98`.
All eight independent witnesses received a refusal and the canonical
control passed. A clean-controlled bypass exposed all eight classes.
The native suite passed at 100% runtime statement coverage with race
and vet. Three guided fuzz targets completed 100,000 inputs apiece.
Linux 386 tests ran under QEMU and the Windows amd64 tests compiled.
The test-only change preserves all 5,986 graph rows and 5,985 recipes.
