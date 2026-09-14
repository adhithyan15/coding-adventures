---
category: TypeScript / JavaScript
---

# Testing Library button names include all descendant text unless an explicit accessible name is provided

A scenario button containing a title and explanatory `<span>` is named `"Title explanation"`, so an exact `{ name: "Title" }` query fails even though the visible title is correct. Use a deliberate `aria-label` when the compact name is part of the UI contract, or use a sufficiently specific regex when the description should remain in the accessible name. Caught while adding NN28 gradient-buffer scenario tests.
