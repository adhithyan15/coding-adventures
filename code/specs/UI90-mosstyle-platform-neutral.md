# UI90 — a typed, platform-neutral mosstyle

**Status:** All §5 decisions settled (2026-10-12). Ready for implementation in the §6 order.
**Implements:** `MOSAIC-BACKLOG.md` X-6, and the direction in #12029
("evolve from a CSS superset into a typed, platform-neutral style
language"). Builds on UI15 (mosstyle), UI49 (slot states) and UI41
(elevation). Feeds #12022 (dropped styles fail the gate).

## 1. The principle

> mosstyle expresses every styling decision without platform-specific
> vocabulary or abstraction leaks. *(Decided 2026-10-08.)*

> Every app can be branded in whatever ways its author wants, and still
> feels like a native citizen of its platform: it uses the platform's own
> controls, and mosstyle exposes ways to style them. *(Decided 2026-10-11,
> §5.1.)*

> Copy what existing design systems already do; do not reinvent it. A
> design made in Figma (or a similar tool) should become a native Mosaic
> app quickly: first a working app on sample data, then wired to a real
> backend. *(Decided 2026-10-12, §5.2–§5.4.)*

The principle has three parts, and the design needs all of them:

- **No leaks.** Nothing in a `.msl` file is written for one backend: no XAML
  resource lookup, no CSS function, no web-only layout trick. The first
  slice is enforced already. A value that starts with `{` is a compile error
  (`PlatformValue`, UI15 §8 rule 8).
- **Enough expressiveness.** Every decision an author really makes has a
  neutral way to say it. Otherwise authors reach for a leak, which is what
  happened.
- **Native controls, styled, never replaced.** Every backend renders the
  platform's own control: a WinUI `Button`, a SwiftUI `Button`, a Material
  `Button`, a `QPushButton`, an HTML `<button>`. A style changes how that
  control looks. It never swaps in a custom-drawn lookalike. Whatever the
  author leaves unstyled keeps the platform's look and behaviour, including
  focus, hover and press feedback, accessibility, and reduced motion.
  Branding is opt-in, one decision at a time.

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
| `type-role` | Material 3's type scale: `display-`, `headline-`, `title-`, `body-` and `label-`, each with `-large`, `-medium` or `-small` (15 roles) | most `font-size` / `font-weight` pairs. Each backend maps it to its own type ramp (§5.2) |
| `font-size` | Length | (allowed when no role fits. It still scales with the user's text size, §5.2) |
| `font-weight` | `regular` \| `medium` \| `semibold` \| `bold` | `400`/`600`, `SemiBold` |
| `font` | `system` \| `mono` \| `serif` \| *brand face token* (a font shipped with the app, §5.4) | CSS font stacks, the `font` shorthand |
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

#12029 was blocked on "brand vs native": a fixed brand palette and native
look-and-feel seemed to conflict. **Decided (2026-10-11, §5.1):** they do not
conflict. Each token says which kind it is, so the author decides one
decision at a time rather than once for the whole app:

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
  covers both modes. This copies Apple's Color Sets, XAML's
  `ThemeDictionaries` and Figma's variable modes (§5.3):

  ```text
  token color-text   { light: #1f1a14 ; dark: #f4efe6 ; }
  token color-card   { light: #ffffff ; dark: #2a2420 / 0.96 ; }
  token color-border { light: #d9cfc2 ; dark: #4a4038 ;
                       light-high-contrast: #000000 ; dark-high-contrast: #ffffff ; }
  ```

  `light` and `dark` are both required; a token with one and not the other
  is a compile error. The high-contrast values are optional and fall back
  to the plain mode's value, as Apple's do.

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
  - render each part as the platform's native control, and apply styles to
    that control (§1);
  - emit nothing for a property the author left unset, so the control keeps
    the platform's default for it;
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

## 5. Decisions

1. **Brand vs native (the #12029 blocker).** **Decided 2026-10-11: yes, the
   per-token split.** In the decision's words: "Every person building an
   app will want to be able to 'brand' the app in whatever ways they want.
   But the app should also feel like a native citizen of the platform. As
   long as we use native controls and expose ways for people to style them,
   I am more than fine with it." This is now part of §1. Brand tokens are
   fixed, system tokens resolve per host, and unstyled properties keep the
   platform default. For example, Trestle can keep its warm surfaces as
   brand tokens and take `$system-accent` and `$system-focus` from the
   host.

Decisions 2–4 were settled on 2026-10-12 by one rule: "We can copy whatever
existing systems have come up with. I do not want to reinvent the wheel."
The goal behind it is going from a Figma design to a native app quickly, so
each choice below is also the one that imports from Figma without
translation.

2. **Type roles.** **Decided: roles first, sizes allowed, as every platform
   does.** Apple, Material and Windows all ship named text styles and still
   accept an explicit size, so mosstyle does the same:
   - `type-role` takes Material 3's 15-role type scale (`display-large` …
     `label-small`). Compose and Flutter use these exact names. The Material
     3 Figma kit names its text styles the same way. Apple and Windows map
     each role to their nearest text style; the mapping table lands with the
     emitter work (§6 step 5).
   - `font-size` stays for the cases no role fits. Like
     `Font.custom(_:size:relativeTo:)` on Apple and `sp` units on Android,
     an explicit size still scales with the user's text-size setting, so
     accessibility is not lost.
   - Values stay typed throughout: an unknown role or a non-length size is
     a compile error.
3. **Mode syntax.** **Decided: mode-aware tokens (§3.3).** Every system
   surveyed keeps mode out of component styles. Components name a role; the
   role holds a value per mode:
   - Apple: a Color Set with Any, Dark and high-contrast appearances;
   - XAML: `ThemeDictionaries` with `Light`, `Dark` and `HighContrast`;
   - Compose and Flutter: a light and a dark `ColorScheme` with the same
     role names, which the emitters generate from the tokens;
   - Figma: one variable with a column per mode;
   - CSS is the outlier with `@media (prefers-color-scheme)` blocks, and it
     added `light-dark()` to move towards per-value modes.

   No `scheme dark { … }` blocks. High contrast is a mode from the start,
   because Apple and Windows both have it.
4. **Brand faces.** **Decided: custom fonts ship with the app.** A font
   file is a package asset, declared in the package manifest with its
   licence file next to it, and each emitter bundles it the platform's own
   way:
   - Apple: `UIAppFonts` (iOS) or `ATSApplicationFontsPath` (macOS);
   - Android: `res/font`;
   - Flutter: `fonts:` in `pubspec.yaml`;
   - Windows: package content, referenced as `ms-appx:///…#Family`;
   - Qt: `QFontDatabase::addApplicationFont`;
   - web: `@font-face`.

   A brand face token names the family. When a weight is missing from the
   shipped files, the platform's own synthesis or fallback applies.

### 5.5 Design-tool alignment

The decisions above line up with what Figma and the W3C Design Tokens
Community Group format already express:

| Figma / DTCG | mosstyle |
|---|---|
| Variable collection with modes | mode-aware tokens (`light`, `dark`, high contrast) |
| Colour, number and dimension variables | brand tokens of the matching type |
| Text styles named on the Material 3 scale | `type-role` |
| Font family in a text style | brand face token, plus the shipped font files |

Importing a design is UI91's job (to be written; `MOSAIC-BACKLOG.md` DN-1).
This table is the contract it imports into.

## 6. Order of work

1. This spec, agreed.
2. **Compiler, behind versioning:**
   - the property table and types;
   - `UnknownProperty` and `TypeMismatch`;
   - the colour syntax;
   - v1 warnings.
3. **Tokens:** system tokens in every emitter, then mode-aware tokens
   (with high contrast), then brand faces shipped as package assets.
4. **`style migrate`**, run on the toolkit; then the toolkit flips to v2.
5. **Emitters:** delete pass-throughs, then lower the new properties
   (`type-role`, `tracking`, `text-case`, `max-lines`, `align-main`,
   `align-cross`, `focus-ring`, `motion`) on all eight backends.
6. **TaskApp to v2.** The 741-drop ratchet should fall to near zero, and
   #12022 can then hard-fail.
7. Every other package, then flip the default and delete v1.
