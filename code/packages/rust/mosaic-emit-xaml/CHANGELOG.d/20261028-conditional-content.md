# Conditional XAML subtree realization

- Keep outer If/Else layout wrappers eager, but defer their content with named x:Load Borders so inactive editors do not process owner text changes.
- Preserve truthiness and inverted branches; reserve component, slot and authored-part names before allocating generated deferred fields.
- Document the changed control lifecycle: model values persist, inactive control-local state does not. Library/integration tests and preliminary WinUI evidence pass; broader lifecycle and performance acceptance remain pending in the draft experiment.