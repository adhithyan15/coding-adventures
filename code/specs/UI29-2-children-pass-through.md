# UI29-2 — Children pass-through for component references

**Status:** Implementation in progress. Default anonymous child blocks now
compile, validate, and expand through package references on all five native
backends. Named regions and standalone exported-component child parameters are
still pending.
**Layer:** UI / cross-cutting (moslayout grammar + every backend emitter +
package convention)
**Depends on:** UI29 (primitive kernel + userland component packages),
UI29-1 (HostDialog).
**Unblocks:** Card, Container, Field in `mosaic-pkg-toolkit`; Grid's
forthcoming "section template" parameterisation in `mosaic-pkg-grid`;
any future toolkit/userland component that wants `<X> ... </X>` inline-
children semantics.

---

## 1. Why this exists

UI29's kernel composes from leaf primitives + container primitives
(Box, Row, Column, Stack). Children of those containers are written
inline in `.mll`:

```moslayout
Box [outer] {
  Text (content: "hi")
  Row {
    Button (...)
  }
}
```

Component *references* — references to userland components — do not
currently accept inline children. Today's grammar only allows
attribute-style props:

```moslayout
Grid (rows: slot: data, headers: slot: cols)
```

That's fine for components whose surface is entirely slot-driven
(Grid, the toolkit's Button, Alert, Spinner, Modal-with-text-only).
It breaks down for components that *wrap* arbitrary content:

```moslayout
Container {
  Card {
    Text (content: "Title")
    Text (content: "Body")
  }
}
```

`Container` has no idea what `Card { ... }` is. Card has no idea
what `Text (...)` children are. Both want to receive arbitrary
layout nodes and place them at a specific spot in their .mll.

Today the workaround is to push everything into typed slots:

```moslayout
component Card {
  slot title : text ;
  slot body  : text ;
  slot footer : text ;
}
```

…which works for simple cases but doesn't compose richer content
(Card with a Button in the footer, an Alert in the body, etc.).
**This spec adds children pass-through so userland components can
accept arbitrary inline children.**

---

## 2. The minimum surface

Two additions:

### 2.1 New slot type: `node` and `list<node>`

`node` already exists in mosmodel's grammar but is only documented
as the type a host passes when injecting an HTML node / SwiftUI
View at runtime. UI29-2 extends its meaning to *also* cover "the
inline children block of a component reference."

The slot-type table grows by one row:

| Slot type     | What it carries                                  |
|---------------|---------------------------------------------------|
| `text`        | string                                            |
| `number`      | f64                                               |
| `bool`        | bool                                              |
| `color`       | platform color                                    |
| `image`       | platform image                                    |
| **`node`**    | A single child layout subtree (existing — extended) |
| **`list<node>`** | An ordered list of child subtrees (new)        |
| `list<T>`     | List of any of the above                          |

### 2.2 Inline children in component references

Today's grammar:

```ebnf
component_ref ::= NAME ('[' PART ']')? ('(' props? ')')?
```

Becomes:

```ebnf
component_ref ::= NAME ('[' PART ']')? ('(' props? ')')? ('{' node* '}')?
```

The trailing `{ ... }` block lists the children to pass into the
referenced component. **At most one** `node`-typed slot per
component receives the children — by convention named `children`
(the analyzer enforces both rules at compile time).

Example:

```moslayout
component Card {
  slot title    : text ;
  slot children : node ;     // ← new
}

layout Card {
  Column [card-root] {
    Box [card-title] {
      Text (content: slot: title)
    }
    Box [card-body] {
      slot: children          // ← the inline children land here
    }
  }
}
```

Caller:

```moslayout
Card (title: "Hello") {
  Text (content: "First paragraph")
  Row {
    Button (label: "OK", onClick: emit: onConfirm)
  }
}
```

The two child nodes inside `Card { ... }` slot in at the
`slot: children` reference inside `Card.mll`. The component's
*structure* (title + body wrapper, padding, borders) is in the .mll;
the *content* of the body is the caller's choice.

### 2.3 Multiple named children slots

When a component needs more than one inline-children region (e.g.
Card with header + body + footer), each region is a separate
`node` or `list<node>` slot, and the caller uses **named child
blocks**:

```moslayout
component Card {
  slot header : node ;
  slot body   : list<node> ;
  slot footer : node ;
}

layout Card {
  Column {
    Box [card-header] { slot: header }
    Box [card-body]   { slot: body }
    Box [card-footer] { slot: footer }
  }
}
```

Caller:

```moslayout
Card {
  header { Text (content: "My Card") }
  body {
    Text (content: "First paragraph.")
    Text (content: "Second paragraph.")
  }
  footer {
    Button (label: "OK")
  }
}
```

The grammar extension for named blocks:

```ebnf
component_ref ::= NAME ('[' PART ']')? ('(' props? ')')? ('{' (named_block | node)* '}')?
named_block   ::= NAME '{' node* '}'
```

When the caller uses *only* a bare `{ node* }` block (no named
blocks), the analyzer routes the nodes to the lone `node` /
`list<node>` slot the component declares (or errors if multiple
exist). When *any* named block appears, every region must be a
named block (no mixing).

---

## 3. Lowering per backend

Each backend already has an idiomatic way to pass children:

| Backend           | Idiomatic shape                                     |
|-------------------|-----------------------------------------------------|
| React             | `<Card title="...">{children}</Card>` — JSX children prop, or named slots via render-prop fields |
| SwiftUI           | `Card(title: "...") { children }` — view-builder closure parameter |
| Qt / QML          | `Card { title: "..." ; Text { } ; Text { } }` — children declared inline |
| HTML              | `<x-card title="..."><div></div></x-card>` — child elements |
| WebComponent      | `<slot></slot>` — Custom Elements named slots |
| XAML / WinUI 3    | `<gen:Card Title="..."><Card.Body>...</Card.Body></gen:Card>` — content properties + named property elements |

Per-backend lowering notes:

### 3.1 React

The component-ref's children block lowers to JSX children. For
`slot: children` (single node), the children are passed unwrapped:
`<MyCard>{nodes}</MyCard>`. For `slot: body : list<node>`, the
component exposes a `body` prop typed `React.ReactNode[]` and the
caller's named block becomes a JSX expression array.

### 3.2 SwiftUI

Single `node` slot → view-builder closure (`Card(title:) { children }`).
Named `list<node>` slots → multiple labelled view-builder
parameters via the `@ViewBuilder` attribute on each.

### 3.3 Qt / QML

Children inline. The `Card.qml` file declares the children container
via `default property list<Item> children` (or named container
properties for named slots).

### 3.4 HTML (static) + WebComponent

`<slot name="children"></slot>` (Custom Elements Spec) for the
default slot. Named children → `<slot name="body">`,
`<slot name="footer">`, etc.

### 3.5 XAML

Single `node` slot → `[ContentProperty("Children")]` attribute on
the partial class + a `Children` property of type
`UIElementCollection`. The caller's inline children become the
`<gen:Card>...</gen:Card>` element's content tree.

Multiple named slots → named property elements:
`<gen:Card><gen:Card.Header>...</gen:Card.Header><gen:Card.Body>...</gen:Card.Body></gen:Card>`.

The XAML emitter (`mosaic-emit-xaml`) needs:
1. When a component's `.mil` declares any `node` / `list<node>`
   slot, emit `[ContentProperty("ChildSlotName")]` on the
   partial class.
2. The XAML component-reference emitter (PR-5's
   `emit_component_reference`) extends to walk the inline
   children block, lowering each child node and emitting them as
   the content tree (single slot) or as named property elements
   (multiple slots).
3. The DependencyProperty for a `node`-typed slot has C# type
   `Microsoft.UI.Xaml.UIElement` (already the case today).
   `list<node>` becomes `IReadOnlyList<UIElement>`.

---

## 4. Validation rules

The moslayout analyzer enforces:

1. **At most one default-children slot** per component. A component
   with two `node` slots and no caller-side named blocks is an
   error — the analyzer can't decide which slot to route to. Mix
   `node` + `node` only when callers use named blocks.

2. **Caller blocks must match declared slots.** A named block
   `header { ... }` against a component that has no `header` slot
   errors. A caller-side bare `{ ... }` block against a component
   with only named slots also errors.

3. **No cycles.** A component referencing itself in its inline
   children block is forbidden (caught by the resolver, not the
   analyzer — same as the existing component-reference resolution
   pass).

4. **Backend feasibility check.** Each backend's lowering of
   inline children has constraints (e.g. XAML's `ContentProperty`
   only supports one default slot). The analyzer warns when a
   component uses multiple `node` slots — the backend emitter
   handles them via named property elements, which works but
   makes the caller's XAML more verbose.

---

## 5. Migration path

### 5.1 mosaic-pkg-card

The existing `mosaic-pkg-card` v0.1 ships fixed `title`/`body`/`footer`
text slots — a workable but limited shape. After UI29-2 lands, a
follow-up PR can publish `mosaic-pkg-card` v0.2 with named
`header` / `body` / `footer` `node` slots. The old text-typed slots
stay alongside under deprecated names so existing consumers don't
break; a v0.3 removes them after a deprecation window.

### 5.2 mosaic-pkg-toolkit

The toolkit v0.1 deferred Card + Container + Field for exactly
this spec. After UI29-2 lands:

| Component | Children-slot story |
|---|---|
| `Card`     | `slot header : node`, `slot body : list<node>`, `slot footer : node` |
| `Container` | `slot children : list<node>` (the default block) |
| `Field`    | `slot children : node` (typically the input), plus the existing label/help/error text slots |

These three lands as a single toolkit PR after the spec lands.

### 5.3 mosaic-pkg-grid

Grid's `Cell` is the textbook case for parameterised templates —
the row's editor cell shape might be a HostInput, a Select, a Date
picker, etc. Today Grid hard-codes `HostInput`. After UI29-2,
Grid can accept a `slot: row-template : node` and let the caller
supply any cell shape. That's a future Grid v0.3 — not in scope
for this spec PR, but enabled by it.

---

## 6. Implementation roadmap

| ID | Work | Depends on |
|---|---|---|
| **UI29-2-0** | This spec | — |
| **UI29-2-G1** | **Done:** generic `{ node* }` component-ref blocks plus typed `slot: children` mounts | UI29-2-0 |
| **UI29-2-G2** | moslayout grammar: `named_block` syntax | UI29-2-G1 |
| **UI29-2-A1** | **Done:** default mount type/uniqueness validation and resolver splicing | UI29-2-G1 |
| **UI29-2-A2** | Named-block analyzer validations (§4) | UI29-2-G2 |
| **UI29-2-K-react** | mosaic-emit-react lowers standalone child parameters to JSX | UI29-2-A1 |
| **UI29-2-K-swiftui** | mosaic-emit-swiftui lowers to view-builder closures | UI29-2-A1 |
| **UI29-2-K-qt** | mosaic-emit-qt lowers to default-property children | UI29-2-A1 |
| **UI29-2-K-webcomp** | mosaic-emit-webcomponent lowers to `<slot>` | UI29-2-A1 |
| **UI29-2-K-html** | mosaic-emit-html lowers to inline child elements | UI29-2-A1 |
| **UI29-2-K-xaml** | mosaic-emit-xaml: ContentProperty attribute + child-node emission in `emit_component_reference` | UI29-2-A1 |
| **UI29-2-P1** | mosaic-pkg-toolkit v0.1.x: Card + Container + Field land | All K-* |

The K-* family is fully parallel after UI29-2-A1. The K-xaml piece
needs the most thought because it builds on PR-5's `ComponentRegistry`
work — see §3.5 for the additions.

---

## 7. Relationship to existing specs

- **UI29 §2.1 kernel** is unchanged. The kernel primitives don't
  receive new inline-children semantics; they already have child
  blocks. Component references — references to userland packages
  — are the only thing that grows.
- **UI29 §3 grammar** grows two productions (`{ node* }` and
  `named_block`). Otherwise unchanged.
- **UI29 §4 packages** — `[components].exports` and the manifest
  shape are unchanged. The .mil's slot table grows to include
  `node` and `list<node>` types (was already permitted; this spec
  just documents the inline-children interpretation).
- **mosaic-emit-xaml** §5 (HostTable) and §11 (component references)
  both extend per §3.5 here. The existing tests stay valid;
  new tests cover the inline-children path.
- **mosaic-pkg-toolkit** spec §6.1 (the `Container`/`Row`/`Col`
  naming collision section) is partly addressed — UI29-2 unblocks
  the Container component but doesn't yet add a Col with span
  props. Spec §6.1's responsive question stays open.

---

## 8. Open questions resolved in implementation PRs

1. **Default-children-slot name.** Convention is `children`, but
   nothing forces it. The analyzer should accept any `node`-typed
   slot when there's exactly one. (Recommendation: bless `children`
   as the convention via warnings if a different name is used.)
2. **Multi-slot ordering.** When the caller's named blocks
   appear in a different order than the .mil's slot declaration,
   does the emitter respect the .mll's slot-use order or the
   caller's text order? (Recommendation: .mll order. Caller-side
   ordering is a layout concern, but the .mll is the layout
   authority.)
3. **Whitespace-only children.** A caller writing `Card { }` (an
   empty block) — does the children slot bind to an empty list or
   error? (Recommendation: empty list. The block delimits the
   region, doesn't require content.)
4. **Self-closing form.** Should `Card (...);` (no block) and
   `Card (...) { }` (empty block) be equivalent? (Recommendation:
   yes; both bind any `node`/`list<node>` slot to an empty
   collection.)
5. **Backend-specific limits.** XAML's `[ContentProperty]` is
   single-only. Should the analyzer warn when multiple `node`
   slots are declared *and* the XAML backend is a target?
   (Recommendation: warn at `mosaic-compile --backend xaml` time,
   not at moslayout analysis time — keeps the front-end
   backend-agnostic.)

These all become PR-time decisions in the K-* family.

---

## 9. What this spec does NOT do

- **Doesn't allow children to be slot-bound through depth.** A
  component can pass-through its own children to another component
  it references in its .mll, but not via a "render prop" /
  function-children pattern. If `A { children }` and A's .mll
  references B with `B { slot: children }`, the children bubble
  through unchanged. Render-prop semantics (parameterising the
  child renderer with row data) are a separate UI29-3 spec.
- **Doesn't add slot-bound dynamic component lookup.** A slot
  whose value is a component-type-by-name (`slot: render-as :
  component`) is also a UI29-3 concern.
- **Doesn't address responsive layout.** UI29-2 just adds inline
  children; the responsive-grid question (toolkit spec §6.2) stays
  with its own spec.
- **Doesn't redesign the existing kernel container syntax.** `Box {
  ... }`, `Row { ... }`, `Column { ... }`, `Stack { ... }` work
  exactly as they do today. UI29-2 grows the *component reference*
  syntax to look like the kernel container syntax, but the kernel
  containers themselves don't change.
