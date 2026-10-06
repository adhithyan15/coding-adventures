---
category: Mosaic compiler pipeline
---

# XAML template state triggers need native launch validation

A generated WinUI app compiled successfully and emitter tests passed, but direct
x:Bind on StateTrigger.IsActive inside an ItemsRepeater template crashed during
ProcessBindings: the generated setter received a null StateTrigger. Replacing
that binding with a row DataContext binding stopped the crash but did not paint
selection. A template UserControl with a first-child Grid, collapsed named Border
proxies holding compiled predicates in Tag, and ElementName bindings from state
triggers to those proxies rendered the selected cell. Keep compiled bindings on
ordinary template elements and verify the launched app; markup compilation alone
does not prove visual-state initialization or activation.
