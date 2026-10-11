# UI90 — a typed, platform-neutral mosstyle

**Status:** Draft, for review. No code until this is agreed (specs first).
**Implements:** `MOSAIC-BACKLOG.md` X-6, and the direction in #12029
("evolve from a CSS superset into a typed, platform-neutral style
language"). Builds on UI15 (mosstyle), UI49 (slot states) and UI41
(elevation). Feeds #12022 (dropped styles fail the gate).

## 1. The principle

> mosstyle expresses every styling decision without platform-specific
> vocabulary or abstraction leaks. *(Decided 2026-10-08.)*

The principle has two halves, and the design needs both:

- **No leaks.** Nothing in a `.msl` file is written for one backend: no XAML
  resource lookup, no CSS function, no web-only layout trick. The first
  slice is enforced already. A value that starts with `{` is a compile error
  (`PlatformValue`, UI15 §8 rule 8).
- **Enough expressiveness.** Every decision an author really makes has a
  neutral way to say it. Otherwise authors reach for a leak, which is what
  happened.

## 2. Where mosstyle is today

An audit of all 186 `.msl` files (2026-10-08) found that, in practice,
mosstyle is CSS:

- **Properties are open.** 66 distinct names are in use, against about 25
  in UI15 §2. `ErrorKind::UnknownProperty` is declared and never raised. The
  web backends pass any CSS through, and the native backends each
  hand-write a partial CSS interpreter.
- **The top of the distribution is healthy.** These eight cover about
  5,300 of 6,300 uses, and each lowers cleanly everywhere:
  - `color` 998
  - `background` 792
  - `font-size` 764
  - `padding` 753
  - `border-radius` 559
  - `border-width` 514
  - `border-color` 485
  - `font-weight` 425
- **The tail is where the leaks are:**
  - `box-shadow` 22, written next to `elevation`;
  - `position`, `top` and `left` (12 each);
  - `flex-shrink` 18, `flex-grow` 17, `flex-wrap` 8;
  - `outline` 10, `transform` 4, `cursor` 4, `display` 2;
  - `border` and `font` shorthands;
  - `letter-spacing` 24 and `text-transform` 22, dropped on every native
    backend;
  - `white-space` and `text-overflow`.
- **Values leak too:**
  - `rgba()` appears 44 times, and only XAML converts it;
  - `currentColor`;
  - CSS font stacks that name Apple and Windows fonts;
  - `vh` units;
  - `rotate(45deg)`;
  - 8-digit hex, which Flutter reads as AARRGGBB and CSS as RRGGBBAA, so the
    same literal is a different colour on different backends.
- **Colours are mostly literal:** 3,258 hex literals against 98 token
  references.
- **Light and dark are separate files.** There are 95 `.light.msl` and 70
  `.dark.msl` files, mostly near-duplicates, and the mode is baked into
  token names (`foundation-color-text-light`).
- **The cost is measurable.** TaskApp's native style-drop ratchet allows 741
  drops (X-3), and most of them are this tail.

## 3. Design

### 3.1 A closed, typed property table

The compiler knows every property, its type and where it may appear. An
unknown name is `UnknownProperty`, and a value of the wrong type is
`TypeMismatch`. Both are errors. Both already exist in UI15 §9, but neither
is raised today.

Every row is a decision every platform has. "Lowers" means each of the
eight backends maps it to a native concept. A backend that cannot must
report a degradation, which #12022 turns into a gate failure.

**Paint**

| Property | Type | Replaces |
|---|---|---|
| `color` | Color | — |
| `background` | Color | — |
| `opacity` | Number 0–1 | — |

**Geometry and border**

| Property | Type | Replaces |
|---|---|---|
| `padding`, `padding-{top,right,bottom,left}` | Length | — |
| `width`, `height`, `min-*`, `max-*` | Length \| `fill` | `100vh`, `100%` (see `fill`) |
| `corner` | Length \| `full` | `border-radius` (`50%` → `full`); per-corner `corner-{top-left,…}` |
| `border-width`, `border-{edge}-width` | Length | the `border` shorthand |
| `border-color`, `border-{edge}-color` | Color | the `border` shorthand |
| `border-stroke` | `solid` \| `dashed` \| `dotted` | `border-style`, `border-*-style` |

**Text** (on `Text`-like parts)

| Property | Type | Replaces |
|---|---|---|
| `type-role` | `display` \| `title` \| `heading` \| `body` \| `label` \| `caption` | most `font-size` / `font-weight` pairs. Each backend maps it to its own type ramp |
| `font-size` | Length | (still allowed when a role does not fit) |
| `font-weight` | `regular` \| `medium` \| `semibold` \| `bold` | `400`/`600`, `SemiBold` |
| `font` | `system` \| `mono` \| `serif` \| *brand face token* | CSS font stacks, the `font` shorthand |
| `tracking` | `tight` \| `normal` \| `wide` | `letter-spacing: 0.07em` |
| `text-case` | `none` \| `upper` \| `lower` | `text-transform` |
| `text-align` | `start` \| `center` \| `end` | `left`/`right` |
| `max-lines` | Integer ≥ 1 | `white-space: nowrap` |
| `overflow-text` | `clip` \| `ellipsis` | `text-overflow`, `overflow` |

**Layout** (the parts of layout that are style)

| Property | Type | Replaces |
|---|---|---|
| `gap` | Length | — |
| `align-main` | `start` \| `center` \| `end` \| `space-between` | `justify-content`, half of `align` (#16293) |
| `align-cross` | `start` \| `center` \| `end` \| `stretch` | `align-items`, the other half of `align` |
| `grow` | Number ≥ 0 | `flex-grow`, `flex: 1` |
| `shrink` | `yes` \| `no` | `flex-shrink: 0` |

**Effects and state**

| Property | Type | Replaces |
|---|---|---|
| `elevation` | `none` \| `raised` \| `overlay` | **every** `box-shadow` (UI41 already defines it) |
| `focus-ring` | `system` \| `none` \| Color | `outline`, `outline-offset` |
| `rotation` | Angle (degrees) | `transform: rotate()` |
| `motion` | `none` \| `subtle` \| `standard` | `transition` timings. Honours reduced motion on every platform |

**Moved out of `.msl`**, because each is structure or behaviour rather than
style:

- `position: absolute/sticky`, `top`/`left`, `z-index` → overlay and
  sticky roles in `.mll`: a `Stack` placement, and a sticky header role on
  `Grid`/`HostTable`.
- `display`, `flex-direction`, `box-sizing`, `flex-wrap` → `.mll`
  containers (`Row`/`Column`/`Wrap`).
- `cursor` → implied by the role: a button, a link, a draggable.
- `color-scheme` → implied by the theme mode (§3.3).
- The moon drawn with an inset `box-shadow` → a `Path` (UI39).

### 3.2 Value types

- **Color.** One syntax with one meaning on every backend:
  - `#RRGGBB`;
  - `#RRGGBB / A`, with the alpha `A` from 0 to 1, replacing both `rgba()`
    and 8-digit hex;
  - a colour token.

  Named colours, `currentColor` and CSS functions are type errors.
- **Length.** A unitless number in device-independent units, as today
  (`gap: 22`). Units are type errors, except where a type says otherwise.
  Negative lengths are allowed where meaningful, such as offsets (#14327).
- **Enumerations.** Closed sets, written the same everywhere (`semibold`,
  never `SemiBold`). The emitters map them. Nothing passes through verbatim.

### 3.3 Tokens: brand, system and mode

#12029 is blocked on "brand vs native": a fixed brand palette and native
look-and-feel conflict. The proposal is to let **each token say which kind
it is**, so the author decides per decision rather than per app:

- **Brand tokens** carry a fixed value to every platform. Today's palettes
  are this kind.
- **System tokens** are reserved names that each backend resolves to the
  host's own value. A system token is the neutral form of what
  `{ThemeResource SystemAccentColor}` tried to say:

  | Token | XAML | SwiftUI | Compose | Qt | Flutter | Web |
  |---|---|---|---|---|---|---|
  | `$system-accent` | `SystemAccentColor` | `.accentColor` | `colorScheme.primary` | `palette.highlight` | `colorScheme.primary` | `AccentColor` |
  | `$system-surface` | layer fill | `systemBackground` | `colorScheme.surface` | `palette.window` | `colorScheme.surface` | `Canvas` |
  | `$system-text` | text primary | `.primary` | `onSurface` | `palette.windowText` | `onSurface` | `CanvasText` |
  | `$system-text-muted` | text secondary | `.secondary` | `onSurfaceVariant` | `palette.placeholderText` | `onSurfaceVariant` | `GrayText` |
  | `$system-separator` | divider stroke | `.separator` | `outlineVariant` | `palette.mid` | `outlineVariant` | (a mixed `CanvasText`) |
  | `$system-focus` | focus stroke | `.accentColor` | `colorScheme.primary` | `palette.highlight` | `colorScheme.primary` | `Highlight` |

  This table is the only place platform names appear, and it lives in the
  emitters, never in a `.msl` file.
- **Mode-aware tokens** hold a light and a dark value, so one stylesheet
  covers both modes:

  ```text
  token color-text   { light: #1f1a14 ; dark: #f4efe6 ; }
  token color-card   { light: #ffffff ; dark: #2a2420 / 0.96 ; }
  ```

  The host's mode, from UI48's environment, selects the value. System
  tokens are mode-aware by nature. The `.light.msl`/`.dark.msl` pairs then
  merge into one file, and a component that wants a different structure per
  mode is the rare case UI49 states already cover.
- **Per-backend token overrides go.** The `backends.{xaml,qt,…}` overrides
  in manifest palettes (`mosstyle-compiler` lib.rs:452) and UI15 §5's
  "ios-tokens" invite platform values. A backend difference that is a real
  design decision becomes a system token. Anything else is a leak.

### 3.4 Conditions

Two kinds of decision depend on the environment, and need a neutral form
rather than a media query:

- **Size class**: `when compact { … }` / `when regular { … }`, using
  UI48's size class, which all eight backends already observe (ENV1–ENV4).
- **Reduced motion**: implied by `motion`, which every backend lowers to
  "no animation" when the host asks for that.

Pointer versus touch is already UI48's input modality, and `.touch.mll`
layouts cover structure.

### 3.5 What the compiler and emitters enforce

- **The compiler** (`mosstyle-compiler`):
  - raises `UnknownProperty`, `TypeMismatch` and `PlatformValue`;
  - refuses a property outside the parts it belongs to (text properties on
    text parts);
  - resolves tokens, including mode-aware and system tokens, into typed
    values;
  - passes the emitters **typed values**, not strings.
- **The emitters**:
  - lower every property in the table, or report a degradation (#12022);
  - delete every pass-through rule. The audit lists ten, including:
    - XAML's "uppercase means native" values and its verbatim font and
      enum fallbacks;
    - Qt's raw font family;
    - the per-backend named-colour sets;
    - the web emitters' "any kebab-case CSS passes".
  - The web backends become one lowering target among eight, not the
    default.

## 4. Migration

There are 186 files and about 6,300 declarations, so migration has to be
mechanical where it can be, and gated where it can't.

1. **Version the language.** A package opts in with `[style] version = 2`
   in `mosaic-package.toml`.
   - A v1 package compiles as today, plus warnings for every v2 error.
   - A v2 package gets the errors.
   - MosaicBook and CI report each package's version and warning count.
2. **`mosaic-compile style migrate`.** It rewrites the mechanical cases in
   place and prints the rest for a person to decide:
   - `rgba()` → `#RRGGBB / A`;
   - `box-shadow` next to `elevation` → removed;
   - a `box-shadow` alone → its nearest `elevation`, noted for review;
   - `border-radius: 50%` → `corner: full`;
   - `text-transform` → `text-case`;
   - `letter-spacing` → `tracking`;
   - `align` → `align-main`/`align-cross`;
   - numeric `font-weight` → its keyword;
   - CSS font stacks → `font: system` (brand faces noted);
   - `left`/`right` text alignment → `start`/`end`;
   - `flex-grow` → `grow`;
   - `flex-shrink: 0` → `shrink: no`.
3. **Merge `.light`/`.dark` pairs** into mode-aware tokens, with a tool that
   diffs each pair and proposes tokens for the values that differ.
4. **Move the structural leaks into `.mll`**: overlays, sticky headers, the
   moon `Path`, the `display: flex` drop targets. This is per component and
   reviewed, not automated.
5. **Flip the default** to v2 once every package is migrated. Then delete the
   v1 paths and the emitters' CSS interpreters.

Order by value: the toolkit first, since every app inherits it; then
TaskApp, which has the drop ratchet; then the rest.

## 5. Decisions needed

1. **Brand vs native (the #12029 blocker).** Is §3.3's per-token split
   (brand tokens fixed, system tokens per host) the intended answer? For
   example: Trestle keeps its warm surfaces as brand tokens, and takes
   `$system-accent` and `$system-focus` from the host.
2. **Type roles.** Should `type-role` be the preferred way to set text size,
   with `font-size` kept only for exceptions, or the only way?
3. **Mode syntax.** Mode-aware tokens (§3.3), or `scheme dark { … }` blocks
   inside a part?
4. **Brand faces.** Should a custom font travel as a package asset
   (bundled per platform), or stay out of scope for now, with `font` limited
   to `system`/`mono`/`serif`?

## 6. Order of work

1. This spec, agreed.
2. **Compiler, behind versioning:**
   - the property table and types;
   - `UnknownProperty` and `TypeMismatch`;
   - the colour syntax;
   - v1 warnings.
3. **Tokens:** system tokens in every emitter, then mode-aware tokens.
4. **`style migrate`**, run on the toolkit; then the toolkit flips to v2.
5. **Emitters:** delete pass-throughs, then lower the new properties
   (`type-role`, `tracking`, `text-case`, `max-lines`, `align-main`,
   `align-cross`, `focus-ring`, `motion`) on all eight backends.
6. **TaskApp to v2.** The 741-drop ratchet should fall to near zero, and
   #12022 can then hard-fail.
7. Every other package, then flip the default and delete v1.
