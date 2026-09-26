- for label and `Text` content, the old output is kept.

`expr_to_mustache_path` (drag keys, path coordinates) now tries it first.

Verified in a browser on SegmentedControl's generated project. Labels
render, the selected option is named "Board, selected", and a label of
`Say "hi" <b>` stays text in both the body and the attribute.


### Fixed -- a `;` inside a CSS value was read as a declaration separator (#15221)

`emit_path_html` recovers `background`, `border-color` and `border-width`
from a serialized declaration body. `css_value` split that body on every
`;`, so a value containing one -- inside a quoted string, or a
`url(data:image/svg+xml;base64,...)` -- started a new "declaration" and a
fragment of it was recovered as the property being looked up.

This is the twin of the React emitter's defect, found in the same review.
It is milder here: what is recovered lands in QUOTED attributes through
`escape_html_attr`, so the failure is a wrong or garbled paint attribute
rather than injection. Fixed for parity all the same -- the parse was simply
wrong, and a reader comparing the two emitters should not find one of them
still doing it.

The scan now honours quoted strings (both kinds), backslash escapes, and
parenthesised functions. No product output changes.

### Changed — `HostScroll` honours its axis, and no longer scrolls both ways (UI61, #14854)

`HostScroll` lowered to a bare `overflow: auto`, which scrolls **both** axes.
Against UI61's default of vertical that is one scrollbar more than anybody
asked for, so this is a deliberate behaviour change, not a refactor: vertical
emits `overflow-y: auto; overflow-x: hidden`, horizontal the mirror, and only
`axis: both` keeps the old shorthand.

### Fixed — an authored `Col (width:)` reached nothing (#14846)

html was the **only web backend** dropping it. For the same declaration react
emits `style={{ width: "48px" }}` and webcomponent `style="width: 48px"`;
this wrote a bare `<col>` — on the very backend where `<col>` is the native
concept.

```html
<colgroup>
  <col style="width: 48px">
```

Literals only. A `For`-bound `Col (width: (w))` carries a runtime expression,
and this backend emits a static template — the loop becomes an
`<!-- mosaic-for -->` comment rather than iterating — so there is no value to
write. react and webcomponent interpolate because their output is code;
html's is not. That is a difference in what the format can express, not a gap.

A part style still wins where both are present, since `build_style_attr`
appends it after the builtin.

### Added — `HostInput.disabled` (#14786)

`HostInput` accepted `read-only` but had no way to say *unavailable*. The
toolkit's Input/Field/InputGroup therefore styled a disabled control — the
`opacity` state landed in #14772 — while the emitted element stayed fully
interactive. It looked disabled and behaved normally.

`disabled` now lowers to the boolean HTML attribute, with `data-disabled` for
the slot-bound form so authored `state disabled` styling and the real attribute
come from the same slot.

Spec: `code/specs/UI58-hostinput-disabled.md` (#14786). Landed on all eight
backends in one change — a partly-landed prop would make a disabled input
*less* restricted on whichever backend lagged.

### Fixed — `HostLink` dropped its children (#14717)

A `HostLink` with no `label:` and a nested subtree emitted `<a ...></a>` and
threw the subtree away. `HostButton` already rendered children in the same
situation; the two disagreed, and only the link was wrong.

That matters because wrapping a display component in an actionable container is
the composition pattern `Card.mil` documents for interaction, so following the
documented advice produced an empty link. HTML5 permits flow content inside
`<a>`, so there was nothing to work around.

An explicit `label:` still wins, matching `HostButton`: a control cannot show
both, and the authored label is the more specific instruction.

### Added - native browser HostSlider

`HostSlider` now lowers to `<input type="range">`, preserving literal and
slot-backed bounds, value, disabled state, accessible label, and hydration
markers for continuous change and commit events.

### Fixed - lower compatibility Input nodes

The pipeline HTML emitter now lowers the legacy `Input` surface, including its
multiline textarea shape, value and placeholder bindings, length/read-only
attributes, and event hydration markers.

### Added - static UI49 one-of slot-state snapshots (#14368)

The three-file HTML pipeline can now bake explicitly supplied `one-of` slot
values into their model-owned mosstyle states. Missing or invalid values retain
the base style, multiple axes compose in `.mil` declaration order, and built-in
interaction/structural states remain unselected in static output. Package and
standalone-project snapshots use the same first-member samples already written
to generated fallback props.

### Fixed - preserve HostInput accessible names (#13717)

Static HTML now emits literal and slot-backed `HostInput.a11y-label` values as
escaped `aria-label` attributes.

### Security - validate literal HostLink.href's URI scheme (#13052)

Follow-up to #12038 (the identical XAML gap). A literal `href` was spliced
into a real `<a href="...">` with only HTML-attribute escaping — no scheme
check — a real XSS vector (`href: "javascript:..."`) reachable from a
third-party layout, since layout/style source is a trust boundary.

Added `has_disallowed_uri_scheme` (new `PipelineEmitError::UnsafeUriScheme`
variant, `emit_host_link` now returns `Result`) rejecting a literal `href`
at compile time when it carries an explicit, disallowed scheme. A relative
reference with no scheme at all (`"#"`, `"/about"`) is unaffected — the
common in-app-routing shape — matching every other backend's #13052 fix.

A slot-bound `href` is left unvalidated here: this backend emits a
`{{slot}}` mustache placeholder substituted by a separate host/preview
template engine outside this crate (see the module doc comment), so a
runtime guard for a dynamic href would need to live in that substitution
step, not this compiler. Out of scope for this fix.

### Fixed - expression drag keys are no longer silently dropped

`append_drag_value` matched only a string literal or a slot ref, so once a layout used
an expression key (`drop-key: ( col[1] )`, as a board must) this backend emitted the
`<div>`s with **no** `data-mosaic-drop-key` / `data-mosaic-drag-key` at all. The runtime
then found no targets and nothing was draggable — an inert board with no diagnostic.

An expression is now rendered as a mustache over the same binding, with index access
lowered to the dotted path the template engine resolves (`col[1]` → `{{col.1}}`).
Deliberately narrow: anything with an operator in it is not a path, and is passed
through to fail visibly rather than resolve to something wrong.


### Fixed - project-shell node-slot hydration

The generated `main.js` runtime now evaluates triple-mustache node markers
before escaped text mustaches. Host-owned `HostSurface` markup therefore mounts
inside the stable generated container instead of leaving a malformed brace
sequence or escaping the content as text.

### Added - host-owned surface composition

`HostSurface ( content: slot: ... )` now emits a stable DOM mount point with a
trusted node-slot template marker for host-provided browser content.

### Added - UI35 drag-and-drop (`HostDraggable` / `HostDropTarget`)

The HTML backend now lowers the kernel's two drag primitives (see
`code/specs/UI35-host-drag-drop.md`), following React as the reference
implementation but keeping this backend's own discipline.

**The markup half is pure markup.** Author-supplied values never reach
JavaScript source — they go into `data-*` attributes through
`escape_html_attr`, and the runtime reads them back via `element.dataset`.
So unlike `HostDialog`, the drag primitives emit **no inline script at all**,
and a drag card costs zero bytes of script per node. `tabindex`, `role`, and
`aria-roledescription` are emitted as markup so the card is announced correctly
even before the runtime loads.

**The behaviour half lives in the emitted `main.js` runtime**, as delegated
listeners on the component root — matching how every other event in this backend
is wired (`data-on-*` marker + delegation), rather than inventing a second
mechanism.

Contracts, all three proven by tests:

