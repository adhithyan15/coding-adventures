### Added — `HostDialog` lowering (UI29-1 §3.6)

- `HostDialog` lowers to WinUI 3's `ContentDialog` (modal: true,
  the default) or `Flyout` (modal: false). Both are platform-level
  top-layer primitives — they provide modal blocking / focus
  trap / dismiss handling out of the box (per UI29-1 §1 these
  cannot be composed from `<Border>`/`<Grid>`).
- `modal: true` (keyword default) → `<ContentDialog>`.
- `modal: false` (keyword) → `<Flyout>` (popover form).
- `title: slot: x` → `Title="{Binding X}"` (matches the spec
  §3.6 sketch's Binding form so the host's DataContext drives
  the title text).
- `title: "literal"` → `Title="literal"` (XAML-escaped).
- `open: slot: x` → `mos:Dialog.IsOpen="{Binding X}"` plus a
  documented stub comment naming the binding so the host's
  code-behind can wire `ShowAsync()` / `Hide()` against the slot.
- `onClose: emit: onX` → `Closed="OnHostDialogClose_N"` plus a
  generated private `void OnHostDialogClose_N(object, object)`
  in the code-behind that dispatches the named emit case
  (matches the HostButton.Click handler pattern).
- `dismiss-on-backdrop: false` → comment stub (XAML's
  ContentDialog has no boolean equivalent — only the
  `LightDismissOverlayMode` enum on Flyout / `IsLightDismissEnabled`
  on a few other controls). Documented in the emitted XAML so the
  gap is visible in diffs.

