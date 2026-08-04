# UI36 — data-driven sizing

**Status:** implemented on the React backend; the other seven backends are follow-on.
**Kernel surface:** the size props `width`, `height`, `min-width`, `max-width`,
`min-height`, `max-height` on any layout node.

---

## 1. The gap

Mosaic splits appearance cleanly: **moslayout** says what the tree is, **mosstyle**
says what it looks like, and mosstyle deliberately bakes its values at compile time.
That split is right for almost everything — and it has one hole.

Some sizes are only knowable at run time:

- the length of a **Gantt bar** — it depends on when the engine scheduled the task;
- the fill of a **progress meter**;
- a **proportional column** whose share comes from data;
- any bar, sparkline, or gauge.

No amount of authored CSS can express *"as wide as this row's data says"*. Before
UI36 there was simply no way to size such a node — and worse, the obvious attempt
**failed silently**:

```mll
Box [ bar ] ( width : slot: bar-width )   // parsed fine, then dropped on the floor
```

The only reader was `find_number_prop`, which matches a literal `Number` and nothing
else, so a slot or expression binding was discarded without a word. An author had no
way to tell a working binding from an ignored one.

> **This is the spec's motivating principle.** A binding that is accepted by the
> grammar must either take effect or be reported. Silently ignoring it is the worst of
> the three options, because it teaches the author that the feature doesn't exist.

## 2. The surface

A size prop accepts the same value shapes the rest of the emitter already understands:

| author writes | emitted (React) | meaning |
|---|---|---|
| `width: 120` | `width: 120` | CSS pixels (React reads a bare number as px) |
| `width: "50%"` | `width: "50%"` | any CSS length, as a string |
| `width: auto` | `width: "auto"` | a CSS keyword |
| `width: slot: bar-width` | `width: barWidth` | bound to a slot |
| `width: ( row[2] )` | `width: row[2]` | an expression — e.g. a `For` binding |

The host may therefore supply a number *or* a string, which is what makes a bar that
scales with its container (`"42%"`) expressible at all.

Anything that is not a size — an emit ref, say — is a **hard error**, not a silent
drop. A non-finite numeric literal is likewise rejected rather than emitted as a bare
`inf` identifier that wouldn't compile.

## 3. Precedence

A bound size is **data**, and data outranks decoration. The emitted style object
therefore orders values:

```
  base part style  →  state-block spreads  →  bound size
```

so a bound width beats both a static `part bar { width: … }` and a
`state hover { width: … }`. Getting this backwards would reintroduce the original
complaint in a subtler form: the binding would appear to do nothing whenever a
stylesheet happened to mention the same property.

> **Requirement.** A bound size must be emitted last. A backend whose style model
> can't express that ordering must reject the combination rather than quietly let the
> stylesheet win.

## 4. What this does *not* change

- **A layout that binds no size emits byte-identical output.** UI36 is purely
  additive; every existing Mosaic app is unaffected. This is pinned by a test.
- **mosstyle is still the home for static appearance.** UI36 is not an invitation to
  move styling into the layout — it covers the values a stylesheet *cannot* know.

## 5. Backend status

| backend | status |
|---|---|
| react | **implemented** (11 tests) |
| html, swiftui, compose, qt, flutter, xaml, webcomponent | **not yet** — a size prop is currently ignored there, which is the very bug this spec exists to close |

Until a backend implements UI36, a layout relying on it renders unsized on that host.
`code/programs/mosaic/task-app`'s timeline is the first consumer and is web-only today
for exactly this reason.

> **Requirement for each remaining backend.** Accept all five value shapes; emit the
> bound size last (§3); and fail loudly on a non-size value.

## 6. Known limitation — expression text is verbatim

An `Expr` value is interpolated into the emitted source verbatim, exactly as it already
is for `content:`, `label:`, `If ( when: … )`, and `For ( each: … )`. A `.mll` is
first-party source under the same trust model as the rest of the repo's code.

This is worth stating plainly because UI36 *widens* that surface — six props on every
node, rather than a handful of specific ones. It also compounds a pre-existing
moslayout defect: `reconstruct_expr_text` re-emits a STRING token's **inner text
without its quotes**, so `( name == "done" )` currently reconstructs as
`name == done` — a correctness bug in its own right, and the mechanism by which an
expression could contain a `}`.

> **Follow-on (tracked separately).** Fix `reconstruct_expr_text` to re-quote and
> escape STRING tokens. That repairs string comparisons inside expressions *and*
> removes the only route by which expression text can carry structural characters. It
> belongs in moslayout, not in any one backend.
