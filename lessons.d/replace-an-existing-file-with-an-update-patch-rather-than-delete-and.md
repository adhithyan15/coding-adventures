---
category: Repo policy / workflow reminders
---

# Replace an existing file with an Update patch rather than Delete and Add in one patch

I tried to replace a spec using Delete and Add operations for the same path
in one apply_patch call. The validator rejected the duplicate target before
mutation; later commands in that cell did not run. Use Update for an existing
file or a saved, preimage-checked replacement. A patch rejection is not a test.
