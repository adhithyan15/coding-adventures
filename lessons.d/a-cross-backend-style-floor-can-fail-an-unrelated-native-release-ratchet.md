---
category: Mosaic compiler pipeline
---

# A cross-backend style floor can fail an unrelated native release ratchet

TaskApp #18031 added min-width for Qt geometry; source compilation and every
backend package build passed, but SwiftUI release validation rejected a new
third min-width drop against its budget of two. Package construction does not
run the release style-drop ratchet. Preserve the existing authored styles and
repair Qt's wrapper using their existing width/non-shrink intent instead of
adding a cross-backend declaration or raising the budget. Check exact release
payload validation even after local package tests pass.
