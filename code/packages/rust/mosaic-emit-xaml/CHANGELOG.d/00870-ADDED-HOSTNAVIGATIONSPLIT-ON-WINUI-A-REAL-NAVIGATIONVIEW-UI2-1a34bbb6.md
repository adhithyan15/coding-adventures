### Added — `HostNavigationSplit` on WinUI: a real `NavigationView` (UI29-6, #15481)

`HostNavigationSplit [ shell ] ( pane-title : … )` with its pane and detail
children now lowers to

```xml
<NavigationView x:Name="Shell" AutomationProperties.AutomationId="shell"
                PaneTitle="Projects" PaneDisplayMode="Auto" OpenPaneLength="236"
                IsSettingsVisible="False" IsBackButtonVisible="Collapsed">
  <NavigationView.PaneCustomContent>
    …the pane…
  </NavigationView.PaneCustomContent>
  …the detail…
</NavigationView>
```

**Why the control and not two `Column`s.** They draw the same pixels at desktop
width. What they cannot do is fold the pane into a flyout as the window
narrows: that happens inside `NavigationView`'s own layout pass, UI48 §5.6 rules
out routing it through an event, and UI30's variant selection is build-time. So
composition has no way to reach it at all.

| prop | lowers to |
| --- | --- |
| `pane-title` | `PaneTitle` — drawn in the pane header, and the pane's UIA name. Literal, slot, `For` binding and expression all supported; a slot binds `OneWay`. |
| `pane-width` | `OpenPaneLength`, the *preferred* width — WinUI keeps its own compact and flyout widths when the pane narrows or folds. |
| `collapse : auto` (default) | `PaneDisplayMode="Auto"`, WinUI's adaptive ladder. |
| `collapse : never` | `PaneDisplayMode="Left"` **and** `IsPaneToggleButtonVisible="False"`, so the toggle cannot undo by hand what the author pinned. |

**Choices worth knowing.** The pane goes in `PaneCustomContent`, not
`MenuItems`: the kernel hands the emitter an arbitrary subtree, and `MenuItems`
would force every child through `NavigationViewItem` and discard whatever is
not a navigation entry. `IsSettingsVisible="False"` and
`IsBackButtonVisible="Collapsed"` switch off the two affordances
`NavigationView` adds on its own and that no Mosaic prop drives — leaving them
on would put a settings row and a back arrow wired to nothing in every
product's shell. An unknown `collapse` keyword is refused rather than read as
`auto`: a typo would otherwise fold away the pane in exactly the product that
asked it not to.

Checked against the real toolchain, not just the emitter tests:
`fixtures/host-navigation-split` nests an adaptive split inside a pinned one
(both collapse modes, both title shapes, and a `NavigationView` inside another
`NavigationView`'s content), generates native-complete with zero degradations,
and is `dotnet build`t in CI. The runtime proof — resizing a window and
watching the pane actually collapse — needs an app with an engine behind it and
lands with TaskApp in the consumer slice: #15486.

