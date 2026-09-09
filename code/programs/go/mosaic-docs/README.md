# mosaic-docs

Generates the Mosaic component documentation site: a landing page listing every
component, and a page per component.

## Why it is generated

The site must republish as soon as a component ships (#14026). A
hand-maintained catalog drifts the moment a component lands, and a stale
catalog is worse than none — it looks authoritative and is wrong. So every page
is derived from the packages on disk, and the only way to change the site is to
change a component.

## Where the data comes from

Two calls to `mosaic-compile` per component, both to the real compiler rather
than a regex over the sources:

| Call | Gives |
| --- | --- |
| `--describe --interface X.mil` | the declared surface: slots, types, and the closed value set of every `one-of` axis |
| `--backend html --emit-project` | a real runnable project, iframed into the page |

Parsing `.mil` here instead would re-implement part of `mosmodel-compiler` in a
second language and drift the moment the grammar changes. It changed twice
recently.

## The preview is the component, not a picture of it

Each page iframes the emitted html project, which loads its own runtime and
hydrates its own `{{slot}}` markers. Inlining the raw fragment would show
unresolved placeholders; substituting them here would duplicate that runtime.

Slot values are samples — the first legal member of each `one-of` axis, and the
slot's own name for text — because components have no stories yet. Once they
do, a page can show a real variant gallery (#14459 made fixtures reach the
compiler).

## Usage

```bash
go run . --root <repo-root> --out public-mosaic --compiler <path-to-mosaic-compile>
```

A component that cannot be emitted gets its page with the compiler's own error
on it, rather than being silently absent from the catalog.

## Backend style coverage

`coverage.html` records which mosstyle properties each backend actually lowers.
It exists because mosstyle properties are freeform — the compiler declares an
`UnknownProperty` error kind it never raises — so each emitter translates the
subset it knows and silently drops the rest. Nothing warns, so the gap is
invisible unless something goes and looks.

It is measured on every build rather than written down, because a number
maintained by hand stops being a measurement the day after it is written.

**Method.** Differential: emit the same probe component with and without a
property and compare the whole generated output. If nothing changes, the
backend does not lower it. Text-matching the authored value fails in both
directions — SwiftUI writes `#abcdef` as `Color(red: 0.671, …)`, so colours
read as unsupported; and a short value like `3` matches unrelated output, so
unsupported properties read as supported.

Two things the method has to get right, both found by disagreeing with a
hand-run measurement:

- **Probe shape.** `gap` on Qt and `align` on XAML lower only inside a `Row`,
  and font properties only reach a text part. Each property is tried on a `Box`
  probe and a `Row` probe, and counts as lowered if either changes.
- **Probe value.** A value equal to the backend's own default changes nothing
  even where support is complete. The most-authored `background` in this
  repository is `transparent`, which on its own reported Qt and Flutter as
  unable to paint a background. Up to three authored values are tried.

Both the property list and the probe values are censused from the repository's
own `.msl` files, so the report cannot test a property nobody declares or a
value nobody writes, and rows are ordered by real usage.

**Limits.** A mark means the declaration changes the generated output. It does
not prove the result is visually correct, and it does not cover value-level
gaps — a backend may lower `border-style` but honour only `solid`.

Pass `-coverage=false` to skip it; it costs a few hundred extra compiler
invocations.
