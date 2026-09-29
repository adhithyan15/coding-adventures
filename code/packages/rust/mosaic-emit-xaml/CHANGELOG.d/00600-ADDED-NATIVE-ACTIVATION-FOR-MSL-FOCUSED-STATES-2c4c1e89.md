### Added - Native activation for MSL focused states

WinUI output now connects UI15's built-in `state focused` blocks on native
focus-capable Host controls to `Control.FocusState` through a generated
`IValueConverter`. Pointer, keyboard, and programmatic focus activate the
shared MSL properties and transitions; DataTemplate instances remain
row-local, and explicit `state-when-focused` predicates remain
author-controlled. A Task App acceptance gate proves its Mosaic-authored
project-composer focus ring reaches the generated TextBox without handwritten
Windows UI.

