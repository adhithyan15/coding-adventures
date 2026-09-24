---
category: Mosaic compiler pipeline
---

# A Mosaic layout expression comparing a kebab-case slot compiles to subtraction in Swift and Kotlin

In C3c (#14018), TaskApp's outline used
`If ( when: ( item[0] == selected-outline-key ) )` to pick the selected button.
It compiled through every Mosaic stage and the package tests passed. But the
SwiftUI and Compose emitters pass expression text through verbatim, so the
generated Swift and Kotlin read `selected - outline - key`:
"cannot find 'selected' in scope" and "Unresolved reference 'outline'".
(Inside an inlined package component the resolver rewrites the identifiers,
which is why RecordList's `row[0] == selected-key` works there.)

**Fix:** have the app send a truthy marker field (`item[5]` = "1" for the
selected row), the convention ChecklistRun and RecordList rows already follow.

**Do differently:**
- In a host component's own layout, never compare a kebab-case slot inside an
  expression. Carry the answer as a marker field in the row.
- Compile the emitted SwiftUI (`swift build -c release`) and Compose
  (`gradle compileKotlin`) projects locally before pushing a layout change.
  The package tests don't compile generated code.
