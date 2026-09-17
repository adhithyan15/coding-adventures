---
category: Cross-platform & Windows BUILD_windows
---

# Generated helper classes must fully qualify framework types

**What went wrong.** UI86's XAML lowering (#15463) appends a helper class to a
component's code-behind:

```csharp
public sealed class FooMosaicSelectableButton : Button
```

A single-component fixture compiled with `dotnet build`, and CI's TaskApp lane
passed. Building the **whole toolkit** did not: the toolkit exports a component
named `Button`, so the generated `public sealed partial class Button :
UserControl` lands in the same namespace, and the unqualified base type binds to
*that*:

```
error CS0509: 'TabsMosaicSelectableButton': cannot derive from sealed type 'Button'
error CS0115: 'TabsMosaicSelectableButton.OnCreateAutomationPeer()': no suitable method found to override
```

Three components failed at once (SegmentedControl, Tabs, ListGroup), because a
package compiles every component into one project.

**The fix.** Every framework type in the generated class is written out in full:
`Microsoft.UI.Xaml.Controls.Button`, `Microsoft.UI.Xaml.DependencyProperty`,
`Microsoft.UI.Xaml.Automation.Peers.ButtonAutomationPeer`, and so on. A test
asserts the code-behind contains no bare `: Button`, `(DependencyObject `, or
` ButtonAutomationPeer,`.

**What to do differently.**
- **A generated class shares its namespace with generated components.** Anything
  the DSL lets an author name — `Button`, `Slider`, `Grid`, `Page` — can shadow
  the framework type of that name. Qualify, or the collision waits for the
  first package that uses the name.
- **A one-component fixture does not exercise this.** When adding generated
  helper code, compile a *package* with several components, ideally the toolkit,
  which is the widest set of names in the repo.
- The same hazard sits unfixed in the older `MosaicSlider : Slider` and
  `MosaicDragSource : ContentControl` helpers; they survive only because no
  package exports a component with those names yet.
