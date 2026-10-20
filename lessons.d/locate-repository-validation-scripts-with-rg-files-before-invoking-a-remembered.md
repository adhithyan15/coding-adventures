---
category: Repo policy / workflow reminders
---

# Locate repository validation scripts with rg --files before invoking a remembered path, because validator locations can differ from their descriptive names.

A validation run guessed `code/scripts/check_doc_shards.py`, but that path does not exist in this checkout, so Python failed before checking anything. Search the repository file inventory for the validator's actual name and location, then invoke the discovered path. This keeps remembered command names from turning into noisy false starts.
