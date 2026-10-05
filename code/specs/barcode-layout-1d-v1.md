# Barcode layout 1D v1

## Status and authority

This document is the normative, language-neutral contract for the portable
integer core of `barcode-layout-1d`. The executable corpus lives in
`fixtures/barcode-layout-1d-v1`. When older package prose or tests disagree
with this document, this document and its corpus govern the portable adapter.

This contract deliberately stops at deterministic module geometry and
rectangle projection. Native paint targets, fonts, text shaping, rasterization,
SVG, devices, files, processes, environment inspection, clocks, randomness,
network access, and host barcode libraries are outside the v1 boundary.

## Limits

Adapters must enforce these ceilings before allocating output:

| Name | Value |
| --- | ---: |
| pattern Unicode scalars | 65,567 |
| emitted runs | 40,979 |
| content modules | 65,567 |
| quiet-zone modules per side | 4,096 |
| inferred or explicit symbols | 40,979 |
| label Unicode scalars | 4,096 |
| source index | signed 32-bit integer |
| caller metadata entries | 64 |
| metadata key Unicode scalars | 128 |
| metadata value Unicode scalars | 4,096 |
| aggregate metadata UTF-8 bytes | 65,536 |
| module width | 8,192 |
| bar height | 8,192 |
| foreground or background color string | 128 Unicode scalars |
| fixture cases | 64 |
| fixture bytes | 131,072 |
| fixture depth | 8 |

Every addition and multiplication is checked before it is performed. This
includes run accumulation, symbol ends, `content + 2 * quietZone`, module-to-
pixel projection, and scene dimensions. Adapters must not depend on host word
size, wrapping arithmetic, floating-point precision, or arbitrary-precision
integers. Failure is atomic and returns no partial result.

## Closed data model

A run contains:

- `color`: exactly `bar` or `space`;
- `modules`: a positive integer;
- `sourceLabel`: an unmodified scalar-valid string;
- `sourceIndex`: a signed 32-bit integer, including the established `-1`
  sentinel for synthetic guards;
- `role`: exactly `data`, `start`, `stop`, `guard`, `check`, or
  `inter-character-gap`.

Adapter ingress rejects ill-formed Unicode, including unpaired UTF-16
surrogates, before production dispatch. A marker is one Unicode scalar, not
one UTF-16 code unit. Overlong labels or out-of-range source indexes fail with
`invalid-source-attribution`.

Run lists may be empty for layout and scene projection. Nonempty run lists must
strictly alternate colors. Returned runs, layouts, rectangles, and metadata
must be fresh deep-owned values; no result may alias caller input or a prior
result.

## Pattern expansion

### Binary patterns

`expand-binary` accepts a nonempty ASCII `0`/`1` pattern. `1` is a bar and `0`
is a space. Adjacent equal bits coalesce into one maximal run. Empty patterns
fail with `empty-pattern`; all other characters fail with
`invalid-binary-token`.

### Width patterns

`expand-width` accepts a nonempty pattern, two distinct one-scalar markers,
positive bounded narrow and wide module counts, and a starting color. Defaults
are `N`, `W`, `1`, `3`, and `bar`. Each marker emits one run and the color
alternates after every marker. Bad markers fail with `invalid-width-token`;
identical or malformed marker configuration fails with
`invalid-marker-configuration`.

Both operations reject an overlong compact or repeated pattern before token
inspection. Run and content ceilings are checked incrementally before append.
The fixture-only `repeat` form denotes `token` repeated `count` times followed
by the optional `suffix`; adapters expand it only after checking its scalar
length.

## Layout

`compute-layout` first validates run structure left to right, including module
counts, alternation, run count, and checked content width. It then validates a
strictly positive quiet zone no greater than 4,096.

The output is:

```text
leftQuietZoneModules  = quietZoneModules
rightQuietZoneModules = quietZoneModules
contentModules        = sum(run.modules)
totalModules          = quietZoneModules + contentModules + quietZoneModules
```

### Inferred symbols

Non-gap runs start or continue a symbol only when the exact tuple
`(sourceLabel, sourceIndex, role)` matches the current symbol. A change closes
the current half-open span and starts a new one. An `inter-character-gap` does
not create a symbol; after a symbol starts, its modules extend that preceding
symbol's exclusive end. Leading gaps belong to no symbol. Repeated identical
tuples separated by a different tuple produce distinct symbols.

### Explicit symbols

An adapter may instead receive ordered descriptors containing `label`,
positive `modules`, `sourceIndex`, and a non-gap role. Their checked widths
must sum exactly to `contentModules`; otherwise the operation fails with
`symbol-width-mismatch`.

## Rectangle scene projection

`project-scene` uses integer `moduleWidth`, integer `barHeight`, symmetric
quiet zones, foreground, background, an optional label, and bounded
string-to-string caller metadata. Defaults are `4`, `120`, `10`, `#000000`,
`#ffffff`, and `1D barcode`.

Spaces advance the module cursor without emitting output. Each bar emits one
rectangle with:

```text
x      = moduleStart * moduleWidth
y      = 0
width  = run.modules * moduleWidth
height = barHeight
```

The scene width is `totalModules * moduleWidth`; its height is `barHeight`.
The portable projection never emits text. Supplying non-null human-readable
text or enabling it fails with `human-readable-text-unsupported` before any
font, backend, filesystem, environment, or platform work.
Foreground and background strings are each limited to 128 Unicode scalars;
longer values fail with `invalid-render-config`.

## Metadata

Caller metadata is bounded string-to-string data. The 65,536-byte aggregate
limit applies to the caller map before canonical keys are inserted. Canonical numeric values use
unsigned base-10 ASCII without signs, exponents, padding, or locale rules.
Canonical keys overwrite caller collisions. Maps are semantically unordered;
when a fixture digest is computed, object keys are serialized in ascending
Unicode code-point order.

Scene keys are `label`, `leftQuietZoneModules`, `rightQuietZoneModules`,
`contentModules`, `totalModules`, `moduleWidthPx`, `barHeightPx`,
`sceneWidthPx`, `sceneHeightPx`, and `symbolCount`. Rectangle keys are
`sourceLabel`, `sourceIndex`, `role`, `moduleStart`, and `moduleEnd`.
Backend names, host paths, font identifiers, and human-readable text are not
portable metadata.

## Stable errors and precedence

Errors are payload-blind identifiers:

`pattern-too-long`, `empty-pattern`, `invalid-binary-token`,
`invalid-width-token`, `invalid-marker-configuration`,
`invalid-module-count`, `invalid-source-attribution`, `too-many-runs`, `content-too-wide`,
`non-alternating-runs`, `invalid-quiet-zone`, `too-many-symbols`,
`symbol-width-mismatch`, `invalid-render-config`, `metadata-too-large`, and
`human-readable-text-unsupported`.

Fixture byte/depth/type/scalar envelopes are rejected by the loader before an
operation is dispatched. Within `expand-binary` and `expand-width`, pattern
length precedes token and source validation. Within `compute-layout`, run
validation precedes quiet-zone and symbol validation. Within `project-scene`,
text rejection precedes render configuration, which precedes layout and then
metadata validation. Metadata validation is last.
Structural JSON type, enum, duplicate-key, scalar, size, and depth failures
belong to the bounded fixture loader rather than the production taxonomy.

Large expected run lists use `runDigest`. Its `runsSha256` is SHA-256 over the
UTF-8 bytes of the complete run array encoded as compact JSON: no insignificant
whitespace, object keys in ascending Unicode code-point order, arrays in run
order, JSON strings without ASCII escaping, and lowercase hexadecimal digest
text. `runCount`, `contentModules`, `firstRun`, and `lastRun` remain explicit so
an adapter cannot satisfy the digest with an empty or truncated projection.

## Zero-authority requirement

The portable adapter's capability profile is empty. Conformance requires a
portable entry point and a call-graph or injected-resolver test proving that
both text request forms fail before native font resolution or shaping; an empty
package capability manifest alone is not sufficient evidence. Loading fixtures, parsing
JSON, and hashing boundary projections are test-only operations and grant no
production authority.

## Adoption evidence

`fixtures/barcode-layout-1d-v1/targets.json` is an evidence registry, not a
planning checklist. A `pending-adoption` entry may name its intended test path
and known divergences, but it must not carry a revision, corpus digest, pull
request, executed-test path, or zero-authority proof. A `conformant` entry has
no known divergence and must carry all of those promotion fields.

The corpus digest is SHA-256 over the raw checked-in `cases.json` bytes. The
verified revision is a reachable evidence commit whose package source and
conformance test match the bytes executed before publication. After a squash
merge, it may be the durable merge commit when that commit preserves the tested
package tree. `package_tree` is that commit's Git tree ID for
the canonical package root and must equal the package tree in the checked-out
adoption commit. This binds the tested package bytes; the repository contract
job fetches history so it can resolve the exact implementation revision rather
than trusting the registry claim. Both the revision and adoption PR are checked
against the durable backlog owner rather than the mutable active-PR pointer.
The adoption PR carries those exact bytes and the registry transition. The
executed test path must exist inside the target package, consume every corpus
case dynamically, and be tracked in the checked-out adoption commit. The
package root must be the canonical root for the declared language.

The capability manifest must exist, validate against the repository schema,
name the same language and package, and contain an empty `capabilities` array.
Zero-authority evidence is structured: it names the executed conformance test
and both required assertions, `text-value-fails-before-native-resolution` and
`text-enabled-fails-before-native-resolution`. The repository gate verifies
the manifest and evidence; neither a free-form claim nor an empty manifest by
itself promotes a target.
