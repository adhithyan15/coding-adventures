---
category: Mosaic compiler pipeline
---

# Deferred XAML branches need a structural wrapper without another content control

An initial conditional-realization experiment nested ContentControl inside the existing ContentControl. Four branch-structure tests rejected the doubled controls. Keep the eager outer wrapper for layout and template-root legality; a named Border can own the deferred subtree without adding another content-control template. Run the conditional fixtures immediately, then compile and exercise actual WinUI output: source structure alone does not establish lifecycle or performance behavior.