# Changelog — mosaic-emit-flutter

All notable changes to `mosaic-emit-flutter` will be documented in
this file.

## [Unreleased]



### Fixed -- containers rendered square on Flutter, ignoring `border-radius`

`emit_container` -- the writer that runs for a styled Row, Column or Box
wrapper -- read background, border and elevation and **never looked at the
radius**. Every container-shaped part rendered with square corners however it
was authored.

Trestle's `task-card` authors `border-radius: 13` and emitted:

```dart
decoration: BoxDecoration(
  color: const Color(0xFF252019),
  border: Border.all(color: const Color(0xFF352E25), width: 1),
  boxShadow: [BoxShadow(...)]),          // no borderRadius
```

**#15225 fixed this in the other writer only**, and left a comment there
stating that `emit_container` already handled the radius. That claim was
false, and it is corrected in this change -- it is exactly the sentence that
would stop the next reader from looking.

Measured in the emitted artifact rather than inferred: across Trestle's dark
theme, authored radii 9, 10 and 13 appeared **zero** times in the generated
Dart, and 20 appeared twice against eleven authored. Sixteen radii are
restored.

**Not every authored radius comes back, by design.** `board-card` and
`board-card-crit` author a 3px left accent alongside `border-radius: 9`, and
Flutter's `Border.paint` throws *"A borderRadius can only be given on borders
with uniform colors"* the moment a non-uniform `Border(...)` carries one --
taking out the widget and everything above it in any debug or profile build.
Those two stay square, gated exactly as the other writer gates them. Radius 9
is therefore still absent from the output, and that is correct.

Qt does not share this gap: it emits `radius: 13` six times for the same six
authored declarations. The defect was Flutter's container writer alone.

Nothing caught this. The suite passed identically before and after the fix,
because no test asserted on this writer's radius at all; two now do, and the
gate test carries the uniform case as its control so neither can pass
vacuously.

### Changed -- Flutter has a style-lowering seam for the first time

Everything `emit_styled_box` lowers from a part's own style props now lives
in a pure `flutter_box_style(base, layers, ctx) -> FlutterBoxStyle`. Not one
byte of emitted output changes.

The point is the shape, not the saving. `emit_styled_box` takes a
`LayoutNode`, the component name and the emit declarations and returns
generated Dart **text**, so the only way to ask "what did lowering do with
`border-radius`?" was to emit an entire widget subtree and read the answer
back out of a string. Compose can answer that question directly, because
`compose_box_style` is a plain function from style props to a style -- and
that, not any shortage of care, is why Compose has a dropped-property
reporter and Flutter does not (#12022).

**What this does not do.** It adds no reporting, records no dropped
property, changes no gate and is wired into nothing. Flutter's
`styleDegradations` is still empty, and still means "nobody looked" rather
than "nothing was lost". The accumulator and the reporter that reads it
belong in one change and will land together once every Flutter writer is
covered: there are three (`emit_container`, `emit_styled_box`, and the
host-widget helpers), and a reporter that saw only one would make the
report look populated while remaining blind to the rest.

The caller keeps what needs the tree. The child is emitted by
`emit_styled_box` and appended as the last `args` entry, because building it
needs the layout and can fail; `inherited_text_style` is returned rather
than applied, because it is threaded into the `TableCtx` the child is
emitted under (#15166).

Verified byte-for-byte rather than by argument: the Flutter artifacts for
all four Mosaic product packages -- VisiCalc, Venture, Engram and Trestle --
are identical before and after (27 files, 618,546 bytes, zero diff lines).
The comparison was itself falsified by planting a marker inside the new
function, which moved 337 lines across 83 sites; an equality that cannot
fail is not evidence.

### Fixed -- `font: inherit` now reaches a `TextField` (#15166)

A Flutter `TextField` does **not** read the enclosing `DefaultTextStyle` the
way a `Text` does -- it falls back to the Material theme. So an input
authoring `font: inherit` had no way to follow the text around it, and the
grid's inline editor rendered at the theme's font while the cell beside it
rendered at the authored one.

Measured in a real `flutter test` with a real font, inside the cell's own
text context:

| editor | height | delta |
| --- | --- | --- |
| display `Text` | 19.0 | — |
| without the inherited font | 24.0 | **+5** |
| with it | 20.0 | **+1** |

The residual 1px is the editable's own line box and is not reachable from a
stylesheet.

`DefaultTextStyle.of(context)` cannot substitute for this: the `context` in
scope at the input is the widget's own, which sits **above** the merge the
emitter just wrote. So the style is computed before the child is emitted and
threaded down on `TableCtx`, beside the `sheet_text_*` fields that already
work this way.

Three things security review found, all inside this change and all inert for
the products:

- **The threaded copy is validated separately from the merge copy.** The
  merge reads `font-size` through `parse_pixel_value`, whose contract is "0
  on anything unreadable" -- harmless for a `Text`, which the theme still
  sizes, but on a `TextField` `fontSize: 0` is an editor the user cannot see
  or place a cursor in. Threading the merge copy verbatim walked around the
  invariant the explicit path already pins.
- **A `For` resets it.** The threaded style is TEXT, and a loop rebinds the
  identifiers inside it -- so carrying it across would change the loop's
  shape (the emitters pick `(_)` vs `(item)` by scanning the emitted body
  for those identifiers) and make the input's copy evaluate a predicate
  against the loop binding while the enclosing merge evaluates it against
  the outer one. `emit_host_table` resets it for the same reason.
- **The test was vacuous.** It asserted over the whole output, and the
  `DefaultTextStyle.merge` line has always contained the same byte
  sequence -- so it passed with the feature entirely reverted. It asserts on
  the `TextField`'s own slice now.

**Only an explicit `inherit` counts.** CSS does not give a text input the
surrounding font by default -- which is exactly why `font: inherit` is a
standard reset -- so applying the enclosing style to every unstyled input
would diverge from the web backends rather than agree with them. Emitted
output gains exactly one `style:` argument, on the inline grid editor.

### Fixed -- a relative length collapsed the subtree to zero (#15213)

`fixed_pixel_length` exists because `parse_pixel_value`'s "unreadable -> 0"
fallback turns a size into a zero-size box and eats whatever is inside it.
It declined `%` and (since #15160) negatives, and let **every other relative
form** through to that exact fallback -- `100vh`, `max-content`, `auto`,
`calc(...)`, `rem`, `em`.

Two writers also bypassed it entirely, calling `parse_pixel_value` for
width and height directly: `emit_styled_box` and
`style_prop_to_container_arg`.

**Three subtrees were collapsed in the shipped products**, measured on the
emitted Dart:

| product | part | authored | emitted |
| --- | --- | --- | --- |
| visicalc | `Column [workbook]` -- **the root** | `height: 100vh` | `Container(height: 0)` |
| task-app | two calendar cells | `width: 100%` | `Container(width: 0)` |

VisiCalc's is the root of the whole component, so the Flutter build rendered
**nothing at all**.

The parse is a positive test now -- a plain pixel value, optionally suffixed
`px` -- so a unit nobody has thought of yet declines by default rather than
collapsing a subtree. Declining means emitting **no size argument**, leaving
the child to size itself, which is what `width: 100%` wants in the first
place.

Two things found in security review, both inside this change:

- **Declining a width removes a bound.** `Expanded` was gated only on
  `direct_row_child`, never on `direct_row_accepts_flex` -- despite a
  comment thirty lines below claiming it was. So removing the `SizedBox`
  that had been a subtree's only width bound could leave an `Expanded`
  measured unbounded, which throws
  `RenderFlex children have non-zero flex but incoming width constraints
  are unbounded`. That would have turned a silently-blank subtree into a
  thrown layout error -- a different failure, not a fixed one. The gate now
  matches what the comment always claimed. No product output changes.
- **The charset gate is not a parse.** `1-2`, `1.2.3` and `.` pass the
  character check and still fail to parse, and delegating those to
  `parse_pixel_value` answered `0` -- the very collapse this function
  exists to prevent, through a narrower door. The result is derived from a
  real parse now.

**Flutter only.** Compose, Qt, SwiftUI, XAML, React and HTML all decline or
pass through relative lengths correctly; measured on VisiCalc, whose root
carries `height: 100vh`. The corpus authors 86 relative lengths, so most
already reached writers that handled them -- these three reached ones that
did not.

### Fixed -- `emit_styled_box` never applied `border-radius` (#15225)

This builder assembled its `BoxDecoration` from background, border and
elevation and simply never looked at the radius, so a styled box came out
SQUARE on Flutter however it was authored -- a plain `border-radius: 8` was
dropped here just as surely as a percentage one.

The same property visibly works elsewhere in the same file because
`emit_container` does read it. Two writers, and only one of them had it.

**This is not scoped to the percentage case.** Measured across the products:
engram-app gains 24 radii it was silently losing, and task-app 17, none of
which involve a percentage at all. Every changed line is the same
`BoxDecoration` with a radius inserted -- verified by stripping the
insertion and asserting the line is then byte-identical to before.

An unreadable radius is dropped rather than coerced to `0`, since a zero
radius is a square.

**The radius is never paired with a per-edge border.** Flutter's
`Border.paint` throws *"A borderRadius can only be given on borders with
uniform colors"* the moment a non-uniform `Border(...)` carries one, and
asserts separately on a hairline side -- taking out the widget and
everything above it in any debug or profile build. Before this change the
builder emitted no radius at all, so the pairing was unreachable; adding one
without the gate turns an authored per-edge border plus a radius into a
runtime crash.

That was not hypothetical. **Three widgets in task-app** author exactly that
combination, and an ungated version emitted all three. Found in security
review, before it shipped.

### Fixed -- a negative length reached four unguarded `BorderSide` writers (#15160)

Flutter's `BorderSide` constructor is `assert(width >= 0.0)`, so a negative
authored width produces Dart that type-checks and then **throws when the
widget builds**, taking out that widget and everything above it in the
tree. It is a runtime crash reachable from any stylesheet, not a rendering
glitch.

`per_edge_border_expr` had guarded its own path for a while. Four other
writers took their width straight from `parse_pixel_value`, which happily
returned `-5`:

| site | construct |
| --- | --- |
| styled container | `border: Border.all(color: .., width: {w})` |
| `emit_styled_box` | `border: Border.all(color: .., width: {w})` |
| button shape | `side: BorderSide(width: {w})` |
| `path_paint` | `Border.all(color: {stroke}, width: {w})` |

`Border.all` builds a `BorderSide`, so all four hit the same assert.

The guard is central now: `parse_pixel_value` rejects negatives and falls
back to `0`, the same answer it already gave for anything unreadable. That
also covers `EdgeInsets`, `SizedBox` and every other length sink at once.

**Audited before centralising**, because a shared helper is the wrong place
for a rule that does not hold everywhere. Every property reaching it is
non-negative geometry -- gap, padding, width, height, min-height,
border-width, border-radius, font-size, stroke width -- and CSS forbids a
negative for each. No margin, inset, offset or letter-spacing is lowered
through it, so nothing that legitimately admits a negative loses one.
(`top`/`left`/`right`/`bottom` appear nearby only as per-edge border names.)

**A dead check the central fix would have created.** `per_edge_border_expr`
tested `w.starts_with('-')` on the PARSED value. Once `parse_pixel_value`
never returns a leading `-`, that check can never fire, and the edge would
have been silently emitted at width 0 instead of skipped -- a different
answer, since skipping lets the shorthand cascade in. It now reads the
authored text. An existing test caught this, which is the only reason it is
not in this release.

**Two corrections from security review, both in this change.**

A central rule is only safe where it holds everywhere, and it did not quite:

- `fixed_pixel_length` delegates here, and it exists *because* the `0`
  fallback collapses a subtree into a zero-width box -- its own doc records
  catching that in a real `flutter test` render. Centralising the guard
  made `width: -5px` on a `Row` part go from `SizedBox(width: -5)`, which
  trips Flutter's `debugAssertIsValid` **loudly**, to `SizedBox(width: 0)`,
  which **silently** eats the subtree. Trading a loud failure for a silent
  one is the wrong direction, so that site now drops a negative outright,
  the same way it already drops a `%`.
- IEEE `-0.0 >= 0.0` is true and Rust prints it `-0`, so `border-width:
  -0px` emitted `width: -0` from a legal input. Not a crash -- Dart reads
  it as `-0.0`, which satisfies the assert -- but it falsified the very
  invariant these tests assert. The sign of zero is normalised.

A second review round found two more of the same shape, both now closed:

- `strict_pixel_length` -- the sibling helper feeding `BorderSide` and
  `TextStyle` on the `HostInput` path -- had the identical negative-zero
  leak, so `border: -0px solid #ff0000` emitted `width: -0` and
  `font-size: -0px` emitted `fontSize: -0`.
- `part_max_width` **validated the parse and emitted the authored text**,
  so its guard proved nothing about what shipped. Rust's float grammar
  accepts a leading `+` and Dart has no unary `+` on a literal, so
  `max-width: +760px` passed validation and emitted `maxWidth: +760` -- a
  hard compile error from one authored value. A 22-digit literal got
  through the same way. It now emits the parsed value, with the same
  magnitude cap as every other length path.

And one of the new tests was **vacuous**: it asserted
`!out.contains("EdgeInsets.all(-")` on a fixture whose path emits
`EdgeInsets.symmetric`, so it could never fail and read as coverage it did
not provide. It now asserts what that path actually emits.

**No product change.** Zero of the 3,536 length declarations in the
authored `.msl` corpus is negative, and emitted Flutter output for
task-app, visicalc and engram-app is byte-identical. This closes a latent
trap; it does not fix a live defect.

### Fixed -- `HostInput` ignored its part style entirely (#15142)

`emit_host_input` took `_part_styles` and never read it, so no authored
style reached a text input on any Flutter build. That is a geometry bug,
not a cosmetic one. A bare `TextField` keeps Material's default decoration
-- an underline border and a 48dp minimum touch target -- and, because a
`TextField` does **not** inherit the enclosing `DefaultTextStyle` the way a
`Text` does, it also renders at the Material theme's font rather than the
one beside it.

Measured with a real `flutter test` and a real font loaded, a display
`Text` against an editor in the same cell:

| editor | height | delta |
| --- | --- | --- |
| bare `TextField` (before) | 48.0 | +28 |
| with the part's decoration | 24.0 | +4 |
| with its decoration and text style | 21.0 | +1 |

In a grid that made the editing row taller than every display row, which
is the Flutter half of #15048.

What now lowers, from the part named on the node:

| authored | Dart |
| --- | --- |
| `padding` | `isDense: true, contentPadding: EdgeInsets.all(N)` |
| `border: 0` / `none` | `border: InputBorder.none` |
| `border: W solid C` | `OutlineInputBorder(borderSide: BorderSide(..))` |
| `background` | `filled: true, fillColor: ..` |
| `color`, `font-size`, `font-family`, `font-weight` | `style: TextStyle(..)` |

`isDense` matters as much as the padding: without it Material enforces the
48dp minimum that no `contentPadding` can undercut, and it is most of the
+28.

**What this does not do.** The residual +1 is the editable's own line box
and is not reachable from a stylesheet. A part that authors `font: inherit`
rather than an explicit size -- `mosaic-pkg-grid`'s `cell-editor`, for
instance -- still lands at +4, because honouring `inherit` needs the style
from a context *below* the generated `DefaultTextStyle.merge`, which a
`Builder` would have to supply; filed separately. Unrecognised properties
are still dropped silently, because this emitter has no style-drop
reporting at all (#12022).

Two build-breaking value ranges were closed while adding these sinks, both
found in security review:

- A **negative** border width reached `BorderSide`, whose constructor is
  `assert(width >= 0.0)` -- so `border: -5px solid #ff0000` type-checked and
  then threw when the widget built, taking out the input and everything
  above it in the tree. `per_edge_border_expr` had guarded this for a while
  and the new parse reintroduced it; it now rejects negatives and falls back
  to Material's default.
- `parse_pixel_value` filtered only `is_finite`, but Rust's `Display` for
  `f64` never uses exponent notation, so a finite `1e300` expanded to a
  **301-digit** bare literal. Dart rejects that outright
  (`integer_literal_imprecise_as_double`), so one authored value stopped the
  whole generated app compiling. Values beyond the exactly-representable
  integer range now fall back to `0` like any other unreadable input. This
  helper is shared, so the fix reaches container padding and sizing too.

A third, subtler one: `font-size` was read through `parse_pixel_value`,
whose "0 on anything unreadable" fallback turned `font-size: inherit`,
`90%` or `0.9rem` into `fontSize: 0` -- Dart that compiles and then renders
the input's text at zero size, invisible. Unreadable lengths on this path
are now dropped so the theme's size survives. Both new length sinks share
one `strict_pixel_length`, which rejects unreadable, negative and absurd
values alike.

The CSS-wide keywords are dropped rather than guessed at: `css_color_to_dart`
returns `None` for `inherit` and `transparent`, so neither invents a brush.
That is deliberate -- inventing one is exactly how Compose and SwiftUI paint
invisible text (#15141).

`decoration:` now has exactly ONE producer. The hint text used to be written
on its own, and a second `decoration:` argument is a Dart compile error; a
test pins the two sharing one. The Qt emitter has this same defect today
with `color` (#15155).


### Added — Row and Column honor `gap` after dynamic children (UI85, #14804)

Authored `gap` values on Flutter `Row` and `Column` parts now insert
horizontal or vertical `SizedBox` separators. The generated helper receives
the already evaluated child list, so `If` and `For` children do not leave
leading, trailing, or doubled spacing when they produce no widget.

The lowering preserves the authored `Row` or `Column`; it does not substitute
`Wrap` and therefore does not change flex behavior. `Stack` and incidental
multi-child `Box` columns remain unsupported for `gap` so the UI84 dropped
style reporter can identify those declarations instead of silently assigning
them incorrect layout semantics.

### Security — an authored value is validated before it becomes Dart

Two injection sinks of the same class, both reachable from any stylesheet.

#### `opacity` passed authored text through verbatim

`dart_opacity_literal`'s fallback arm was `trimmed.to_string()`, interpolated
straight into `Opacity(opacity: {expr}, ..)`. So `opacity: "1 ), child:
evil(/*"` emitted

```dart
Opacity(
  opacity: 1 ), child: evil(/*,
  child: Container(
```

escaping the argument and commenting out what followed. A non-numeric opacity
now falls back to fully opaque — the same choice the colour path makes, and an
authored value the emitter cannot understand should not become code.

This one predates UI79 entirely and is untouched by the border work; it was
found by re-reviewing after the colour fix and asking whether the same shape
existed elsewhere in the file. It did.

#### A hex colour is validated by its digits, not its length

`css_color_to_dart` checked only that 6 or 8 characters followed the `#`, then
interpolated them straight into a Dart expression. The `.msl` grammar's
`HASH_COLOR` really is hex-only, but `style_value` also admits a quoted
`STRING`, which passes through verbatim and is unquoted before it reaches here
— and quoted colours are idiomatic in this repo.

So an authored `border-top-color: "#00)+E(/*"` emitted

```dart
border: Border(top: BorderSide(color: const Color(0x00)+E(/*), width: 1), ...
```

escaping the `Color(..)` argument and opening a comment that swallowed the next
widget property. That is code injection into generated Dart, reachable from any
stylesheet, and it predates UI79 — `background`, `background-color`, `color`
and `border-color` all funnel through the same function. UI79 added four more
entry points to it, which is how it was found.

Fixed at that one function so every sink is covered; a rejected colour lands on
each caller's existing fallback. Verified by re-emitting the hostile stylesheet
and seeing `Colors.transparent`.

Checked rather than assumed: **SwiftUI, Compose and Qt already validate the
digits** — the review that surfaced this claimed SwiftUI shared the flaw and it
does not. **html escapes** the value (`&quot;`) and **react** preserves the
backslash escaping, so both keep a hostile value inert inside its literal.
Flutter was the only backend affected.

A negative per-edge width is also skipped: `BorderSide` asserts `width >= 0` at
*runtime*, so it would type-check and then throw in the app.

Also hardened `parse_pixel_value`: `f64::parse` accepts `inf`, `NaN` and
overflowing literals like `1e400`, and `{f}` printed them as bare Dart
identifiers that do not compile. Not injection — no punctuation survives the
parse — but it broke the "generated source still type-checks" contract the `0`
fallback exists to keep.

### Added — a border has edges (UI79, #14835)

`border-{top,right,bottom,left}-{width,color}` now lowers. Flutter is the one
backend whose toolkit expresses this directly — `Border(top: BorderSide(..),
bottom: ..)` carries a colour *and* a width per side — so unlike Compose
(#15009) nothing has to be hand-drawn.

A part that authors no edge keeps the byte-identical `Border.all(..)` it emits
today; the per-side form appears only when an edge is authored, and unauthored
sides fall back to the `border-width`/`border-color` shorthand, which is the
CSS cascade answer (UI79 §3 rule 3).

#### Measured in the rendered widget tree, not in the Dart

Trestle's generated app, walked with `find.byType(Container)` and each
`BoxDecoration.border` inspected:

| | before | after |
| --- | --- | --- |
| containers with a uniform four-edge border | 5 | 5 |
| containers with a border on **fewer** than four edges | **0** | **2** |

The two are `right=1.0` and `top=3.0`, matching Trestle's authored
`border-right-width: 1` and `border-top-width: 3`. The uniform count is
identical either way, so existing borders are untouched. Falsified by making
the new path return `None` and confirming the count returns to zero.

#### The writer that runs is not the one that looks like it does

Worth recording, because it cost most of the work. The crate has two
`Border.all` writers. `emit_styled_box` is the one a reader finds first, and it
is gated behind `part_has_decoration`, which is only consulted for a `Box` node
lowering to a `Container`. The writer that actually runs for a styled container
is in `emit_container`. Patching the first one emitted the per-edge border
**nowhere**, and a grep for `Border.all` had hidden the second because its
`format!` sits on the previous line.

This was settled by planting a marker in the emitted string and finding it
absent from the output — not by reading the call graph, which had already been
wrong twice.

`border-<edge>-style` is not consulted: only `solid` is drawn. Flutter has no
style-drop reporting at all (#12022), so unlike Compose a non-solid style is
lost **silently** here. That is a gap in the report, not in this lowering, and
it is recorded rather than worked around.

### Added — `HostScroll` honours its axis (UI61, #14854)

Flutter has no two-axis scroll view, so `both` composes the idiomatic pair: a
vertical `SingleChildScrollView` wrapping a horizontal one. The test asserts
that as real **nesting** — two widgets, one `scrollDirection` — rather than as
a string, because an emitter that merely wrote `scrollDirection:
Axis.horizontal` for `both` would scroll one way only and still satisfy any
assertion that looked for the word.

`vertical` emits a bare `SingleChildScrollView`, Flutter's own default, so all
158 existing tests passed through the change untouched.

### Fixed — an unresolved component reference returned Ok and emitted a hole (#14892)

Flutter was the only backend of the eight that accepted a component reference it
could not resolve, emitting

```dart
/* TODO: component reference 'Cell' not yet resolved */ const SizedBox.shrink()
```

and returning `Ok`. Every gate downstream of `from_pipeline` then passed: the
build was green, `flutter analyze` was clean, and the component was simply
absent, occupying zero pixels.

That is how `mosaic-pkg-grid` appeared to work on Flutter while six backends
refused it (#14867). Flutter was not the one backend that supported the package;
it was the one that could not report the failure.

The placeholder was deliberate when written — its comment said package
resolution was a follow-up and a labelled placeholder let the file type-check
meanwhile. **That follow-up landed.** UI34's resolver inlines every `pkg::P::C`
node before emit and leaves backends a tree with no qualified tags, so the
reason for the placeholder is gone while the hole it can ship is not.

An unresolved reference now falls through to `UnknownPrimitive`, matching the
other seven backends, all of which were measured rejecting the same input
(#14886).

**Nothing shipping relied on it**, measured rather than assumed: Trestle, Engram
and VisiCalc were each generated on Flutter before the change and emitted
**zero** placeholders, and all three still generate afterwards. No checked-in
generated output contains the string either.

`clean_pascal_case_component_reference_emits_placeholder` was retargeted rather
than deleted — the rule it protects, that a non-kernel PascalCase tag is handled
deliberately rather than falling through by accident, is unchanged. A second
test asserts no successful emit can produce the placeholder by another route.

### Fixed — Material's default button minimum overrode authored padding (#14858)

`ButtonStyle` carried the authored padding but not `minimumSize`, so Flutter's
own `Size(64, 36)` floor decided the button box instead. Measured on Trestle's
generated Flutter app before changing anything:

- **every** button came back exactly `48.0` high
- the two narrowest were exactly `64.0` wide
- the task-row toggle, one glyph with `padding: 3`, was `64.0 x 48.0`

Neither number is in the `.msl`; both are Material defaults.

Adding `minimumSize: WidgetStatePropertyAll(Size.zero)` hands the size back to
the authored padding:

| toggle `○` | tap target | painted box |
| --- | --- | --- |
| before | 64.0 x 48.0 | 64.0 x 48.0 |
| after | **48.0 x 48.0** | **20.1 x 26.0** |

**This deliberately does not set `tapTargetSize: shrinkWrap`.** That would also
shrink the *touch* target below the 48dp accessibility minimum. The two are
separate properties precisely so the painted box can follow the design while the
hit area stays accessible, and the measurement above shows both holding: the
toggle paints at 20x26 and remains 48x48 tappable. A test asserts `shrinkWrap`
is absent, so a later "fix" cannot quietly trade the tap target away.

Sizes were measured by walking to each button's `Material` ink surface in a
widget test, not read off the emitted Dart — the padding was always *in* the
source, which is exactly why this survived.

### Fixed — a `max-width` cap did not bound its subtree through the Row-child reset (#14902)

A `ConstrainedBox(maxWidth: ..)` bounds its subtree at runtime, but the
emitter's boundedness analysis did not treat the cap as a bound. A `Row` inside
a capped subtree therefore judged itself unbounded and refused to hand its
children `Flexible`.

The cap is now a **source** of boundedness rather than an inherited flag:

```rust
let width_bounded = width.is_some()
    || part_max_width(node, part_styles).is_some()   // <- new
    || part_flex_grow(node, part_styles).is_some()
    || (!ctx.direct_row_child && ctx.width_bounded);
```

That last clause is why the obvious version of this fix does not work, and it
was tried first: threading `width_bounded: true` down from `part_max_width`
before emitting the subtree (dropped in #14901). A capped node is itself usually
a *direct Row child*, so the reset discarded the flag the moment it arrived. The
reset is correct for an inherited flag — a direct Row child genuinely is
unbounded in the general case, which
`nested_if_inside_row_branch_column_resets_direct_row_context` protects. An
explicit cap is different in kind: the `ConstrainedBox` is emitted a few lines
later, unconditionally, so the bound is a fact about the output rather than an
assumption about the context.

**No shipped product output changes.** Trestle's one cap and Engram's seven all
sit below their Rows rather than above them, and the generated Dart for both is
byte-identical with and without this change — verified rather than assumed,
because that is exactly the check the inert version of the fix also passed. The
difference is that this version changes the output for the shape it targets:
`a_max_width_cap_bounds_its_subtree_through_the_row_child_reset` fails without
it, and the fixture deliberately puts the capped node where boundedness would
otherwise be lost.

### Fixed — `max-width` reached nothing; Flutter was the last of eight (#14851)

With #14833 landing Compose, Flutter was the only backend of the eight still
discarding it. All seven others were measured on a minimal probe rather than
assumed: html, react and webcomponent emit `max-width`, SwiftUI
`.frame(maxWidth:)`, Qt `Layout.maximumWidth`, XAML `MaxWidth`, Compose
`.widthIn(max = ..)`.

Engram authors it on all seven of its screens and Trestle on its task list, so
a Flutter build of either rendered those as full-width sprawl.

```dart
ConstrainedBox(
  constraints: const BoxConstraints(maxWidth: 760),
  child: SizedBox(
    width: double.infinity,
    child: …,
  ),
)
```

Flutter has no max-width *argument* — `ConstrainedBox` is a widget that wraps —
so this reuses the `emit_widget_tree` / `emit_widget_tree_inner` split built
for `Opacity` in #14708, which applies a wrapping widget once on the way out
and so covers every return site by construction.

The `width: double.infinity` is not decoration: a ceiling is not a width, and
without a fill the child sizes to its content where CSS gives a block
`max-width: 760px` its parent's width up to 760. Compose needed the same
pairing. Unlike Compose the order cannot go wrong here, because the constraint
and the fill sit on different widgets rather than in one modifier chain.

`Opacity` wraps outside the cap, so a fade applies to the capped box rather
than inside it — pinned by a test.

Verified on generated projects with the real conformance widget test, not only
`flutter analyze`: Trestle and Engram both build and pass, 7 `maxWidth`
constraints emitted across Engram and 1 in Trestle, with the drop reporter
correctly no longer naming the property.

**A companion change was built, measured, and dropped.** A cap bounds its
subtree at runtime, so it looked as though `part_max_width` should mark that
subtree `width_bounded: true` before emitting it, letting a Row inside a cap
hand its children `Flexible`. Measured, it changed **not one byte** of Trestle's
or Engram's output, and a unit test placing a Row directly under a cap still
emitted zero `Flexible`. The reason is that the capped node is itself typically
a direct Row child, and

```rust
let width_bounded = width.is_some() || part_flex_grow(..).is_some()
    || (!ctx.direct_row_child && ctx.width_bounded);
```

discards an inherited flag for any direct Row child, so the mark was overwritten
immediately. Restoring boundedness from a cap is a real gap, filed separately;
it needs that formula to treat an explicit cap as a source of boundedness rather
than a threaded flag.

### Fixed — a conditional Row child holding a single `Text` never got its `Flexible` (#14857)

The wrap was gated on `if_node.children.len() > 1`, i.e. only a branch the
emitter renders as a `Column`. `If ( when: .. ) { Text .. }` is a single-child
branch, so it matched neither that rule nor the plain-child rule beside it,
which keys on `child.tag == "Text"` — the conditional's tag is `If`. The `Text`
measured at its natural width and the Row overflowed.

Replaced the child-count test with `branch_can_shrink`, which asks whether a
branch can give width back: a multi-widget branch (a `Column`, the original
case) or a single `Text`. Anything else — an `Icon`, a sized `Box` — is left
alone; a fixed-size widget should not be handed a loose constraint it never
asked for. An absent `else` renders as `const SizedBox.shrink()` and is
excluded from the decision, being already zero-width.

Measured on TaskApp's generated Flutter project, same clean state both runs:

| | `Flexible` wraps | conformance test |
| --- | --- | --- |
| before | 2 | `A RenderFlex overflowed by 316 pixels` — fails |
| after | 25 | passes |

The task row is five of these conditionals side by side, which is why it
overflowed by a figure that did not move across three attempts at the
surrounding layout: none of them had touched the children that were actually
overflowing. Reading the generated Dart showed the row contained zero
`Flexible` wraps and answered in one step what the layout reasoning could not.

**This does not change** how a plain (non-conditional) Row child is treated,
nor the nested-Row case, where a loose `Flexible` remains invalid because the
parent shrink-wraps its child: `multi_widget_if_inside_nested_row_does_not_emit_flexible`
still holds.

### Fixed — a Row-child `Text` could not shrink, and unboundedness was mis-detected (#14857)

A Flutter `Row` measures a non-flexible child with an **unbounded** max width,
so a `Text` never wraps — it extends the row until the row throws
`A RenderFlex overflowed by N pixels`. UI59 §11 records the same rule on
Compose, where it starves a sibling to zero instead; Flutter is the louder of
the two.

A `Text` that is a direct Row child now gets `Flexible` (default loose fit): it
may take up to its share and measures to its content when smaller, so a short
`Text` is unaffected and a long one wraps rather than overrunning. Only a
`Text`, because a Text is the child that can reflow.

**And a second, load-bearing bug found while doing it.** The existing
`direct_row_accepts_flex` asked only `!ctx.direct_row_child` — "my parent is
not a Row" — and so treated one level of indirection as bounded. Trestle's
`Row [subline]` sits at `Row [topbar] > Column [title-block] > Row [subline]`,
and was judged flex-accepting while measuring unbounded. Putting a flexible
child in it throws
`RenderFlex children have non-zero flex but incoming width constraints are unbounded`
— an assertion, not a layout quirk. Unboundedness now propagates through
intermediate containers as `width_bounded`.

The component root had to be marked bounded explicitly: it is built with
`..TableCtx::default()`, and `bool::default()` is `false`, which suppressed
flex everywhere.

Emitted wraps: TaskApp 18, Engram 11, VisiCalc 6. `flutter analyze` reports no
issues and TaskApp's real conformance test passes.

**This does not unblock #14851.** Reviving the closed `max-width` change on
top still throws at exactly 316px, because the task row's chain is judged
unbounded and its `Text` gets no `Flexible` — the `ConstrainedBox` the cap adds
re-bounds the width at runtime, and the static analysis does not know. The next
step is for `part_max_width` to mark its subtree bounded before emitting it.

### Fixed — a bare `Input` became an empty widget in a shipped product (#14861)

`Input` is the pre-UI29 spelling of `HostInput`, retained for capabilities the
first `HostInput` contract did not carry. **Every other emitter of the eight
accepts both.** Flutter accepted only `HostInput`, so a bare `Input` fell
through to the unresolved-component fallback:

```dart
/* TODO: component reference 'Input' not yet resolved */ const SizedBox.shrink(),
```

An empty widget where a text field belongs — and **no degradation reported**.

It reached a shipped product. `mosaic-pkg-notes` authors
`Input [ notes-body-input ]`, TaskApp embeds Notes, and TaskApp's Flutter
build had no notes input at all while its report said zero degradations and
its `native-complete` gate passed.

Found while investigating #14861 — that the grid package cannot be emitted on
seven of eight backends. Flutter was the one that "succeeded", and looking at
what it produced is what turned up this.

The fallback itself is unchanged and still deliberate for a genuine
unresolved component reference; `Input` simply is not one.

### Fixed — authored `opacity` reached nothing (#14708)

`opacity` was parsed and then dropped. The toolkit authors it on eight
controls — `state disabled { opacity : $opacity-disabled ; }` is what UI57's
disabled treatment is *made of* — so every disabled control on Flutter
rendered at full strength, looking enabled.

Flutter has no opacity argument: `Opacity` is a widget that **wraps** a
subtree. `emit_widget_tree` is therefore now a thin wrapper around
`emit_widget_tree_inner`, which applies the wrap once, on the way out. That
shape is deliberate — the real emitter has nine-plus `return` sites, and
wrapping each would have missed some silently. A test pins a child-only
opacity for exactly that reason.

- `StateLayer` gained an `opacity` field, populated where the other state
  properties are parsed, so state-layered opacity arrives the same way
  background and padding do.
- Each arm of the conditional renders through `dart_double_literal`
  (`... ? 0.4 : 1.0`). Dart accepts a bare int *literal* in a double context,
  so this is normalization rather than a fix — but a `num`-typed expression is
  not accepted, which is why the conversion is per-arm.
- Verified by `flutter analyze` on the generated toolkit project, not by
  inspection: 50 emitted Dart files, no issues.


### Added — `HostInput.disabled` (#14786)

`HostInput` had `read-only` but no way to say *unavailable*. Flutter spells
availability positively, so `disabled` lowers to a negated `enabled`
argument, distinct from `readOnly`.

Spec: `code/specs/UI58-hostinput-disabled.md` (#14786). Landed on all eight
backends in one change — a partly-landed prop would make a disabled input
*less* restricted on whichever backend lagged.

- Emit one Expanded wrapper for a direct Row input with explicit flex-grow.
  Preserve the declared flex weight without nesting the input's default wrapper,
  which caused Flutter ParentDataWidget assertions in Venture's live shell.

### Added - UI49 slot-owned style-state activation (#14348)

Flutter now activates mosstyle states owned by `one-of` slots from the
generated widget's corresponding property. Multiple enum axes follow `.mil`
slot declaration order, then existing `state-when-*` predicates take
precedence. Conditional background, foreground, border, and padding values
reach both generic styled containers and native `HostButton` `ButtonStyle`
output, including state-only styles with no base paint.

### Fixed — a `list<list<text>>` slot never read the host

A slot typed as a list of rows fell through the host-binding match to the
*sample* value — a constant — while every neighbouring prop read from the host
correctly. The generated shell therefore compiled, ran, and showed an **empty
table** no matter what the host sent.

Rows are how every table in Mosaic is modelled: Engram's deck list, TaskApp's
project nav. So this was not an exotic corner, it was the one slot shape whose
whole purpose is to carry data, silently bound to nothing.

Adds a nested-list reader (`mosaicStringListList` / `MosaicHostValue.stringListList`)
and the match arm that reaches it.

### Fixed - nested conditionals inherited an invalid Row flex context (#13725)

Multi-widget `If`/`Else` branches introduce an intermediate `Column`. Their
nested widgets now use that Column as their parent context instead of
incorrectly inheriting the outer Row context, preventing `Expanded(TextField)`
from being emitted under unbounded vertical constraints. Direct-Row
conditionals also wrap the generated Column in `Flexible` when no explicit
branch flex is authored and the containing Row has bounded width, giving the
Column a finite horizontal constraint without introducing flex into a nested,
shrink-wrapped Row. A generated-project widget test pumps both nested branches
to keep the runtime layout valid.

### Fixed - preserve dynamic HostButton accessible names (#13754)

Flutter buttons now wrap authored accessible names in native `Semantics`,
including repeated-row expressions, without changing their visible labels.

### Fixed - preserve HostInput accessible names (#13717)

Flutter text fields now retain `HostInput.a11y-label` through a native
`Semantics` wrapper with the editable-text role.

### Fixed - strict shells rejected nested Rust list props (#13545)

Native-complete Flutter shells now rebuild nested JSON lists at every declared
generic depth before passing them to generated widgets. Dart's JSON decoder
produces `List<dynamic>` values, so the previous exact
`value is List<List<String>>` check rejected valid Rust projections such as
TaskApp's `project-rows` during the first real render. The recursive decoder
preserves the declared `List<List<T>>` shape, keeps flat-list behavior intact,
and reports indexed paths when a nested value has the wrong shape.

Verified by regenerating the real TaskApp with its Rust runtime and driving the
Flutter controls through create, due-date scheduling, complete/reopen/delete,
and persisted restart widget tests.

### Fixed - `Row`/`Column` never applied `width`/`height`/`flex-grow` (#13429)

A real crash: `mosaic-compile pkg --backend flutter --emit-project` on the
real `task-app` package produced a `TaskApp.dart` that hard-crashed on
mount with `RenderFlex children have non-zero flex but incoming width
constraints are unbounded`. Root cause was a pre-existing architectural
gap this session's elevation-tokens work had already partially chipped
away at (see the `elevation`-only `Container`-wrap entry below, from the
same PR sequence): `Row`/`Column`/`Stack` lowered straight to their bare
Flutter widget with **no** style application at all beyond that one
`elevation` case — a part's `width:`, `height:`, and `flex-grow:` were all
silently dropped.

The app-shell's `rail` sidebar (`width: 236px`) is a `Column`, so it lost
its width entirely; a `Row`'s non-`Expanded` children get UNBOUNDED width
by Flutter's own design (so they can shrink-wrap to content), and an
unsized `Column` propagates that same unboundedness straight down through
its own descendants — so the first `Expanded` anywhere further down that
subtree (a very common real shape: `Row(children: [Expanded(...), ...])`)
crashes outright.

Fix, generalizing `emit_container`'s existing elevation-only wrap:

- A `Row`/`Column`/`Stack` part declaring `width`/`height` (and/or
  `elevation`, unchanged) now gets wrapped in a `SizedBox` carrying those
  properties — `Container` only when `elevation` is also present, since
  `SizedBox` has no `decoration:` slot for the shadow and `flutter
  analyze`'s `sized_box_for_whitespace` lint prefers `SizedBox` when
  there's nothing to decorate.
- A percentage `width`/`height` (e.g. TaskApp's real `title-block` part,
  `width: 100%`) is left unapplied rather than parsed as `0` — a real
  regression caught by re-verifying against the actual generated file: a
  `%` value has no fixed-pixel translation inside a `Row`/`Column`
  wrapper, and naively zeroing it collapsed `title-block` into a
  `SizedBox(width: 0, ...)` that then overflowed its own content.
- A part declaring `flex-grow` that is a **direct `Row` child** (`Expanded`
  is a compile error anywhere else) is wrapped in `Expanded(flex: N, ...)`
  — both for a plain child (`emit_paired_children`) and for the single
  child of an `If`/`Else` ternary (`emit_if_dart`/`render_branch`, added
  after Notes' real `notes-editor`/`notes-empty` toggle — "show the editor
  when a note is selected, else an empty-state message" — turned out to
  declare the identical `flex-grow: 1` on *both* branches, a shape the
  first pass's plain-child-only wrap didn't reach).

Verified against the real toolchain, not just unit tests: regenerated the
real `task-app`, `mosaic-pkg-notes`, and `mosaic-pkg-calendar` packages
via `mosaic-compile pkg --backend flutter --emit-project`, confirmed
`flutter analyze` clean (no issues, including no `sized_box_for_whitespace`
regressions) and `flutter test` mounting the real generated app shell with
zero exceptions on all three — TaskApp specifically re-checked at a
realistic desktop window size (`1600×1000`, not `flutter_test`'s narrow
default), since the original crash only reproduces once the widget tree is
laid out at a size resembling how a desktop task app is actually used.
That same real-toolchain pass surfaced a second, unrelated, previously
unreachable issue — TaskApp's topbar content is too wide to fit below
roughly 1670px once `main` has a real bounded width for the first time —
filed separately as
[#13465](https://github.com/adhithyan15/coding-adventures/issues/13465)
rather than folded into this fix, since it's a content/design question,
not an emitter defect.

### Added - drive a native shadow from `elevation` (#12028 item 1, UI41)

Fifth and final backend in the elevation-tokens cascade (mosstyle-compiler
contract → XAML → Compose → Qt → **Flutter**; SwiftUI stays out of scope,
tracked separately in #13206). A part declaring `elevation: raised;` or
`elevation: overlay;` now gets a real native shadow instead of the shadow
silently vanishing.

Unlike Compose (one shared `compose_box_style`) but similar to Qt (three
separate paint-line builders), this crate has **two** distinct styling
mechanisms with no shared function between them:

- `Box` → `Container`, via `emit_styled_box`'s `BoxDecoration` builder —
  elevation adds a `boxShadow: [BoxShadow(...)]` entry alongside the
  existing `color`/`border`. Also updated `part_has_decoration` to route a
  part with ONLY `elevation` set (no background/border) through the
  decoration path, since `boxShadow` can only live inside `BoxDecoration`,
  not the lighter `style_to_container_args` fast path.
- `Row`/`Column`/`Stack` have **no** decoration mechanism of their own —
  today they lower straight to their native Flutter widget with no
  `Container` wrapper at all, so a part with ONLY `elevation` (matching
  real `task-card`, a `Column`-tagged part) would have nothing to attach a
  shadow to. Elevation now force-wraps them in a `Container(decoration:
  BoxDecoration(boxShadow: [...]))` around the original widget — narrowly
  scoped to elevation, not a general fix for `Row`/`Column`'s missing
  background/border support (a separate, larger pre-existing gap, correctly
  out of scope here).
- `HostButton` uses Flutter's own first-class Material `ButtonStyle.
  elevation` (a real, depth-scaling shadow) instead of `BoxShadow` — a
  materially better fit than the `BoxShadow` hack every other primitive
  needs, verified via a real widget-test golden render showing the shadow
  visibly deepen at `elevation: 0.0 → default → 8.0 → 24.0` (the harder,
  more solid-edged look at higher values, compared to `BoxShadow`'s softer
  blur, is a known characteristic of `flutter test`'s software Skia
  rasterizer for `Material`/`PhysicalModel` shadows — real GPU-rendered
  apps render the same elevation value as the intended soft drop shadow).
- `HostDraggable` delegates its whole body straight to
  `emit_container(node, "Container", ...)`, reading `node.part_name`
  directly — elevation flows through for free with **no**
  `HostDraggable`-specific code, unlike XAML, which hit a real
  XamlCompiler bug on this exact primitive and needed a workaround.
- `HostSurface` was left out of scope: it has no `part_styles` parameter
  in its signature at all today (a separate, pre-existing "no styling
  support whatsoever" gap, not just missing `elevation`), and no real
  component declares `elevation` on a `HostSurface` part.

Shadow constants: `boxShadow` uses `color: 0x40000000`, `blurRadius: 8`
(raised) / `24` (overlay), `offset: Offset(0, 4)` / `Offset(0, 16)` — the
4/16 vertical-offset numbers match `mosaic-emit-xaml`'s
`ElevationTier::translation_z`, `mosaic-emit-compose`'s `dp()`, and
`mosaic-emit-qt`'s `shadowVerticalOffset`, so a part reads the same "how
far off the surface" intent on every backend; `blurRadius` was tuned
independently via a real `flutter test` golden render — an initial
`overlay` attempt at `blurRadius: 16` rendered as a flat, solid grey block
rather than a soft shadow (the blur radius was too small relative to the
16px offset for a wide/tall card to feather anywhere but the edges), fixed
by bumping to `blurRadius: 24`. `HostButton`'s native `ButtonStyle.
elevation` uses the plain Material dp values `4.0`/`16.0`.

Verified against the real toolchain: `flutter analyze` on a real scratch
Flutter project (Flutter 3.47.0 / Dart 3.13.0) confirmed
`BoxShadow`/`BoxDecoration`/`ButtonStyle.elevation`/`WidgetStatePropertyAll`
all compile cleanly; a real `flutter test --update-goldens` render (not
just static analysis) confirmed both `BoxShadow` and `ButtonStyle.
elevation` produce a real, visible shadow — this is also where the
`overlay`-tier `blurRadius` bug above was caught and fixed. All four real
components (`TaskApp`, `ProjectNav`, `Notes`, `Calendar`) build clean via
`mosaic-compile pkg --backend flutter --profile native-complete` — zero
degradations; the real generated `TaskApp.dart` produces a shadow
(`boxShadow` or `ButtonStyle.elevation`) for all 13 real
`elevation`-declaring parts (16 total instances — matching Compose's and
Qt's identical counts against the same source, a third independent
cross-backend consistency check). `flutter analyze` on the real generated
`TaskApp` `--emit-project` scaffold (with `flutter pub get` run for real)
is clean, zero issues.

A full **live widget mount** of the complete real `TaskApp` (via
`flutter test` against the real `--emit-project` output) was attempted but
hit a genuine, pre-existing runtime layout crash ("RenderFlex children
have non-zero flex but incoming width constraints are unbounded") deep in
`TaskApp`'s own `Row`/`Column` structure — confirmed via a `git stash`
comparison to be present on the pre-elevation baseline too, i.e.
unrelated to this change and out of scope for this PR. Filed as
[#13429](https://github.com/adhithyan15/coding-adventures/issues/13429)
rather than silently worked around. Verification for elevation
specifically relied instead on the isolated widget-test golden renders
above (which mount real `BoxShadow`/`ButtonStyle.elevation` widgets
directly, sidestepping the unrelated crash) plus `flutter analyze` and
the grep-based full-package coverage check.

### Added - `HostProgressRing` native lowering (#13176)

`HostProgressRing` now lowers to Flutter's native
`CircularProgressIndicator` in its determinate form (`value: fraction`)
instead of reporting `primitive.progress-ring-unimplemented`. The same
widget already renders indeterminate for `Icon(glyph: "spinner")`; the
determinate form is the identical widget with an explicit `value:`
argument.

`CircularProgressIndicator` has no size parameters of its own (unlike
WinUI's `ProgressRing`, which has real `Width`/`Height` DependencyProperties)
— it sizes to fill whatever constraints its parent gives it. `emit_host_progress_ring`
wraps it in `SizedBox(width:, height:, child: ...)`, reading the same
`width`/`height` style props every other primitive's part-style lookup
already resolves, and omitting them from the `SizedBox` args entirely
when unset (matching how every other Box-style sizing prop in this
file only emits what's actually styled).

`value` is required and supports the same `Number`/`SlotRef`/`Expr`
three-way binding every other numeric prop has — unlike `Path`'s
geometry props, this needed live binding from day one, since the whole
point is rendering a live `ring-percent-value`. Mosaic's `value` is a
0..100 percent; Flutter's is a 0.0..1.0 fraction, so the generated Dart
divides by 100.0. `a11y-label` wraps the ring in `Semantics(label:
...)`, matching `HostSlider`'s own accessibility pattern.

Narrows `mosaic-package-artifact-builder`'s `HostProgressRing`
degradation arm to also exclude `Backend::Flutter`.

Verified against the real toolchain: a `mosaic-compile pkg --backend
flutter --profile native-complete` build of a `HostProgressRing` bound
to a `ring-percent-value` slot produces `nativeComplete: true` with
zero degradations, and the generated `Ring` widget — both standalone
and mounted with real constructor args inside a `MaterialApp` — passes
`dart analyze` cleanly via the same minimal one-time `pubspec.yaml`
scaffold used for the `Path` Flutter PR.

### Added - `Path` drawing primitive lowering (#12028 item 3, UI39)

`Path [name] (kind: circle|line|curve, ...)` now lowers to real Flutter
vector geometry instead of reporting `primitive.path-unimplemented` on
every build. `circle` reuses `Container(decoration: BoxDecoration(shape:
BoxShape.circle, ...))` directly — `BoxDecoration`'s native `color`/
`border` properties already match `Path`'s `background`/`border-color`+
`border-width` 1:1, the same reuse Qt's `Rectangle` lowering made for
the same shape. `line`/`curve` have no equivalent native declarative
widget, so they lower to `CustomPaint` backed by one of two small
painter classes (`_MosaicLinePainter`, `_MosaicCurvePainter`) emitted
once per file when `Path` is present anywhere in the tree — mirrors the
existing `emit_drag_helpers`/`emit_dialog_helper` "shared top-level
helper, emitted once" pattern. `arc` is a stretch goal not implemented
in this PR; it hard-errors with a named "not yet supported" message
rather than silently rendering nothing, matching the XAML and Qt
lowerings' posture for the same gap.

Only `circle` needs explicit placement: a `Container` always draws from
its own top-left corner, so two overlapping circles (the crescent moon)
need `Positioned(left: cx - r, top: cy - r, ...)` to land at their
authored centers instead of collapsing onto Flutter `Stack`'s default
top-left-aligned, un-positioned-child placement. Unlike WinUI's
`Margin` (valid in any panel) or QML's implicit-`Item` positioning
(valid in any non-Layout parent), `Positioned` only type-checks as a
*direct* `Stack` child in Flutter, so `emit_path` only wraps in
`Positioned` when a new `TableCtx.direct_stack_child` field (threaded
the same way `direct_row_child` already is, set in `emit_container`
when `widget == "Stack"`) confirms the immediate parent actually is
one — a standalone circle with no `Stack` parent renders unwrapped,
sized to its own `2r × 2r` box, since `Positioned` would otherwise
panic at runtime outside a `Stack`. `line`/`curve` never need this:
their own drawn geometry already encodes the correct offset from
`(0, 0)` (absolute, un-offset canvas coordinates, mirroring Qt's
`ShapePath`), and an un-positioned `Stack` child already renders at
`(0, 0)` by default (`Stack`'s own `alignment` default is top-left).

Coordinate props (`cx`/`cy`/`r`/`x1`/`y1`/`x2`/`y2`) accept a literal
`Number` only; a `SlotRef`/`Expr`-bound coordinate is a clear compile
error, not a silent 0 — full data-driven binding is future work, the
same not-yet-landed gap the XAML and Qt lowerings both note.

Verified against the real toolchain: a `mosaic-compile pkg --backend
flutter --profile native-complete` build of the crescent-moon shape
(two overlapping circles, a line, and a curve) produces
`nativeComplete: true` with zero degradations, and the generated
`Moon.dart` passes `dart analyze` (via a minimal one-time `pubspec.yaml`
scaffold pulling in the `flutter` SDK dependency) with "No issues
found!".

### Added - native radio-group mutual exclusion (#13007)

`emit_host_radio`'s `group:` prop only fed a per-radio-independent
`groupValue` that defaulted to `null` for a literal string group value
(discarding it entirely) — real usage's radios never actually rendered
as selected, let alone exclusively. A literal `group: "..."` value
shared by 2+ `HostRadio`s with a resolvable `checked:` prop anywhere in
the component (see `collect_radio_group_members`) now gets a
synthesized shared `groupValue`: the SAME nested-ternary Dart
expression (`cond1 ? val1 : (cond2 ? val2 : (... : null))`) emitted at
every member's own call site. Flutter's `Radio<T>` computes its checked
state as `value == groupValue`, so sharing one reactive expression
across siblings gives real native mutual exclusion with zero
restructuring of the widget tree — no ancestor wrapper needed, and
`onChanged` is untouched (each radio still dispatches its own
`onSelect` independently).

Deliberately uses the classic `groupValue`/`onChanged` API rather than
Flutter's newer `RadioGroup<T>` ancestor widget, which requires Flutter
3.35+ (this repo's pinned floor is 3.32) — the classic API remains
fully supported and achieves the identical exclusivity guarantee with
no SDK bump.

Threaded via a new `TableCtx.radio_group_members` field, computed once
in `emit_widget_class` and inherited unchanged through every recursive
call. A `slot:`-bound group, or a literal value with no qualifying
peer, keeps the pre-#13007 behavior exactly (including its existing
`groupValue: null`-always-unselected gap for those cases).

New `pub fn radio_groups_with_native_semantics` lets
`mosaic-package-artifact-builder`'s degradation analyzer stop reporting
`property.radio-group-ignored` wherever this lowering actually applies.

Verified against a real regenerated `mosaic-pkg-deck-options` project
(the real multi-radio usage this targets): `flutter pub get` +
`flutter analyze` — "No issues found!".

### Added - native indeterminate checkbox state (#13006)

`emit_host_checkbox`'s `indeterminate:` prop was accepted but ignored —
the doc comment even named Flutter's own `Checkbox(tristate: true)`
mode as the mechanism, but nothing in the function ever read the prop.
When `indeterminate:` is authored as anything other than a literal
`Keyword("false")`, the emitter now adds `tristate: true` and collapses
`value:` to `{indeterminate} ? null : {checked}` (Flutter's own
"`null` renders the dash" convention). No changes were needed to the
`onChanged` wiring: `Checkbox.onChanged` is always `void
Function(bool?)` even in two-state mode, and the existing
`host_checkbox_event_args` already treats its `v` parameter as
nullable (`v ?? false`).

A `slot:`/expression-valued `indeterminate` can't be evaluated at
compile time, so — mirroring XAML's `IsThreeState` treatment — its mere
*presence* unconditionally enables `tristate: true`; the runtime value
decides whether `null` (the mixed dash) actually renders.

New `pub fn host_checkbox_has_native_semantics` lets
`mosaic-package-artifact-builder`'s degradation analyzer stop reporting
`property.checkbox-indeterminate-ignored` for Flutter wherever this
lowering actually applies.

Verified against a real regenerated `mosaic-pkg-toolkit` project:
`flutter pub get` + `flutter analyze` — "No issues found!" — on the
whole package, including the real `Checkbox.dart` this change touches.

### Fixed - HostDialog lowers to a real native dialog, not a placeholder (#13010)

`emit_host_dialog` unconditionally emitted `const SizedBox.shrink() /*
TODO: HostDialog showDialog wiring */` — every Mosaic `Modal`/`HostDialog`
on Flutter rendered nothing at all. Genuinely incomplete, not a
documented platform limitation the way XAML's Modal gap is (#13008).

Implemented the `modal: true` (default, and the only value the
toolkit's own `Modal` component ever authors) case for real: a new
shared `_MosaicDialogHost` `StatefulWidget` (vanilla Flutter, no new
package dependency) bridges the declarative `open: bool` contract
every backend shares to Flutter's imperative `showDialog`/`Navigator`
API — `open` flipping false→true schedules `showDialog` on the next
frame; flipping true→false while still showing (the host closed it via
its own slot, not backdrop-tap) pops the route programmatically. Either
dismissal path resolves `showDialog`'s `Future`, which is where
`onClose` dispatches exactly once regardless of which side closed it.
`title`, `dismiss-on-backdrop` (→ `barrierDismissible`), and children
(→ `AlertDialog.content`, wrapped in a `Column` for multiple children)
are all wired following the exact property table every other backend
already implements.

`modal: false` is NOT implemented and deliberately out of scope here:
Flutter's `showDialog` is inherently modal (a full-screen route +
barrier), with no vanilla-Flutter equivalent to SwiftUI's `.popover`/
Qt's non-modal `Popup` short of a custom `Overlay`. That shape keeps
the old placeholder and keeps reporting `interaction.dialog-placeholder`
(now gated on a new `host_dialog_has_native_semantics` predicate,
mirroring the existing `host_table_has_native_semantics` pattern, so
`mosaic-package-artifact-builder` only reports the degradation for the
still-unimplemented shape). Removed the now-stale
`(Backend::Flutter, "Modal", "interaction.dialog-placeholder")`
allowlist entry from `mosaic-pkg-toolkit`'s native-complete gate — the
toolkit's `Modal` always authors `modal: true`, so it's genuinely clean
on Flutter now.

Verified with 7 new tests plus a real `flutter analyze` (0 issues) and
`flutter pub get` against the actual generated project shell for all 23
`mosaic-pkg-toolkit` components — not just Rust-level string assertions.

### Security - validate literal HostLink.href's URI scheme (#13052)

Follow-up to #12038 (the identical XAML gap). Added `has_disallowed_uri_scheme`
(new `PipelineEmitError::UnsafeUriScheme` variant), checked when `external`
is not `false` — the path a real `launchUrl` call will eventually use.
Today that path is still a `/* TODO: launchUrl(Uri.parse(...)) */` comment,
never real code, so there's no live navigation vulnerability yet; this is a
preventative fix ahead of that landing rather than a response to a
currently-reachable gap. A relative reference with no scheme at all (`"#"`,
`"/about"`) is unaffected, and `external: false` (dispatch-only, never
reaching `launchUrl`) is unaffected either way — matching every other
backend's #13052 scoping. No runtime (slot-bound) guard was added, for the
same reason: there is no real navigation call yet to guard.

### Fixed - MIL slot defaults

Slots with authored MIL defaults now emit non-null Dart fields and matching
optional constructor defaults. Reusable components can consume defaulted text,
number, and boolean values without analyzer errors.

### Added - accessible HostSlider names

Literal and slot-backed `HostSlider.a11y-label` values now annotate the native
Material slider while preserving its adjustable range semantics.

Slot-bound or expression-backed `step` values now derive Material Slider
divisions at runtime and remain continuous when non-positive.

### Added - native HostSlider

`HostSlider` now lowers to Flutter Material's native adjustable `Slider`, with
controlled value/range, discrete or continuous steps, disabled state,
per-tick `onChange`, and release-value `onCommit`. A strict fixture must remain
native-complete, analyzer-clean, and widget-tested in CI; its test also proves
disabled sliders expose no interactive callbacks.

### Added - portable Text accessibility

`Text` now emits Flutter `Semantics`/`ExcludeSemantics` wrappers for literal or
slot-backed accessible names, headings, and intentionally hidden text. Custom
labels replace the widget's built-in text semantics to avoid duplicate speech.

### Fixed - analyzer-clean Flutter bootstrap

Project emission now owns `analysis_options.yaml`, the matching
`flutter_lints` dependency, and a package-name-correct Mosaic widget smoke test,
so the documented `flutter create` bootstrap no longer installs a broken
counter-app test or an unresolved lint include. Indexed and plain `For`
lowering also omits authored item bindings when the generated body never reads
them. CI analyzes the whole generated project and runs its permissive widget
test instead of hiding bootstrap failures behind `dart analyze lib`.

### Added - native accessible dynamic tables

Canonical UI31 tables now lower dynamic header, row, and cell `For` loops to
Flutter's native `DataTable`, `DataColumn`, `DataRow`, and `DataCell` widgets.
This restores the platform table semantics that the former nested
`Column`/`Row` visual fallback could not expose while preserving editable cell
content, event dispatch, stable row keys, authored cell styling, and RTL.
Unsupported table shapes deliberately retain the visual fallback so strict
capability analysis continues to report them rather than making a false
accessibility claim. The generated Flutter floor is now 3.32, the first stable
release with explicit table/row/cell semantic roles; generated runtime helpers
are also analyzer-clean on Dart 3.12.

### Added - native accessible drag and drop

`HostDraggable` and `HostDropTarget` now lower to Flutter's native
`Draggable`/`DragTarget` pair. A component-instance scope registers eligible
targets for keyboard movement, pointer and keyboard releases share one accepted
drop path, disabled and accepted-kind rules are enforced before target
selection, and the generated runtime emits drag lifecycle payloads and
screen-reader announcements. Space/Enter grabs and drops, arrow keys move among
valid targets, Escape cancels, and semantic activation exposes the same flow to
assistive technologies. The native wrappers retain their authored part
decoration and spacing instead of discarding the card and target styling.

### Fixed - complete toolkit Dart compilation

Flutter control lowerings now avoid generated component-name collisions with
Material's `Checkbox`, `Radio`, and `Tooltip`, derive callback fields from the
declared event payload, carry indexed `HostLink` events through their loop
shadow, preserve decimal numeric payloads as `num`, and omit unused truthiness
and loop-index bindings. The complete 23-component toolkit is analyzer-clean
and builds as a native Flutter desktop application.

### Added - native-complete runtime-required shell

`EmitOptions::require_runtime` now selects a fail-loud Flutter application
shell that requires Mosaic's standard Rust host, waits for its first props
envelope, and omits nullable-host, event-print, and generated sample-value
fallbacks. The default remains byte-compatible permissive project emission.

### Fixed - complete TaskApp Dart compilation

Generated widgets now normalize Mosaic value truthiness before values enter
Dart boolean positions, including `If`/`Else`, conditional styles,
`HostButton.disabled`, and `HostInput.read-only`. Native text-input callbacks
also synthesize the declared zero- or one-field event payload for both change
and commit handlers. Together these fixes remove the non-boolean conditions
and missing required constructor argument that prevented the package-expanded
TaskApp from passing the Dart analyzer and building as a native Flutter app.

### Added - host-driven prop refresh callback

Generated Flutter shells register a prop-change handler on `MosaicHost` and
reapply the host's current props when native content-surface input changes
browser state. The default generated host keeps a no-op implementation, so
existing injected hosts remain source-compatible.

### Fixed - native form state and generated-shell interaction acceptance

Slot-backed `HostButton.disabled` and `HostInput.read-only` values now reach
Flutter's native `onPressed` and `readOnly` contracts, while `HostInput`
`onCommit` dispatches from `TextField.onSubmitted`. Direct `Row` inputs are
wrapped in `Expanded` so generated toolbars have a finite width and can render.
Generated `MosaicApp` shells also accept an injected `MosaicHost`, allowing
widget tests to exercise initial props, native input, event envelopes, and
host-driven prop refresh without replacing the generated component.

### Fixed - project-shell widget-slot hydration

Generated Flutter shells now accept a host-provided `Widget` in the props map
for `node` slots and pass it to the component through `mosaicWidget`. Missing or
mistyped values retain the deterministic `SizedBox.shrink` fallback.

### Fixed - host-owned surface composition

`HostSurface ( content: slot: ... )` now mounts the supplied `Widget` instead
of silently emitting the unresolved-component `SizedBox` placeholder.
The package builder mirrors the component into `lib/` so Dart imports stay
inside the package boundary, and the generated README documents the one-time
`flutter create --platforms=... .` runner bootstrap. That exact flow now
produces a native macOS app from Venture's generated browser chrome.

### Added - Flutter Mosaic event envelopes

Generated Dart event classes now expose `mosaicName`, `mosaicPayload`, and
`mosaicEnvelope`, and the generated Flutter app shell logs the envelope. This
lets Flutter hosts forward the same event map used by HTML, Electron, SwiftUI,
XAML, and Qt shells.

### Fixed - `--emit-project` Flutter shell supplies constructor inputs

`lib/main.dart` now mounts the generated widget with deterministic sample
values for every declared slot plus a dispatch callback. Previously the
Flutter shell emitted `{Component}()` even though generated widgets always
require `dispatch` and may require slot constructor arguments.

### Added — UI29-FU-flutter / UI28-1 §6.2 — Flutter `For` / `If` / `Else` lowering

Replaces the v0.1 placeholder (`/* TODO: For not yet wired in the Flutter
emitter */ const SizedBox.shrink()`) with real Dart lowering for all
three control-flow primitives. Required by `mosaic-pkg-grid` v0.2.0 per
[UI28-1 §6.2](../../../specs/UI28-1-grid-v3-userland-revised.md).

- **`For`** lowers to a `Column(children: ...)` whose list is built
  by `.map(...).toList()` over the iterated collection:
  - `For ( each: slot: X , as: y )` → `Column(children: x.map((y) => <body>).toList())`
  - `For ( each: slot: X , as: y , index: i )` → `Column(children:
    x.asMap().entries.map((entry) { final i = entry.key; final y =
    entry.value; return KeyedSubtree(key: ValueKey(i), child:
    <body>); }).toList())` — the `KeyedSubtree(ValueKey(i))` wrapper
    gives Flutter's element tree the stable identity Flutter's diff
    needs when rows reorder (matches React `key={i}` and SwiftUI
    `id: \.offset`; UI28-1 §5 performance property).
  - `each:` accepting `SlotRef` or `Expr` — Expr text passes through
    verbatim (author-controlled, matches React/SwiftUI/Qt behaviour).
  - `as:` / `index:` kebab-case names lower via `to_camel_case_first_lower`
    so the emitted Dart bindings are valid identifiers.
- **`If`** lowers to a Dart ternary returning a Widget:
  - `If { then }` → `((cond) ? <then> : const SizedBox.shrink())`
  - `If { then } Else { e }` → `((cond) ? <then> : <else>)`
  - `when:` accepting `SlotRef` (lowered via camelCase) or `Expr`
    (verbatim). Empty branches collapse to `const SizedBox.shrink()`
    so the ternary always returns a concrete Widget.
- **Sibling pairing** — `emit_paired_children` walks each container's
  children with peek-ahead so an `If` followed by `Else` fuses into a
  single ternary. Orphan `Else` (analyzer should reject) renders a
  documenting comment instead of crashing. Pattern mirrors the SwiftUI
  backend's `emit_children`. `HostScroll` and `HostTooltip`'s
  multi-child paths also route through the paired walker.

8 new tests cover the lowering shapes: unindexed `For`, indexed `For`
with `KeyedSubtree`, `Expr`-as-each pass-through, kebab-case binding
camel-casing, standalone `If` with empty-else SizedBox fallback,
paired `If`/`Else` inside a Box (Cell.mll shape), `Expr` `when:`
(verifies the Cell.mll predicate `cellRow == editRow && cellCol ==
editCol` lowers without UI29 §3.4 since slots are in scope), empty
`For` body, and nested `For` with `Expr` inner each (the v0.2.0 Grid
composition shape). Total tests: 57 (was 49, +8).

### Added — UI32-K-flutter — `--emit-project` Flutter app shell

L5 of UI32 ([spec PR #4286](https://github.com/adhithyan15/coding-adventures/pull/4286); L2 React #4297, L3 HTML #4309, L4 WebComponent #4315). `mosaic-compile --backend flutter --emit-project` now produces a flutter-create-shaped scaffold alongside the component `.dart`:

- `pubspec.yaml` — pinned Flutter SDK `>=3.32.0 <4.0.0` + Dart `>=3.5.0 <4.0.0` per UI32 §3.6.3. Dart pub package name follows snake_case rules (§3.6.2 Flutter row) — auto-derived as `mosaic_{snake(name)}` (e.g., `ProfileCard` → `mosaic_profile_card`).
- `lib/main.dart` — `MaterialApp` shell that mounts the component as `Scaffold.body`'s `Center(child: <Component>())`. Imports the component package-locally from `lib/{Component}.dart`, which keeps the generated shell valid under Dart's package-boundary rules.
- `README.md` — `flutter pub get && flutter run` recipe + file map.

New public API (matches L2/L3/L4 pattern):

- `pub struct EmitOptions` — `emit_project`, `pinned_flutter_sdk`, `pinned_dart_sdk`, `package_name` override.
- `pub struct ProjectFiles` — `pubspec_yaml`, `main_dart`, `readme`.
- `pub enum ProjectShellError` — `InvalidDartPubName(String)` surfaced through `PipelineEmitError::UnsafeSlotName`.
- `pub struct PipelineEmitResultWithProject` — `output`, `component_name`, `project: Option<ProjectFiles>`.
- `pub fn from_pipeline_with_options(...)` — new entry. Existing `from_pipeline(...)` unchanged.

UI32 §3.6.2 Flutter row: Dart pub names MUST match `[a-z][a-z0-9_]*` (lowercase, digits, underscores; must start with letter; no leading underscore; no hyphens; no uppercase). `is_valid_dart_pub_name` enforces this; an explicit invalid `package_name` fails-loud via `ProjectShellError::InvalidDartPubName`.

11 new tests cover the spec §3 gates plus a Dart-pub-name truth table (8 accept/reject vectors) and a main.dart structural test (MaterialApp + Scaffold + Component() mount). Total tests: 48 (was 37, +11).

### Added

- **UI31-K-flutter** — RTL contract for `HostTable`. The lowering
  now wraps the emitted `DataTable` in `Directionality(textDirection:
  ..., child: ...)` when the layout author writes `dir:`:
  - `dir: rtl` → `Directionality(textDirection: TextDirection.rtl, child: DataTable(...))`
  - `dir: ltr` → `Directionality(textDirection: TextDirection.ltr, child: DataTable(...))`
  - `dir: auto` → no wrapper (Flutter has no `TextDirection.auto`
    enum; the ambient `Directionality` from `MaterialApp` flows
    through, which is the correct semantic for "let the host decide")
  - `dir: slot: layout-direction` → `Directionality(textDirection:
    layoutDirection, child: DataTable(...))`. The slot is expected to
    evaluate to a `TextDirection` Dart value; the slot name passes
    through `is_safe_dart_identifier` so it can't smuggle bad source
    into the format string.
  - Unknown keywords drop silently — the allow-list is the security
    gate against attacker-controlled keywords sneaking `, child:
    pwn(),` style payloads into the generated source. The bare
    `DataTable` still renders so the rest of the layout is intact.
  - 7 new tests cover the a11y gate (must lower to native
    `DataTable`, not a `Container`/`Row` substitute), the three
    allow-listed keywords (including the `auto` no-wrap case which
    is uniquely Flutter), the slot-ref interpolation, the silent-
    drop on an injection-style unknown keyword, and a bare-table
    regression guard. Total tests: 37 (was 30).
  - Full sub-tag (`HostTableHead` / `HostTableBody` / `HostTableFoot`
    / `HostTableColGroup`) walk is still a follow-up — the
    `DataTable` body remains a `columns: const [], rows: const []`
    placeholder, matching the existing UI29 §2.1 stub. The RTL
    contract is independent of the sub-tag walk and can ship now.

## [0.2.0] - 2026-05-23 — UI29-4 host primitives

### Added

- **`HostLink` (kernel primitive #19)** → `InkWell(onTap:, child:
  Text(...))`. Flutter has no built-in URL launcher, so the
  `onTap` body carries a `/* TODO: launchUrl(Uri.parse(href)) */`
  comment that hosts wire to the `url_launcher` package. The
  `external: false` keyword suppresses the `launchUrl` comment
  for in-app routing (host handles via the `onActivate`
  dispatch). The `target` keyword (`same`/`new-tab`/`parent`/
  `top`) is preserved in the comment as a hint to the host.
- **`HostTooltip` (kernel primitive #20)** → `Tooltip(message:,
  child:)`. Flutter's built-in tooltip handles hover (web /
  desktop) and long-press (mobile) automatically; the overlay
  layer escapes parent clipping by default. Single-child shape
  matches the spec exactly.
- **`HostNumberInput` (kernel primitive #21)** → `TextField`
  configured with `keyboardType: TextInputType.number`, which
  surfaces the numeric keypad on iOS/Android. `min`/`max`/`step`
  numeric literals are emitted as a `/* min: N, max: N, step: N
  */` range hint; full `inputFormatters` clamping is a follow-up.
  `onChange` wires `onSubmitted` (commit-on-Enter), matching the
  UI29-4 spec's rejection of per-keystroke dispatch for numeric
  fields.
- New `find_number_prop` helper for fishing `LayoutPropValue::
  Number(f64)` values out of a node's prop list. Used by
  HostNumberInput for `min`/`max`/`step`.
- 10 new tests covering all three primitives plus a security
  regression test for Dart-string injection through HostLink's
  `href` slot (`$`-interpolation and `"`-quote escaping).

### Security notes

- `href`, `label`, `text`, and `placeholder` slot/string values
  all flow through `escape_dart_string`, which handles `\`, `"`,
  `$` (critical: Dart interpolates `$ident` inside double-quoted
  strings), `\n`, and `\r`.
- `external`/`target` keywords are validated against allow-lists
  before splicing into block comments — a malicious keyword like
  `false*/dispatch(evil())/*` is impossible because the grammar
  layer guarantees keywords are bare identifiers and we further
  match against `same|new-tab|parent|top` / `false|true`.
- `min`/`max`/`step` are `LayoutPropValue::Number(f64)` from the
  IR, never strings — no injection possible by construction.

## [0.1.0] - 2026-05-23 — initial release

Brand-new backend bringing Flutter (Dart) as the seventh supported
target alongside React, SwiftUI, Qt, HTML, WebComponent, and XAML.

### Added

- `pipeline::from_pipeline(interface, layout, style)` — the standard
  three-IR entry point matching the other six backends' signatures
  so `mosaic-compile` and `mosaic-package-artifact-builder` can
  dispatch uniformly.
- `PipelineEmitResult` / `PipelineEmitError` mirror the cross-backend
  shape (output string + component name; same error variants:
  `ComponentNameMismatch`, `UnsafeSlotName`, `UnsafeEmitName`,
  `UnknownPrimitive`).
- Dart `StatelessWidget` output: one class per Mosaic component, plus
  a sealed `<Component>Event` base class and one subclass per
  declared emit. Constructor uses Dart's named-required parameter
  syntax so component-by-component prop changes don't break
  call-site order.
- **Slot → Dart field type map:**
  - `text` / `image` / `color` → `String`
  - `number` → `double`
  - `bool` → `bool`
  - `node` → `Widget`
  - `list<T>` → `List<dart-type-of-T>`, including the nested
    `list<list<text>>` case → `List<List<String>>` (motivated by
    VisiCalc's viewport-rows slot)
  - `Component(Name)` → `Name` (assumes the host imports that
    Dart class)
- **Kernel primitive lowerings (v0.1 coverage):**
  - `Box` → `Container` (with single-child / multi-child / styled
    variants)
  - `Row` / `Column` / `Stack` → Flutter's same-named widgets with
    a `children: [...]` list
  - `Text` → `Text("...")` or `Text(<slot>)`
  - `Image` → `Image.network(...)` for `http(s)://` sources,
    `Image.asset(...)` otherwise
  - `Spacer` → `SizedBox(width: 8, height: 8)`
  - `Divider` → `Divider()`
  - `Icon` → `Icon(Icons.<name>)` (`source:` keyword feeds the
    `Icons` constant lookup)
  - `HostInput` → `TextField` with `TextEditingController(text:
    <slot>)` and an `InputDecoration(hintText: ...)` when
    `placeholder` is bound
  - `HostButton` → `ElevatedButton(onPressed:, child: Text(...))`;
    `disabled: true` → `onPressed: null`
  - `HostCheckbox` → `Checkbox(value:, onChanged:)` with optional
    sibling `Text` label inside a `Row`
  - `HostRadio` → `Radio<String>(value:, groupValue:, onChanged:)`
    with optional sibling `Text` label
  - `HostScroll` → `SingleChildScrollView(child:)` (wraps multi-
    child case in a `Column`)
  - `HostDialog` → placeholder `SizedBox.shrink()` (full
    `showDialog` plumbing deferred to a follow-up — see
    "Deferred" below)
  - `HostTable` → placeholder `DataTable(columns: const [], rows:
    const [])` (full sub-tag walk deferred — see "Deferred")
- **Style → widget property pass:**
  - `padding: N` / `padding: Npx` → `padding: const EdgeInsets.all(N)`
  - `width: N` / `height: N` → matching Container properties
  - `background-color: #RRGGBB` → `color: const Color(0xFFRRGGBB)`
  - Unknown / unsupported style props are silently dropped (TODO:
    surface as Dart comments).
- **Dart-safety helpers:**
  - `escape_dart_string` handles `\\`, `\"`, `\$` (Dart double-
    quoted strings interpolate `$ident`), and `\n` / `\r`.
  - `is_safe_dart_identifier` checks the ascii identifier shape AND
    rejects Dart reserved keywords (`class`, `if`, `return`, etc.)
    so a kebab-case slot like `class` doesn't compile to broken
    Dart.
- **Test coverage:** 17 unit tests cover the smoke path, slot-type
  matrix (required vs optional, scalar vs list), event-union
  shape, container nesting, every wired host primitive, the
  component-name-mismatch error path, the dart-string escape
  regressions, the reserved-keyword rejection, and the style →
  Container args mapping.

### Deferred to follow-up PRs

- **HostDialog** full `showDialog` plumbing. Flutter's `showDialog`
  is imperative — it requires a `BuildContext` and is called from
  a callback, not declared in the widget tree. The cleanest shape
  uses either `flutter_hooks`' `useEffect` (third-party package)
  or a `StatefulWidget` wrapper with `WidgetsBinding.instance
  .addPostFrameCallback`. v1 emits a placeholder; the follow-up
  picks one of those two approaches.
- **HostTable** sub-tag walk (`HostTableHead` / `HostTableBody` /
  `HostTableFoot` / `HostTableColGroup`). Flutter's `DataTable` has
  a richer API than the other backends' table widgets (it carries
  per-column sort logic, per-row selection, etc.); a separate PR
  will design the lowering surface.
- **`For` / `If` / `Else`** meta-primitives. Flutter widget trees
  are expressions, not statements, so Dart's `if`/`for` need to be
  used in *collection-literal* contexts (`children: [for (var x
  in xs) Widget(x)]`). Wiring this through the recursive walker
  needs the same sibling-pair lookahead the other backends use;
  deferred so the v1 PR stays reviewable.
- **`onTap` / `onChange` / `onToggle` / `onSelect` dispatch wiring**
  currently writes `/* TODO: dispatch <name> */` comments inline.
  The full dispatch payload synthesis (mapping `event.target.value`
  / Checkbox's `bool?` callback into the right `<Component>Event<Case>`
  constructor invocation) needs the component-name to be threaded
  down to the per-primitive emitters. The plumbing is a small but
  cross-cutting refactor; deferred to the follow-up.
- **Theme integration.** Generated widgets ignore
  `Theme.of(context)`. Hosts that want themed colours should wrap
  the generated widget in a `Theme(...)` override for v1.
- **Component reference resolution.** PascalCase tags that aren't
  kernel primitives emit a labelled `SizedBox.shrink()` placeholder
  instead of importing + instantiating the referenced component.
  Package-resolver wiring follows the same pattern the other
  backends use; it'll land in the next iteration.
