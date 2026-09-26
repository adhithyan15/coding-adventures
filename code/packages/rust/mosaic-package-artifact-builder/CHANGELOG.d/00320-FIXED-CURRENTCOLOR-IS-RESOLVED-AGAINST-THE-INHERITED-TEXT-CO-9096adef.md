### Fixed -- `currentColor` is resolved against the inherited text colour (#15169)

`currentColor` means "whatever `color` is in effect here". CSS resolves it
natively, so the html and react backends were always correct. **No native
backend has an equivalent** -- a brush must be an actual colour -- and each
failed differently, and silently.

Trestle's pill status dot is authored `background: currentColor` precisely so
it tracks its pill's text colour. Measured, per backend:

| backend | what the dot rendered |
| --- | --- |
| html, react | correct |
| compose, flutter, swiftui, xaml | nothing -- an invisible box |
| qt | a **white** square: `Rectangle.color` defaults to `#ffffff` (measured, not assumed) |

So the dot had not rendered on five of seven backends for as long as the part
has existed, and on the sixth it rendered the wrong colour.

Resolution happens once, where `ComposedComponent` is built -- the single
point both package builds and standalone pipeline builds pass through, which
is the reason that type exists. Doing it in each emitter would be eight
implementations of one cascade rule, which is how they drift.

**The pass COPIES a value between properties**, which moves it into sinks the
property it was written on never reaches. So only values positively
recognisable as a colour literal are eligible -- hex (3/4/6/8 digits) or a
bare alphabetic keyword. `rgb()` / `rgba()` are declined despite being valid
CSS: they contain commas, and react's `path_paint_jsx` recovers `background`
from a serialized style fragment by splitting on commas and writes it into an
unquoted `fill={...}`, so the default `$color-border` token
`rgba(255,255,255,0.12)` becomes `fill={"rgba(255}` and the generated
component stops compiling. That sink is a defect in its own right, reachable
without this pass; declining here means this pass cannot trigger it.

A part resolves against its **own** `color` when it declares one, falling
back to the inherited one only when it does not -- which is what CSS does.
Recording the inherited colour unconditionally pinned the *parent's* colour
on such a part, and meant a part whose own colour varies by state declined
only for its subtree, not for itself.

Duplicate declarations follow **last-wins**, matching every emitter:
`part p { color: #aaa; color: #bbb }` renders `#bbb`, so taking the first
pinned a colour that never renders.

**Ambiguity is left unresolved, not guessed.** These cases are deliberately
declined, each with a test:

- a part used under two different inherited colours (no literal serves both),
- a part with no `color` declared anywhere above it,
- an ancestor whose `color` **changes with state** -- CSS follows the state at
  runtime, so pinning the base colour would REGRESS html and react, the two
  backends this keyword already worked on. Such an ancestor **poisons its
  subtree** rather than being skipped: skipping it let the child resolve
  against the GRANDPARENT, a literal wrong in every state.
- a part **name** that is ambiguous across the merged style list.
  `merge_dependency_styles` concatenates dependency parts with the
  component's own without namespacing, so names collide routinely. Resolution
  is decided per NAME over every instance sharing it -- deciding per instance
  is last-write-wins, which let a same-named part from another package supply
  the colour and defeated the state-dependent guard on the instance that lost.

A part's own `color` applies to its subtree, not to itself, so a part with
both `color` and `background: currentColor` does not paint its background its
own text colour.


