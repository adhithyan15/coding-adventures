---
category: TypeScript / JavaScript
---

# `grid-template-columns` has no effect until the element is a grid container

A new NN28 workbench inherited spacing from `.workspace` but not `display: grid`; computed columns looked correct in devtools while the stage and sidebar still stacked at full width. Declare `display: grid` on each standalone workspace variant and assert both computed display and element rectangles during desktop browser QA.
