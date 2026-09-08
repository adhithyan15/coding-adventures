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
