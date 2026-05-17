# UI23 — mosaic-pipeline: Mosaic Composition Pipeline

**Status:** Planned  
**Layer:** UI  
**Depends on:** UI13 (mosmodel), UI14 (moslayout), UI15 (mosstyle), UI16 (mosaic-compiler-pipeline), UI17 (mosaic-emit-webcomponent), UI18 (mosaic-emit-html), UI20 (mosaic-emit-react), UI21 (mosaic-emit-qt), UI22 (mosaic-emit-paint)

---

## Overview

Today the Mosaic system compiles a single tuple of files — one `.mil`, one `.mll`,
one `.msl` — into one output artefact. That model works for a single-target,
single-theme application. Real products require more: a spreadsheet app needs a
desktop layout and a mobile layout; a component library ships dark and light
themes; an accessibility requirement demands a high-contrast variant; a design
system team wants to validate that every theme works with every layout before a
release ships.

This spec defines the **Mosaic Composition Pipeline** — the mechanism that
scales Mosaic from "one layout, one style" to "N named layout variants × M
named style variants, assembled into a named, versioned pipeline that compiles
them all at once."

The composition pipeline introduces:

1. **Version declarations** on all three source languages (`.mil`, `.mll`, `.msl`).
2. **Named layout variants** (`Grid.desktop`, `Grid.mobile`, `Grid.compact`) — multiple
   `.mll` files for the same component, each with its own version and semver
   compatibility constraint.
3. **Named style variants** (`Grid.dark`, `Grid.light`, `Grid.high-contrast`) — multiple
   `.msl` files targeting either an interface (any layout) or a specific layout variant.
4. **Interface-scoped vs. layout-scoped styles** — a style that targets `Grid 2.x`
   guarantees it references only parts that exist in every `Grid 2.x` layout;
   a style that targets `Grid.desktop 1.x` may reference parts specific to the
   desktop layout.
5. **The `.mospipeline` manifest** — a TOML file that selects one interface version,
   one layout variant, and one style variant per component, and names the output
   backend.
6. **The `--pipeline` CLI flag** on `mosaic-compile` — drives all compilation from
   a single manifest.
7. **Stacked style resolution** — when both interface-scoped and layout-scoped
   styles are active, a defined cascade order merges them without surprises.

---

## Position in the Stack

```
mosmodel (.mil)          ← UI13: interface + slot/emit declarations
     │  now: component <Name> version <semver>
     ▼
moslayout (.mll)         ← UI14: structural layout
     │  now: layout <Name>.<variant> version <semver> implements <Name> <range>
     ▼
mosstyle (.msl)          ← UI15: visual appearance
     │  now: style <Name>.<variant> version <semver> for <Name> <range>
     ▼
mosaic-pipeline (.mospipeline)   ← THIS SPEC
     │  assembles: interface@range + layout@range + style@range → output
     ▼
mosaic-compile --pipeline       ← THIS SPEC (CLI)
     │  resolves + validates + dispatches
     ▼
backend emitters (react, webcomponent, html, paint, qt, …)
     │
     ▼
per-component output artefacts (Grid.tsx, FormulaBar.tsx, …)
```

---

## §1 Problem Statement

### §1.1 The single-tuple limitation

In UI16, a component is compiled from exactly one `.mil` + one `.mll` + one `.msl`.
The mosaic-driver binary selects files by naming convention: `Grid.mil`,
`Grid.mll`, `Grid.msl` from the current directory. Platform-specific layout
overrides (`Grid.desktop.mll`, `Grid.mobile.mll`) are supported as an informal
naming convention in UI14 §1, but:

- There is no formal version constraint between a layout override and the
  interface it implements.
- There is no mechanism to declare which style files pair with which layout
  variants.
- There is no way to express "compile all valid (layout, style) combinations in
  one invocation and fail fast if any pairing is invalid."
- Adding a new theme means manually tracking which layout variants it covers.

### §1.2 The variant explosion

A production component library for a single application might require:

```
Grid.mil                 (interface, stable)
Grid.desktop.mll         (desktop layout)
Grid.mobile.mll          (mobile layout)
Grid.compact.mll         (compact layout for embedded panels)
Grid.dark.msl            (dark theme — works with any layout)
Grid.light.msl           (light theme — works with any layout)
Grid.high-contrast.msl   (accessibility theme)
Grid.desktop.dark.msl    (dark theme with desktop-specific overrides)
```

Eight source files, but only some combinations are valid. `Grid.desktop.dark.msl`
targets the desktop layout — it may reference parts that only `Grid.desktop.mll`
exports. Applying it with `Grid.mobile.mll` is an error. Currently the compiler
has no way to express or enforce this.

### §1.3 What this spec adds

The composition pipeline adds versioning and pairing constraints at the source
level, validated by the compiler before any code is generated. A `.mospipeline`
manifest is the declarative record of which valid pairings are used in a
particular product build.

---

## §2 Versioning Syntax Additions

Each source language gains a version declaration at the top level. Version
numbers follow **semantic versioning** (major.minor.patch). In range
specifications, a wildcard suffix (`2.x`) matches any patch-and-minor version
with the same major: `2.x` means `>=2.0.0, <3.0.0`.

### §2.1 mosmodel version declaration (`.mil`)

Current grammar (UI13 §4):
```
component_def = KW_COMPONENT IDENT LBRACE { member } RBRACE ;
```

Extended grammar:
```
component_def = KW_COMPONENT IDENT [ KW_VERSION version_num ] LBRACE { member } RBRACE ;

version_num = NUMBER DOT NUMBER DOT NUMBER
            | NUMBER DOT NUMBER ;      # patch defaults to 0
```

Example:
```mosmodel
component Grid version 2.0 {
  slot column-headers  : list<text> ;
  slot column-widths   : list<number> ;
  slot total-rows      : number ;
  slot viewport-offset : number = 0 ;
  slot viewport-rows   : list<list<text>> ;
  slot selected-row    : number = 0 ;
  slot selected-col    : number = 0 ;
  slot edit-row        : number = -1 ;
  slot edit-col        : number = -1 ;
  slot edit-content    : text = "" ;

  emit onNavigate    ( row : number , col : number ) ;
  emit onEditStart   ( row : number , col : number ) ;
  emit onEditCommit  ( value : text ) ;
  emit onEditCancel ;
  emit onScroll      ( offset : number ) ;
  emit onSelect      ( start-row : number , start-col : number ,
                       end-row   : number , end-col   : number ) ;
}
```

The `version` clause is optional. A `.mil` file without a version declaration
is treated as `version 1.0.0` by the pipeline compiler, and a warning is emitted
when used inside a `.mospipeline` manifest.

### §2.2 moslayout version declaration (`.mll`)

A layout variant declares both its own version and the interface version range
it is compatible with. The `implements` clause names the component and a version
range.

Current grammar (UI14 §6):
```
layout_def = KW_LAYOUT IDENT LBRACE { node } RBRACE ;
```

Extended grammar:
```
layout_def = KW_LAYOUT dotted_name [ KW_VERSION version_num ]
             [ KW_IMPLEMENTS IDENT version_range ]
             LBRACE { node } RBRACE ;

dotted_name   = IDENT [ DOT IDENT ] ;          # e.g. Grid or Grid.desktop
version_range = NUMBER DOT ( NUMBER | STAR ) ; # e.g. 2.x or 2.3
```

New tokens:
```
KW_VERSION     = "version"
KW_IMPLEMENTS  = "implements"
DOT            = "."
STAR           = "x"
```

Examples:

```moslayout
// Default layout — works with any Grid 2.x interface
layout Grid version 1.0 implements Grid 2.x {
  Column [ root ] {
    Grid [ cell-grid ] (
      headers:        slot: column-headers ,
      widths:         slot: column-widths  ,
      rows:           slot: viewport-rows  ,
      selected-row:   slot: selected-row   ,
      selected-col:   slot: selected-col   ,
      edit-row:       slot: edit-row       ,
      edit-col:       slot: edit-col       ,
      edit-content:   slot: edit-content   ,
      on-navigate:    emit: onNavigate     ,
      on-edit-start:  emit: onEditStart    ,
      on-edit-commit: emit: onEditCommit   ,
      on-edit-cancel: emit: onEditCancel   ,
      on-scroll:      emit: onScroll
    )
  }
}
```

```moslayout
// Desktop layout — exports additional parts (column-resizer, frozen-header)
// not present in the default layout
layout Grid.desktop version 1.3 implements Grid 2.x {
  Column [ root ] {
    Row [ frozen-header ] {
      Grid [ column-resizer ] (
        headers: slot: column-headers ,
        widths:  slot: column-widths
      )
    }
    Grid [ cell-grid ] (
      headers:        slot: column-headers ,
      widths:         slot: column-widths  ,
      rows:           slot: viewport-rows  ,
      selected-row:   slot: selected-row   ,
      selected-col:   slot: selected-col   ,
      edit-row:       slot: edit-row       ,
      edit-col:       slot: edit-col       ,
      edit-content:   slot: edit-content   ,
      on-navigate:    emit: onNavigate     ,
      on-edit-start:  emit: onEditStart    ,
      on-edit-commit: emit: onEditCommit   ,
      on-edit-cancel: emit: onEditCancel   ,
      on-scroll:      emit: onScroll
    )
  }
}
```

The dotted name `Grid.desktop` means: "this is the `desktop` layout variant
of the `Grid` component." The part before the dot is the component name; the
part after is the variant name. A layout with no dot (e.g. `Grid`) is the
*default* variant.

### §2.3 mosstyle version declaration (`.msl`)

A style variant declares its own version and the *target* it applies to. The
target is either:

- An **interface target**: `for Grid 2.x` — this style may only reference parts
  in the *intersection* of all `Grid 2.x` layout variant part sets.
- A **layout target**: `for Grid.desktop 1.x` — this style may reference any
  part exported by the `Grid.desktop` layout.

Current grammar (UI15 §7):
```
style_def = KW_STYLE IDENT LBRACE { part_def } RBRACE ;
```

Extended grammar:
```
style_def = KW_STYLE dotted_name [ KW_VERSION version_num ]
            [ KW_FOR dotted_name version_range ]
            LBRACE { part_def } RBRACE ;

# New keyword:
KW_FOR = "for"
```

Examples:

```mosstyle
// Interface-scoped style — dark theme for any Grid 2.x layout.
// May only reference parts in the intersection of ALL Grid 2.x layouts.
style Grid.dark version 2.0 for Grid 2.x {

  part root {
    background:    $color-surface ;
    border-radius: $radius-md ;
  }

  part cell-grid {
    background:   $color-surface ;
    border-color: $color-border ;
    border-width: 1px ;
  }

}
```

```mosstyle
// Layout-scoped style — dark theme extensions for the desktop layout only.
// May reference parts that only Grid.desktop exports (column-resizer,
// frozen-header) in addition to the shared parts.
style Grid.desktop.dark version 2.0 for Grid.desktop 1.x {

  part root {
    background:    $color-surface ;
    border-radius: $radius-md ;
  }

  part frozen-header {
    background:   lighten($color-surface, 4%) ;
    border-color: $color-border ;
    border-width: 0 0 1px 0 ;
  }

  part column-resizer {
    background: $color-surface ;
  }

  part cell-grid {
    background:   $color-surface ;
    border-color: $color-border ;
    border-width: 1px ;
  }

}
```

The dotted style name `Grid.desktop.dark` carries three segments: component
(`Grid`), layout variant (`desktop`), and style variant name (`dark`). The
`for` clause makes the pairing explicit and machine-checkable.

A style file with no `for` clause and no version is treated as an
interface-scoped style for `<Component> 1.x`, and a warning is emitted.

---

## §3 Interface-Scoped vs. Layout-Scoped Styles

The distinction between these two scoping modes is the most important semantic
rule in the composition pipeline. This section defines it precisely.

### §3.1 The part intersection rule for interface-scoped styles

When a style is declared `for Grid 2.x` (interface-scoped), the mosstyle
compiler validates every `part <name>` block against the **intersection** of
part names exported by all layout variants that satisfy `Grid 2.x`.

Think of it this way: an interface-scoped style is a promise that says "I will
work correctly no matter which Grid 2.x layout the host application selects."
For that promise to be enforceable at compile time, the style may only reference
parts that *all* Grid 2.x layouts have in common. If even one Grid 2.x layout
does not export a part named `column-resizer`, then no interface-scoped style
may reference `column-resizer`.

**Algorithm:**

1. Collect all `.mll` files whose `implements` clause satisfies the target
   interface version range (e.g. `Grid 2.x`).
2. Compute the part name intersection: parts that appear in *every* collected
   layout's part map.
3. Validate the style file's `part` blocks against this intersection set.
4. Any `part` block referencing a name outside the intersection is a compile
   error.

**Example:**

Suppose three layout variants implement `Grid 2.x`:

| Layout | Parts exported |
|--------|----------------|
| `Grid` (default) | `root`, `cell-grid` |
| `Grid.desktop` | `root`, `frozen-header`, `column-resizer`, `cell-grid` |
| `Grid.mobile` | `root`, `cell-grid` |

Intersection = {`root`, `cell-grid`}.

A `style Grid.dark for Grid 2.x` may only reference `root` and `cell-grid`.
Referencing `frozen-header` is a compile error, even though one layout exports it.

### §3.2 The full part set for layout-scoped styles

When a style is declared `for Grid.desktop 1.x` (layout-scoped), the mosstyle
compiler validates `part` blocks against the full part set exported by the
`Grid.desktop` layout (all versions satisfying `1.x`). The intersection rule
does not apply.

Layout-scoped styles are useful for:
- Adding visual polish to platform-specific elements that exist only in one layout.
- Overriding base interface-scoped styles at a finer granularity.
- Providing theme extensions that would be meaningless on other layouts.

### §3.3 Scoping summary table

| `for` clause | Valid part references | When to use |
|---|---|---|
| `for Grid 2.x` (interface-scoped) | Intersection of all Grid 2.x layouts | Universal themes that work with any layout |
| `for Grid.desktop 1.x` (layout-scoped) | All parts in Grid.desktop | Desktop-specific styling or overrides |
| (no `for` clause) | Treated as interface-scoped for `1.x`, with a warning | Legacy single-layout components |

---

## §4 The `.mospipeline` Manifest Format

A `.mospipeline` file is a TOML document that assembles a named, versioned
pipeline. It declares which components are compiled, which interface/layout/style
versions are used, which output backend is targeted, and where to find the
source files.

### §4.1 Complete TOML schema

```toml
# ─────────────────────────────────────────────────────────────
# [pipeline] — top-level metadata
# ─────────────────────────────────────────────────────────────

[pipeline]
name    = "visicalc-desktop-dark"   # required; used as the output directory name
version = "1.0"                     # required; version of this pipeline definition
description = "VisiCalc desktop build with dark theme"  # optional

# search-path: ordered list of directories searched for .mil/.mll/.msl files.
# Relative paths are resolved from the location of the .mospipeline file.
# Default: ["."] (the directory containing the .mospipeline file).
search-path = ["components", "../shared-components"]

# ─────────────────────────────────────────────────────────────
# [global-style] — optional; token/theme file shared across all components
# ─────────────────────────────────────────────────────────────
# Points to a Lattice token file (.lattice) applied before component-level
# styles. Useful for brand tokens, platform adaptations, and color schemes.
# Multiple files are applied in order; later files override earlier ones for
# tokens not declared with !default.

[global-style]
tokens = ["tokens/base.lattice", "tokens/dark.lattice"]

# ─────────────────────────────────────────────────────────────
# [[component]] — one block per component in the pipeline
# ─────────────────────────────────────────────────────────────
# Each [[component]] block compiles one (interface, layout, style) tuple
# into one output artefact.
#
# Fields:
#   interface   required  "<Name>@<version-range>"  — which .mil to use
#   layout      required  "<Name>.<variant>@<range>" or "<Name>@<range>"
#   style       required  "<Name>.<variant>@<range>" or "<Name>@<range>"
#   output      required  backend name: "react" | "webcomponent" | "html" |
#                           "paint" | "qt" | "swiftui" | "compose"
#   out-dir     optional  per-component output directory override
#   fixture     optional  path to slot fixture JSON (used by html/paint backends)

[[component]]
interface = "Grid@2.x"
layout    = "Grid.desktop@1.x"
style     = "Grid.desktop.dark@2.x"
output    = "react"
out-dir   = "generated/react/desktop"

[[component]]
interface = "Grid@2.x"
layout    = "Grid.mobile@1.x"
style     = "Grid.dark@2.x"
output    = "react"
out-dir   = "generated/react/mobile"

[[component]]
interface = "FormulaBar@1.x"
layout    = "FormulaBar.desktop@1.x"
style     = "FormulaBar.dark@1.x"
output    = "react"

[[component]]
interface = "FormulaBar@1.x"
layout    = "FormulaBar@1.x"
style     = "FormulaBar.dark@1.x"
output    = "paint"
fixture   = "fixtures/formula-bar.json"
```

### §4.2 Version range syntax in manifests

The `@` separator in a component spec separates the component/variant name from
its version range. The version range syntax mirrors the `implements` and `for`
clauses in the source languages:

| Range | Meaning |
|---|---|
| `2.x` | Any version `>=2.0.0, <3.0.0` |
| `1.3.x` | Any version `>=1.3.0, <1.4.0` |
| `2.1` | Exactly version `2.1.0` |
| `2.1.4` | Exactly version `2.1.4` |
| `*` | Any version (not recommended; use for rapid prototyping only) |

The pipeline compiler resolves each range to the highest matching version found
on the search path. If no file satisfies the range, compilation fails with a
`VersionNotFound` error.

### §4.3 The `search-path` resolution algorithm

For each component spec (e.g. `interface = "Grid@2.x"`):

1. Split on `@` to get `name = "Grid"` and `range = "2.x"`.
2. Scan each directory in `search-path` in order.
3. In each directory, look for files matching `Grid.mil` (for the default variant)
   or `Grid.<variant>.mll` / `Grid.<variant>.<style>.msl` (for named variants).
4. Parse the `version` header from each matching file.
5. Collect all files whose version satisfies the range.
6. Select the one with the highest version number.
7. If multiple files in different search-path directories have the same highest
   version, the one found earlier in `search-path` wins (first-directory-wins).

### §4.4 The `global-style` token layer

The `[global-style]` section specifies Lattice token files that are loaded
*before* any component-level style compilation. This is the correct place for:

- **Brand tokens** — `$color-accent`, `$font-family-body`.
- **Color scheme** — `dark.lattice` or `light.lattice`.
- **Platform tokens** — `ios-tokens.lattice` with platform-appropriate sizing.

Component-level `.msl` files declare tokens with `!default` (per UI15 §1). A
token declared in a global-style file without `!default` overrides the
component's default. A pipeline can therefore ship multiple output directories
(one for dark, one for light) by using two different `.mospipeline` files that
differ only in their `[global-style]` section.

---

## §5 CLI `--pipeline` Flag

### §5.1 Invocation

```
mosaic-compile --pipeline <path-to.mospipeline>
```

The `--pipeline` flag is a new mode of the `mosaic-compile` binary (alongside
the existing single-component mode described in UI16 §1). The two modes are
mutually exclusive: `--pipeline` and `--target` / `--interface` / `--layout` /
`--style` flags cannot be combined in one invocation.

Full flag list:

```
mosaic-compile --pipeline visicalc-desktop-dark.mospipeline
mosaic-compile --pipeline visicalc-desktop-dark.mospipeline --dry-run
mosaic-compile --pipeline visicalc-desktop-dark.mospipeline --verbose
mosaic-compile --pipeline visicalc-desktop-dark.mospipeline --check
mosaic-compile --pipeline visicalc-desktop-dark.mospipeline --out-dir ./dist
```

### §5.2 Flag semantics

| Flag | Description |
|---|---|
| `--pipeline <path>` | Path to the `.mospipeline` TOML file (required in this mode). |
| `--dry-run` | Parse and validate the manifest and all source files. Print what would be compiled. Do not write any output files. Exit 0 if valid, non-zero if any error. |
| `--verbose` | Print each compilation step to stderr: file resolved, version selected, part map computed, style validated, emitter invoked, output written. |
| `--check` | Like `--dry-run` but also validates that existing output files match what would be generated. Useful in CI to detect stale generated files. |
| `--out-dir <dir>` | Global output directory override. Merged with per-component `out-dir` fields: the per-component value is appended to the global override as a subdirectory. |

### §5.3 Compilation order

The pipeline compiler processes components in dependency order:

1. Parse the `.mospipeline` manifest.
2. Build the component dependency graph: if component B includes component A as
   a slot value of type `A`, then A must be compiled before B.
3. Resolve all source files for each component (search-path scan, version
   resolution).
4. Validate all cross-component constraints (§6).
5. Compile each component in topological order. Independent components may be
   compiled in parallel (same goroutine/thread model as the build tool in §12).
6. Write output artefacts.

### §5.4 Exit codes

| Code | Meaning |
|---|---|
| 0 | All components compiled successfully. |
| 1 | One or more validation errors (version mismatch, part reference error, missing file, etc.). |
| 2 | Internal error (I/O failure, deserialization failure). |

---

## §6 Compatibility Validation Rules

The pipeline compiler enforces these rules before dispatching to any backend
emitter. All errors are collected and reported together (no fail-fast after the
first error) so that the developer sees the full list of problems in one
invocation.

### §6.1 Layout → interface compatibility

For each `[[component]]` block:

**Rule L1.** The `layout` version range must intersect the `interface` version
range, *and* the selected layout file must declare an `implements` clause naming
the same component as the `interface` field with a range that satisfies the
pipeline's `interface` range.

Formally: if the pipeline says `interface = "Grid@2.x"` and
`layout = "Grid.desktop@1.x"`, then the selected `Grid.desktop@1.x` layout file
must contain `implements Grid 2.x` (or `implements Grid 2.3`, etc.) where the
declared implements range includes `2.x`.

**Rule L2.** The selected layout file must implement the same component name as
the `interface` field (before the dot). A layout declaring
`layout Grid.desktop implements Grid 2.x` cannot be used with
`interface = "FormulaBar@1.x"`.

### §6.2 Style → interface/layout compatibility

**Rule S1.** If the selected style file is interface-scoped (`for Grid 2.x`),
its target version range must satisfy the pipeline's `interface` version range.

**Rule S2.** If the selected style file is layout-scoped (`for Grid.desktop 1.x`),
its target version range must satisfy the pipeline's `layout` version range,
*and* the layout variant name in the `for` clause must match the variant in the
pipeline's `layout` field.

**Rule S3.** An interface-scoped style must not reference any part name outside
the intersection of all layout variant part maps for the target interface version.
(This is evaluated at style file compile time, not only at pipeline level — the
mosstyle compiler enforces it when it receives the part map intersection as input.)

**Rule S4.** A layout-scoped style must not reference any part name outside the
full part map of the targeted layout variant. (Enforced by the mosstyle compiler
when given the layout-specific part map as input.)

### §6.3 Circular style references

**Rule C1.** A style file may not reference another style file as a base.
Cascade is handled only by the stacked resolution order defined in §7. The
pipeline compiler rejects any `@import` directive or cross-file token reference
that would create a dependency between two `.msl` files outside of the Lattice
token layer.

**Rule C2.** Lattice token files may be imported from other Lattice token files
using the `@import` directive. Circular `@import` chains across Lattice token
files are detected by the pipeline compiler (DFS cycle check) and reported as a
`CircularTokenImport` error.

### §6.4 Missing required slots

**Rule M1.** The pipeline compiler checks each component's `.mil` file for
required slots (slots with no default). If a fixture file is specified in the
`[[component]]` block (`fixture = "…"`), the compiler validates that the fixture
JSON provides a value for every required slot. If any required slot is absent
from the fixture and there is no default, compilation fails with a
`MissingRequiredSlot` error.

**Rule M2.** This validation applies only to backends that resolve slot values
at compile time (html, paint). React, webcomponent, and qt backends generate
code that receives slot values at runtime; the missing-slot check is not
applicable to them.

### §6.5 Version resolution failures

**Rule V1.** If no file satisfying the version range is found on any
search-path directory, a `VersionNotFound` error is raised before any compilation
begins.

**Rule V2.** If multiple files in the same search-path directory share the same
version number for the same component variant, the pipeline compiler raises an
`AmbiguousVersion` error. Two files in different directories with the same
version are resolved by first-directory-wins (§4.3, step 7), not an error.

---

## §7 Stacked Style Resolution

When multiple style files apply to a component simultaneously, they are merged
in a defined cascade order. The cascade is resolved at compile time; no ambiguity
reaches the backend emitter.

### §7.1 The four-layer cascade

From lowest priority (applied first, overridable by everything above) to highest
priority (applied last, wins all conflicts):

```
Layer 4 (highest) ─ Layout-scoped style
                     e.g. Grid.desktop.dark.msl, for Grid.desktop 1.x
                     Wins over everything. Most specific.

Layer 3           ─ Interface-scoped style
                     e.g. Grid.dark.msl, for Grid 2.x
                     Applies to all Grid 2.x layouts.

Layer 2           ─ Global token layer (from [global-style] in .mospipeline)
                     e.g. dark.lattice, base.lattice
                     Provides token values; overridden by any component style.

Layer 1 (lowest)  ─ Component base styles (implicit defaults)
                     Values declared directly in .msl files with !default.
                     The fallback if nothing higher sets a value.
```

Think of this as analogous to CSS specificity: more specific targeting wins.
Interface-scoped is less specific than layout-scoped, just as a class selector
is less specific than a compound selector.

### §7.2 Merge semantics

The cascade merges at the level of individual *properties within a part within a
state*. If layer 3 declares `part root { background: $color-surface; }` and
layer 4 declares `part root { background: $color-surface-elevated; }`, then the
merged output for `root.background` is `$color-surface-elevated` (layer 4 wins).

Properties not mentioned in a higher layer are inherited from the lower layer.
This means a layout-scoped style needs only to override the properties that
differ from the interface-scoped style; it does not need to repeat shared
declarations.

### §7.3 Part presence rules during cascade

- If a part exists only in layer 4 (layout-scoped), it is included in the final
  merged map with its layer-4 declarations. Layer 3 and below have no entry for
  this part (it is not in the intersection), and nothing is inherited.
- If a part exists only in layer 3 (interface-scoped), it is included in the
  final merged map with its layer-3 declarations. A layout-scoped style that
  does not mention this part implicitly inherits all layer-3 properties for it.
- Parts in neither layer are unstyled; the backend emitter applies no visual
  properties.

### §7.4 Cascade worked example

**Source files:**

`Grid.dark.msl` (interface-scoped, layer 3):
```mosstyle
style Grid.dark version 2.0 for Grid 2.x {
  part root {
    background:    $color-surface ;
    border-radius: $radius-md ;
  }
  part cell-grid {
    background:   $color-surface ;
    border-color: $color-border ;
    border-width: 1px ;
  }
}
```

`Grid.desktop.dark.msl` (layout-scoped, layer 4):
```mosstyle
style Grid.desktop.dark version 2.0 for Grid.desktop 1.x {
  part frozen-header {
    background:   lighten($color-surface, 4%) ;
    border-color: $color-border ;
  }
  part root {
    border-radius: $radius-lg ;   // override: desktop uses larger radius
  }
}
```

**Merged result** (after token resolution):

| Part | Property | Value | Source layer |
|---|---|---|---|
| `root` | `background` | `#1e1e1e` | Layer 3 (Grid.dark) |
| `root` | `border-radius` | `12px` | Layer 4 (Grid.desktop.dark overrides) |
| `cell-grid` | `background` | `#1e1e1e` | Layer 3 (Grid.dark) |
| `cell-grid` | `border-color` | `rgba(255,255,255,0.12)` | Layer 3 (Grid.dark) |
| `cell-grid` | `border-width` | `1px` | Layer 3 (Grid.dark) |
| `frozen-header` | `background` | `#232323` | Layer 4 (Grid.desktop.dark only) |
| `frozen-header` | `border-color` | `rgba(255,255,255,0.12)` | Layer 4 (Grid.desktop.dark only) |

The `frozen-header` part is present only in the `Grid.desktop` layout. It cannot
appear in `Grid.dark` (interface-scoped) because it is not in the intersection.
It appears in the final merged map only because the layout-scoped style provides
it.

---

## §8 Error Messages

Every error includes:
- The `.mospipeline` file path (for pipeline-level errors).
- The source file path and line number (for source-level errors).
- A clear human-readable description explaining what went wrong and how to fix it.

All errors in a single `mosaic-compile --pipeline` run are collected and
reported together.

### §8.1 Version resolution errors

| Error code | Condition | Example message |
|---|---|---|
| `VersionNotFound` | No file satisfying the range found on search path | `Grid.desktop@1.x: no layout file found satisfying version range 1.x in search paths: [components, ../shared-components]. Available versions: Grid.desktop 2.0 (components/Grid/Grid.desktop.mll)` |
| `AmbiguousVersion` | Two files in the same directory share a version | `Grid.dark@2.x: ambiguous — two files in components/Grid/ declare version 2.0: Grid.dark.msl and Grid.dark.v2.msl. Remove or rename one.` |
| `VersionMissing` | Source file has no version declaration (warning only, not error) | `components/Button/Button.mll: no version declaration found; treated as version 1.0.0. Add 'layout Button version 1.0 implements Button 1.x' to suppress this warning.` |

### §8.2 Layout → interface compatibility errors

| Error code | Condition | Example message |
|---|---|---|
| `ImplementsMismatch` | Layout's `implements` clause names a different component | `components/Grid/Grid.desktop.mll:1: layout declares 'implements Grid 2.x' but pipeline uses interface 'FormulaBar@1.x'. These are different components.` |
| `ImplementsRangeDisjoint` | Layout's `implements` range does not include the pipeline's interface range | `components/Grid/Grid.desktop.mll:1: layout implements Grid 2.x but pipeline requests interface Grid 3.x. Version ranges are disjoint — update the layout's implements clause or request a 2.x interface.` |
| `MissingImplements` | Layout has no `implements` clause but pipeline provides an interface version | `components/Grid/Grid.desktop.mll:1: no 'implements' clause found. Pipeline requires interface Grid@2.x. Add 'implements Grid 2.x' to the layout declaration.` |

### §8.3 Style → interface/layout compatibility errors

| Error code | Condition | Example message |
|---|---|---|
| `StyleTargetMismatch` | Style's `for` clause names a different component | `components/Grid/Grid.dark.msl:1: style declares 'for Grid 2.x' but pipeline uses interface 'FormulaBar@1.x'.` |
| `StyleRangeDisjoint` | Style's `for` range does not satisfy the pipeline's target range | `components/Grid/Grid.dark.msl:1: style targets Grid 1.x but pipeline requests interface Grid 2.x. Version ranges are disjoint.` |
| `LayoutVariantMismatch` | Layout-scoped style targets a different layout variant than the pipeline selects | `components/Grid/Grid.mobile.dark.msl:1: style targets Grid.mobile but pipeline selects layout Grid.desktop. Use 'Grid.desktop.dark.msl' or change the pipeline's layout field.` |
| `IntersectionViolation` | Interface-scoped style references a part not in the intersection | `components/Grid/Grid.dark.msl:8: part 'frozen-header' is not in the intersection of all Grid 2.x layout part sets. The following layouts do not export 'frozen-header': Grid (components/Grid/Grid.mll), Grid.mobile (components/Grid/Grid.mobile.mll). Either target a specific layout (e.g. 'for Grid.desktop 1.x') or remove the 'frozen-header' block.` |
| `LayoutPartViolation` | Layout-scoped style references a part not in the targeted layout | `components/Grid/Grid.desktop.dark.msl:22: part 'sidebar' is not exported by Grid.desktop 1.x. Available parts: root, frozen-header, column-resizer, cell-grid.` |

### §8.4 Circular reference errors

| Error code | Condition | Example message |
|---|---|---|
| `CircularTokenImport` | Lattice token files form an import cycle | `tokens/dark.lattice: circular import detected: dark.lattice → brand.lattice → dark.lattice. Break the cycle by extracting shared tokens into a third file.` |

### §8.5 Missing slot errors

| Error code | Condition | Example message |
|---|---|---|
| `MissingRequiredSlot` | Fixture JSON omits a required slot | `components/FormulaBar/FormulaBar.mil:3: slot 'cell-address' is required (no default) but is absent from fixture fixtures/formula-bar.json. Add {"cell-address": "A1"} to the fixture.` |
| `FixtureTypeMismatch` | Fixture provides a value of the wrong type | `fixtures/formula-bar.json: slot 'total-rows' expects type number but fixture provides a string "42". Remove the quotes.` |
| `FixtureUnknownSlot` | Fixture provides a key that names no slot | `fixtures/formula-bar.json: unknown slot 'header-text' — FormulaBar declares no slot with this name. Valid slots: cell-address, formula, read-only.` |

### §8.6 Manifest parse errors

| Error code | Condition | Example message |
|---|---|---|
| `ManifestMissingField` | Required TOML field absent | `visicalc-desktop-dark.mospipeline: [[component]] block at index 0 is missing required field 'interface'.` |
| `ManifestInvalidRange` | Version range string is malformed | `visicalc-desktop-dark.mospipeline: [[component]] block at index 1: 'layout = "Grid.desktop@"' — version range is missing after '@'. Valid examples: 1.x, 2.3, 1.2.4.` |
| `ManifestUnknownBackend` | `output` names an unrecognised backend | `visicalc-desktop-dark.mospipeline: [[component]] block at index 2: unknown backend 'tsx'. Valid backends: react, webcomponent, html, paint, qt, swiftui, compose.` |

---

## §9 Backend Mapping Table

Each `[[component]]` block selects one backend via its `output` field. The
pipeline compiler dispatches to the corresponding emitter after all validation
passes.

| Backend | Output artefact | Notes |
|---|---|---|
| `react` | `ComponentName.tsx` | TypeScript React functional component. Props interface + typed function. Follows UI20. |
| `webcomponent` | `ComponentName.js` | Custom Element (`class ComponentNameElement extends HTMLElement`). Follows UI17. |
| `html` | `ComponentName.html` | Static HTML snapshot with slot values injected from the fixture file. Follows UI18. |
| `paint` | `ComponentName.png` | Rasterized PNG preview. Uses the `mosaic-emit-paint` backend (UI22). Requires a fixture file. |
| `qt` | `ComponentName.h` + `ComponentName.cpp` | QObject subclass with Q_PROPERTY for each slot. Follows UI21. |
| `swiftui` | `ComponentName.swift` | SwiftUI `View` struct with `@State` and `Binding` wrappers. (Future — not yet implemented.) |
| `compose` | `ComponentName.kt` | Jetpack Composable function. (Future — not yet implemented.) |

### §9.1 Backend availability matrix

| Backend | Status | Crate |
|---|---|---|
| `react` | Implemented (v1) | `mosaic-emit-react` |
| `webcomponent` | Implemented (v1) | `mosaic-emit-webcomponent` |
| `html` | Implemented (v1) | `mosaic-emit-html` |
| `paint` | Implemented (v1) | `mosaic-emit-paint` |
| `qt` | Implemented (v1) | `mosaic-emit-qt` |
| `swiftui` | Planned | `mosaic-emit-swiftui` (future) |
| `compose` | Planned | `mosaic-emit-compose` (future) |

A `mosaic-compile --pipeline` invocation that references a `Planned` backend
fails at validation time with a `BackendNotImplemented` error that names the
planned crate and links to the relevant spec.

### §9.2 Per-backend fixture requirements

| Backend | Fixture required? | Notes |
|---|---|---|
| `react` | No | Slot values are props; fixture not used |
| `webcomponent` | No | Slot values are attributes; fixture not used |
| `html` | Yes (for required slots) | Static output needs concrete values |
| `paint` | Yes (for required slots) | Raster output needs concrete values |
| `qt` | No | Slot values are Q_PROPERTY; fixture not used |
| `swiftui` | No | Slot values are `@Binding`; fixture not used |
| `compose` | No | Slot values are parameters; fixture not used |

---

## §10 Directory Conventions

### §10.1 Canonical project layout

A multi-component project using the composition pipeline should follow this
directory structure:

```
<project-root>/
  components/
    Grid/
      Grid.mil                    ← interface (versioned)
      Grid.mll                    ← default layout (versioned, implements Grid)
      Grid.desktop.mll            ← desktop layout variant
      Grid.mobile.mll             ← mobile layout variant
      Grid.compact.mll            ← compact layout variant
      Grid.dark.msl               ← dark theme (interface-scoped)
      Grid.light.msl              ← light theme (interface-scoped)
      Grid.high-contrast.msl      ← a11y theme (interface-scoped)
      Grid.desktop.dark.msl       ← desktop-specific dark overrides (layout-scoped)
      Grid.desktop.light.msl      ← desktop-specific light overrides (layout-scoped)
    FormulaBar/
      FormulaBar.mil
      FormulaBar.mll
      FormulaBar.desktop.mll
      FormulaBar.dark.msl
      FormulaBar.light.msl
    Button/
      Button.mil
      Button.mll
      Button.dark.msl
      Button.light.msl
  tokens/
    base.lattice                  ← base token values (all !default)
    dark.lattice                  ← dark color overrides
    light.lattice                 ← light color overrides
    high-contrast.lattice         ← a11y overrides
  fixtures/
    Grid.json                     ← sample slot values for paint/html backends
    FormulaBar.json
  pipelines/
    visicalc-desktop-dark.mospipeline
    visicalc-desktop-light.mospipeline
    visicalc-mobile-dark.mospipeline
    visicalc-mobile-light.mospipeline
    visicalc-preview-paint.mospipeline   ← paint backend, all components
  generated/
    react/
      desktop/
        Grid.tsx
        FormulaBar.tsx
      mobile/
        Grid.tsx
        FormulaBar.tsx
    paint/
      Grid.png
      FormulaBar.png
```

### §10.2 Naming convention rules

The file naming convention encodes the component name, variant chain, and
extension. The convention is:

```
<ComponentName>[.<layout-variant>][.<style-variant>].<extension>
```

| Pattern | Example | Meaning |
|---|---|---|
| `<Name>.mil` | `Grid.mil` | Interface file |
| `<Name>.mll` | `Grid.mll` | Default layout |
| `<Name>.<variant>.mll` | `Grid.desktop.mll` | Named layout variant |
| `<Name>.msl` | `Grid.msl` | Default style (interface-scoped) |
| `<Name>.<style>.msl` | `Grid.dark.msl` | Named style variant (interface-scoped) |
| `<Name>.<layout>.<style>.msl` | `Grid.desktop.dark.msl` | Layout-scoped style variant |

The pipeline compiler uses this naming convention during the search-path scan
to determine:
- Whether a file is a layout (`*.mll`) or style (`*.msl`).
- Whether a style is interface-scoped (two-segment name like `Grid.dark`) or
  layout-scoped (three-segment name like `Grid.desktop.dark`).
- Whether a style's embedded component/variant names match the `for` clause in
  the file (discrepancy = a warning, because the naming is advisory but the
  `for` clause is authoritative).

### §10.3 The `generated/` directory

The pipeline writes all artefacts under the global `out-dir` (default: `generated/`)
or the per-component `out-dir` field. The pipeline compiler creates this directory
if it does not exist. Existing files are overwritten without warning; use `--check`
in CI to detect stale outputs.

Generated files carry a header comment:

```tsx
// Auto-generated by mosaic-compile --pipeline visicalc-desktop-dark.mospipeline
// Component: Grid | Interface: Grid@2.0 | Layout: Grid.desktop@1.3 | Style: Grid.desktop.dark@2.0
// DO NOT EDIT — regenerate by running: mosaic-compile --pipeline pipelines/visicalc-desktop-dark.mospipeline
```

---

## §11 Complete Worked Example

This section walks through a complete VisiCalc desktop dark pipeline from
source files to generated output.

### §11.1 Source files

**`components/Grid/Grid.mil`**:
```mosmodel
component Grid version 2.0 {
  slot column-headers  : list<text> ;
  slot column-widths   : list<number> ;
  slot total-rows      : number ;
  slot viewport-offset : number = 0 ;
  slot viewport-rows   : list<list<text>> ;
  slot selected-row    : number = 0 ;
  slot selected-col    : number = 0 ;
  slot edit-row        : number = -1 ;
  slot edit-col        : number = -1 ;
  slot edit-content    : text = "" ;

  emit onNavigate    ( row : number , col : number ) ;
  emit onEditStart   ( row : number , col : number ) ;
  emit onEditCommit  ( value : text ) ;
  emit onEditCancel ;
  emit onScroll      ( offset : number ) ;
}
```

**`components/Grid/Grid.desktop.mll`**:
```moslayout
layout Grid.desktop version 1.3 implements Grid 2.x {
  Column [ root ] {
    Row [ frozen-header ] {}
    Grid [ cell-grid ] (
      headers:        slot: column-headers ,
      widths:         slot: column-widths  ,
      rows:           slot: viewport-rows  ,
      selected-row:   slot: selected-row   ,
      selected-col:   slot: selected-col   ,
      edit-row:       slot: edit-row       ,
      edit-col:       slot: edit-col       ,
      edit-content:   slot: edit-content   ,
      on-navigate:    emit: onNavigate     ,
      on-edit-start:  emit: onEditStart    ,
      on-edit-commit: emit: onEditCommit   ,
      on-edit-cancel: emit: onEditCancel   ,
      on-scroll:      emit: onScroll
    )
  }
}
```

Part map: {`root`, `frozen-header`, `cell-grid`}.

**`components/Grid/Grid.dark.msl`** (interface-scoped):
```mosstyle
style Grid.dark version 2.0 for Grid 2.x {
  part root {
    background:    $color-surface ;
    border-radius: $radius-md ;
  }
  part cell-grid {
    background:   $color-surface ;
    border-color: $color-border ;
    border-width: 1px ;
  }
}
```

**`components/Grid/Grid.desktop.dark.msl`** (layout-scoped):
```mosstyle
style Grid.desktop.dark version 2.0 for Grid.desktop 1.x {
  part root {
    border-radius: $radius-lg ;
  }
  part frozen-header {
    background:   lighten($color-surface, 4%) ;
    border-color: $color-border ;
    border-width: 0 0 1px 0 ;
  }
}
```

### §11.2 Pipeline manifest

**`pipelines/visicalc-desktop-dark.mospipeline`**:
```toml
[pipeline]
name    = "visicalc-desktop-dark"
version = "1.0"
search-path = ["components"]

[global-style]
tokens = ["tokens/base.lattice", "tokens/dark.lattice"]

[[component]]
interface = "Grid@2.x"
layout    = "Grid.desktop@1.x"
style     = "Grid.desktop.dark@2.x"
output    = "react"
out-dir   = "generated/react/desktop"
```

### §11.3 Compilation trace (with `--verbose`)

```
[1/5] Parsing manifest: pipelines/visicalc-desktop-dark.mospipeline
      Pipeline: visicalc-desktop-dark v1.0
      Global tokens: tokens/base.lattice, tokens/dark.lattice
      Components: 1

[2/5] Resolving sources for Grid (component 1/1)
      interface → components/Grid/Grid.mil (version 2.0, satisfies 2.x) ✓
      layout    → components/Grid/Grid.desktop.mll (version 1.3, satisfies 1.x) ✓
      style     → components/Grid/Grid.desktop.dark.msl (version 2.0, satisfies 2.x) ✓
      Also resolved interface-scoped style: components/Grid/Grid.dark.msl (v2.0, for Grid 2.x)

[3/5] Validating compatibility
      Grid.desktop 1.3 implements Grid 2.x → satisfies pipeline interface Grid@2.x ✓
      Grid.dark 2.0 for Grid 2.x → satisfies pipeline interface Grid@2.x ✓
      Grid.desktop.dark 2.0 for Grid.desktop 1.x → satisfies pipeline layout Grid.desktop@1.x ✓
      Part intersection (Grid 2.x layouts found: Grid@1.0, Grid.desktop@1.3, Grid.mobile@1.1):
        intersection = {root, cell-grid}
      Grid.dark references: root ✓, cell-grid ✓ — all in intersection
      Grid.desktop.dark references: root ✓, frozen-header (layout-scoped, in Grid.desktop full set) ✓

[4/5] Compiling Grid
      mosmodel-compiler: Grid.mil → descriptor_json (2 required slots, 8 optional, 5 emits)
      moslayout-compiler: Grid.desktop.mll → part_map_json (3 parts: root, frozen-header, cell-grid)
      Token resolution: base.lattice + dark.lattice → 23 tokens resolved
      mosstyle-compiler (interface-scoped): Grid.dark.msl → style_map (2 parts)
      mosstyle-compiler (layout-scoped): Grid.desktop.dark.msl → style_map (2 parts)
      Style cascade merge: layers 2+3+4 → 3 parts, 11 resolved properties
      mosaic-emit-react: → Grid.tsx

[5/5] Writing output
      generated/react/desktop/Grid.tsx (1.4 KB) ✓

Pipeline complete: 1 component, 1 artefact, 0 errors, 0 warnings.
```

### §11.4 Generated React output

```tsx
// Auto-generated by mosaic-compile --pipeline pipelines/visicalc-desktop-dark.mospipeline
// Component: Grid | Interface: Grid@2.0 | Layout: Grid.desktop@1.3 | Style: Grid.desktop.dark@2.0
// DO NOT EDIT
import React from "react";

export interface GridProps {
  columnHeaders:   string[];
  columnWidths:    number[];
  totalRows:       number;
  viewportOffset?: number;
  viewportRows:    Array<string[]>;
  selectedRow?:    number;
  selectedCol?:    number;
  editRow?:        number;
  editCol?:        number;
  editContent?:    string;
  onNavigate?:     (row: number, col: number) => void;
  onEditStart?:    (row: number, col: number) => void;
  onEditCommit?:   (value: string) => void;
  onEditCancel?:   () => void;
  onScroll?:       (offset: number) => void;
}

export function Grid({
  columnHeaders, columnWidths, totalRows,
  viewportOffset = 0, viewportRows,
  selectedRow = 0, selectedCol = 0,
  editRow = -1, editCol = -1, editContent = "",
  onNavigate, onEditStart, onEditCommit, onEditCancel, onScroll,
}: GridProps) {
  return (
    <div className="mos-Grid-root">
      <div className="mos-Grid-frozen-header" />
      <table className="mos-Grid-cell-grid">
        <thead>
          <tr>{columnHeaders.map((h, _i) => <th key={_i}>{h}</th>)}</tr>
        </thead>
        <tbody>
          {viewportRows.map((row, _i) => (
            <tr key={_i}>
              {row.map((cell, _j) => <td key={_j}>{cell}</td>)}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
```

Generated CSS (written to `generated/react/desktop/Grid.css`):

```css
/* Auto-generated — do not edit */
.mos-Grid-root {
  background: #1e1e1e;
  border-radius: 12px;  /* Layer 4 wins over Layer 3's 8px */
}
.mos-Grid-frozen-header {
  background: #232323;
  border-color: rgba(255,255,255,0.12);
  border-width: 0 0 1px 0;
}
.mos-Grid-cell-grid {
  background: #1e1e1e;
  border-color: rgba(255,255,255,0.12);
  border-width: 1px;
}
```

---

## §12 Internal Architecture of the Pipeline Compiler

This section specifies how `mosaic-compile --pipeline` is implemented internally.
It is informative for contributors, not normative for users.

### §12.1 Crate structure

The pipeline compiler lives in a new crate `mosaic-pipeline`:

```
code/packages/rust/
  mosaic-pipeline/
    src/
      lib.rs              ← public API: PipelineCompiler::new(), compile()
      manifest.rs         ← TOML parsing → PipelineManifest struct
      resolver.rs         ← search-path scanning, version resolution
      validator.rs        ← Rules L1/L2/S1-S4/C1-C2/M1-M2/V1-V2
      cascade.rs          ← §7 stacked style resolution
      driver.rs           ← orchestrates per-component compilation
      parallel.rs         ← concurrent compilation of independent components
      error.rs            ← PipelineError enum, Display impl for all error codes
    tests/
      manifest_parse.rs
      resolver_tests.rs
      validator_tests.rs
      cascade_tests.rs
      end_to_end.rs
    Cargo.toml
    README.md
    CHANGELOG.md
```

### §12.2 Data flow

```
manifest.rs:  parse .mospipeline → PipelineManifest
resolver.rs:  PipelineManifest + search-path → ResolvedPipeline
              (each component slot carries file paths + parsed headers)
validator.rs: ResolvedPipeline → ValidationResult (collect all errors)
cascade.rs:   [interface-scoped StyleDef] + [layout-scoped StyleDef] → MergedStyleDef
driver.rs:    ResolvedPipeline + ValidationResult.ok()
              → per-component: mosmodel-compiler → moslayout-compiler
                              → mosstyle-compiler (with MergedStyleDef)
                              → backend emitter
              → write output files
```

### §12.3 Parallel compilation

Independent components (no slot-type dependency between them) are compiled
concurrently using Rust's `std::thread::spawn` with a bounded thread pool (size
= number of logical CPUs, capped at 8). The thread pool is the same design used
in the Go build tool (`code/programs/go/build-tool/`).

A component A depends on component B if A declares a slot of type `B` (named
component type). The dependency graph is computed before compilation begins and
used to determine which components may start immediately versus which must wait
for their dependencies.

---

## §13 Grammar Token and Grammar File Additions

This section lists the exact token and grammar changes needed in the three
existing grammar files to support the version/implements/for syntax described
in §2.

### §13.1 `mosmodel.tokens` additions

```
# New tokens (add after existing keywords)
KW_VERSION    = "version"
DOT           = "."
```

### §13.2 `mosmodel.grammar` changes

```diff
-component_def = KW_COMPONENT IDENT LBRACE { member } RBRACE ;
+component_def = KW_COMPONENT IDENT [ version_clause ] LBRACE { member } RBRACE ;
+
+version_clause = KW_VERSION version_num ;
+version_num    = NUMBER DOT NUMBER [ DOT NUMBER ] ;
```

### §13.3 `moslayout.tokens` additions

```
# New tokens (add after existing keywords)
KW_VERSION     = "version"
KW_IMPLEMENTS  = "implements"
DOT            = "."
STAR           = "x"
```

### §13.4 `moslayout.grammar` changes

```diff
-layout_def = KW_LAYOUT IDENT LBRACE { node } RBRACE ;
+layout_def = KW_LAYOUT dotted_name [ version_clause ] [ implements_clause ]
+             LBRACE { node } RBRACE ;
+
+dotted_name       = IDENT [ DOT IDENT ] ;
+version_clause    = KW_VERSION version_num ;
+version_num       = NUMBER DOT NUMBER [ DOT NUMBER ] ;
+implements_clause = KW_IMPLEMENTS IDENT version_range ;
+version_range     = NUMBER DOT ( NUMBER | STAR ) ;
```

### §13.5 `mosstyle.tokens` additions

```
# New tokens (add after existing keywords)
KW_VERSION    = "version"
KW_FOR        = "for"
DOT           = "."
STAR          = "x"
```

### §13.6 `mosstyle.grammar` changes

```diff
-style_def = KW_STYLE IDENT LBRACE { part_def } RBRACE ;
+style_def = KW_STYLE dotted_name [ version_clause ] [ for_clause ]
+            LBRACE { part_def } RBRACE ;
+
+dotted_name    = IDENT [ DOT IDENT [ DOT IDENT ] ] ;  # up to 3 segments
+version_clause = KW_VERSION version_num ;
+version_num    = NUMBER DOT NUMBER [ DOT NUMBER ] ;
+for_clause     = KW_FOR dotted_name version_range ;
+version_range  = NUMBER DOT ( NUMBER | STAR ) ;
```

After any grammar change, regenerate the embedded Rust using grammar-tools
as described in UI16 §7. Commit the text grammar files and the generated
`_grammar.rs` file together.

---

## §14 Out of Scope

The following are explicitly deferred and not part of this spec:

- **Runtime theming** — swapping token layers without recompiling. The DOM
  backend can implement this via CSS custom properties as a development aid,
  but production builds always use fully resolved concrete values (inherited
  from UI15 §5).
- **Incremental compilation** — recompiling only the components whose source
  files have changed since the last pipeline run. The pipeline recompiles all
  components on every invocation. Incremental compilation using `mtime` or
  content hashes is a v2 deliverable.
- **Cross-pipeline component sharing** — a component compiled in one pipeline
  being directly included in a second pipeline's output without recompilation.
  The `search-path` mechanism allows source sharing; binary sharing is deferred.
- **Pipeline inheritance** — a `.mospipeline` file extending another pipeline
  manifest with additional or overriding component blocks. Deferred to v2.
- **Style conditionals** — `if desktop { border-radius: $radius-lg }` in a
  single `.msl` file instead of a separate layout-scoped file. Deferred. The
  separate-file model keeps each file's scope declaration explicit and
  checkable.
- **Component versioning enforcement at the host** — verifying that a host
  application that embeds a `Grid@2.x` component was compiled against a
  compatible version of `Grid`. This is a runtime/package-manager concern
  beyond the Mosaic compiler's scope.
- **Parallel multi-pipeline runs** — running two `.mospipeline` compilations
  concurrently in the same build invocation. Each `mosaic-compile --pipeline`
  invocation is independent; parallelism at the inter-pipeline level is the
  caller's responsibility (e.g. the build-tool's goroutine model).

---

## §15 Related Specs

| Spec | Title | Relationship |
|---|---|---|
| UI13 | mosmodel | Interface language; §2.1 extends its grammar |
| UI14 | moslayout | Layout language; §2.2 extends its grammar |
| UI15 | mosstyle | Style language; §2.3 extends its grammar; §5 defines token cascade this spec builds on |
| UI16 | mosaic-compiler-pipeline | Single-component pipeline; this spec extends it to multi-component with versioning |
| UI17 | mosaic-emit-webcomponent | WebComponent backend wired by `output = "webcomponent"` |
| UI18 | mosaic-emit-html | HTML backend wired by `output = "html"` |
| UI20 | mosaic-emit-react | React backend wired by `output = "react"` |
| UI21 | mosaic-emit-qt | Qt backend wired by `output = "qt"` |
| UI22 | mosaic-emit-paint | Paint VM backend wired by `output = "paint"` |
| UI19 | mosaicbook | Component storybook tool; can be driven by a `.mospipeline` to preview all variants |
| 17-lattice-transpiler | Lattice | Token layer resolved before mosstyle; global-style section points to Lattice files |
