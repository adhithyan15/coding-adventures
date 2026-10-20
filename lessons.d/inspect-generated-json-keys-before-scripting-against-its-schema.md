---
category: Repo policy / workflow reminders
---

# Inspect generated JSON keys before scripting against its schema

A one-off planner summary assumed the queue lived at `items` and called
`p.items.slice(...)`; the planner's actual top-level key is `head`, so the
measurement command failed after successfully producing valid JSON.

Before writing an ad hoc extractor, parse the artifact and print its top-level
keys (or read the typed producer). Then use the confirmed schema. This keeps a
consumer typo from being confused with a failed measurement.
