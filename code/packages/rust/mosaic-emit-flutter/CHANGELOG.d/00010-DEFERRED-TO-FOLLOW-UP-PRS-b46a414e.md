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
