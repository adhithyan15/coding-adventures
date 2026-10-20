### Added — a multiline TextBox commits on Ctrl+Enter (UI25 §4)

- An `AcceptsReturn` `TextBox` with `onCommit` gets the Enter branch back in
  its `PreviewKeyDown` handler, now requiring Control down
  (`InputKeyboardSource.GetKeyStateForCurrentThread(VirtualKey.Control)`) and
  setting `e.Handled = true`, so Ctrl+Enter commits without a line break and
  bare Enter stays a line break. Not compiled or run natively: WinUI needs
  Windows.
