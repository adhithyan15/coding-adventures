# UI91 — Design import: from a Figma design to a native Mosaic app

**Status:** Draft. The owner delegated the import decisions (2026-10-12: "please
feel free to choose the best decision based on the knowledge that you have"),
so §2 records the choices made and the reasons for them. They stay open to
the owner's review like any draft.
**Implements:** `MOSAIC-BACKLOG.md` DN-1 and DN-2. Depends on UI90 (the
typed, platform-neutral mosstyle this imports into, especially §5.5) and
needs DN-3 (fixtures on every native backend) for its last step.

## 1. Goal

> We should be able to go from a Figma design or something to beautiful
> design very fast. It should be able to use Mosaic to produce native apps
> that follow the design. The native app could be dummy and we can add the
> backend to connect it up. *(Owner, 2026-10-12.)*

So the pipeline has three stages, and each must be useful on its own:

```text
 design tool                 mosaic-compile design import            every backend
┌──────────────┐  tokens    ┌──────────────────────────────┐        ┌─────────────┐
│ Figma        │──(DTCG)──▶ │ 1. tokens → mode-aware brand │        │ XAML, Qt,   │
│ (or Penpot,  │            │    tokens, type roles, fonts │──────▶ │ Flutter,    │
│ Tokens       │  screens   │ 2. frames → .mil/.mll/.msl   │ native │ Compose,    │
│ Studio, …)   │──(JSON)──▶ │ 3. design text → stories     │ app on │ SwiftUI,    │
└──────────────┘            └──────────────────────────────┘ sample │ web …       │
                                                             data   └─────────────┘
                                         later: a real backend fills the .mil slots
```

1. **Tokens.** Colours, sizes, text styles and fonts make any Mosaic app look
   like the design. This alone is valuable: an existing app can adopt a new
   brand by importing a token file.
2. **Screens.** Each top-level frame becomes a Mosaic component built from
   the toolkit's native controls.
3. **Sample data.** The text and images in the design become story
   fixtures, so the generated app runs as a "dummy" app with no backend.
   Its `.mil` interface is the seam where a real backend connects later,
   exactly as for hand-written Mosaic apps.

## 2. Decisions

### 2.1 Files on disk first; the network is a separate, optional step

The importer reads **only local files**. Getting them out of Figma is a
separate step that a person can do by hand or with `design fetch` (§6).

Why:

- **Reproducible and reviewable.** The exported files are committed with the
  package, so an import is a pure function of files in the repo, a PR shows
  exactly what changed in the design, and CI can re-run it.
- **No secrets in the build.** Only the optional fetch step needs a Figma
  token, read from the environment and never written anywhere.
- **Not tied to one vendor or plan.** Figma's Variables REST API is
  Enterprise-only, but the token *file* format is a W3C community standard
  that Figma, the Tokens Studio plugin, Penpot and Style Dictionary all
  read or write.
- **Testable.** The test suite imports small hand-written fixture designs
  with no network.

### 2.2 Two inputs, each in the most widely supported format

| Input | Format | Produced by | Used for |
|---|---|---|---|
| Tokens | [W3C Design Tokens format 2025.10](https://www.designtokens.org/) (DTCG), with the 2025.10 Resolver module for modes | Figma's native variable export (rolling out since late 2025), the Tokens Studio plugin (any plan), Penpot | colours, sizes, spacing, radii, fonts, text styles, light/dark modes |
| Screens | Figma's file document JSON (the body of `GET /v1/files/:key`) | `design fetch`, or saved by hand with a personal access token (available on every plan) | frames, auto layout, text, component instances, image fills, text styles |

Figma's native DTCG export does not yet include composite tokens such as
typography. So text styles are read from the DTCG file when it has
`typography` tokens (Tokens Studio exports them), and otherwise from the
`styles` in the file JSON. Either source maps to the same `type-role`s.

### 2.3 Tokens stay synced with the design; screens are generated once

| Output | Owner after import | Re-import |
|---|---|---|
| Token palette (`tokens/design.json`) | the design | regenerated every time. The file is marked generated. Hand overrides live in a separate `tokens/overrides.json`, which wins |
| Screens (`src/<Screen>.mil/.mll/.msl/.stories.json`) | the author | never overwritten. A re-import writes `IMPORT-DIFF.md` describing what changed in the design, so the author applies what they want |

Why: tokens are data, so the design can change them freely and every app
follows. Screens become code the moment someone wires a backend or adjusts a
layout. Overwriting them would destroy that work, and a three-way merge of
generated layouts is not worth its complexity in v1.

### 2.4 Native controls, never redrawn

Per UI90 §1, a button in the design becomes a native button, not a rectangle
with text drawn to look like one. Design components are matched by name to
Mosaic components:

- A built-in table covers the toolkit and std-controls exports (`Button`,
  `Checkbox`, `Input`, `Radio`, `Select`, `Tabs`, `Modal`, …). It also covers
  the components of the Material 3 Figma kit; for example, its buttons map
  to `Button` and its text fields to `Input`. The exact kit names are pinned
  when the table is written.
- A package can extend or override the table in `design-map.toml`.
- An instance that matches nothing becomes a generated local component
  composed from kernel primitives, and is listed in `IMPORT-REPORT.md` so
  the author can map it to a real control.

### 2.5 The design's own content is the sample data

- Every text layer inside a screen becomes a `text` slot. Its text in the
  design becomes that slot's value in the screen's first story.
- Repeated sibling instances of the same component (a list of rows) become
  a `list` slot rendered with `For`, with one fixture item per instance.
- A mapped control's action (a button press, a switch toggle) becomes an
  `emit` event on the screen's interface. Nothing handles it until a backend
  is connected.
- Image fills become `Image` leaves backed by package assets.

The result runs in MosaicBook immediately, and on native backends once DN-3
lands.

## 3. Tokens: DTCG to mosstyle

| DTCG `$type` | mosstyle | Notes |
|---|---|---|
| `color` | Color brand token | Alpha becomes `#RRGGBB / A` (UI90 §3.2). Colour spaces other than sRGB are converted to sRGB and reported |
| `dimension` | Length brand token | `px` and `rem` (16 px) |
| `number` | number token | |
| `fontFamily` | brand face token | the font files must be supplied (§5); otherwise it falls back to `system` with a report |
| `fontWeight` | `font-weight` keyword | nearest of `regular`/`medium`/`semibold`/`bold` |
| `typography` (composite) | `type-role` | matched to Material 3 role names; an unmatched style keeps its size and weight as explicit values, with a report |
| `shadow` | `elevation` | nearest UI41 level, reported |
| `duration`, `cubicBezier` | `motion` | nearest UI90 motion value |
| `border`, `strokeStyle`, `gradient`, `transition` | — | reported, not imported in v1 |

Aliases (`{color.brand.primary}`) are resolved, and the alias name is kept as
the mosstyle token name so components keep meaning, not raw values.

**Modes.** A Resolver-module context (or a Figma variable mode) named
`light`, `dark`, `light-high-contrast` or `dark-high-contrast` becomes the
matching value of a mode-aware token (UI90 §3.3). Case and common spellings
("Light mode") are accepted. A collection with any other modes, such as
brands or densities, imports its default mode only and is reported. A token
with a light value and no dark one is an error, as in UI90.

**Names.** DTCG group paths become kebab-case token names
(`color.surface.card` → `color-surface-card`). Names that collide after
normalising get a numeric suffix and a report line.

Until UI90 §6 step 3 lands mode-aware tokens, the importer writes today's
palette convention (`schema_version: 1`, `-light`/`-dark` suffix pairs, as in
`mosaic-std-controls/tokens/controls.json`). It switches to mode-aware
tokens with that step. The palette is attached through the existing
`[styles].token_palette` manifest key.

## 4. Screens: Figma nodes to Mosaic

Every top-level frame on the chosen page is a screen. Each becomes one
component (`<Screen>.mil`, `.mll`, `.msl`, `.stories.json`) named from the
frame.

| Figma node | Mosaic | Mapping |
|---|---|---|
| Frame, auto layout vertical | `Column` | `itemSpacing` → gap token, padding → `padding`, primary/counter axis alignment → `align-main`/`align-cross` |
| Frame, auto layout horizontal | `Row` | as above; wrapping is reported |
| Frame without auto layout | `Stack` | absolute positions kept, reported as fragile on resize |
| Frame with only fill/stroke/radius/effects | `Box` | fill → `background`, stroke → border, `cornerRadius` → `corner`, drop shadow → `elevation` |
| Text | `Text` | text style → `type-role` (or explicit size and weight); characters → slot fixture |
| Instance of a mapped component | that component | its overridden properties become slot values |
| Instance of an unmapped component | generated local component | reported |
| Rectangle or frame with an image fill | `Image` | image exported to `assets/`, fixture points at it |
| Vector or boolean group | `Icon` if it comes from a known icon set, otherwise an SVG asset in an `Image` | reported |
| Spacer-like empty frame in auto layout | `Spacer` | |
| Scrolling frame (`overflowDirection`) | `HostScroll` | |
| Line | `Divider` | |

Fills, strokes and spacing that equal a token value are written as that
token (`$color-surface-card`), never as the raw value. This keeps screens
themeable. Raw values remain only where the design used no variable, and are
listed in the report.

Layer names become identifiers: kebab-case, ASCII, unique within the
component. Prototype links between frames become `emit` events named after
the target (`navigate-settings`). Wiring navigation is author work in v1.

## 5. Output package

```text
my-app-design/
├── mosaic-package.toml        [components].exports = the screens; [styles].token_palette
├── design/                    the inputs, committed: tokens.json, file.json, design-map.toml
├── tokens/design.json         generated palette (do not edit)
├── tokens/overrides.json      hand-written, optional, wins
├── src/<Screen>.{mil,mll,msl,stories.json}
├── assets/fonts/…             font files + their licence files (UI90 §5.4)
├── assets/images/…            exported images
├── IMPORT-REPORT.md           every degradation, with node ids
├── README.md, CHANGELOG.md, BUILD   (repo standards)
```

`IMPORT-REPORT.md` is the import's honesty mechanism, like the emitters'
degradation reports (#12022). Nothing the importer could not represent is
dropped silently.

## 6. The tool

A new Rust crate, `mosaic-design-import`, exposed as two `mosaic-compile`
subcommands:

- `mosaic-compile design import --design <dir> --out <package dir>` reads
  only local files. It is deterministic: the same inputs give byte-identical
  output.
- `mosaic-compile design fetch --file-key <key> --out <dir>` downloads the
  file JSON and its image fills from `api.figma.com`, using a token from
  the `FIGMA_TOKEN` environment variable. It is the only networked step.

## 7. Safety

The design files are untrusted input:

- caps on file size and on node count and depth;
- every name sanitised before it becomes an identifier, a file name or a
  string in `.mll`/`.msl`, with no path separators or `..` in asset names;
- strings escaped by the same rules as hand-written sources; a value that
  would start with `{` cannot reach XAML (UI90 rule 8 already refuses it);
- `fetch` talks only to `api.figma.com`, never logs or writes the token, and
  validates image URLs before downloading.

## 8. Order of work

1. **Fixture design.** A small hand-written design (tokens file + file JSON:
   a list screen and a detail screen with buttons, a text field, a list and
   an image) under the crate's `tests/fixtures`. It drives every later step.
2. **Tokens → palette** (DN-1), writing today's v1 palette convention.
   Golden tests on the fixture.
3. **Fonts as assets** (UI90 §5.4), shared with the UI90 token work.
4. **Screens → `.mil`/`.mll`/`.msl`** with the component table and
   `IMPORT-REPORT.md` (DN-2).
5. **Stories from design content.** The fixture design then runs in
   MosaicBook.
6. **Native fixtures** (DN-3). The same dummy app then runs on all eight
   backends. This becomes the end-to-end acceptance test: import the
   fixture design and build every backend.
7. **`design fetch`.**
8. **Mode-aware tokens and type roles**, switched on when UI90 §6 steps 2–3
   land.

## 9. Not in v1

- keeping screens synced after hand edits (§2.3);
- generated navigation between screens;
- prototype animations and smart animate;
- variable modes other than light, dark and high contrast;
- Sketch, Adobe XD or other proprietary files (export them to DTCG via a
  plugin instead);
- reading designs live through Figma's Dev Mode MCP server. This is a
  natural later input, since it already returns variables with their modes.
