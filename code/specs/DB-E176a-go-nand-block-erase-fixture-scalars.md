# DB-E176a — Validate scalar escapes in the Go NAND erase fixture

The published fixture reader needs the same Unicode scalar
admission as the Go NAND page-program reader. In the exact
block-erase candidate `6f7aa2fd0a8`, an isolated high
surrogate, isolated low surrogate and reversed pair in a case
name were converted to replacement characters. Each altered
fixture matched the independent erase model and replayed.
[Erase model](DB04d-nand-block-erase-cuts.md) · [Prior scalar rule](DB-E128-go-nand-page-program-fixture-scalars.md)

Apply the admitted lexical scalar rule before decoding JSON
tokens. A high surrogate needs the immediately following low
surrogate. Refuse an isolated low surrogate, reversed pair,
incomplete escape and malformed hexadecimal escape. Preserve
the existing UTF-8, size, depth and case limits and the order of
model admission before runtime replay.

Accept complete surrogate pairs, literal and escaped replacement
characters, and text containing a JSON-escaped backslash before
a literal surrogate-shaped sequence. Check exact case names.
Retain all canonical erase outcomes and the original fixture
and runtime bytes. Add a clean control that detects bypassing
scalar admission through a named failing witness.

Run both lists, race, vet, tidy, Linux/386, the fixture and
history Go fuzz targets for at least 100,000 each, and the
existing and new named mutation controls. This changes only
the Go fixture adapter, its tests and mutation proof.

## Local verification

Commit `f645338c4b3` changes the fixture test adapter and mutation
runner. The runtime and canonical vector matched their pinned
hashes. The independent reader refused nine malformed names and
preserved the exact text of seven valid names before replaying
each canonical trace. Both lists reached 100% statement coverage.
Race, vet, tidy, Linux/386, both 100,000-input Go fuzz campaigns
and all four exact controls passed. The build graph and recipes
matched the prior product export.
