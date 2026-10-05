# XAML structural table header widths

For structural HostTable output, a single repeated Col in HostTableColGroup supplies the same per-column pixel widths to the corresponding repeated header cells. Scope this mapping to the table header at its entry loop depth; unrelated and nested loops retain their current behavior. Fixed leading row-label columns retain their authored widths.

Generate a Width field and binding for the header item projection, using the colgroup's declared width slot, and invalidate that projection when the width slot changes. Do not infer header widths from application or slot names. Tables with ambiguous multiple repeated colgroups retain existing behavior.

Validation: generic emitter regressions for header width source and invalidation, unchanged unrelated loops, full XAML emitter tests, generated VisiCalc WinUI build and actual screenshot review. Contrast, selection and formula-field layout remain separate failures under #14274.
