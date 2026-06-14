# mosstyle — Component Style Language

## Overview

`mosstyle` is a strictly compiled language for declaring the **visual
appearance** of a UI component. A `.msl` file answers exactly one question:
*what do the parts of this component look like, in each of their possible
states?*

It does this by assigning visual properties to named **parts** (declared in
`.mll`) across named **states** (normal, hover, pressed, disabled,
focused). All values are expressed as design tokens resolved at compile time.
No token reaches the runtime unresolved.

Three things are explicitly forbidden in `.msl` files:

1. **Layout properties** — no direction, alignment, flex-grow, or anything
   structural. Those belong in `.mll`.
2. **Slot or emit references** — the style layer has no knowledge of the
   component's interface. It knows part names only.
3. **Arbitrary logic** — no conditionals, no loops, no expressions beyond
   Lattice-compatible value computation (arithmetic, color functions, token
   references).

This boundary means a designer can retheme an entire component library by
supplying a single override style file. They never touch `.mil` or
`.mll`. The compiler validates the override is complete and type-correct
before anything runs.

---

## Position in the Stack

```
mosmodel (.mil)          ← UI13-mosmodel.md
     │
moslayout (.mll)        ← UI14-moslayout.md
     │  exports: part names
     ▼
mosstyle (.msl)          ← THIS SPEC
     │  imports: part names, token values
     │  exports: resolved style map per part per state
     ▼
backend emitter
     │
     ├── DOM / Web Component  → scoped CSS
     ├── AppKit               → NSColor, NSFont, CALayer values
     ├── paint-vm / Metal     → Color structs, f32 values in PaintRect calls
     └── Qt                   → QPalette, QFont, QSS values
```

---

## §1 Design Tokens

A **design token** is a named, typed value that represents a single visual
decision: a color, a size, a font, a duration. Components reference token names,
not concrete values. Concrete values are supplied by a token file that can be
swapped to retheme the entire system.

### Token types

| Type | Example values |
|---|---|
| `color` | `#1e1e1e`, `rgba(0,0,0,0.5)` |
| `length` | `4px`, `1.5rem`, `8pt` |
| `number` | `0.4`, `1`, `2` |
| `duration` | `120ms`, `0.3s` |
| `easing` | `ease-out`, `linear`, `cubic-bezier(0.4,0,0.2,1)` |
| `font-family` | `"Inter"`, `"system-ui"` |
| `font-weight` | `400`, `600`, `bold` |

### Token declaration

Token files use Lattice syntax. Every token in a base theme file is declared
with `!default` so it can be overridden by a consuming theme:

```lattice
// tokens/base.lattice

$color-surface:       #1e1e1e  !default;
$color-surface-hover: lighten(#1e1e1e, 10%) !default;  // computed from base
$color-text-primary:  #ffffff  !default;
$color-text-muted:    rgba(255,255,255,0.6) !default;
$color-accent:        #4a90d9  !default;
$color-border:        rgba(255,255,255,0.12) !default;
$color-danger:        #e53e3e  !default;

$radius-sm:           4px   !default;
$radius-md:           8px   !default;
$radius-lg:           12px  !default;

$spacing-xs:          4px   !default;
$spacing-sm:          8px   !default;
$spacing-md:          16px  !default;
$spacing-lg:          24px  !default;

$font-family-body:    "Inter", system-ui !default;
$font-size-sm:        12px !default;
$font-size-body:      14px !default;
$font-size-lg:        18px !default;
$font-weight-normal:  400  !default;
$font-weight-bold:    600  !default;

$duration-fast:       80ms  !default;
$duration-normal:     150ms !default;
$duration-slow:       300ms !default;
$easing-out:          ease-out !default;
$easing-spring:       cubic-bezier(0.34, 1.56, 0.64, 1) !default;

$opacity-disabled:    0.4 !default;
```

A brand theme overrides only what it needs to change:

```lattice
// tokens/acme-brand.lattice  — loaded before base.lattice
$color-accent:  #ff6b00;
$color-surface: #0a0a0a;
// Everything else falls through to base.lattice defaults
```

---

## §2 Style Properties

The set of visual properties is closed. The compiler knows every property name
and its expected token type. Specifying an unknown property name is a compile
error.

### Color properties

| Property | Token type | Description |
|---|---|---|
| `background` | `color` | Fill color of the part's bounding box |
| `color` | `color` | Foreground / text color |
| `border-color` | `color` | Color of the border stroke |
| `outline-color` | `color` | Color of the focus outline (drawn outside border) |
| `shadow-color` | `color` | Color of the drop shadow |

### Geometry properties

| Property | Token type | Description |
|---|---|---|
| `border-radius` | `length` | Corner rounding radius |
| `border-width` | `length` | Width of border stroke |
| `outline-width` | `length` | Width of focus outline |
| `padding` | `length` | Inner spacing (shorthand: all four sides) |
| `padding-top` | `length` | Top inner spacing |
| `padding-right` | `length` | Right inner spacing |
| `padding-bottom` | `length` | Bottom inner spacing |
| `padding-left` | `length` | Left inner spacing |
| `gap` | `length` | Spacing between children (mirrors flex gap) |
| `shadow-radius` | `length` | Blur radius of the drop shadow |
| `shadow-offset-x` | `length` | Horizontal shadow offset |
| `shadow-offset-y` | `length` | Vertical shadow offset |

### Typography properties (valid on `Text` parts only)

| Property | Token type | Description |
|---|---|---|
| `font-family` | `font-family` | Font face |
| `font-size` | `length` | Type size |
| `font-weight` | `font-weight` | Weight (400 = regular, 600 = semi-bold, 700 = bold) |
| `line-height` | `number` | Line height multiplier (unitless) |
| `letter-spacing` | `length` | Tracking |
| `text-align` | `start` \| `center` \| `end` | Horizontal alignment |
| `text-decoration` | `none` \| `underline` | Decoration |

### Visibility / compositing properties

| Property | Token type | Description |
|---|---|---|
| `opacity` | `number` | 0.0 (transparent) to 1.0 (opaque) |

---

## §3 State Declarations

States represent interactive conditions. Visual properties inside a state block
apply when the component is in that state; they override the base properties.

### Built-in states

| State | When active |
|---|---|
| (none) | Always — the baseline appearance |
| `hover` | Pointer is over the part (desktop / trackpad only) |
| `pressed` | Pointer is down within the part |
| `focused` | Part has keyboard focus |
| `disabled` | The component's `disabled` slot is `true` |
| `selected` | The part is in a selected state (e.g. a grid row) |
| `editing` | The part is in edit mode (e.g. an active grid cell) |
| `error` | The component is in an error state |

State blocks are additive: only properties listed inside the block change. All
other properties retain their base values.

---

## §4 Animation and Transition Declarations

Transitions describe how properties animate when their state changes. They live
in `.msl` because they are a presentational concern — whether a hover
change is instant or animated is the designer's decision, not the engineer's.

```
transition <property> <duration-token> <easing-token> ;
transition <property> <duration-token> ;          // easing defaults to ease-out
```

A `transition` declaration inside a state block applies when **entering** that
state. A `transition` declaration at the base level applies to **all** state
changes for that property.

Keyframe animations (for non-state-driven motion) are declared with `@keyframes`
(Lattice-compatible syntax):

```
@keyframes spin {
  from { transform: rotate(0deg); }
  to   { transform: rotate(360deg); }
}
```

Then applied to a part:

```
animation: spin $duration-slow linear infinite ;
```

Animation declarations are v1-deferred for native backends; they are fully
supported for the DOM backend in v1.

---

## §5 The Override / Cascade System

When multiple style files apply to a component, the compiler resolves them in
priority order from lowest to highest:

```
1. Component base styles        (Button.msl — lowest priority)
2. Platform token overrides     (ios-tokens.lattice, desktop-tokens.lattice)
3. Brand theme                  (acme-brand.lattice)
4. Color scheme                 (dark.lattice, light.lattice)
5. Component-specific override  (highest priority)
```

Each layer is a `.lattice` token file. Because component base styles declare
all tokens with `!default`, a higher-priority file that declares the same token
without `!default` wins automatically.

The compiler:
1. Loads token files in priority order (highest-priority first).
2. Runs the Lattice transformer to resolve all variables, functions, and
   expressions to concrete values.
3. Validates the fully-resolved token map against the mosstyle property
   declarations for the component.
4. Emits the final concrete style map — no tokens remain in the output.

**Warning on undefined tokens:** if a property references a token that is not
defined anywhere in the chain, compilation fails with a clear error message.

**Warning on unused tokens:** if a token is defined in a theme file but no
component in the target set references it, the compiler warns. This prevents
dead token accumulation in theme files.

---

## §6 Complete Component Examples

### Button

```mosstyle
style Button {

  // Root — the clickable container
  part root {
    background:    $color-surface ;
    border-radius: $radius-sm ;
    border-width:  1px ;
    border-color:  $color-border ;
    padding:       $spacing-xs $spacing-md ;
    gap:           $spacing-xs ;

    transition background $duration-fast $easing-out ;
    transition border-color $duration-fast $easing-out ;

    state hover {
      background:   $color-surface-hover ;
      border-color: $color-accent ;
    }

    state pressed {
      background: darken($color-surface, 8%) ;
    }

    state focused {
      outline-color: $color-accent ;
      outline-width: 2px ;
    }

    state disabled {
      opacity: $opacity-disabled ;
    }
  }

  // Icon — the image part
  part icon {
    opacity: 0.8 ;

    state disabled {
      opacity: $opacity-disabled ;
    }
  }

  // Label — the text part
  part label {
    color:       $color-text-primary ;
    font-family: $font-family-body ;
    font-size:   $font-size-body ;
    font-weight: $font-weight-normal ;

    state disabled {
      color: $color-text-muted ;
    }
  }

}
```

### Grid — cell styling

```mosstyle
style Grid {

  part cell-grid {
    background:   $color-surface ;
    border-color: $color-border ;
    border-width: 1px ;
  }

  part cell {
    background:   transparent ;
    color:        $color-text-primary ;
    font-family:  $font-family-body ;
    font-size:    $font-size-body ;
    padding:      $spacing-xs ;

    state selected {
      background: $color-accent ;
      color:      #ffffff ;
    }

    state editing {
      background:    $color-surface-hover ;
      outline-color: $color-accent ;
      outline-width: 2px ;
    }

    state hover {
      background: rgba(255,255,255,0.04) ;
    }
  }

  part column-header {
    background:  lighten($color-surface, 5%) ;
    color:       $color-text-muted ;
    font-size:   $font-size-sm ;
    font-weight: $font-weight-bold ;
    padding:     $spacing-xs ;
    border-color: $color-border ;
    border-width: 1px ;
  }

  part row-header {
    background:  lighten($color-surface, 5%) ;
    color:       $color-text-muted ;
    font-size:   $font-size-sm ;
    padding:     $spacing-xs ;
    text-align:  end ;
  }

}
```

---

## §7 Grammar

### Token file (`mosstyle.tokens`)

```
# Keywords
KW_STYLE      = "style"
KW_PART       = "part"
KW_STATE      = "state"
KW_TRANSITION = "transition"
KW_ANIMATION  = "animation"
KW_KEYFRAMES  = "@keyframes"
KW_FROM       = "from"
KW_TO         = "to"

# State names
KW_HOVER      = "hover"
KW_PRESSED    = "pressed"
KW_FOCUSED    = "focused"
KW_DISABLED   = "disabled"
KW_SELECTED   = "selected"
KW_EDITING    = "editing"
KW_ERROR      = "error"

# Identifiers and values
IDENT         = /[a-zA-Z][a-zA-Z0-9]*/
KEBAB_IDENT   = /[a-z][a-z0-9]*(-[a-z][a-z0-9]*)*/
TOKEN_REF     = /\$[a-z][a-z0-9]*(-[a-z][a-z0-9]*)*/  # $token-name
NUMBER        = /[0-9]+(\.[0-9]+)?/
DIMENSION     = /[0-9]+(\.[0-9]+)?(px|rem|em|pt|ms|s|deg|%)/
STRING        = /"([^"\\]|\\.)*"/
HASH_COLOR    = /#[0-9a-fA-F]{3,8}/

# Punctuation
LBRACE        = "{"
RBRACE        = "}"
LPAREN        = "("
RPAREN        = ")"
SEMICOLON     = ";"
COLON         = ":"
COMMA         = ","

# Whitespace and comments — skipped
WHITESPACE    = /\s+/           skip
LINE_COMMENT  = /\/\/[^\n]*/   skip
BLOCK_COMMENT = /\/\*.*?\*\//  skip
```

### Grammar file (`mosstyle.grammar`)

```
mosstyle_file = style_def ;

style_def = KW_STYLE IDENT LBRACE { part_def } RBRACE ;

part_def = KW_PART KEBAB_IDENT LBRACE { part_item } RBRACE ;

part_item = property_decl
          | transition_decl
          | animation_decl
          | state_block ;

state_block = KW_STATE state_name LBRACE { state_item } RBRACE ;

state_name = KW_HOVER | KW_PRESSED | KW_FOCUSED | KW_DISABLED
           | KW_SELECTED | KW_EDITING | KW_ERROR ;

state_item = property_decl | transition_decl ;

property_decl = KEBAB_IDENT COLON style_value SEMICOLON ;

style_value = TOKEN_REF
            | HASH_COLOR
            | DIMENSION
            | NUMBER
            | STRING
            | IDENT           # keyword values: none, underline, start, center, end, bold
            | function_call ;

function_call = IDENT LPAREN style_value { COMMA style_value } RPAREN ;

transition_decl = KW_TRANSITION KEBAB_IDENT TOKEN_REF [ TOKEN_REF ] SEMICOLON ;
                  # property-name  duration-token  optional-easing-token

animation_decl = KW_ANIMATION COLON IDENT TOKEN_REF IDENT [ IDENT ] SEMICOLON ;
                 # @keyframes-name  duration  easing  fill-mode
```

The grammar is context-free and LL(1). All state names are keywords, removing
any ambiguity with part names. All property names are `kebab-case` identifiers
validated by the semantic layer against the known property table, not by the
grammar.

---

## §8 Compiler Behaviour

### Inputs

1. A `.msl` file.
2. The part map JSON exported by the `moslayout` compiler for the same component.
3. The resolved token map produced by the Lattice transformer (after loading all
   token files in priority order).

### Validation

1. **Known parts** — every `part <name>` block must correspond to a name in the
   part map. Referencing a part that does not exist in the layout is an error.
2. **Known properties** — every property name is checked against the closed
   property table in §2. Unknown property names are compile errors.
3. **Type compatibility** — the resolved value for each property must be
   type-compatible. `background` expects a `color`-typed token; providing a
   `length` token is a compile error.
4. **Typography on non-Text parts** — font properties are only valid on parts
   whose primitive is `Text`. Applying `font-size` to a `Box` part is an error.
5. **Token resolution** — every `$token-ref` must resolve to a concrete value
   after the Lattice pass. Unresolved tokens are compile errors.
6. **Valid states** — only states from the built-in state list are valid inside
   a state block.
7. **Transition references** — the property named in a `transition` declaration
   must be a property declared elsewhere in the same part block.

### Outputs

**1. Resolved style map (internal, JSON)**

After compilation, all tokens are replaced with concrete values. No `$token-ref`
remains.

```json
{
  "component": "Button",
  "parts": {
    "root": {
      "base": {
        "background":    "#1e1e1e",
        "border-radius": "4px",
        "border-width":  "1px",
        "border-color":  "rgba(255,255,255,0.12)",
        "padding":       "4px 16px",
        "gap":           "8px"
      },
      "transitions": [
        { "property": "background",   "duration": "80ms", "easing": "ease-out" },
        { "property": "border-color", "duration": "80ms", "easing": "ease-out" }
      ],
      "states": {
        "hover":    { "background": "#2e2e2e", "border-color": "#4a90d9" },
        "pressed":  { "background": "#161616" },
        "focused":  { "outline-color": "#4a90d9", "outline-width": "2px" },
        "disabled": { "opacity": "0.4" }
      }
    },
    "label": {
      "base": {
        "color":        "#ffffff",
        "font-family":  "\"Inter\", system-ui",
        "font-size":    "14px",
        "font-weight":  "400"
      },
      "states": {
        "disabled": { "color": "rgba(255,255,255,0.6)" }
      }
    }
  }
}
```

**2. Backend-specific output**

*DOM / Web Component — scoped CSS:*

```css
/* Generated — do not edit */
.mos-Button-root {
  background:    #1e1e1e;
  border-radius: 4px;
  border:        1px solid rgba(255,255,255,0.12);
  padding:       4px 16px;
  gap:           8px;
  transition:    background 80ms ease-out, border-color 80ms ease-out;
}
.mos-Button-root:hover    { background: #2e2e2e; border-color: #4a90d9; }
.mos-Button-root:active   { background: #161616; }
.mos-Button-root:focus    { outline: 2px solid #4a90d9; }
.mos-Button-root:disabled { opacity: 0.4; }

.mos-Button-label {
  color: #ffffff;
  font-family: "Inter", system-ui;
  font-size: 14px;
  font-weight: 400;
}
.mos-Button-root:disabled .mos-Button-label { color: rgba(255,255,255,0.6); }
```

*Metal / paint-vm (Rust):*

```rust
// Generated — do not edit
pub struct ButtonStyle {
  pub root:  RootPartStyle,
  pub label: LabelPartStyle,
}

pub struct RootPartStyle {
  pub background:    Color,
  pub border_radius: f32,
  pub border_width:  f32,
  pub border_color:  Color,
  pub padding:       EdgeInsets,
  pub gap:           f32,
  // states
  pub hover_background:    Color,
  pub hover_border_color:  Color,
  pub pressed_background:  Color,
  pub focused_outline_color: Color,
  pub focused_outline_width: f32,
  pub disabled_opacity:    f32,
}

impl ButtonStyle {
  pub fn default() -> Self {
    Self {
      root: RootPartStyle {
        background:    Color::from_hex("#1e1e1e"),
        border_radius: 4.0,
        border_width:  1.0,
        border_color:  Color::rgba(255,255,255,0.12),
        padding:       EdgeInsets { top: 4.0, right: 16.0, bottom: 4.0, left: 16.0 },
        gap:           8.0,
        hover_background:   Color::from_hex("#2e2e2e"),
        hover_border_color: Color::from_hex("#4a90d9"),
        pressed_background: Color::from_hex("#161616"),
        focused_outline_color: Color::from_hex("#4a90d9"),
        focused_outline_width: 2.0,
        disabled_opacity: 0.4,
      },
      // …
    }
  }
}
```

---

## §9 Error Messages

| Error | Condition | Example message |
|---|---|---|
| `UnknownPart` | Part name not in layout's part map | `Unknown part 'body' at line 4 — Button layout exports: root, icon, label` |
| `UnknownProperty` | Property name not in property table | `Unknown style property 'flex-direction' at line 8 — layout properties belong in .mll` |
| `TypeMismatch` | Token type incompatible with property | `Property 'background' expects a color token, but '$font-size-body' resolves to a length at line 12` |
| `TypographyOnNonText` | Font property on non-Text part | `'font-size' is only valid on Text parts, but 'root' is a Box at line 16` |
| `UnresolvedToken` | Token reference has no definition | `Token '$color-brand-primary' is not defined in any token file at line 9` |
| `UnusedToken` | Theme token not referenced by any component | `Token '$old-accent-color' defined in acme-brand.lattice is not used by any component` |
| `UnknownState` | Unrecognised state name | `Unknown state 'hovered' at line 21 — did you mean 'hover'?` |
| `TransitionPropertyNotDeclared` | Transition references undeclared property | `Transition on 'color' at line 25, but 'color' is not declared in the base style for part 'root'` |

---

## §10 Relationship to Other Specs

- **UI14-moslayout.md** — supplies the part map that this compiler imports.
- **17-lattice-transpiler.md** and **lattice-v2.md** — the Lattice transformer
  runs as a pre-pass before this compiler, resolving all token references to
  concrete values. `mosstyle` consumes the resolved output, not raw Lattice.
- **UI04-layout-to-paint.md** — for the Metal/paint-vm backend, the resolved
  style map is consumed here to produce PaintRect, PaintGlyphRun, and
  PaintPath calls with correct colors, radii, and font metrics.

---

## §11 Out of Scope

- Layout properties (`direction`, `align`, `grow`, etc.) — see `moslayout`
- Platform-specific layout — see `moslayout`
- Slot or emit declarations or references — see `mosmodel`
- Runtime theming (changing tokens after compile time) — the DOM backend
  naturally supports this via CSS custom properties as a tooling aid during
  development; production builds use fully resolved concrete values
- Complex keyframe animations (v1) — fully supported for DOM backend; native
  backends (Metal, AppKit, Qt) animate via their own animation systems using
  the duration and easing values from the resolved style map; keyframe support
  for native backends is a v2 deliverable
- Conditional styles based on slot values — the host changes slot values and
  the layout re-renders; visibility and state changes are driven by the host
  updating the relevant state slot (e.g. `disabled`), not by mosstyle
  conditionals
